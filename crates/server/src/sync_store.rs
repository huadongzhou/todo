use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension, Row};
use todo_contracts::{SyncCursor, SyncOperationKind, TodoPatch, TodoSyncChange};

/// Server-side SQLite storage for the sync log.
///
/// The log is append-only: one row per accepted operation, keyed by the
/// revision it was given. Everything the sync service used to keep in memory is
/// derived from it — the set of operation ids already seen (the `operation_id`
/// unique index), the current cursor (the largest revision) and the changes a
/// device still has to pull (`revision > cursor`) — so a restart costs nothing.
///
/// Unlike the client store, opening is not best effort. A desktop app with no
/// database still has a user in front of it and falls back to browser storage;
/// a sync server with no database can only hand out wrong answers, so it
/// refuses to start instead.
pub struct SyncStore {
    connection: Mutex<Connection>,
}

/// Revision of the server table layout, stamped into `PRAGMA user_version`.
///
/// Same contract as the client store, and the same caveat learned there: the
/// recorded revision is a *claim about* the layout, not the layout itself. When
/// a revision 2 arrives, the step that brings older files up must read
/// `PRAGMA table_info` and add whichever columns are missing rather than branch
/// on the stored number — a file whose number was written without the layout to
/// back it would otherwise be judged "already current" and skip the migration
/// it needs. There is exactly one revision today, so there is nothing to
/// reconcile yet and no machinery for it.
///
/// * 1 — sync_changes: revision, operation_id, todo_id, kind, occurred_at,
///   patch.
const SCHEMA_VERSION: i64 = 1;

const CREATE_SCHEMA: &str = "CREATE TABLE IF NOT EXISTS sync_changes (
    revision INTEGER PRIMARY KEY NOT NULL,
    operation_id TEXT NOT NULL UNIQUE,
    todo_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    patch TEXT
)";

const SELECT_OPERATION: &str = "SELECT revision FROM sync_changes WHERE operation_id = ?1";

const SELECT_LATEST_REVISION: &str = "SELECT COALESCE(MAX(revision), 0) FROM sync_changes";

const APPEND_CHANGE: &str = "INSERT INTO sync_changes
    (revision, operation_id, todo_id, kind, occurred_at, patch)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6)";

const SELECT_CHANGES_SINCE: &str = "SELECT revision, operation_id, todo_id, kind, occurred_at,
    patch
    FROM sync_changes
    WHERE revision > ?1
    ORDER BY revision";

#[derive(Debug)]
pub enum SyncStoreError {
    /// A previous request panicked while holding the connection lock.
    Poisoned,
    /// SQLite refused the statement.
    Sqlite(String),
    /// A patch could not be turned into the JSON the column stores.
    UnusablePatch(String),
    /// A stored revision does not fit the cursor the protocol speaks, so no
    /// further operation can be numbered.
    UnusableRevision(i64),
}

impl std::fmt::Display for SyncStoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => formatter.write_str("the sync log lock is poisoned"),
            Self::Sqlite(message) => write!(formatter, "sync log database error: {message}"),
            Self::UnusablePatch(message) => {
                write!(formatter, "the patch is not storable: {message}")
            }
            Self::UnusableRevision(revision) => {
                write!(formatter, "stored revision {revision} is out of range")
            }
        }
    }
}

impl std::error::Error for SyncStoreError {}

impl From<rusqlite::Error> for SyncStoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error.to_string())
    }
}

fn kind_to_text(kind: &SyncOperationKind) -> &'static str {
    match kind {
        SyncOperationKind::Upsert => "upsert",
        SyncOperationKind::Delete => "delete",
    }
}

/// An unknown kind has no safe reading. The client store degrades an unknown
/// status to `Open` because both readings show the same task; upsert and delete
/// are opposite actions, so guessing one would apply a change nobody asked for.
/// The row is reported unreadable instead.
fn kind_from_text(value: &str) -> Option<SyncOperationKind> {
    match value {
        "upsert" => Some(SyncOperationKind::Upsert),
        "delete" => Some(SyncOperationKind::Delete),
        _ => None,
    }
}

/// Rebuilds one change from its row, or answers `None` when no reading of the
/// stored bytes is honest.
///
/// Degrading is not on the table here the way it is for a todo column: an
/// upsert whose patch will not decode would become "change nothing", which
/// misreports the operation to every device that pulls it. The row is skipped
/// and said out loud instead — and skipping costs no data, because the log is
/// append-only and nothing ever rewrites the bytes that were not understood.
fn read_change(row: &Row<'_>) -> Result<Option<TodoSyncChange>, rusqlite::Error> {
    let stored_revision: i64 = row.get("revision")?;
    let operation_id: String = row.get("operation_id")?;
    let kind_text: String = row.get("kind")?;
    let stored_patch: Option<String> = row.get("patch")?;

    let Ok(revision) = SyncCursor::try_from(stored_revision) else {
        tracing::warn!(
            revision = stored_revision,
            %operation_id,
            "skipping a sync log row whose revision is out of range"
        );
        return Ok(None);
    };

    let Some(kind) = kind_from_text(&kind_text) else {
        tracing::warn!(
            revision,
            %operation_id,
            kind = %kind_text,
            "skipping a sync log row with an unreadable kind"
        );
        return Ok(None);
    };

    let patch = match stored_patch {
        None => None,
        Some(raw) => match serde_json::from_str::<TodoPatch>(&raw) {
            Ok(patch) => Some(patch),
            Err(error) => {
                tracing::warn!(
                    revision,
                    %operation_id,
                    %error,
                    "skipping a sync log row with an unreadable patch"
                );
                return Ok(None);
            }
        },
    };

    Ok(Some(TodoSyncChange {
        operation_id,
        todo_id: row.get("todo_id")?,
        kind,
        occurred_at: row.get("occurred_at")?,
        patch,
        revision,
    }))
}

/// Creates the schema and records the layout revision.
///
/// WAL keeps readers out of the writer's way, and `synchronous = FULL` fsyncs
/// every commit: the response tells the device its operations are accepted and
/// the device drops them from its outbox on the strength of that, so a commit
/// that is only in the operating system's page cache when the power goes is an
/// operation lost with nobody left holding a copy.
fn initialise(connection: &Connection) -> Result<(), rusqlite::Error> {
    // `PRAGMA journal_mode` answers with a row, so it cannot go through
    // `execute_batch`.
    let _mode: String = connection.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;
    connection.execute_batch("PRAGMA synchronous = FULL")?;
    connection.execute_batch(CREATE_SCHEMA)?;

    let stored: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if stored == SCHEMA_VERSION {
        return Ok(());
    }

    if stored > SCHEMA_VERSION {
        // Only a newer build can have written it, and its extra layout is none
        // of this build's business.
        tracing::warn!(
            stored,
            known = SCHEMA_VERSION,
            "sync log schema revision is newer than this build knows; leaving it untouched"
        );
        return Ok(());
    }

    connection.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}

impl SyncStore {
    /// Opens (creating when missing) the sync log at `path`.
    pub fn open(path: &Path) -> Result<Self, SyncStoreError> {
        let connection = Connection::open(path)?;
        initialise(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    /// A log that lives only as long as the process, for tests.
    pub fn in_memory() -> Result<Self, SyncStoreError> {
        let connection = Connection::open_in_memory()?;
        initialise(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    /// Runs `action` against the log inside one transaction, committing when it
    /// succeeds and rolling back when it does not.
    ///
    /// One sync request reads the current revision, appends to the log and
    /// reads back what the device is owed; splitting that across transactions
    /// would let a second request number an operation with a revision this one
    /// has already handed out. The caller's error type only has to be able to
    /// carry a storage failure, so deciding whether a request is acceptable
    /// stays with the caller instead of leaking into this module.
    pub fn with_log<T, E>(&self, action: impl FnOnce(&SyncLog<'_>) -> Result<T, E>) -> Result<T, E>
    where
        E: From<SyncStoreError>,
    {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| E::from(SyncStoreError::Poisoned))?;
        let transaction = connection
            .transaction()
            .map_err(|error| E::from(SyncStoreError::from(error)))?;

        let value = action(&SyncLog {
            connection: &transaction,
        })?;

        transaction
            .commit()
            .map_err(|error| E::from(SyncStoreError::from(error)))?;
        Ok(value)
    }
}

/// The sync log as seen from inside one transaction.
pub struct SyncLog<'a> {
    connection: &'a Connection,
}

impl SyncLog<'_> {
    /// Whether this operation was already accepted — the replay check that lets
    /// a device push its whole outbox again without duplicating anything.
    pub fn contains_operation(&self, operation_id: &str) -> Result<bool, SyncStoreError> {
        let existing: Option<i64> = self
            .connection
            .query_row(SELECT_OPERATION, params![operation_id], |row| row.get(0))
            .optional()?;
        Ok(existing.is_some())
    }

    /// The revision most recently handed out, or 0 for an empty log.
    pub fn latest_revision(&self) -> Result<SyncCursor, SyncStoreError> {
        let stored: i64 = self
            .connection
            .query_row(SELECT_LATEST_REVISION, [], |row| row.get(0))?;
        SyncCursor::try_from(stored).map_err(|_| SyncStoreError::UnusableRevision(stored))
    }

    /// Records one accepted operation at the revision it was given.
    pub fn append(&self, change: &TodoSyncChange) -> Result<(), SyncStoreError> {
        let patch = change
            .patch
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|error| SyncStoreError::UnusablePatch(error.to_string()))?;

        self.connection.execute(
            APPEND_CHANGE,
            params![
                i64::from(change.revision),
                change.operation_id,
                change.todo_id,
                kind_to_text(&change.kind),
                change.occurred_at,
                patch,
            ],
        )?;
        Ok(())
    }

    /// Everything recorded after `cursor`, oldest first.
    pub fn changes_since(&self, cursor: SyncCursor) -> Result<Vec<TodoSyncChange>, SyncStoreError> {
        let mut statement = self.connection.prepare(SELECT_CHANGES_SINCE)?;
        let rows = statement.query_map(params![i64::from(cursor)], read_change)?;
        let mut changes = Vec::new();
        for change in rows {
            if let Some(change) = change? {
                changes.push(change);
            }
        }
        Ok(changes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file path no other test shares, plus the WAL sidecars SQLite puts next
    /// to it.
    struct TempDatabase {
        path: std::path::PathBuf,
    }

    impl TempDatabase {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("todo-server-{name}-{}.sqlite3", std::process::id()));
            let database = Self { path };
            database.remove();
            database
        }

        fn remove(&self) {
            for suffix in ["", "-wal", "-shm"] {
                let mut path = self.path.clone().into_os_string();
                path.push(suffix);
                let _ = std::fs::remove_file(path);
            }
        }

        fn open(&self) -> SyncStore {
            SyncStore::open(&self.path).expect("open the sync log")
        }
    }

    impl Drop for TempDatabase {
        fn drop(&mut self) {
            self.remove();
        }
    }

    fn change(operation_id: &str, revision: SyncCursor) -> TodoSyncChange {
        TodoSyncChange {
            operation_id: operation_id.to_owned(),
            todo_id: "todo-1".to_owned(),
            kind: SyncOperationKind::Upsert,
            occurred_at: "2026-07-23T00:00:00Z".to_owned(),
            patch: Some(TodoPatch {
                title: Some("write it down".to_owned()),
                ..TodoPatch::default()
            }),
            revision,
        }
    }

    #[test]
    fn a_new_log_is_stamped_and_configured_for_durable_commits() {
        let store = SyncStore::in_memory().expect("open an in-memory log");

        let (version, synchronous) = store
            .with_log(|log| {
                let version: i64 = log
                    .connection
                    .query_row("PRAGMA user_version", [], |row| row.get(0))?;
                let synchronous: i64 =
                    log.connection
                        .query_row("PRAGMA synchronous", [], |row| row.get(0))?;
                Ok::<_, SyncStoreError>((version, synchronous))
            })
            .expect("read the pragmas");

        assert_eq!(version, SCHEMA_VERSION);
        // 2 is FULL: every commit is fsynced before it is reported as accepted.
        assert_eq!(synchronous, 2);
    }

    #[test]
    fn a_log_from_a_newer_revision_is_left_untouched() {
        let store = SyncStore::in_memory().expect("open an in-memory log");
        store
            .with_log(|log| {
                log.connection
                    .pragma_update(None, "user_version", SCHEMA_VERSION + 1)?;
                Ok::<_, SyncStoreError>(())
            })
            .expect("stamp a newer revision");

        let connection = store.connection.lock().expect("take the connection");
        initialise(&connection).expect("re-initialise");
        let stored: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("read user_version");

        assert_eq!(stored, SCHEMA_VERSION + 1);
    }

    #[test]
    fn appended_changes_are_still_there_after_closing_and_reopening_the_file() {
        let database = TempDatabase::new("reopen");

        {
            let store = database.open();
            store
                .with_log(|log| {
                    log.append(&change("operation-1", 1))?;
                    log.append(&change("operation-2", 2))
                })
                .expect("append two changes");
        }

        let reopened = database.open();
        let (latest, changes) = reopened
            .with_log(|log| {
                Ok::<_, SyncStoreError>((log.latest_revision()?, log.changes_since(0)?))
            })
            .expect("read the reopened log");

        assert_eq!(latest, 2);
        let ids: Vec<&str> = changes
            .iter()
            .map(|change| change.operation_id.as_str())
            .collect();
        assert_eq!(ids, vec!["operation-1", "operation-2"]);
        assert_eq!(changes[0].revision, 1);
        assert_eq!(changes[1].revision, 2);
    }

    #[test]
    fn an_operation_id_is_still_recognised_after_a_reopen() {
        let database = TempDatabase::new("replay");

        {
            let store = database.open();
            store
                .with_log(|log| log.append(&change("operation-1", 1)))
                .expect("append");
        }

        let reopened = database.open();
        let (known, unknown) = reopened
            .with_log(|log| {
                Ok::<_, SyncStoreError>((
                    log.contains_operation("operation-1")?,
                    log.contains_operation("operation-2")?,
                ))
            })
            .expect("check for replays");

        assert!(
            known,
            "a replayed operation must be recognised after a restart"
        );
        assert!(!unknown);
    }

    #[test]
    fn a_patch_keeps_exactly_the_fields_it_carried() {
        let store = SyncStore::in_memory().expect("open an in-memory log");
        let mut carried = change("operation-1", 1);
        carried.patch = Some(TodoPatch {
            status: Some(todo_contracts::TodoStatus::Completed),
            completed_at: Some("2026-07-23T10:00:00Z".to_owned()),
            tag_ids: Some(vec!["tag-1".to_owned()]),
            important: Some(true),
            sort_order: Some(2.5),
            ..TodoPatch::default()
        });

        let read_back = store
            .with_log(|log| {
                log.append(&carried)?;
                log.changes_since(0)
            })
            .expect("round trip the patch")
            .remove(0);

        let patch = read_back.patch.expect("the patch survives");
        assert!(matches!(
            patch.status,
            Some(todo_contracts::TodoStatus::Completed)
        ));
        assert_eq!(patch.completed_at.as_deref(), Some("2026-07-23T10:00:00Z"));
        assert_eq!(patch.tag_ids, Some(vec!["tag-1".to_owned()]));
        assert_eq!(patch.important, Some(true));
        assert_eq!(patch.sort_order, Some(2.5));
        // The fields the operation did not carry must stay absent: "unchanged"
        // and "cleared" are different instructions to the merge on the device.
        assert!(patch.title.is_none());
        assert!(patch.due_date.is_none());
        assert!(patch.notes.is_none());
        assert!(patch.subtasks.is_none());
    }

    #[test]
    fn a_delete_is_stored_without_a_patch() {
        let store = SyncStore::in_memory().expect("open an in-memory log");
        let mut removal = change("operation-1", 1);
        removal.kind = SyncOperationKind::Delete;
        removal.patch = None;

        let read_back = store
            .with_log(|log| {
                log.append(&removal)?;
                log.changes_since(0)
            })
            .expect("round trip the deletion")
            .remove(0);

        assert!(matches!(read_back.kind, SyncOperationKind::Delete));
        assert!(read_back.patch.is_none());
    }

    #[test]
    fn changes_up_to_the_cursor_are_not_sent_again() {
        let store = SyncStore::in_memory().expect("open an in-memory log");

        let changes = store
            .with_log(|log| {
                log.append(&change("operation-1", 1))?;
                log.append(&change("operation-2", 2))?;
                log.append(&change("operation-3", 3))?;
                log.changes_since(2)
            })
            .expect("read past the cursor");

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].operation_id, "operation-3");
    }

    #[test]
    fn a_failed_action_leaves_the_log_as_it_was() {
        let store = SyncStore::in_memory().expect("open an in-memory log");

        let outcome = store.with_log(|log| {
            log.append(&change("operation-1", 1))?;
            Err::<(), _>(SyncStoreError::UnusableRevision(-1))
        });

        assert!(outcome.is_err());
        let latest = store
            .with_log(|log| log.latest_revision())
            .expect("read the latest revision");
        assert_eq!(latest, 0, "the rolled back append must not have been kept");
    }

    #[test]
    fn an_unreadable_row_is_skipped_instead_of_failing_the_whole_read() {
        let store = SyncStore::in_memory().expect("open an in-memory log");

        let changes = store
            .with_log(|log| {
                log.append(&change("operation-1", 1))?;
                log.append(&change("operation-2", 2))?;
                log.append(&change("operation-3", 3))?;
                // Bytes no build can read back: a patch that is not JSON and a
                // kind that names neither action.
                log.connection.execute(
                    "UPDATE sync_changes SET patch = ?1 WHERE revision = 2",
                    params!["{not json"],
                )?;
                log.connection.execute(
                    "UPDATE sync_changes SET kind = 'sideways' WHERE revision = 3",
                    [],
                )?;
                log.changes_since(0)
            })
            .expect("read a log with unreadable rows");

        let ids: Vec<&str> = changes
            .iter()
            .map(|change| change.operation_id.as_str())
            .collect();
        assert_eq!(ids, vec!["operation-1"]);

        // The bytes are still in the log — the read skipped them, it did not
        // clear them, and nothing ever rewrites an appended row.
        let kept: i64 = store
            .with_log(|log| {
                log.connection
                    .query_row("SELECT COUNT(*) FROM sync_changes", [], |row| row.get(0))
                    .map_err(SyncStoreError::from)
            })
            .expect("count the rows");
        assert_eq!(kept, 3);
    }
}
