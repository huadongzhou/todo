use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, Row};
use serde::Serialize;
use specta::Type;
use todo_contracts::{Todo, TodoStatus};

/// Client-side SQLite storage for todos.
///
/// The connection is opened once during `setup` and shared through Tauri
/// managed state. Opening is best effort: when the database cannot be created
/// the app keeps running with an unavailable store, every command reports an
/// error, and the view layer falls back to its browser storage path.
pub struct TodoDb {
    connection: Option<Mutex<Connection>>,
}

/// Revision of the `todos` table layout, stamped into `PRAGMA user_version`
/// when the database file is created.
///
/// This is the **schema version and nothing else** — not a counter of applied
/// migration steps. Two rules follow from that and are the whole contract for
/// later work:
///
/// * the value changes only when the column layout changes, and the build that
///   raises it must also carry every lower revision up to the new one (an
///   `ALTER TABLE` step keyed on the stored value);
/// * moving data around — draining the writes the view layer parked outside
///   SQLite, for instance — never touches it, because the layout is the same
///   before and after.
///
/// So `stored == SCHEMA_VERSION` means "this file matches the layout this build
/// compiles against", which is the only question the code ever asks.
const SCHEMA_VERSION: i64 = 1;

const CREATE_SCHEMA: &str = "CREATE TABLE IF NOT EXISTS todos (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    completed_at TEXT,
    due_date TEXT,
    reminder_at TEXT
)";

const SELECT_TODOS: &str = "SELECT id, title, status, created_at, completed_at, due_date, reminder_at
    FROM todos
    ORDER BY created_at DESC, id";

const UPSERT_TODO: &str =
    "INSERT INTO todos (id, title, status, created_at, completed_at, due_date, reminder_at)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
    ON CONFLICT(id) DO UPDATE SET
        title = excluded.title,
        status = excluded.status,
        created_at = excluded.created_at,
        completed_at = excluded.completed_at,
        due_date = excluded.due_date,
        reminder_at = excluded.reminder_at";

/// Outcome of replaying the writes the view layer parked outside SQLite.
///
/// Entries are applied one by one, so a row SQLite refuses cannot stop the
/// rest: accepted ids come back in `applied` and the caller drops them from its
/// journal, refused ones come back in `rejected` with the reason and the caller
/// keeps them — still shown to the user, retried on the next start.
#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PendingWriteReport {
    pub applied: Vec<String>,
    pub rejected: Vec<RejectedWrite>,
}

/// A single entry SQLite would not take, with the reason for diagnostics.
#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RejectedWrite {
    pub id: String,
    pub error: String,
}

#[derive(Debug)]
pub enum TodoDbError {
    /// The database could not be opened at startup.
    Unavailable,
    /// A previous command panicked while holding the connection lock.
    Poisoned,
    /// SQLite rejected the statement.
    Sqlite(String),
}

impl std::fmt::Display for TodoDbError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable => formatter.write_str("the local todo database is unavailable"),
            Self::Poisoned => formatter.write_str("the local todo database lock is poisoned"),
            Self::Sqlite(message) => write!(formatter, "local todo database error: {message}"),
        }
    }
}

impl std::error::Error for TodoDbError {}

impl From<rusqlite::Error> for TodoDbError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error.to_string())
    }
}

fn status_to_text(status: &TodoStatus) -> &'static str {
    match status {
        TodoStatus::Open => "open",
        TodoStatus::Completed => "completed",
    }
}

/// Unknown values degrade to `Open` so a corrupted row still shows up as an
/// actionable task instead of failing the whole read.
fn status_from_text(value: &str) -> TodoStatus {
    match value {
        "completed" => TodoStatus::Completed,
        _ => TodoStatus::Open,
    }
}

fn row_to_todo(row: &Row<'_>) -> Result<Todo, rusqlite::Error> {
    let status: String = row.get("status")?;
    Ok(Todo {
        id: row.get("id")?,
        title: row.get("title")?,
        status: status_from_text(&status),
        created_at: row.get("created_at")?,
        completed_at: row.get("completed_at")?,
        due_date: row.get("due_date")?,
        reminder_at: row.get("reminder_at")?,
    })
}

/// Creates the schema and records the schema revision. WAL keeps writes durable
/// without an fsync per statement, which matters because every todo mutation
/// writes immediately.
fn initialise(connection: &Connection) -> Result<(), rusqlite::Error> {
    // `PRAGMA journal_mode` answers with a row, so it cannot go through
    // `pragma_update`.
    let _mode: String = connection.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;
    connection.execute_batch(CREATE_SCHEMA)?;

    // Read the recorded revision before writing one. SQLite answers 0 for a
    // database that was never stamped, which is exactly the file we just
    // created, so only that case gets stamped with the current version. An
    // existing database keeps the revision it already carries — otherwise
    // bumping `SCHEMA_VERSION` later would relabel old files as migrated
    // without any migration having run, and the version would stop being usable
    // as evidence of the on-disk layout.
    let stored: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if stored == 0 {
        connection.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    } else if stored > SCHEMA_VERSION {
        // Only a newer build can have written a higher revision, and its layout
        // is unknown here. Matching the threshold to `SCHEMA_VERSION` is what
        // keeps the warning honest: every value this build understands is
        // exactly `SCHEMA_VERSION`.
        log::warn!(
            "Todo database schema revision {stored} is newer than the {SCHEMA_VERSION} this build \
             knows; leaving it untouched"
        );
    }
    Ok(())
}

impl TodoDb {
    /// Opens (creating when missing) the todo database at `path`.
    pub fn open(path: &Path) -> Self {
        match Connection::open(path).and_then(|connection| {
            initialise(&connection)?;
            Ok(connection)
        }) {
            Ok(connection) => Self {
                connection: Some(Mutex::new(connection)),
            },
            Err(error) => {
                log::warn!(
                    "Unable to open the todo database at {}: {error}",
                    path.display()
                );
                Self::unavailable()
            }
        }
    }

    /// A store that answers every call with `TodoDbError::Unavailable`.
    pub fn unavailable() -> Self {
        Self { connection: None }
    }

    fn with_connection<T>(
        &self,
        action: impl FnOnce(&Connection) -> Result<T, rusqlite::Error>,
    ) -> Result<T, TodoDbError> {
        let connection = self
            .connection
            .as_ref()
            .ok_or(TodoDbError::Unavailable)?
            .lock()
            .map_err(|_| TodoDbError::Poisoned)?;
        action(&connection).map_err(TodoDbError::from)
    }

    /// Every stored todo, newest first (matching the list order in the UI).
    pub fn list(&self) -> Result<Vec<Todo>, TodoDbError> {
        self.with_connection(|connection| {
            let mut statement = connection.prepare(SELECT_TODOS)?;
            let rows = statement.query_map([], row_to_todo)?;
            rows.collect()
        })
    }

    /// Inserts the todo or overwrites the stored row with the same id.
    pub fn save(&self, todo: &Todo) -> Result<(), TodoDbError> {
        self.with_connection(|connection| {
            connection.execute(
                UPSERT_TODO,
                params![
                    todo.id,
                    todo.title,
                    status_to_text(&todo.status),
                    todo.created_at,
                    todo.completed_at,
                    todo.due_date,
                    todo.reminder_at,
                ],
            )?;
            Ok(())
        })
    }

    /// Applies the writes the view layer could not get into SQLite earlier: the
    /// todos it had to park in its fallback store, plus the ids whose deletion
    /// it could not perform.
    ///
    /// A parked write only exists while SQLite holds nothing newer for that id
    /// — the view layer drops it the moment a native write succeeds — so an
    /// upsert here is always the fresher copy and must overwrite, and a parked
    /// deletion is a tombstone that must delete. Skipping ids the database
    /// already knows would silently throw the newer copy away instead.
    ///
    /// Each entry is applied on its own, which is the unit the caller settles
    /// up in: `Err` means the database as a whole is unusable and nothing was
    /// applied, while a single unusable row is a `rejected` entry that leaves
    /// the rest of the batch untouched. Replaying is idempotent (an upsert of
    /// the same row, a delete of an absent row), so a crash between commit and
    /// the caller's bookkeeping costs nothing but one repeated write.
    pub fn replay_pending(
        &self,
        upserts: &[Todo],
        deletions: &[String],
    ) -> Result<PendingWriteReport, TodoDbError> {
        let mut report = PendingWriteReport {
            applied: Vec::new(),
            rejected: Vec::new(),
        };

        for todo in upserts {
            match self.save(todo) {
                Ok(()) => report.applied.push(todo.id.clone()),
                Err(TodoDbError::Sqlite(error)) => report.rejected.push(RejectedWrite {
                    id: todo.id.clone(),
                    error,
                }),
                Err(fatal) => return Err(fatal),
            }
        }

        for id in deletions {
            match self.delete(id) {
                Ok(()) => report.applied.push(id.clone()),
                Err(TodoDbError::Sqlite(error)) => report.rejected.push(RejectedWrite {
                    id: id.clone(),
                    error,
                }),
                Err(fatal) => return Err(fatal),
            }
        }

        Ok(report)
    }

    /// Removes the todo; deleting an unknown id succeeds.
    pub fn delete(&self, id: &str) -> Result<(), TodoDbError> {
        self.with_connection(|connection| {
            connection.execute("DELETE FROM todos WHERE id = ?1", params![id])?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory_db() -> TodoDb {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        initialise(&connection).expect("initialise schema");
        TodoDb {
            connection: Some(Mutex::new(connection)),
        }
    }

    fn todo(id: &str, created_at: &str) -> Todo {
        Todo {
            id: id.to_owned(),
            title: format!("task {id}"),
            status: TodoStatus::Open,
            created_at: created_at.to_owned(),
            completed_at: None,
            due_date: None,
            reminder_at: None,
        }
    }

    #[test]
    fn saves_and_lists_todos_newest_first() {
        let db = memory_db();
        db.save(&todo("a", "2026-07-01T00:00:00Z")).expect("save a");
        db.save(&todo("b", "2026-07-02T00:00:00Z")).expect("save b");

        let stored = db.list().expect("list todos");
        let ids: Vec<&str> = stored.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(ids, vec!["b", "a"]);
    }

    #[test]
    fn saving_the_same_id_updates_the_row() {
        let db = memory_db();
        db.save(&todo("a", "2026-07-01T00:00:00Z")).expect("save");

        let mut updated = todo("a", "2026-07-01T00:00:00Z");
        updated.title = "renamed".to_owned();
        updated.status = TodoStatus::Completed;
        updated.completed_at = Some("2026-07-03T00:00:00Z".to_owned());
        updated.due_date = Some("2026-07-04".to_owned());
        db.save(&updated).expect("update");

        let stored = db.list().expect("list todos");
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].title, "renamed");
        assert!(matches!(stored[0].status, TodoStatus::Completed));
        assert_eq!(stored[0].completed_at.as_deref(), Some("2026-07-03T00:00:00Z"));
        assert_eq!(stored[0].due_date.as_deref(), Some("2026-07-04"));
    }

    #[test]
    fn deletes_by_id_and_tolerates_unknown_ids() {
        let db = memory_db();
        db.save(&todo("a", "2026-07-01T00:00:00Z")).expect("save");

        db.delete("a").expect("delete existing");
        db.delete("missing").expect("delete unknown");

        assert!(db.list().expect("list todos").is_empty());
    }

    #[test]
    fn an_unavailable_database_reports_an_error_instead_of_panicking() {
        let db = TodoDb::unavailable();

        assert!(matches!(db.list(), Err(TodoDbError::Unavailable)));
        assert!(matches!(
            db.save(&todo("a", "2026-07-01T00:00:00Z")),
            Err(TodoDbError::Unavailable)
        ));
        assert!(matches!(db.delete("a"), Err(TodoDbError::Unavailable)));
        assert!(matches!(
            db.replay_pending(&[todo("a", "2026-07-01T00:00:00Z")], &["b".to_owned()]),
            Err(TodoDbError::Unavailable)
        ));
    }

    fn user_version(connection: &Connection) -> i64 {
        connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("read user_version")
    }

    #[test]
    fn a_never_stamped_database_records_the_current_schema_version() {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        assert_eq!(user_version(&connection), 0);

        initialise(&connection).expect("initialise schema");

        assert_eq!(user_version(&connection), SCHEMA_VERSION);
    }

    #[test]
    fn an_already_recorded_schema_version_is_not_overwritten() {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        initialise(&connection).expect("initialise schema");
        // Stand in for a database written by a future revision of the schema.
        connection
            .pragma_update(None, "user_version", SCHEMA_VERSION + 1)
            .expect("stamp another version");

        initialise(&connection).expect("re-initialise schema");

        assert_eq!(user_version(&connection), SCHEMA_VERSION + 1);
    }

    fn version_of(db: &TodoDb) -> i64 {
        db.with_connection(|connection| {
            connection.query_row("PRAGMA user_version", [], |row| row.get(0))
        })
        .expect("read user_version")
    }

    #[test]
    fn replaying_pending_writes_stores_them_without_touching_the_schema_version() {
        let db = memory_db();
        assert_eq!(version_of(&db), SCHEMA_VERSION);

        let report = db
            .replay_pending(
                &[
                    todo("a", "2026-07-01T00:00:00Z"),
                    todo("b", "2026-07-02T00:00:00Z"),
                ],
                &[],
            )
            .expect("replay pending writes");

        assert_eq!(report.applied, vec!["a".to_owned(), "b".to_owned()]);
        assert!(report.rejected.is_empty());
        assert_eq!(db.list().expect("list todos").len(), 2);
        // Moving data is not a layout change, so the schema revision stands.
        assert_eq!(version_of(&db), SCHEMA_VERSION);
    }

    #[test]
    fn a_parked_write_overwrites_the_row_it_was_never_able_to_update() {
        let db = memory_db();
        let mut stored = todo("a", "2026-07-01T00:00:00Z");
        stored.title = "the row SQLite already had".to_owned();
        db.save(&stored).expect("save");

        // The view layer edited the same todo while SQLite was refusing writes,
        // so its parked copy is the newer one — the case a "skip known ids"
        // import used to drop on the floor.
        let mut parked = stored.clone();
        parked.title = "written while the database was refusing".to_owned();
        parked.status = TodoStatus::Completed;
        parked.completed_at = Some("2026-07-05T00:00:00Z".to_owned());

        let report = db.replay_pending(&[parked], &[]).expect("replay");

        assert_eq!(report.applied, vec!["a".to_owned()]);
        let rows = db.list().expect("list todos");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "written while the database was refusing");
        assert!(matches!(rows[0].status, TodoStatus::Completed));
    }

    #[test]
    fn a_parked_deletion_removes_the_row_and_tolerates_unknown_ids() {
        let db = memory_db();
        db.save(&todo("a", "2026-07-01T00:00:00Z")).expect("save");

        let report = db
            .replay_pending(&[], &["a".to_owned(), "never-stored".to_owned()])
            .expect("replay");

        assert_eq!(
            report.applied,
            vec!["a".to_owned(), "never-stored".to_owned()]
        );
        assert!(db.list().expect("list todos").is_empty());
    }

    #[test]
    fn one_rejected_entry_does_not_hold_up_the_rest_of_the_journal() {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        initialise(&connection).expect("initialise schema");
        // Stands in for any single row SQLite refuses; without per-entry
        // application it would block every other pending write forever.
        connection
            .execute_batch(
                "CREATE TRIGGER reject_b BEFORE INSERT ON todos
                 WHEN NEW.id = 'b' BEGIN SELECT RAISE(ABORT, 'rejected'); END",
            )
            .expect("create trigger");
        let db = TodoDb {
            connection: Some(Mutex::new(connection)),
        };

        let report = db
            .replay_pending(
                &[
                    todo("a", "2026-07-01T00:00:00Z"),
                    todo("b", "2026-07-02T00:00:00Z"),
                    todo("c", "2026-07-03T00:00:00Z"),
                ],
                &[],
            )
            .expect("replay");

        assert_eq!(report.applied, vec!["a".to_owned(), "c".to_owned()]);
        assert_eq!(report.rejected.len(), 1);
        assert_eq!(report.rejected[0].id, "b");
        assert!(report.rejected[0].error.contains("rejected"));
        let ids: Vec<String> = db
            .list()
            .expect("list todos")
            .into_iter()
            .map(|item| item.id)
            .collect();
        assert_eq!(ids, vec!["c".to_owned(), "a".to_owned()]);
    }

    #[test]
    fn reopening_the_same_file_keeps_stored_todos() {
        let path = std::env::temp_dir().join(format!("todo-db-test-{}.sqlite", std::process::id()));
        let _ = std::fs::remove_file(&path);

        {
            let db = TodoDb::open(&path);
            db.save(&todo("a", "2026-07-01T00:00:00Z")).expect("save");
        }

        let reopened = TodoDb::open(&path);
        let stored = reopened.list().expect("list todos");
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].id, "a");

        drop(reopened);
        let _ = std::fs::remove_file(&path);
    }
}
