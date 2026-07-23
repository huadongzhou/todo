use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, Row};
use serde::de::DeserializeOwned;
use serde::Serialize;
use specta::Type;
use todo_contracts::{ContractValidationError, RecurrenceRule, Todo, TodoStatus};

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
/// compiles against", which is the only question the code ever asks — with one
/// caveat learned the hard way, see `reconcile_columns`: a recorded revision is
/// a claim about the layout, not the layout itself.
///
/// * 1 — id, title, status, created_at, completed_at, due_date, reminder_at.
/// * 2 — the v1 task fields: notes, start date, timed block, estimate,
///   recurrence, list, Eisenhower axes, manual order, tags, subtasks,
///   attachments, dependencies.
/// * 3 — archived_at, the instant a completed task was moved out of the list.
const SCHEMA_VERSION: i64 = 3;

const CREATE_SCHEMA: &str = "CREATE TABLE IF NOT EXISTS todos (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    completed_at TEXT,
    archived_at TEXT,
    due_date TEXT,
    reminder_at TEXT,
    notes TEXT,
    start_date TEXT,
    starts_at TEXT,
    ends_at TEXT,
    estimated_minutes INTEGER,
    recurrence TEXT,
    list_id TEXT,
    important INTEGER,
    urgent INTEGER,
    sort_order REAL,
    tag_ids TEXT,
    subtasks TEXT,
    attachments TEXT,
    depends_on TEXT
)";

/// Bytes of a stored value that no build can read back, kept out of the way of
/// the row they came from.
///
/// It is the native half of the view layer's `todos.quarantine` key and holds
/// the same line: unreadable data is still the user's data, so it is preserved
/// with its origin and a timestamp instead of being overwritten by the empty
/// value the read degraded to. Nothing reads it yet — presenting and clearing
/// quarantined data is a single open design question across both layers — so it
/// deliberately has no schema revision of its own; `IF NOT EXISTS` is the whole
/// migration.
const CREATE_QUARANTINE: &str = "CREATE TABLE IF NOT EXISTS todo_quarantine (
    todo_id TEXT NOT NULL,
    column_name TEXT NOT NULL,
    raw TEXT NOT NULL,
    parked_at TEXT NOT NULL,
    PRIMARY KEY (todo_id, column_name, raw)
)";

/// Parks one unreadable value. The primary key makes re-reading the same bad
/// row a no-op rather than a growing pile of identical copies.
const PARK_UNREADABLE: &str = "INSERT OR IGNORE INTO todo_quarantine
    (todo_id, column_name, raw, parked_at)
    VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))";

/// Columns added after revision 1, with the type each one carries. A file older
/// than this build is brought up by adding whichever of these it is missing —
/// all nullable, so existing rows need no rewrite.
///
/// List-valued fields are stored as JSON text. Nothing queries inside them yet
/// (ordering and filtering still happen in the view layer), so a column per
/// entry would be a join to maintain for no reader.
const ADDED_COLUMNS: &[(&str, &str)] = &[
    ("notes", "TEXT"),
    ("start_date", "TEXT"),
    ("starts_at", "TEXT"),
    ("ends_at", "TEXT"),
    ("estimated_minutes", "INTEGER"),
    ("recurrence", "TEXT"),
    ("list_id", "TEXT"),
    ("important", "INTEGER"),
    ("urgent", "INTEGER"),
    ("sort_order", "REAL"),
    ("tag_ids", "TEXT"),
    ("subtasks", "TEXT"),
    ("attachments", "TEXT"),
    ("depends_on", "TEXT"),
    ("archived_at", "TEXT"),
];

const SELECT_TODOS: &str = "SELECT id, title, status, created_at, completed_at, archived_at,
    due_date, reminder_at, notes, start_date, starts_at, ends_at, estimated_minutes, recurrence,
    list_id, important, urgent, sort_order, tag_ids, subtasks, attachments, depends_on
    FROM todos
    ORDER BY created_at DESC, id";

const UPSERT_TODO: &str = "INSERT INTO todos (id, title, status, created_at, completed_at,
    archived_at, due_date, reminder_at, notes, start_date, starts_at, ends_at, estimated_minutes,
    recurrence, list_id, important, urgent, sort_order, tag_ids, subtasks, attachments, depends_on)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19,
        ?20, ?21, ?22)
    ON CONFLICT(id) DO UPDATE SET
        title = excluded.title,
        status = excluded.status,
        created_at = excluded.created_at,
        completed_at = excluded.completed_at,
        archived_at = excluded.archived_at,
        due_date = excluded.due_date,
        reminder_at = excluded.reminder_at,
        notes = excluded.notes,
        start_date = excluded.start_date,
        starts_at = excluded.starts_at,
        ends_at = excluded.ends_at,
        estimated_minutes = excluded.estimated_minutes,
        recurrence = excluded.recurrence,
        list_id = excluded.list_id,
        important = excluded.important,
        urgent = excluded.urgent,
        sort_order = excluded.sort_order,
        tag_ids = excluded.tag_ids,
        subtasks = excluded.subtasks,
        attachments = excluded.attachments,
        depends_on = excluded.depends_on";

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
    /// Whether retrying is pointless. A busy or full database refuses a write
    /// this minute and takes it the next, so the caller keeps the entry; an
    /// entry the contract refuses will be refused by every future build too, so
    /// the caller must park it instead of replaying it forever.
    pub permanent: bool,
}

/// Why a single write did not land, for the commands that write one todo.
///
/// It carries the same `permanent` split as `RejectedWrite`, because the
/// caller's decision is the same one: a write refused for good must not be
/// parked in the journal, or the user is promised a retry that can never
/// succeed — and told so by a banner — while the entry quietly waits to be
/// discarded on the next start.
#[derive(Clone, Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WriteRejection {
    pub error: String,
    pub permanent: bool,
}

impl From<TodoDbError> for WriteRejection {
    fn from(error: TodoDbError) -> Self {
        Self {
            permanent: error.is_permanent(),
            error: error.to_string(),
        }
    }
}

#[derive(Debug)]
pub enum TodoDbError {
    /// The database could not be opened at startup.
    Unavailable,
    /// A previous command panicked while holding the connection lock.
    Poisoned,
    /// SQLite rejected the statement.
    Sqlite(String),
    /// The todo does not satisfy the contract, so there is nothing to store.
    Invalid(ContractValidationError),
}

impl TodoDbError {
    /// Whether no future attempt can succeed. Only the contract answers that:
    /// a database that is busy, full or unopened today may take the very same
    /// write tomorrow, while a todo the contract refuses is refused forever.
    fn is_permanent(&self) -> bool {
        matches!(self, Self::Invalid(_))
    }
}

impl std::fmt::Display for TodoDbError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable => formatter.write_str("the local todo database is unavailable"),
            Self::Poisoned => formatter.write_str("the local todo database lock is poisoned"),
            Self::Sqlite(message) => write!(formatter, "local todo database error: {message}"),
            Self::Invalid(error) => write!(formatter, "the todo is not storable: {error}"),
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

/// A stored column value no build could read back, kept as the bytes SQLite
/// holds rather than as the empty value the read degraded to.
struct UnreadableValue {
    todo_id: String,
    column: &'static str,
    raw: String,
}

/// Reads a JSON-encoded column. A value that cannot be read back degrades to
/// the empty one for the same reason `status_from_text` degrades: a single
/// unreadable field must cost that field, not the whole todo. The bytes are
/// handed to the caller so `park_unreadable` can keep them.
fn decode_json<T: Default + DeserializeOwned>(
    raw: Option<String>,
    column: &'static str,
    todo_id: &str,
    unreadable: &mut Vec<UnreadableValue>,
) -> T {
    let Some(raw) = raw else {
        return T::default();
    };

    match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            log::warn!("Parking an unreadable `{column}` value on todo {todo_id}: {error}");
            unreadable.push(UnreadableValue {
                todo_id: todo_id.to_owned(),
                column,
                raw,
            });
            T::default()
        }
    }
}

fn encode_json<T: Serialize>(value: &T) -> Result<String, rusqlite::Error> {
    serde_json::to_string(value)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

/// SQLite has no boolean type, so the tri-state travels as NULL / 0 / 1.
fn bool_from_int(value: Option<i64>) -> Option<bool> {
    value.map(|stored| stored != 0)
}

fn row_to_todo(
    row: &Row<'_>,
    unreadable: &mut Vec<UnreadableValue>,
) -> Result<Todo, rusqlite::Error> {
    let status: String = row.get("status")?;
    let id: String = row.get("id")?;
    Ok(Todo {
        title: row.get("title")?,
        status: status_from_text(&status),
        created_at: row.get("created_at")?,
        completed_at: row.get("completed_at")?,
        archived_at: row.get("archived_at")?,
        due_date: row.get("due_date")?,
        reminder_at: row.get("reminder_at")?,
        notes: row.get("notes")?,
        start_date: row.get("start_date")?,
        starts_at: row.get("starts_at")?,
        ends_at: row.get("ends_at")?,
        estimated_minutes: row.get("estimated_minutes")?,
        recurrence: decode_json::<Option<RecurrenceRule>>(
            row.get("recurrence")?,
            "recurrence",
            &id,
            unreadable,
        ),
        list_id: row.get("list_id")?,
        important: bool_from_int(row.get("important")?),
        urgent: bool_from_int(row.get("urgent")?),
        sort_order: row.get("sort_order")?,
        tag_ids: decode_json(row.get("tag_ids")?, "tag_ids", &id, unreadable),
        subtasks: decode_json(row.get("subtasks")?, "subtasks", &id, unreadable),
        attachments: decode_json(row.get("attachments")?, "attachments", &id, unreadable),
        depends_on: decode_json(row.get("depends_on")?, "depends_on", &id, unreadable),
        id,
    })
}

/// Keeps the bytes of every value the read could not make sense of.
///
/// The read degrades to the empty value so one bad column cannot cost the whole
/// todo — but every `save` writes all 22 columns, so the next write of that
/// todo would replace those bytes with the empty value and lose them for good.
/// Parking them first is the same stance the view layer takes with entries it
/// cannot read (`todos.quarantine`): unreadable is not a reason to delete, so
/// the bytes are kept aside with a timestamp and said out loud.
///
/// Failing to park is logged rather than propagated: the read itself succeeded,
/// and refusing to answer with the user's todos would turn one bad column into
/// an empty application.
fn park_unreadable(connection: &Connection, values: &[UnreadableValue]) {
    for value in values {
        if let Err(error) = connection.execute(
            PARK_UNREADABLE,
            params![value.todo_id, value.column, value.raw],
        ) {
            log::warn!(
                "Unable to park the unreadable `{}` value of todo {}: {error}",
                value.column,
                value.todo_id
            );
        }
    }
}

/// Adds whichever post-v1 columns the `todos` table is missing and answers how
/// many were added.
///
/// The table itself is the evidence, not `user_version`, and that ordering is
/// deliberate: a pre-release build stamped 2 into files whose layout was still
/// v1, so trusting the recorded revision would skip the very migration those
/// files need and freeze the mistake in place. Reading the columns costs one
/// pragma and answers the real question — is the column there or not — for
/// every file, however it came to carry the number it carries.
fn reconcile_columns(connection: &Connection) -> Result<usize, rusqlite::Error> {
    let mut statement = connection.prepare("PRAGMA table_info(todos)")?;
    let present: Vec<String> = statement
        .query_map([], |row| row.get::<_, String>("name"))?
        .collect::<Result<_, _>>()?;
    drop(statement);

    let mut added = 0;
    for (name, column_type) in ADDED_COLUMNS {
        if present.iter().any(|existing| existing == name) {
            continue;
        }
        // Every added column is nullable, so existing rows stay valid and the
        // statement cannot fail on data.
        connection.execute_batch(&format!(
            "ALTER TABLE todos ADD COLUMN {name} {column_type}"
        ))?;
        added += 1;
    }
    Ok(added)
}

/// Creates the schema, brings an older file up to the current layout and
/// records the schema revision. WAL keeps writes durable without an fsync per
/// statement, which matters because every todo mutation writes immediately.
fn initialise(connection: &Connection) -> Result<(), rusqlite::Error> {
    // `PRAGMA journal_mode` answers with a row, so it cannot go through
    // `pragma_update`.
    let _mode: String = connection.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;
    connection.execute_batch(CREATE_SCHEMA)?;
    connection.execute_batch(CREATE_QUARANTINE)?;

    let stored: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let added = reconcile_columns(connection)?;
    if added > 0 {
        log::info!(
            "Brought the todos table up to schema revision {SCHEMA_VERSION} (+{added} columns)"
        );
    }

    if stored == SCHEMA_VERSION {
        return Ok(());
    }

    if stored > SCHEMA_VERSION && added == 0 {
        // The file already had every column this build knows and claims a
        // higher revision: only a newer build can have written it, and its
        // extra layout is none of this build's business.
        log::warn!(
            "Todo database schema revision {stored} is newer than the {SCHEMA_VERSION} this build \
             knows; leaving it untouched"
        );
        return Ok(());
    }

    if stored > SCHEMA_VERSION {
        log::warn!(
            "Todo database claimed schema revision {stored} but was missing columns from \
             {SCHEMA_VERSION}; correcting the recorded revision after the migration"
        );
    }
    connection.pragma_update(None, "user_version", SCHEMA_VERSION)?;
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
            let mut unreadable = Vec::new();
            let todos = {
                let mut statement = connection.prepare(SELECT_TODOS)?;
                let rows = statement.query_map([], |row| row_to_todo(row, &mut unreadable))?;
                rows.collect::<Result<Vec<Todo>, _>>()?
            };
            // Reading is what discovers the bad bytes, and the write that would
            // destroy them can come at any time after that, so they are kept
            // here rather than at some later, better-looking moment.
            park_unreadable(connection, &unreadable);
            Ok(todos)
        })
    }

    /// Inserts the todo or overwrites the stored row with the same id.
    ///
    /// The contract is checked first: the database would happily store a todo
    /// with a blank title or a duplicated subtask id, and the row would then be
    /// unreadable to every consumer that trusts the contract.
    pub fn save(&self, todo: &Todo) -> Result<(), TodoDbError> {
        todo.validate().map_err(TodoDbError::Invalid)?;

        self.with_connection(|connection| {
            connection.execute(
                UPSERT_TODO,
                params![
                    todo.id,
                    todo.title,
                    status_to_text(&todo.status),
                    todo.created_at,
                    todo.completed_at,
                    todo.archived_at,
                    todo.due_date,
                    todo.reminder_at,
                    todo.notes,
                    todo.start_date,
                    todo.starts_at,
                    todo.ends_at,
                    todo.estimated_minutes,
                    todo.recurrence
                        .as_ref()
                        .map(encode_json)
                        .transpose()?
                        .as_deref(),
                    todo.list_id,
                    todo.important,
                    todo.urgent,
                    todo.sort_order,
                    encode_json(&todo.tag_ids)?,
                    encode_json(&todo.subtasks)?,
                    encode_json(&todo.attachments)?,
                    encode_json(&todo.depends_on)?,
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
                Err(error @ (TodoDbError::Sqlite(_) | TodoDbError::Invalid(_))) => {
                    report.rejected.push(RejectedWrite {
                        id: todo.id.clone(),
                        permanent: error.is_permanent(),
                        error: error.to_string(),
                    })
                }
                Err(fatal) => return Err(fatal),
            }
        }

        for id in deletions {
            match self.delete(id) {
                Ok(()) => report.applied.push(id.clone()),
                Err(error @ TodoDbError::Sqlite(_)) => report.rejected.push(RejectedWrite {
                    id: id.clone(),
                    permanent: false,
                    error: error.to_string(),
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
    use todo_contracts::{
        Attachment, AttachmentKind, RecurrenceCalendar, RecurrenceFrequency, Subtask, Weekday,
    };

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
            archived_at: None,
            due_date: None,
            reminder_at: None,
            notes: None,
            start_date: None,
            starts_at: None,
            ends_at: None,
            estimated_minutes: None,
            recurrence: None,
            list_id: None,
            important: None,
            urgent: None,
            sort_order: None,
            tag_ids: Vec::new(),
            subtasks: Vec::new(),
            attachments: Vec::new(),
            depends_on: Vec::new(),
        }
    }

    /// The exact layout revision 1 shipped with, used to stand in for a file
    /// written by an earlier build.
    const V1_SCHEMA: &str = "CREATE TABLE todos (
        id TEXT PRIMARY KEY NOT NULL,
        title TEXT NOT NULL,
        status TEXT NOT NULL,
        created_at TEXT NOT NULL,
        completed_at TEXT,
        due_date TEXT,
        reminder_at TEXT
    )";

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
        // Stand in for a database written by a future revision of the schema:
        // it has every column this build knows plus, presumably, more.
        connection
            .pragma_update(None, "user_version", SCHEMA_VERSION + 1)
            .expect("stamp another version");

        initialise(&connection).expect("re-initialise schema");

        assert_eq!(user_version(&connection), SCHEMA_VERSION + 1);
    }

    fn columns(connection: &Connection) -> Vec<String> {
        let mut statement = connection
            .prepare("PRAGMA table_info(todos)")
            .expect("read table info");
        let names = statement
            .query_map([], |row| row.get::<_, String>("name"))
            .expect("map columns")
            .collect::<Result<Vec<String>, _>>()
            .expect("collect columns");
        names
    }

    /// A file left behind by revision 1: the original seven columns, stamped 1.
    fn v1_database(stamped_as: i64) -> Connection {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        connection
            .execute_batch(V1_SCHEMA)
            .expect("create v1 table");
        connection
            .execute(
                "INSERT INTO todos (id, title, status, created_at, completed_at, due_date, \
                 reminder_at) VALUES ('a', 'from the old build', 'open', '2026-07-01T00:00:00Z', \
                 NULL, '2026-07-02', '2026-07-02T09:00:00Z')",
                [],
            )
            .expect("insert a v1 row");
        connection
            .pragma_update(None, "user_version", stamped_as)
            .expect("stamp the revision");
        connection
    }

    #[test]
    fn a_revision_one_database_gains_the_new_columns_without_losing_data() {
        let connection = v1_database(1);

        initialise(&connection).expect("migrate the old file");

        assert_eq!(user_version(&connection), SCHEMA_VERSION);
        let names = columns(&connection);
        for (column, _) in ADDED_COLUMNS {
            assert!(names.iter().any(|name| name == column), "missing {column}");
        }

        let db = TodoDb {
            connection: Some(Mutex::new(connection)),
        };
        let stored = db.list().expect("list todos");
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].title, "from the old build");
        assert_eq!(stored[0].due_date.as_deref(), Some("2026-07-02"));
        assert_eq!(
            stored[0].reminder_at.as_deref(),
            Some("2026-07-02T09:00:00Z")
        );
        // The fields the row predates read back empty rather than failing it.
        assert!(stored[0].tag_ids.is_empty());
        assert!(stored[0].subtasks.is_empty());
        assert_eq!(stored[0].important, None);
        assert_eq!(stored[0].archived_at, None);
    }

    #[test]
    fn an_archive_stamp_survives_being_written_and_cleared_again() {
        // Restoring writes the same column back to NULL, and a row that could
        // be archived but never un-archived would strand the task out of sight.
        let db = memory_db();
        let mut archived = todo("a", "2026-07-01T00:00:00Z");
        archived.status = TodoStatus::Completed;
        archived.completed_at = Some("2026-07-10T09:00:00Z".to_owned());
        archived.archived_at = Some("2026-07-23T00:00:05Z".to_owned());
        db.save(&archived).expect("store an archived todo");

        let stored = db.list().expect("list todos").remove(0);
        assert_eq!(stored.archived_at.as_deref(), Some("2026-07-23T00:00:05Z"));
        assert_eq!(stored.completed_at.as_deref(), Some("2026-07-10T09:00:00Z"));

        let mut restored = stored;
        restored.archived_at = None;
        restored.status = TodoStatus::Open;
        restored.completed_at = None;
        db.save(&restored).expect("store the restored todo");

        let read_back = db.list().expect("list todos").remove(0);
        assert_eq!(read_back.archived_at, None);
        assert!(matches!(read_back.status, TodoStatus::Open));
    }

    #[test]
    fn a_database_whose_recorded_revision_overstates_its_layout_is_still_migrated() {
        // Exactly what a pre-release build left behind: the v1 column layout
        // with `user_version` already written as 2. Trusting the number would
        // decide "nothing to do" and freeze the file one migration short.
        let connection = v1_database(SCHEMA_VERSION);
        assert_eq!(columns(&connection).len(), 7);

        initialise(&connection).expect("migrate the mis-stamped file");

        assert_eq!(user_version(&connection), SCHEMA_VERSION);
        let names = columns(&connection);
        for (column, _) in ADDED_COLUMNS {
            assert!(names.iter().any(|name| name == column), "missing {column}");
        }

        let db = TodoDb {
            connection: Some(Mutex::new(connection)),
        };
        let mut migrated = db.list().expect("list todos").remove(0);
        migrated.tag_ids = vec!["tag-1".to_owned()];
        migrated.notes = Some("written after the migration".to_owned());
        db.save(&migrated).expect("save into the migrated layout");
        let stored = db.list().expect("list todos");
        assert_eq!(stored[0].tag_ids, vec!["tag-1".to_owned()]);
        assert_eq!(
            stored[0].notes.as_deref(),
            Some("written after the migration")
        );
    }

    #[test]
    fn a_future_revision_still_missing_columns_is_migrated_and_stamped_back_down() {
        // The one branch that both adds columns and rewrites the recorded
        // revision downward: a file claiming a newer revision than this build
        // yet still missing columns, i.e. a version number written without the
        // layout to back it. Left uncorrected it would keep tripping the
        // "newer than this build" warning on every start.
        let connection = v1_database(SCHEMA_VERSION + 1);
        assert_eq!(columns(&connection).len(), 7);
        assert!(user_version(&connection) > SCHEMA_VERSION);

        initialise(&connection).expect("migrate the mis-stamped future file");

        assert_eq!(user_version(&connection), SCHEMA_VERSION);
        let names = columns(&connection);
        for (column, _) in ADDED_COLUMNS {
            assert!(names.iter().any(|name| name == column), "missing {column}");
        }
    }

    #[test]
    fn an_unreadable_json_column_is_parked_rather_than_lost() {
        let db = memory_db();
        db.save(&todo("a", "2026-07-01T00:00:00Z")).expect("save");
        // Corrupt a JSON column the way a payload from a build this one does not
        // understand would look: bytes that will not decode to the column type.
        db.with_connection(|connection| {
            connection.execute(
                "UPDATE todos SET tag_ids = ?1 WHERE id = 'a'",
                params!["{not json"],
            )
        })
        .expect("corrupt the column");

        // The read degrades the bad column to empty but keeps its bytes aside.
        let listed = db.list().expect("list todos");
        assert!(listed[0].tag_ids.is_empty());

        let parked: Vec<(String, String, String)> = db
            .with_connection(|connection| {
                let mut statement =
                    connection.prepare("SELECT todo_id, column_name, raw FROM todo_quarantine")?;
                let rows =
                    statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
                rows.collect()
            })
            .expect("read the quarantine table");
        assert_eq!(parked.len(), 1);
        assert_eq!(parked[0].0, "a");
        assert_eq!(parked[0].1, "tag_ids");
        assert_eq!(parked[0].2, "{not json");

        // Reading again must not pile up a second copy of the same bad bytes.
        db.list().expect("list again");
        let count: i64 = db
            .with_connection(|connection| {
                connection.query_row("SELECT COUNT(*) FROM todo_quarantine", [], |row| row.get(0))
            })
            .expect("count the quarantine table");
        assert_eq!(count, 1);
    }

    #[test]
    fn only_a_contract_rejection_is_reported_permanent() {
        let contract =
            WriteRejection::from(TodoDbError::Invalid(ContractValidationError::InvalidTitle));
        assert!(contract.permanent);

        let busy = WriteRejection::from(TodoDbError::Sqlite("database is locked".to_owned()));
        assert!(!busy.permanent);

        assert!(!WriteRejection::from(TodoDbError::Unavailable).permanent);
    }

    #[test]
    fn migrating_twice_changes_nothing() {
        let connection = v1_database(1);
        initialise(&connection).expect("migrate once");
        let after_first = columns(&connection);

        initialise(&connection).expect("migrate again");

        assert_eq!(columns(&connection), after_first);
        assert_eq!(user_version(&connection), SCHEMA_VERSION);
    }

    #[test]
    fn every_new_field_survives_a_round_trip_through_sqlite() {
        let db = memory_db();
        let mut stored = todo("a", "2026-07-01T00:00:00Z");
        stored.notes = Some("the note".to_owned());
        stored.start_date = Some("2026-07-02".to_owned());
        stored.starts_at = Some("2026-07-03T09:00:00Z".to_owned());
        stored.ends_at = Some("2026-07-03T10:00:00Z".to_owned());
        stored.estimated_minutes = Some(45);
        stored.list_id = Some("list-1".to_owned());
        stored.important = Some(true);
        stored.urgent = Some(false);
        stored.sort_order = Some(2.5);
        stored.tag_ids = vec!["tag-1".to_owned(), "tag-2".to_owned()];
        stored.depends_on = vec!["b".to_owned()];
        stored.subtasks = vec![Subtask {
            id: "step-1".to_owned(),
            title: "first step".to_owned(),
            done: true,
        }];
        stored.attachments = vec![Attachment {
            id: "link-1".to_owned(),
            kind: AttachmentKind::Link,
            url: "https://example.invalid/plan".to_owned(),
            name: None,
        }];
        stored.recurrence = Some(RecurrenceRule {
            frequency: RecurrenceFrequency::Weekly,
            interval: 2,
            weekdays: vec![Weekday::Monday, Weekday::Friday],
            month_day: None,
            on_last_day: false,
            calendar: RecurrenceCalendar::Lunar,
            until: Some("2026-12-31".to_owned()),
            count: None,
        });

        db.save(&stored).expect("save the fully filled todo");
        let read_back = db.list().expect("list todos").remove(0);

        assert_eq!(read_back.notes.as_deref(), Some("the note"));
        assert_eq!(read_back.start_date.as_deref(), Some("2026-07-02"));
        assert_eq!(read_back.ends_at.as_deref(), Some("2026-07-03T10:00:00Z"));
        assert_eq!(read_back.estimated_minutes, Some(45));
        assert_eq!(read_back.list_id.as_deref(), Some("list-1"));
        assert_eq!(read_back.important, Some(true));
        assert_eq!(read_back.urgent, Some(false));
        assert_eq!(read_back.sort_order, Some(2.5));
        assert_eq!(
            read_back.tag_ids,
            vec!["tag-1".to_owned(), "tag-2".to_owned()]
        );
        assert_eq!(read_back.depends_on, vec!["b".to_owned()]);
        assert_eq!(read_back.subtasks.len(), 1);
        assert!(read_back.subtasks[0].done);
        assert_eq!(read_back.attachments[0].url, "https://example.invalid/plan");
        let recurrence = read_back.recurrence.expect("the rule survives");
        assert_eq!(recurrence.interval, 2);
        assert_eq!(recurrence.weekdays.len(), 2);
        assert!(matches!(recurrence.calendar, RecurrenceCalendar::Lunar));
        assert_eq!(recurrence.until.as_deref(), Some("2026-12-31"));
    }

    #[test]
    fn a_todo_the_contract_refuses_is_not_stored() {
        let db = memory_db();
        let mut blank_title = todo("a", "2026-07-01T00:00:00Z");
        blank_title.title = "   ".to_owned();

        assert!(matches!(
            db.save(&blank_title),
            Err(TodoDbError::Invalid(_))
        ));
        assert!(db.list().expect("list todos").is_empty());

        // The same entry coming out of the journal is refused per entry, so it
        // cannot hold up the rest of the replay.
        let report = db
            .replay_pending(&[blank_title, todo("b", "2026-07-02T00:00:00Z")], &[])
            .expect("replay");
        assert_eq!(report.applied, vec!["b".to_owned()]);
        assert_eq!(report.rejected.len(), 1);
        assert_eq!(report.rejected[0].id, "a");
        // No future build will take it either, so the caller must park it
        // rather than replay it on every start.
        assert!(report.rejected[0].permanent);
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
        // A database that refuses a write today may take it tomorrow.
        assert!(!report.rejected[0].permanent);
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
