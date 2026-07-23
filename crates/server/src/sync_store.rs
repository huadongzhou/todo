use std::cell::RefCell;
use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, ErrorCode, OpenFlags, OptionalExtension, Row};
use serde::de::{DeserializeSeed, Error as _, IntoDeserializer, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
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
    /// A path cannot be handed to SQLite because it is not valid UTF-8.
    UnusablePath(String),
    /// SQLite read the file and found it damaged; the text is what it said.
    Damaged(String),
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
            Self::UnusablePath(path) => {
                write!(formatter, "the path {path} is not valid UTF-8")
            }
            Self::Damaged(report) => {
                write!(formatter, "the sync log is damaged: {report}")
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

/// The first field name a reading met that the struct it was reading into does
/// not have, and the route to the object that held it.
///
/// The route is empty for the patch itself and `["subtasks", "0"]` for the
/// first subtask, so the name can be dropped from exactly where it was found
/// rather than from every object that happens to use it.
type UnknownField = RefCell<Option<(Vec<String>, String)>>;

/// A `serde_json::Value` read exactly the way serde reads it, with the first
/// field name the target type has no place for noted down.
///
/// Which names a build knows is a question only serde can answer, and it
/// answers it in one spot: the derived `Deserialize` of a struct with
/// `deny_unknown_fields` refuses the *key* of a field it does not have, before
/// the value is ever looked at. A key the seed refuses is therefore a name this
/// build cannot place — at whatever depth the object sits, because the same
/// refusal happens inside a subtask or a recurrence rule as on the patch
/// itself. Keeping the note rather than a list of field names is what stops it
/// drifting from the contract.
///
/// Nothing is skipped or repaired here. The note is a location; the caller
/// removes what it points at and reads again strictly, so the patch that comes
/// out is one plain serde produced.
struct Watched<'a> {
    value: serde_json::Value,
    /// Route from the patch to this value.
    path: Vec<String>,
    found: &'a UnknownField,
}

impl<'de> Deserializer<'de> for Watched<'_> {
    type Error = serde_json::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            serde_json::Value::Object(fields) => visitor.visit_map(WatchedMap {
                fields: fields.into_iter(),
                pending: None,
                path: self.path,
                found: self.found,
            }),
            serde_json::Value::Array(items) => visitor.visit_seq(WatchedSeq {
                items: items.into_iter(),
                index: 0,
                path: self.path,
                found: self.found,
            }),
            // A scalar holds no names, so there is nothing to watch and
            // serde_json reads it itself.
            scalar => Deserializer::deserialize_any(scalar, visitor),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            serde_json::Value::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    /// Enums are handed to serde_json whole.
    ///
    /// Every enum the contract puts in a patch is a set of unit variants named
    /// by a string, so there is no object inside one for a name to hide in. A
    /// variant that carried a struct would be the one place unknown names went
    /// unnoted — which leaves such a row skipped exactly as it was before any
    /// of this, never read as something it does not say.
    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Deserializer::deserialize_enum(self.value, name, variants, visitor)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct newtype_struct seq tuple tuple_struct
        map struct identifier ignored_any
    }
}

struct WatchedMap<'a> {
    fields: serde_json::map::IntoIter,
    /// The entry whose key was just accepted, waiting for its value to be
    /// asked for.
    pending: Option<(String, serde_json::Value)>,
    path: Vec<String>,
    found: &'a UnknownField,
}

impl<'de> MapAccess<'de> for WatchedMap<'_> {
    type Error = serde_json::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        let Some((name, value)) = self.fields.next() else {
            return Ok(None);
        };

        let key: serde::de::value::StrDeserializer<'_, Self::Error> =
            name.as_str().into_deserializer();
        match seed.deserialize(key) {
            Ok(key) => {
                self.pending = Some((name, value));
                Ok(Some(key))
            }
            Err(error) => {
                // A name was refused rather than a value: the struct being read
                // has no field by it. The first one is kept because the reading
                // stops here and starts over once it has been dropped.
                let mut found = self.found.borrow_mut();
                if found.is_none() {
                    *found = Some((self.path.clone(), name));
                }
                Err(error)
            }
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let Some((name, value)) = self.pending.take() else {
            return Err(serde_json::Error::custom(
                "a map value was asked for before its key",
            ));
        };

        let mut path = self.path.clone();
        path.push(name);
        seed.deserialize(Watched {
            value,
            path,
            found: self.found,
        })
    }
}

struct WatchedSeq<'a> {
    items: std::vec::IntoIter<serde_json::Value>,
    index: usize,
    path: Vec<String>,
    found: &'a UnknownField,
}

impl<'de> SeqAccess<'de> for WatchedSeq<'_> {
    type Error = serde_json::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let Some(item) = self.items.next() else {
            return Ok(None);
        };

        let mut path = self.path.clone();
        path.push(self.index.to_string());
        self.index += 1;
        seed.deserialize(Watched {
            value: item,
            path,
            found: self.found,
        })
        .map(Some)
    }
}

/// Removes `name` from the object `path` leads to, answering whether it was
/// there to remove.
fn remove_field(value: &mut serde_json::Value, path: &[String], name: &str) -> bool {
    let mut here = value;
    for step in path {
        let next = match here {
            serde_json::Value::Object(fields) => fields.get_mut(step),
            serde_json::Value::Array(items) => step
                .parse::<usize>()
                .ok()
                .and_then(|index| items.get_mut(index)),
            _ => None,
        };
        match next {
            Some(next) => here = next,
            None => return false,
        }
    }

    match here {
        serde_json::Value::Object(fields) => fields.remove(name).is_some(),
        _ => false,
    }
}

/// How a dropped field is named in the log: `focusMinutes` for one on the patch
/// itself, `subtasks.0.note` for one inside the first subtask.
fn field_path(path: &[String], name: &str) -> String {
    if path.is_empty() {
        return name.to_owned();
    }
    format!("{}.{name}", path.join("."))
}

/// Decodes a stored patch, and says which of its fields this build had no name
/// for.
///
/// `deny_unknown_fields` is right on the wire — a request carrying a field the
/// server cannot honour must be refused rather than half applied — and wrong on
/// the way out of storage. The log is written by whichever build was running at
/// the time, so a server rolled back to a build that predates a contract field
/// would fail to decode every row carrying it. Those rows are skipped while
/// `next_cursor` still counts them, which is worse than a delay: the device
/// advances its cursor past changes it never received and never asks again.
///
/// The strict reading is therefore tried first. When it fails, the reading is
/// repeated under `Watched`, which notes where a name this build cannot place
/// sits; that name is dropped and the strict reading tried again, until it
/// succeeds or there is no such name left to blame. The patch handed back is
/// always one a strict reading produced — the watched pass only ever points at
/// a name.
///
/// Fields that are known but carry unusable values stay in, so such a row still
/// fails: degrading "set the title to <garbage>" into "leave the title alone"
/// would misreport the operation, which is the one thing this must not do. The
/// same holds a level down, where a subtask with an unreadable `done` keeps the
/// whole row out rather than losing the step.
///
/// Dropping is only about this build's answer. The row keeps the bytes it was
/// written with, so rolling forward again serves the whole patch to every
/// device whose cursor is still behind it.
fn patch_from_json(raw: &str) -> Result<(TodoPatch, Vec<String>), serde_json::Error> {
    let strict = match serde_json::from_str::<TodoPatch>(raw) {
        Ok(patch) => return Ok((patch, Vec::new())),
        Err(error) => error,
    };

    let Ok(mut fields) = serde_json::from_str::<serde_json::Value>(raw) else {
        return Err(strict);
    };

    let mut dropped = Vec::new();
    loop {
        match serde_json::from_value::<TodoPatch>(fields.clone()) {
            Ok(patch) if !dropped.is_empty() => return Ok((patch, dropped)),
            // Nothing had to be dropped, so the strict reading of the same
            // bytes should have worked; a disagreement between the two is not
            // something to settle by guessing.
            Ok(_) => return Err(strict),
            Err(_) => {}
        }

        let found = UnknownField::new(None);
        let _ = TodoPatch::deserialize(Watched {
            value: fields.clone(),
            path: Vec::new(),
            found: &found,
        });

        // No unplaceable name means the reading failed over a field this build
        // does know, and guessing what it should have said is not on the table.
        let Some((path, name)) = found.into_inner() else {
            return Err(strict);
        };
        if !remove_field(&mut fields, &path, &name) {
            return Err(strict);
        }
        dropped.push(field_path(&path, &name));
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
        Some(raw) => match patch_from_json(&raw) {
            Ok((patch, unknown)) => {
                if !unknown.is_empty() {
                    tracing::warn!(
                        revision,
                        %operation_id,
                        fields = %unknown.join(", "),
                        "serving a sync log row without the patch fields this build cannot name"
                    );
                }
                Some(patch)
            }
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

    /// Opens an existing log read-only, creating and stamping nothing.
    ///
    /// Restoring is the one moment when the file being opened is not the file
    /// the process serves: a snapshot has to be read before it is put in place
    /// over live data. Read-only is what makes that inspection safe — `open`
    /// would turn a mistyped path into an empty log and `initialise` would
    /// write to the snapshot it was only meant to look at.
    pub fn open_snapshot(path: &Path) -> Result<Self, SyncStoreError> {
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    /// Whether something else still has the log at `path` open.
    ///
    /// A restore replaces the file a running server holds open, and on POSIX
    /// that succeeds quietly: the server carries on writing to the file that
    /// was moved aside, so every operation it accepts afterwards is gone at the
    /// next start. There is no portable way to ask an operating system whether
    /// a file is open, but there is a SQLite way to ask this one — a connection
    /// in exclusive locking mode has to take the database for itself, and while
    /// any other connection holds it, which in WAL mode every open connection
    /// does, that is answered `SQLITE_BUSY`.
    ///
    /// Anything else the probe meets — a missing file, bytes that are not a
    /// database, no permission to open it — answers "not in use". The usual
    /// reason to restore is that the live file is rubbish, and refusing to
    /// replace it because it cannot be read would be the wrong way round.
    pub fn is_in_use(path: &Path) -> bool {
        let Ok(connection) = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) else {
            return false;
        };

        if connection
            .pragma_update(None, "locking_mode", "exclusive")
            .is_err()
        {
            return false;
        }

        // The pragma only states the intent; the lock is taken by the first
        // statement that needs the file, and a transaction that writes nothing
        // is enough to ask for it.
        match connection.execute_batch("BEGIN EXCLUSIVE; ROLLBACK") {
            Ok(()) => false,
            Err(error) => matches!(
                error.sqlite_error_code(),
                Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked)
            ),
        }
    }

    /// Asks SQLite to read the whole file and say whether it is intact.
    ///
    /// `PRAGMA quick_check` is the cheap half of `integrity_check`: it walks
    /// every b-tree rather than only the roots, which is the difference between
    /// "the table is where it should be" and "the rows can be read". Reading
    /// `MAX(revision)` off a snapshot only touches the pages that answer it, so
    /// a snapshot damaged anywhere else would pass that and still be put in
    /// place over good data.
    pub fn quick_check(&self) -> Result<(), SyncStoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| SyncStoreError::Poisoned)?;
        let outcome: String = connection.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
        if outcome == "ok" {
            return Ok(());
        }
        Err(SyncStoreError::Damaged(outcome))
    }

    /// A log that lives only as long as the process, for tests.
    pub fn in_memory() -> Result<Self, SyncStoreError> {
        let connection = Connection::open_in_memory()?;
        initialise(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    /// Writes a self-contained copy of the log to `path`, which must not exist
    /// yet.
    ///
    /// `VACUUM INTO` runs on the one connection this store owns, behind the
    /// same lock every sync request takes, and that is the whole design: what
    /// keeps two requests from handing out the same revision is that the
    /// process owns exactly one connection (see `with_log`). The online backup
    /// API and a read-only replica would both shorten the pause a snapshot
    /// costs by opening a second connection — and would spend the guarantee to
    /// buy it. The pause is bounded by the size of the log, which is one row
    /// per operation ever accepted.
    ///
    /// What lands is a fully checkpointed database: no WAL sidecar travels with
    /// the snapshot, so restoring it is a file copy and not a set of files that
    /// have to stay together.
    pub fn snapshot_into(&self, path: &Path) -> Result<(), SyncStoreError> {
        let destination = path
            .to_str()
            .ok_or_else(|| SyncStoreError::UnusablePath(path.display().to_string()))?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| SyncStoreError::Poisoned)?;
        connection.execute("VACUUM INTO ?1", params![destination])?;
        Ok(())
    }

    /// Runs `action` against the log inside one transaction, committing when it
    /// succeeds and rolling back when it does not.
    ///
    /// One sync request reads the current revision, appends to the log and
    /// reads back what the device is owed, and two requests must never number
    /// an operation with the same revision. What rules that out is the lock
    /// taken here: the process owns exactly one `Connection`, so those steps
    /// run one request at a time. The transaction is what makes a request
    /// all-or-nothing — a failure half way leaves no rows behind — but it is
    /// not what provides the mutual exclusion: `Connection::transaction()` is
    /// `BEGIN DEFERRED`, and a *second* connection reading `MAX(revision)`
    /// before upgrading itself to a writer would be answered
    /// `SQLITE_BUSY_SNAPSHOT`, which no busy handler retries and which no
    /// amount of `busy_timeout` waits out. Anything that would open a second
    /// connection to this file has to be weighed against that. Snapshots do not
    /// (see `snapshot_into`), and a restore runs with the server stopped.
    ///
    /// The caller's error type only has to be able to carry a storage failure,
    /// so deciding whether a request is acceptable stays with the caller
    /// instead of leaking into this module.
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

    #[test]
    fn a_patch_written_by_a_newer_build_is_served_without_the_fields_this_one_lacks() {
        let store = SyncStore::in_memory().expect("open an in-memory log");

        let changes = store
            .with_log(|log| {
                log.append(&change("operation-1", 1))?;
                // What a build one contract field ahead of this one would have
                // written: the fields this build knows, plus one it does not.
                log.connection.execute(
                    "UPDATE sync_changes SET patch = ?1 WHERE revision = 1",
                    params![r#"{"title":"write it down","focusMinutes":25}"#],
                )?;
                log.changes_since(0)
            })
            .expect("read a log written by a newer build");

        assert_eq!(
            changes.len(),
            1,
            "a row a rolled back build cannot fully read must still be served"
        );
        let patch = changes[0].patch.as_ref().expect("the patch survives");
        assert_eq!(patch.title.as_deref(), Some("write it down"));
    }

    #[test]
    fn a_row_whose_newer_field_sits_inside_a_subtask_is_served_too() {
        let store = SyncStore::in_memory().expect("open an in-memory log");

        let changes = store
            .with_log(|log| {
                log.append(&change("operation-1", 1))?;
                // The same rollback, one level down. Every name at the top is
                // one this build knows, so the row only gives itself away when
                // the subtask is read.
                log.connection.execute(
                    "UPDATE sync_changes SET patch = ?1 WHERE revision = 1",
                    params![
                        r#"{"title":"write it down","subtasks":[{"id":"s1","title":"step","done":false,"note":"new"}]}"#
                    ],
                )?;
                log.changes_since(0)
            })
            .expect("read a log written by a newer build");

        assert_eq!(changes.len(), 1, "a row is not a cursor's worth of nothing");
        let patch = changes[0].patch.as_ref().expect("the patch survives");
        assert_eq!(patch.title.as_deref(), Some("write it down"));
        assert_eq!(
            patch
                .subtasks
                .as_ref()
                .map(|subtasks| subtasks.len())
                .unwrap_or_default(),
            1
        );
    }

    #[test]
    fn a_field_a_newer_build_added_inside_a_nested_type_is_dropped_on_its_own() {
        // Contract types nest, and every one of them denies unknown fields, so
        // a field added to a subtask, a recurrence rule or an attachment is
        // just as able to take a whole row down as one added to the patch —
        // and harder to see, because every name at the top level is one this
        // build can place.
        let nested = [
            (
                r#"{"title":"t","subtasks":[{"id":"s1","title":"step","done":false,"note":"new"}]}"#,
                "subtasks.0.note",
            ),
            (
                r#"{"recurrence":{"frequency":"weekly","interval":1,"onLastDay":false,"calendar":"gregorian","weekOfMonth":2}}"#,
                "recurrence.weekOfMonth",
            ),
            (
                r#"{"attachments":[{"id":"a1","kind":"link","url":"https://example.com","caption":"new"}]}"#,
                "attachments.0.caption",
            ),
        ];

        for (raw, expected) in nested {
            let (patch, dropped) =
                patch_from_json(raw).unwrap_or_else(|error| panic!("{raw} was skipped: {error}"));
            assert_eq!(dropped, vec![expected.to_owned()]);
            // What the row does say has to survive intact.
            match expected {
                "subtasks.0.note" => {
                    let subtasks = patch.subtasks.expect("the subtasks survive");
                    assert_eq!(subtasks.len(), 1);
                    assert_eq!(subtasks[0].title, "step");
                }
                "recurrence.weekOfMonth" => {
                    let rule = patch.recurrence.expect("the rule survives");
                    assert_eq!(rule.interval, 1);
                }
                _ => {
                    let attachments = patch.attachments.expect("the attachments survive");
                    assert_eq!(attachments[0].url, "https://example.com");
                }
            }
        }
    }

    #[test]
    fn several_fields_this_build_cannot_name_are_all_reported() {
        let (patch, dropped) = patch_from_json(
            r#"{"title":"t","focusMinutes":25,"subtasks":[{"id":"s1","title":"step","done":true,"note":"new"},{"id":"s2","title":"next","done":false,"owner":"me"}]}"#,
        )
        .expect("a row a rolled back build cannot fully read is still served");

        assert_eq!(
            dropped,
            vec![
                "focusMinutes".to_owned(),
                "subtasks.0.note".to_owned(),
                "subtasks.1.owner".to_owned()
            ]
        );
        assert_eq!(patch.title.as_deref(), Some("t"));
        assert_eq!(patch.subtasks.expect("the subtasks survive").len(), 2);
    }

    #[test]
    fn a_nested_field_whose_value_is_unusable_is_still_skipped() {
        // `done` is a name this build knows, so the row fails over a value it
        // cannot read rather than a name — and reading it as "the step is not
        // done" would report a state the operation never described.
        let outcome =
            patch_from_json(r#"{"subtasks":[{"id":"s1","title":"step","done":"maybe"}]}"#);

        assert!(outcome.is_err());
    }

    #[test]
    fn every_field_the_patch_has_survives_a_relaxed_reading() {
        // Written out in full on purpose: a field added to `TodoPatch` stops
        // this compiling, so the promise "only names this build cannot place
        // are dropped" is re-checked against the contract rather than against a
        // list kept here.
        let carried = TodoPatch {
            title: Some("write it down".to_owned()),
            status: Some(todo_contracts::TodoStatus::Completed),
            due_date: Some("2026-07-24".to_owned()),
            completed_at: Some("2026-07-23T10:00:00Z".to_owned()),
            reminder_at: Some("2026-07-24T09:00:00Z".to_owned()),
            notes: Some("the details".to_owned()),
            start_date: Some("2026-07-22".to_owned()),
            starts_at: Some("2026-07-22T08:00:00Z".to_owned()),
            ends_at: Some("2026-07-22T09:00:00Z".to_owned()),
            estimated_minutes: Some(30),
            recurrence: Some(todo_contracts::RecurrenceRule {
                frequency: todo_contracts::RecurrenceFrequency::Weekly,
                interval: 2,
                weekdays: vec![todo_contracts::Weekday::Monday],
                month_day: None,
                on_last_day: false,
                calendar: todo_contracts::RecurrenceCalendar::Gregorian,
                until: Some("2026-12-31".to_owned()),
                count: Some(10),
            }),
            list_id: Some("list-1".to_owned()),
            important: Some(true),
            urgent: Some(false),
            sort_order: Some(1.5),
            tag_ids: Some(vec!["tag-1".to_owned()]),
            subtasks: Some(vec![todo_contracts::Subtask {
                id: "s1".to_owned(),
                title: "step".to_owned(),
                done: false,
            }]),
            attachments: Some(vec![todo_contracts::Attachment {
                id: "a1".to_owned(),
                kind: todo_contracts::AttachmentKind::Link,
                url: "https://example.com".to_owned(),
                name: Some("the link".to_owned()),
            }]),
            depends_on: Some(vec!["todo-2".to_owned()]),
        };

        let mut written = serde_json::to_value(&carried).expect("serialise the patch");
        // What a build one contract field ahead would have added, at the two
        // depths a name can sit at.
        written["focusMinutes"] = serde_json::json!(25);
        written["subtasks"][0]["note"] = serde_json::json!("new");

        let (read_back, dropped) =
            patch_from_json(&written.to_string()).expect("the row is still served");

        assert_eq!(
            dropped,
            vec!["focusMinutes".to_owned(), "subtasks.0.note".to_owned()]
        );
        assert_eq!(
            serde_json::to_value(&read_back).expect("serialise what came back"),
            serde_json::to_value(&carried).expect("serialise the patch"),
            "every field this build does have a name for must come back unchanged"
        );
    }

    #[test]
    fn a_patch_whose_own_field_is_unusable_is_still_skipped() {
        let store = SyncStore::in_memory().expect("open an in-memory log");

        let changes = store
            .with_log(|log| {
                log.append(&change("operation-1", 1))?;
                // A field this build does know, carrying something it cannot
                // read. Relaxing that into "leave the title alone" would tell
                // every device the operation said something it did not.
                log.connection.execute(
                    "UPDATE sync_changes SET patch = ?1 WHERE revision = 1",
                    params![r#"{"title":42}"#],
                )?;
                log.changes_since(0)
            })
            .expect("read a log with a malformed patch");

        assert!(changes.is_empty());
    }

    #[test]
    fn a_snapshot_holds_the_log_as_it_stood_when_it_was_taken() {
        let database = TempDatabase::new("snapshot-source");
        let snapshot = TempDatabase::new("snapshot-copy");
        let store = database.open();
        store
            .with_log(|log| {
                log.append(&change("operation-1", 1))?;
                log.append(&change("operation-2", 2))
            })
            .expect("append two changes");

        store
            .snapshot_into(&snapshot.path)
            .expect("take a snapshot");

        // The log carries on after the copy is taken; the copy must not follow
        // it.
        store
            .with_log(|log| log.append(&change("operation-3", 3)))
            .expect("append after the snapshot");

        let copy = SyncStore::open_snapshot(&snapshot.path).expect("open the snapshot");
        let (latest, changes) = copy
            .with_log(|log| {
                Ok::<_, SyncStoreError>((log.latest_revision()?, log.changes_since(0)?))
            })
            .expect("read the snapshot");

        assert_eq!(latest, 2);
        let ids: Vec<&str> = changes
            .iter()
            .map(|change| change.operation_id.as_str())
            .collect();
        assert_eq!(ids, vec!["operation-1", "operation-2"]);

        // A snapshot travels alone: `VACUUM INTO` checkpoints as it copies, so
        // there is no sidecar that has to be carried with the file.
        let mut sidecar = snapshot.path.clone().into_os_string();
        sidecar.push("-wal");
        assert!(!std::path::Path::new(&sidecar).exists());
    }

    #[test]
    fn looking_at_a_snapshot_that_is_not_there_does_not_create_one() {
        let database = TempDatabase::new("absent-snapshot");

        assert!(SyncStore::open_snapshot(&database.path).is_err());
        assert!(
            !database.path.exists(),
            "a mistyped snapshot path must not become an empty log"
        );
    }
}
