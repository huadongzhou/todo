//! Scheduled snapshots of the sync log, and putting one back.
//!
//! A snapshot is a plain SQLite file written by `SyncStore::snapshot_into`, so
//! restoring is "stop the server, put the file in place, start it again" and
//! needs no tool this crate does not ship.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::Duration,
};

use time::OffsetDateTime;
use todo_contracts::SyncCursor;

use crate::{SyncService, SyncStore, SyncStoreError};

/// Where snapshots go when `BACKUP_DIR` says nothing — a directory next to the
/// process, in the same spirit as the default database path: self-hosting stays
/// "run the binary".
pub const DEFAULT_BACKUP_DIRECTORY: &str = "todo-server-backups";

/// How often a snapshot is taken when `BACKUP_INTERVAL_SECONDS` says nothing.
///
/// Hourly: the window of operations that only exist in the live database — the
/// data a lost disk would cost — is at most an hour, and the copy is a file of
/// one row per operation ever accepted, so taking it hourly costs almost
/// nothing. The unit is seconds rather than hours so a restore drill does not
/// have to wait an hour to see the schedule work.
pub const DEFAULT_INTERVAL_SECONDS: u64 = 3600;

/// How many snapshots are kept when `BACKUP_KEEP` says nothing.
///
/// 72 hourly snapshots is three days, which is what it takes to cover a
/// weekend: damage done on Friday and noticed on Monday still has a snapshot
/// from before it. Retention is counted rather than dated on purpose — a server
/// that was down for a week would, under a dated rule, come back and delete the
/// only copies it has.
pub const DEFAULT_KEEP: usize = 72;

const SNAPSHOT_PREFIX: &str = "sync-log-";
const SNAPSHOT_EXTENSION: &str = ".sqlite3";

/// How many snapshots may share one millisecond before naming gives up. Only a
/// drill or a test takes snapshots back to back; the schedule cannot.
const MAX_NAMES_PER_MILLISECOND: u32 = 100;

/// What the snapshot schedule does and where it puts things.
pub struct BackupPolicy {
    /// The directory snapshots are written to, created when missing.
    pub directory: PathBuf,
    /// How long to wait between snapshots.
    pub interval: Duration,
    /// How many snapshots to keep; the oldest beyond it are removed.
    pub keep: usize,
}

#[derive(Debug)]
pub enum BackupError {
    /// The snapshot directory could not be created or listed.
    Directory { path: PathBuf, reason: String },
    /// A file could not be copied, moved or removed.
    File { path: PathBuf, reason: String },
    /// The log or the snapshot refused to be read or copied.
    Store(SyncStoreError),
    /// The snapshot named for a restore is not there.
    MissingSnapshot(PathBuf),
    /// Something still has the log open, so replacing it would be replacing a
    /// file out from under its owner.
    TargetInUse(PathBuf),
}

impl std::fmt::Display for BackupError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Directory { path, reason } => {
                write!(
                    formatter,
                    "the snapshot directory {} is unusable: {reason}",
                    path.display()
                )
            }
            Self::File { path, reason } => {
                write!(
                    formatter,
                    "the file {} is unusable: {reason}",
                    path.display()
                )
            }
            Self::Store(error) => write!(formatter, "{error}"),
            Self::MissingSnapshot(path) => {
                write!(formatter, "there is no snapshot at {}", path.display())
            }
            Self::TargetInUse(path) => {
                write!(
                    formatter,
                    "the sync log at {} is still open; stop the server before restoring",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for BackupError {}

impl From<SyncStoreError> for BackupError {
    fn from(error: SyncStoreError) -> Self {
        Self::Store(error)
    }
}

fn directory_error(path: &Path, error: std::io::Error) -> BackupError {
    BackupError::Directory {
        path: path.to_path_buf(),
        reason: error.to_string(),
    }
}

fn file_error(path: &Path, error: std::io::Error) -> BackupError {
    BackupError::File {
        path: path.to_path_buf(),
        reason: error.to_string(),
    }
}

/// The name a snapshot taken at `moment` goes by, without its extension.
///
/// Fixed width and coarse to fine, so sorting the names by text sorts the
/// snapshots by age — which is all the retention rule below needs to know.
/// Milliseconds are in the name because a drill takes snapshots faster than a
/// second.
fn snapshot_stem(moment: OffsetDateTime) -> String {
    format!(
        "{SNAPSHOT_PREFIX}{:04}{:02}{:02}T{:02}{:02}{:02}{:03}Z",
        moment.year(),
        u8::from(moment.month()),
        moment.day(),
        moment.hour(),
        moment.minute(),
        moment.second(),
        moment.millisecond(),
    )
}

/// A path in `directory` no snapshot occupies yet.
///
/// `VACUUM INTO` refuses a destination that already exists, and refusing to
/// take today's snapshot because one was taken this millisecond would be the
/// wrong answer, so a counter is added when the name is taken.
fn free_snapshot_path(directory: &Path, moment: OffsetDateTime) -> Result<PathBuf, BackupError> {
    let stem = snapshot_stem(moment);
    for attempt in 0..MAX_NAMES_PER_MILLISECOND {
        let name = match attempt {
            0 => format!("{stem}{SNAPSHOT_EXTENSION}"),
            _ => format!("{stem}-{attempt}{SNAPSHOT_EXTENSION}"),
        };
        let candidate = directory.join(name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(BackupError::Directory {
        path: directory.to_path_buf(),
        reason: format!("{stem} is taken {MAX_NAMES_PER_MILLISECOND} times over"),
    })
}

/// Every snapshot in `directory`, oldest first.
fn snapshots_in(directory: &Path) -> Result<Vec<PathBuf>, BackupError> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        // Nothing has been taken yet, so there is nothing to list.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(directory_error(directory, error)),
    };

    let mut snapshots = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| directory_error(directory, error))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        // Anything else in the directory is somebody else's file and is left
        // alone, deleting included.
        if name.starts_with(SNAPSHOT_PREFIX) && name.ends_with(SNAPSHOT_EXTENSION) {
            snapshots.push(entry.path());
        }
    }

    snapshots.sort();
    Ok(snapshots)
}

/// Copies the log into `policy.directory` and answers where it landed.
pub fn take_snapshot(store: &SyncStore, policy: &BackupPolicy) -> Result<PathBuf, BackupError> {
    fs::create_dir_all(&policy.directory)
        .map_err(|error| directory_error(&policy.directory, error))?;
    let path = free_snapshot_path(&policy.directory, OffsetDateTime::now_utc())?;
    store.snapshot_into(&path)?;
    Ok(path)
}

/// Removes the oldest snapshots until at most `keep` are left, and answers with
/// the ones removed.
///
/// `keep` is read as "at least one": a policy that would leave no copies at all
/// is a configuration mistake, and the snapshot just taken is not the place to
/// honour it.
pub fn prune(directory: &Path, keep: usize) -> Result<Vec<PathBuf>, BackupError> {
    let keep = keep.max(1);
    let mut snapshots = snapshots_in(directory)?;
    if snapshots.len() <= keep {
        return Ok(Vec::new());
    }

    let expired: Vec<PathBuf> = snapshots.drain(..snapshots.len() - keep).collect();
    for path in &expired {
        fs::remove_file(path).map_err(|error| file_error(path, error))?;
    }
    Ok(expired)
}

/// What a restore did.
pub struct Restored {
    /// The revision the restored log stops at — the number the next operation
    /// carries on from.
    pub latest_revision: SyncCursor,
    /// The name what was in place was moved to, when there was anything to
    /// move. Sidecars keep their own suffixes next to it, so a restore that
    /// found only a `-wal` reports the name that `-wal` now hangs off rather
    /// than a file that exists.
    ///
    /// Nothing ever removes it: `BACKUP_KEEP` counts snapshots, not the
    /// databases a restore set aside, and deciding a restore went well is the
    /// operator's call to make.
    pub preserved: Option<PathBuf>,
}

/// Moves the database at `target` and its sidecars to `kept`, answering where
/// they went when there was anything to move.
///
/// The sidecars move whether or not the database itself is there. A database
/// file that is gone while its `-wal` remains is one of the ordinary shapes a
/// disaster takes — a bad disk, or an operator who deleted the file they could
/// see — and a `-wal` left behind is replayed into whatever takes the
/// database's name next, which after a restore is the snapshot. Belonging to
/// the same database, it hands back operations the snapshot never held;
/// belonging to an older one, it writes pages into a file SQLite gives no
/// meaning for.
fn move_aside(target: &Path, kept: &Path) -> Result<Option<PathBuf>, BackupError> {
    let mut moved = false;
    for suffix in ["", "-wal", "-shm"] {
        let from = with_suffix(target, suffix);
        if !from.exists() {
            continue;
        }
        let to = with_suffix(kept, suffix);
        fs::rename(&from, &to).map_err(|error| file_error(&from, error))?;
        moved = true;
    }

    Ok(moved.then(|| kept.to_path_buf()))
}

/// Puts `snapshot` in place as the live log at `target`.
///
/// Run with the server stopped: the process that serves requests owns the only
/// connection to `target` (see `SyncStore::with_log`), and swapping the file
/// under it is not something SQLite is asked to survive. That is checked rather
/// than asked for — on POSIX the swap would otherwise succeed and the running
/// server would carry on writing to the file that was moved aside.
///
/// Four things happen before anything is overwritten. The snapshot is opened,
/// checked and read, so a mistyped path, a truncated file or a damaged page is
/// refused instead of replacing good data with rubbish. The live log is probed,
/// so a restore run against a server that is still up is refused rather than
/// half done. The copy is made under a temporary name, so a copy that fails
/// half way does not leave half a database in place, and the name is cleaned up
/// on the way out of every failure past it. And whatever was at `target` is
/// moved aside rather than removed, sidecars included — restoring the wrong
/// snapshot is a mistake an operator gets to undo.
pub fn restore(snapshot: &Path, target: &Path) -> Result<Restored, BackupError> {
    if !snapshot.exists() {
        return Err(BackupError::MissingSnapshot(snapshot.to_path_buf()));
    }

    let copy = SyncStore::open_snapshot(snapshot)?;
    copy.quick_check()?;
    let latest_revision = copy.with_log(|log| log.latest_revision())?;
    drop(copy);

    if SyncStore::is_in_use(target) {
        return Err(BackupError::TargetInUse(target.to_path_buf()));
    }

    let stamp = snapshot_stem(OffsetDateTime::now_utc());
    let staged = with_suffix(target, ".restoring");
    let _ = fs::remove_file(&staged);
    fs::copy(snapshot, &staged).map_err(|error| file_error(&staged, error))?;

    let kept = with_suffix(target, &format!(".before-{stamp}"));
    let preserved = match move_aside(target, &kept) {
        Ok(preserved) => preserved,
        Err(error) => {
            let _ = fs::remove_file(&staged);
            return Err(error);
        }
    };

    if let Err(error) = fs::rename(&staged, target) {
        let _ = fs::remove_file(&staged);
        return Err(file_error(&staged, error));
    }

    Ok(Restored {
        latest_revision,
        preserved,
    })
}

/// `<path><suffix>`, which is how SQLite names sidecars and how the files this
/// module moves aside are named too.
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut extended = path.to_path_buf().into_os_string();
    extended.push(suffix);
    PathBuf::from(extended)
}

/// Runs the schedule on its own thread for the life of the process.
///
/// A thread rather than a Tokio task: a snapshot is blocking work that holds
/// the log's lock, and a task doing it would park a worker the server needs for
/// requests anyway. The first snapshot is taken before the first wait, so a
/// misconfigured directory is heard about at startup rather than an hour in,
/// and a fresh deployment has a copy from the moment it is up.
///
/// A snapshot that fails is logged and the schedule carries on. Opening the log
/// at startup is fatal because a server without it answers wrongly; a backup
/// that did not happen makes no answer wrong, and stopping the service over it
/// would turn a missing copy into an outage.
pub fn spawn_scheduler(service: Arc<SyncService>, policy: BackupPolicy) -> thread::JoinHandle<()> {
    thread::spawn(move || loop {
        match take_snapshot(service.store(), &policy) {
            Ok(path) => {
                tracing::info!(path = %path.display(), "sync log snapshot taken");
                match prune(&policy.directory, policy.keep) {
                    Ok(expired) if !expired.is_empty() => {
                        tracing::info!(
                            removed = expired.len(),
                            kept = policy.keep,
                            "expired sync log snapshots removed"
                        );
                    }
                    Ok(_) => {}
                    Err(error) => {
                        tracing::error!(%error, "expired sync log snapshots could not be removed");
                    }
                }
            }
            Err(error) => tracing::error!(%error, "the sync log could not be snapshotted"),
        }

        thread::sleep(policy.interval);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory no other test shares.
    struct TempDirectory {
        path: PathBuf,
    }

    impl TempDirectory {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("todo-server-backup-{name}-{}", std::process::id()));
            let directory = Self { path };
            directory.remove();
            std::fs::create_dir_all(&directory.path).expect("create the directory");
            directory
        }

        fn remove(&self) {
            let _ = fs::remove_dir_all(&self.path);
        }

        fn file(&self, name: &str) -> PathBuf {
            self.path.join(name)
        }
    }

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            self.remove();
        }
    }

    fn policy(directory: &TempDirectory, keep: usize) -> BackupPolicy {
        BackupPolicy {
            directory: directory.path.clone(),
            interval: Duration::from_secs(DEFAULT_INTERVAL_SECONDS),
            keep,
        }
    }

    fn log_with(operations: u32) -> SyncStore {
        let store = SyncStore::in_memory().expect("open an in-memory log");
        store
            .with_log(|log| {
                for revision in 1..=operations {
                    log.append(&todo_contracts::TodoSyncChange {
                        operation_id: format!("operation-{revision}"),
                        todo_id: "todo-1".to_owned(),
                        kind: todo_contracts::SyncOperationKind::Upsert,
                        occurred_at: "2026-07-23T00:00:00Z".to_owned(),
                        patch: None,
                        revision,
                    })?;
                }
                Ok::<_, SyncStoreError>(())
            })
            .expect("fill the log");
        store
    }

    fn revisions_in(path: &Path) -> SyncCursor {
        SyncStore::open_snapshot(path)
            .expect("open the log")
            .with_log(|log| log.latest_revision())
            .expect("read the latest revision")
    }

    #[test]
    fn snapshot_names_sort_oldest_first() {
        let earlier = snapshot_stem(OffsetDateTime::from_unix_timestamp(1_784_000_000).unwrap());
        let later = snapshot_stem(OffsetDateTime::from_unix_timestamp(1_784_003_600).unwrap());

        assert!(earlier < later, "{earlier} should sort before {later}");
        assert!(earlier.starts_with(SNAPSHOT_PREFIX));
    }

    #[test]
    fn two_snapshots_in_the_same_millisecond_get_two_names() {
        let directory = TempDirectory::new("same-millisecond");
        let store = log_with(1);

        let first = take_snapshot(&store, &policy(&directory, DEFAULT_KEEP)).expect("first");
        let second = take_snapshot(&store, &policy(&directory, DEFAULT_KEEP)).expect("second");

        assert_ne!(first, second);
        assert_eq!(revisions_in(&first), 1);
        assert_eq!(revisions_in(&second), 1);
    }

    #[test]
    fn pruning_keeps_the_newest_snapshots_and_removes_the_rest() {
        let directory = TempDirectory::new("prune");
        for name in ["sync-log-1", "sync-log-2", "sync-log-3", "sync-log-4"] {
            fs::write(directory.file(&format!("{name}{SNAPSHOT_EXTENSION}")), "x")
                .expect("write a snapshot");
        }

        let expired = prune(&directory.path, 2).expect("prune");

        let left: Vec<String> = snapshots_in(&directory.path)
            .expect("list")
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(expired.len(), 2);
        assert_eq!(
            left,
            vec![
                "sync-log-3.sqlite3".to_owned(),
                "sync-log-4.sqlite3".to_owned()
            ]
        );
    }

    #[test]
    fn pruning_never_leaves_the_directory_empty() {
        let directory = TempDirectory::new("prune-zero");
        fs::write(directory.file("sync-log-1.sqlite3"), "x").expect("write a snapshot");

        prune(&directory.path, 0).expect("prune");

        assert_eq!(snapshots_in(&directory.path).expect("list").len(), 1);
    }

    #[test]
    fn pruning_leaves_files_that_are_not_snapshots_alone() {
        let directory = TempDirectory::new("prune-strangers");
        fs::write(directory.file("sync-log-1.sqlite3"), "x").expect("write a snapshot");
        fs::write(directory.file("sync-log-2.sqlite3"), "x").expect("write a snapshot");
        fs::write(directory.file("notes.txt"), "x").expect("write a stranger");

        prune(&directory.path, 1).expect("prune");

        assert!(directory.file("notes.txt").exists());
        assert!(directory.file("sync-log-2.sqlite3").exists());
        assert!(!directory.file("sync-log-1.sqlite3").exists());
    }

    #[test]
    fn a_restore_puts_the_snapshot_in_place_and_keeps_what_was_there() {
        let directory = TempDirectory::new("restore");
        let snapshot = take_snapshot(&log_with(2), &policy(&directory, DEFAULT_KEEP))
            .expect("take a snapshot");

        // A live log that has moved on past the snapshot, so telling the two
        // apart afterwards is possible.
        let live = directory.file("live.db");
        SyncStore::open(&live)
            .expect("open the live log")
            .with_log(|log| {
                for revision in 1..=5 {
                    log.append(&todo_contracts::TodoSyncChange {
                        operation_id: format!("live-{revision}"),
                        todo_id: "todo-9".to_owned(),
                        kind: todo_contracts::SyncOperationKind::Upsert,
                        occurred_at: "2026-07-23T01:00:00Z".to_owned(),
                        patch: None,
                        revision,
                    })?;
                }
                Ok::<_, SyncStoreError>(())
            })
            .expect("fill the live log");

        let restored = restore(&snapshot, &live).expect("restore");

        assert_eq!(restored.latest_revision, 2);
        assert_eq!(revisions_in(&live), 2);
        let preserved = restored.preserved.expect("the live log is kept");
        assert_eq!(
            revisions_in(&preserved),
            5,
            "the database that was replaced must still be readable"
        );
    }

    #[test]
    fn a_restore_takes_stale_sidecars_away_even_when_the_database_is_gone() {
        let directory = TempDirectory::new("restore-sidecars");
        let snapshot = take_snapshot(&log_with(3), &policy(&directory, DEFAULT_KEEP))
            .expect("take a snapshot");

        // The shape a lost database file leaves behind: no database, but the
        // sidecars of the log that was there, holding work the snapshot does
        // not have. Left in place they are replayed into whatever takes the
        // database's name next.
        let live = directory.file("live.db");
        fs::write(with_suffix(&live, "-wal"), "a write-ahead log").expect("write a sidecar");
        fs::write(with_suffix(&live, "-shm"), "an index").expect("write a sidecar");

        let restored = restore(&snapshot, &live).expect("restore");

        assert_eq!(restored.latest_revision, 3);
        assert!(
            !with_suffix(&live, "-wal").exists(),
            "a sidecar next to the restored log would be replayed into it"
        );
        assert!(!with_suffix(&live, "-shm").exists());
        assert_eq!(revisions_in(&live), 3);

        let preserved = restored
            .preserved
            .expect("the sidecars are kept, not dropped");
        assert!(with_suffix(&preserved, "-wal").exists());
        assert!(with_suffix(&preserved, "-shm").exists());
    }

    #[test]
    fn a_restore_refuses_while_the_log_is_still_open() {
        let directory = TempDirectory::new("restore-in-use");
        let snapshot = take_snapshot(&log_with(2), &policy(&directory, DEFAULT_KEEP))
            .expect("take a snapshot");

        // A server that was not stopped: a connection to the live log, held
        // open the way a running process holds it.
        let live = directory.file("live.db");
        let running = SyncStore::open(&live).expect("open the live log");
        running
            .with_log(|log| {
                log.append(&todo_contracts::TodoSyncChange {
                    operation_id: "live-1".to_owned(),
                    todo_id: "todo-9".to_owned(),
                    kind: todo_contracts::SyncOperationKind::Upsert,
                    occurred_at: "2026-07-23T01:00:00Z".to_owned(),
                    patch: None,
                    revision: 1,
                })
            })
            .expect("fill the live log");

        let outcome = restore(&snapshot, &live);

        assert!(matches!(outcome, Err(BackupError::TargetInUse(_))));
        assert!(
            !with_suffix(&live, ".restoring").exists(),
            "a refused restore leaves nothing behind"
        );
        assert_eq!(
            running
                .with_log(|log| log.latest_revision())
                .expect("read the live log"),
            1,
            "the log the running server holds must be untouched"
        );
    }

    #[test]
    fn a_restore_refuses_a_snapshot_that_is_not_a_log_and_changes_nothing() {
        let directory = TempDirectory::new("restore-rubbish");
        let live = directory.file("live.db");
        SyncStore::open(&live).expect("open the live log");
        let rubbish = directory.file("truncated.sqlite3");
        fs::write(&rubbish, "SQLite format 3\0 and then nothing").expect("write rubbish");

        let outcome = restore(&rubbish, &live);

        assert!(outcome.is_err());
        assert!(live.exists(), "the live log must still be in place");
        assert_eq!(revisions_in(&live), 0);
    }

    #[test]
    fn a_restore_refuses_a_snapshot_whose_pages_are_damaged() {
        let directory = TempDirectory::new("restore-damaged");
        // Big enough to spill past the first pages, so there is somewhere to
        // damage that reading the last revision does not go.
        let snapshot = take_snapshot(&log_with(400), &policy(&directory, DEFAULT_KEEP))
            .expect("take a snapshot");
        let mut bytes = fs::read(&snapshot).expect("read the snapshot");
        let page = 4096;
        assert!(bytes.len() > page * 3, "the snapshot needs pages to damage");
        // A page in the middle: neither the header nor the root of the table,
        // and not the rightmost leaf the last revision is read from.
        let damaged = page * (bytes.len() / page / 2);
        for byte in &mut bytes[damaged..damaged + 512] {
            *byte = 0xff;
        }
        fs::write(&snapshot, &bytes).expect("damage the snapshot");

        // The check the restore used to do — open it, read the last revision —
        // still passes: that answer comes off the rightmost pages and never
        // touches the damage.
        assert_eq!(revisions_in(&snapshot), 400);

        let live = directory.file("live.db");
        SyncStore::open(&live).expect("open the live log");
        let outcome = restore(&snapshot, &live);

        assert!(
            matches!(outcome, Err(BackupError::Store(SyncStoreError::Damaged(_)))),
            "a snapshot that cannot be read whole must not replace a log"
        );
        assert_eq!(revisions_in(&live), 0, "the live log must be untouched");
    }

    #[test]
    fn a_restore_refuses_a_snapshot_that_is_not_there() {
        let directory = TempDirectory::new("restore-missing");
        let live = directory.file("live.db");

        let outcome = restore(&directory.file("nothing.sqlite3"), &live);

        assert!(matches!(outcome, Err(BackupError::MissingSnapshot(_))));
        assert!(!live.exists());
    }
}
