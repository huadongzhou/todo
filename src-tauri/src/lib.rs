mod todo_db;
// Desktop only: the native reminder scheduler is a background thread, which a
// mobile process the OS suspends cannot rely on — mobile local notifications are
// their own scheduling work (TODO.md 2.3), which will plug in here.
#[cfg(desktop)]
mod reminder_scheduler;

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use specta::Type;
#[cfg(any(debug_assertions, test))]
use specta_typescript::Typescript;
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_store::StoreExt;
use tauri_specta::{collect_commands, Builder};
use todo_contracts::{RecurrenceRule, Todo};
use todo_domain::{habit, ics, recurrence};
#[cfg(test)]
use todo_contracts::{
    Attachment, AttachmentKind, HealthResponse, RecurrenceCalendar, RecurrenceFrequency, Subtask,
    SyncOperationKind, SyncRequest, SyncResponse, TodoPatch, TodoStatus, TodoSyncChange,
    TodoSyncOperation, Weekday,
};
use ts_rs::TS;
#[cfg(test)]
use ts_rs::Config;

use todo_db::{PendingWriteReport, TodoDb, WriteRejection};

#[derive(Clone, Debug, Serialize, TS, Type)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInfo {
    pub platform: String,
    pub app_version: String,
}

/// Returns non-sensitive runtime metadata for a typed IPC smoke test.
#[tauri::command]
#[specta::specta]
fn get_runtime_info() -> RuntimeInfo {
    RuntimeInfo {
        platform: std::env::consts::OS.to_owned(),
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
    }
}

/// Returns every locally stored todo, newest first.
#[tauri::command]
#[specta::specta]
fn list_todos(db: tauri::State<'_, TodoDb>) -> Result<Vec<Todo>, String> {
    db.list().map_err(|error| error.to_string())
}

/// Inserts a todo or overwrites the stored row with the same id.
///
/// The error carries whether the refusal is permanent so the view layer can
/// tell "the database is busy, keep the write and retry" from "the contract
/// refuses this todo, no retry will help" — parking the second in the retry
/// journal would promise the user a write that can never land.
#[tauri::command]
#[specta::specta]
fn save_todo(db: tauri::State<'_, TodoDb>, todo: Todo) -> Result<(), WriteRejection> {
    db.save(&todo).map_err(WriteRejection::from)
}

/// Removes a todo by id; removing an unknown id succeeds.
#[tauri::command]
#[specta::specta]
fn delete_todo(db: tauri::State<'_, TodoDb>, id: String) -> Result<(), String> {
    db.delete(&id).map_err(|error| error.to_string())
}

/// The next date a repeat rule lands on strictly after `after`, or `null` when
/// the series has ended.
///
/// The rule engine is a pure `todo-domain` function and the app's single source
/// of "when does this repeat"; the view layer reaches it through this command to
/// generate a recurring task's next instance rather than reimplementing the rule
/// in TypeScript, which would be a second interpretation waiting to disagree with
/// the exported calendar. Only the date is returned: the engine's working-day
/// caveat is a display concern the list does not carry, and generating the next
/// instance needs the date alone. `anchor` and `after` are local `YYYY-MM-DD`
/// dates, the shape a due date is stored in.
#[tauri::command]
#[specta::specta]
fn next_occurrence(
    rule: RecurrenceRule,
    anchor: String,
    after: String,
) -> Result<Option<String>, String> {
    recurrence::next_occurrence(&rule, &anchor, &after)
        .map(|occurrence| occurrence.map(|occurrence| occurrence.date))
        .map_err(|error| error.to_string())
}

/// A habit's streak and the recent scheduled days it could still be checked in
/// on.
///
/// `streak` is how many scheduled days it has been kept up in a row; `makeup` are
/// the recent days that fell due unchecked, most recent first, for the row's
/// make-up buttons. Empty for a rule that is not a daily or weekly habit.
#[derive(Clone, Debug, Serialize, TS, Type)]
#[serde(rename_all = "camelCase")]
pub struct HabitProgress {
    pub streak: u32,
    pub makeup: Vec<String>,
}

/// The streak and recent make-up days for a daily or weekly habit.
///
/// Habits are shown by advancing a single row and recording each check-in's
/// scheduled date rather than leaving completed siblings behind (任务管理/09), so
/// the streak is not a stored number but is counted afresh from the check-in set
/// over the schedule. The view layer reaches this so that counting stays in the
/// one engine that reads the calendar, never a second reading in TypeScript.
/// `due_date` is the task's current due date (its next occurrence) and
/// `check_ins` the scheduled dates already checked, both local `YYYY-MM-DD`.
#[tauri::command]
#[specta::specta]
fn habit_progress(
    rule: RecurrenceRule,
    due_date: String,
    check_ins: Vec<String>,
) -> Result<HabitProgress, String> {
    habit::habit_progress(&rule, &due_date, &check_ins)
        .map(|progress| HabitProgress {
            streak: progress.streak,
            makeup: progress.makeup,
        })
        .map_err(|error| error.to_string())
}

/// Opens a native file picker and hands back the chosen file's path, or `null`
/// when the user cancels. Single selection: one attachment references one file.
///
/// Async on purpose, so the blocking dialog runs off the main thread — a
/// synchronous command would block the very event loop the dialog needs and
/// deadlock. The path is the user's own choice; it is only ever stored as an
/// attachment's location and later handed back to `open_path` to open with the
/// default program — this process never executes it.
#[tauri::command]
#[specta::specta]
async fn pick_attachment_file(app: tauri::AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .blocking_pick_file()
        .map(|path| path.to_string())
}

/// Opens a URL in the system default browser, reporting a capturable failure
/// rather than swallowing it.
///
/// The opener's free function hands the URL to the OS as a single argument (it
/// uses `ShellExecuteW`/`xdg-open`, never `cmd /c start`), so a crafted URL
/// cannot inject a shell command — and it needs no app state, so this command
/// takes none. Which schemes are worth opening (`javascript:`/`data:` are
/// refused) is the view layer's call; this only opens what it is given and
/// returns the reason on failure so the caller can say so instead of throwing.
#[tauri::command]
#[specta::specta]
fn open_url(url: String) -> Result<(), String> {
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(|error| error.to_string())
}

/// Opens a file with the system default program, reporting a capturable failure.
///
/// The path is the one the user picked from the native dialog; the opener hands
/// it to the OS as a single argument (no shell) to open with its default
/// program — this code never runs it as a command. The error is returned rather
/// than thrown so a file that has been moved or deleted degrades to a message the
/// caller can show: the free function checks `metadata()` first, so a missing
/// path is an `Err` before anything is launched (任务管理/11: a missing file must
/// not crash).
#[tauri::command]
#[specta::specta]
fn open_path(path: String) -> Result<(), String> {
    tauri_plugin_opener::open_path(path, None::<&str>).map_err(|error| error.to_string())
}

/// Applies the writes the view layer is still holding outside SQLite: todos it
/// had to park in its fallback store and deletions it could not perform.
///
/// Every entry is applied on its own, so one unusable row cannot block the
/// rest; the report says which ids the caller may drop from its journal and
/// which it must keep (with the reason). An `Err` means the database itself is
/// unusable and nothing at all was applied.
#[tauri::command]
#[specta::specta]
fn replay_pending_writes(
    db: tauri::State<'_, TodoDb>,
    upserts: Vec<Todo>,
    deletions: Vec<String>,
) -> Result<PendingWriteReport, String> {
    db.replay_pending(&upserts, &deletions)
        .map_err(|error| error.to_string())
}

/// Where an export landed, and how much of it there was.
///
/// `path` is absent for the one outcome that is neither success nor failure: no
/// task carried a date, so there was nothing to put in a file and no file was
/// written. Saying that with an absent path rather than an error keeps "nothing
/// to export" out of the error channel, where the view layer would have to tell
/// it apart from a disk that refused the write.
///
/// `unrepeatable_recurrences` is how many of those events carry a repeat rule
/// RFC 5545 cannot express and therefore appear once instead of repeating. It
/// travels with the result so the view layer can say so: a repeat the file drops
/// without a word is a repeat the user believes they exported.
#[derive(Clone, Debug, Serialize, TS, Type)]
#[serde(rename_all = "camelCase")]
pub struct CalendarExport {
    pub path: Option<String>,
    pub event_count: u32,
    pub unrepeatable_recurrences: u32,
}

/// Writes the tasks that carry a date to an .ics file and says where it went.
///
/// The tasks come from the caller rather than from the database on purpose: the
/// view layer holds writes the database has not taken yet (see
/// `replay_pending_writes`), so reading here would export a list the user is not
/// looking at. Building the document is a pure rule and lives in `todo-domain`;
/// this command only supplies the clock and the file system.
///
/// `zone_offset_minutes` is where the device sits, in minutes **east** of UTC —
/// the negation of JavaScript's `getTimezoneOffset()`, which counts west. It
/// comes from the caller rather than from this process because the view layer is
/// what turns a wall-clock time the user typed into the stored instant, and a
/// repeat rule's end date is a local date that has to be read back in the same
/// zone; a device west of UTC would otherwise export a rule that stops one
/// repetition early.
#[tauri::command]
#[specta::specta]
fn export_calendar(
    app: tauri::AppHandle,
    todos: Vec<Todo>,
    zone_offset_minutes: i32,
) -> Result<CalendarExport, String> {
    export_calendar_to_disk(&app, &todos, i64::from(zone_offset_minutes) * 60)
        .map_err(|error| error.to_string())
}

fn export_calendar_to_disk(
    app: &tauri::AppHandle,
    todos: &[Todo],
    zone_offset_seconds: i64,
) -> Result<CalendarExport, CalendarExportError> {
    let generated_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or_default();
    let calendar = ics::build_calendar(todos, generated_at, zone_offset_seconds);

    if calendar.event_count == 0 {
        return Ok(CalendarExport {
            path: None,
            event_count: 0,
            unrepeatable_recurrences: 0,
        });
    }

    let directory = export_directory(app)?;
    std::fs::create_dir_all(&directory)
        .map_err(|error| CalendarExportError::Directory(error.to_string()))?;
    let path = write_new_file(&directory, generated_at, &calendar.content)?;

    Ok(CalendarExport {
        path: Some(path.display().to_string()),
        event_count: calendar.event_count as u32,
        unrepeatable_recurrences: calendar.unrepeatable_recurrences as u32,
    })
}

/// How many names to try before giving up. A user who exports more than this
/// many times inside one second is not a case worth looping over forever.
const MAX_EXPORT_NAME_ATTEMPTS: u32 = 64;

/// Writes the document under a name nothing else holds, and says which one.
///
/// The file is created with `create_new`, so the check for "is this name taken"
/// and the claim on it are the same operation — a plain `exists()` test followed
/// by a write would still overwrite a file another export created in between.
/// The name itself, counter included, is `todo-domain`'s rule.
fn write_new_file(
    directory: &Path,
    generated_at: i64,
    content: &str,
) -> Result<PathBuf, CalendarExportError> {
    write_new_file_with(directory, generated_at, |file| {
        file.write_all(content.as_bytes())
    })
}

/// The body of `write_new_file`, with the write itself passed in so a test can
/// make it fail — the failures this has to survive (a full disk, a volume
/// pulled out mid-write) cannot be staged on a normal machine.
fn write_new_file_with<W>(
    directory: &Path,
    generated_at: i64,
    write: W,
) -> Result<PathBuf, CalendarExportError>
where
    W: FnOnce(&mut std::fs::File) -> std::io::Result<()>,
{
    for attempt in 0..MAX_EXPORT_NAME_ATTEMPTS {
        let path = directory.join(ics::export_file_name(generated_at, attempt));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                let Err(error) = write(&mut file) else {
                    return Ok(path);
                };
                // Creating the name and filling it are two steps, so a failed
                // write leaves an empty or half-written file behind. It has to
                // go: a calendar application will happily import it, and the
                // user was just told the export failed. Closing first because
                // on Windows a removal races an open handle.
                drop(file);
                // Whether the removal itself worked changes nothing the user
                // can act on, and it must not replace the reason the export
                // failed — that reason is what the message has to carry.
                let _ = std::fs::remove_file(&path);
                return Err(CalendarExportError::Write(error.to_string()));
            }
            // The name is taken; the next attempt carries a counter.
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(CalendarExportError::Write(error.to_string())),
        }
    }

    Err(CalendarExportError::Write(format!(
        "{MAX_EXPORT_NAME_ATTEMPTS} file names in this folder are already taken"
    )))
}

/// The downloads folder, because that is where a user looks for a file an
/// application just produced. A platform that has none falls back to the app
/// data directory, which is the same directory the todo database lives in and
/// always exists — an export the user has to be told the path of beats no
/// export at all.
fn export_directory(app: &tauri::AppHandle) -> Result<PathBuf, CalendarExportError> {
    if let Ok(directory) = app.path().download_dir() {
        return Ok(directory);
    }
    app.path()
        .app_data_dir()
        .map_err(|error| CalendarExportError::Directory(error.to_string()))
}

#[derive(Debug)]
enum CalendarExportError {
    /// No directory could be resolved or created to write into.
    Directory(String),
    /// The directory was there; the file itself would not be written.
    Write(String),
}

impl std::fmt::Display for CalendarExportError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Directory(message) => {
                write!(formatter, "no folder to export into: {message}")
            }
            Self::Write(message) => write!(formatter, "the calendar file was not written: {message}"),
        }
    }
}

impl std::error::Error for CalendarExportError {}

fn ipc_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            get_runtime_info,
            list_todos,
            save_todo,
            delete_todo,
            replay_pending_writes,
            export_calendar,
            next_occurrence,
            habit_progress,
            pick_attachment_file,
            open_url,
            open_path
        ])
        // The two contract limits a user can reach by typing, so the view layer
        // caps both inputs at the numbers the contract enforces. Sharing the
        // constants keeps them from drifting apart and recreating "the field
        // accepts input the database then refuses".
        .constant("MAX_TITLE_CHARS", todo_contracts::MAX_TITLE_CHARS)
        .constant("MAX_NOTES_CHARS", todo_contracts::MAX_NOTES_CHARS)
}

#[cfg(debug_assertions)]
fn export_type_bindings() {
    ipc_builder()
        .export(Typescript::default(), "../src/bindings/commands.ts")
        .expect("failed to export tauri-specta bindings");
}

/// Reads the user's "close to tray" preference.
///
/// Defaults to `true`: closing the main window hides to the tray instead of exiting.
/// Any missing / malformed value is treated as `true` so the app always degrades
/// toward keeping its desktop presence rather than silently quitting.
fn close_to_tray_enabled(app: &tauri::AppHandle) -> bool {
    let store = match app.store("settings.json") {
        Ok(store) => store,
        Err(error) => {
            log::warn!("Unable to open settings store for close-to-tray check: {error}");
            return true;
        }
    };

    store
        .get("behavior.closeToTray")
        .and_then(|value| value.as_bool())
        .unwrap_or(true)
}

/// Builds the application tray. Returns `None` if the tray cannot be created
/// (e.g. a headless or restricted desktop environment), in which case the app
/// continues without tray features rather than failing to launch.
fn build_tray(app: &tauri::AppHandle) -> Option<()> {
    use tauri::tray::{TrayIconBuilder, TrayIconEvent};

    let tray = TrayIconBuilder::with_id("main")
        .tooltip("待办")
        .icon(app.default_window_icon()?.clone())
        .build(app)
        .map_err(|error| log::warn!("Failed to create system tray icon: {error}"))
        .ok()?;

    let handle = app.clone();
    tray.on_menu_event(move |tray_app, event| {
        let id = event.id().as_ref();
        match id {
            "toggle" | "show" => {
                if let Some(window) = tray_app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                tray_app.exit(0);
            }
            _ => {}
        }
        let _ = handle;
    });

    let handle = app.clone();
    tray.on_tray_icon_event(move |_tray, event| {
        if let TrayIconEvent::DoubleClick { .. } = event {
            if let Some(window) = handle.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    });

    Some(())
}

fn setup_tray(app: &tauri::AppHandle) {
    if build_tray(app).is_none() {
        log::warn!("Running without system tray; close will exit the application");
    }
}

/// Opens the todo database inside the app data directory. A failure here is not
/// fatal: the store degrades to unavailable, the todo commands report an error
/// and the view layer keeps working on its browser storage fallback.
fn setup_todo_db(app: &tauri::AppHandle) -> TodoDb {
    let directory = match app.path().app_data_dir() {
        Ok(directory) => directory,
        Err(error) => {
            log::warn!("Unable to resolve the app data directory for the todo database: {error}");
            return TodoDb::unavailable();
        }
    };

    if let Err(error) = std::fs::create_dir_all(&directory) {
        log::warn!(
            "Unable to create the app data directory {}: {error}",
            directory.display()
        );
        return TodoDb::unavailable();
    }

    TodoDb::open(&directory.join("todos.db"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(debug_assertions)]
    export_type_bindings();

    let ipc_builder = ipc_builder();

    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // The database is managed before the scheduler starts, so the first
            // poll always finds it; the scheduler reads todos and reminders
            // through this one managed store, the same the IPC commands write.
            app.manage(setup_todo_db(app.handle()));
            setup_tray(app.handle());
            // Reminder timing lives natively now (see `reminder_scheduler`), so a
            // reminder fires while the app is only in the tray and a reminder
            // missed while it was closed is re-raised on the next launch — neither
            // of which the webview's `setTimeout` could do.
            #[cfg(desktop)]
            reminder_scheduler::spawn(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            use tauri::WindowEvent;
            if let WindowEvent::CloseRequested { api, .. } = event {
                if close_to_tray_enabled(window.app_handle()) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(ipc_builder.invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running Todo application");
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// A failed write must not leave the half-made file in the folder: the user
    /// is told the export failed, so a `.ics` a calendar application would
    /// import must not be sitting there, and the name it briefly held must be
    /// free again for the next try.
    ///
    /// The write is injected rather than provoked because the real causes (a
    /// full disk, a volume pulled out mid-write) cannot be staged on the
    /// machine that runs the suite; everything around the write — creating the
    /// file, removing it, the reported reason, the name that stays free — is
    /// the shipping code against a real folder.
    #[test]
    fn a_failed_write_leaves_no_file_and_no_taken_name() {
        let directory = std::env::temp_dir().join(format!(
            "todo-ics-write-failure-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("create the folder to export into");
        let generated_at = 1_774_000_000;

        let error = write_new_file_with(&directory, generated_at, |_| {
            Err(std::io::Error::new(
                std::io::ErrorKind::StorageFull,
                "no space left on device",
            ))
        })
        .expect_err("the write was rigged to fail");

        assert!(
            error.to_string().contains("no space left on device"),
            "the user has to be told why the export failed, got {error}"
        );
        let leftovers: Vec<String> = std::fs::read_dir(&directory)
            .expect("read the folder back")
            .map(|entry| entry.expect("read a folder entry").file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .collect();
        assert!(
            leftovers.is_empty(),
            "a failed export left files behind: {leftovers:?}"
        );

        // The name is free again, so the next export is not pushed onto a
        // counter by a file that was never finished.
        let written = write_new_file(&directory, generated_at, "BEGIN:VCALENDAR\r\n")
            .expect("the next export writes");
        assert_eq!(
            written.file_name().and_then(|name| name.to_str()),
            Some(ics::export_file_name(generated_at, 0).as_str())
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    fn recurrence_rule(
        frequency: RecurrenceFrequency,
        calendar: RecurrenceCalendar,
    ) -> RecurrenceRule {
        RecurrenceRule {
            frequency,
            interval: 1,
            weekdays: Vec::new(),
            month_day: None,
            on_last_day: false,
            calendar,
            until: None,
            count: None,
        }
    }

    /// The command is a thin wrapper, so its job is only to reach the engine and
    /// hand back its three answers as the view layer reads them: a date, `null`
    /// for a series that has ended, and an error string rather than a panic for a
    /// date it cannot read. The engine's own correctness is `todo-domain`'s to
    /// test.
    #[test]
    fn next_occurrence_hands_the_engine_answer_to_the_view_layer() {
        let weekly = recurrence_rule(
            RecurrenceFrequency::Weekly,
            RecurrenceCalendar::Gregorian,
        );
        // 2026-07-23 is a Thursday; the next weekly landing is a week on.
        assert_eq!(
            next_occurrence(weekly.clone(), "2026-07-23".to_owned(), "2026-07-23".to_owned()),
            Ok(Some("2026-07-30".to_owned()))
        );

        // A rule that repeats once has no successor: the anchor is its only
        // occurrence, so the command must answer `null`, which is what tells the
        // view layer to complete the task without generating.
        let once = RecurrenceRule {
            count: Some(1),
            ..weekly.clone()
        };
        assert_eq!(
            next_occurrence(once, "2026-07-23".to_owned(), "2026-07-23".to_owned()),
            Ok(None)
        );

        // An unreadable date is an error string, never a panic that would take
        // down the IPC call.
        assert!(next_occurrence(weekly, "not-a-date".to_owned(), "2026-07-23".to_owned()).is_err());
    }

    /// The habit command is a thin wrapper too: it reaches the engine and hands
    /// back the streak and make-up days the row draws, or an error string for a
    /// date it cannot read. The counting itself is `todo-domain`'s to test.
    #[test]
    fn habit_progress_hands_the_engine_answer_to_the_view_layer() {
        let daily = recurrence_rule(RecurrenceFrequency::Daily, RecurrenceCalendar::Gregorian);

        // Due today, the two days before it checked: a streak of two, and today
        // is not among the days offered to make up.
        let progress = habit_progress(
            daily.clone(),
            "2026-07-23".to_owned(),
            vec!["2026-07-21".to_owned(), "2026-07-22".to_owned()],
        )
        .expect("a habit the engine can read");
        assert_eq!(progress.streak, 2);
        assert!(!progress.makeup.contains(&"2026-07-23".to_owned()));

        // An unreadable due date is an error string, not a panic.
        assert!(habit_progress(daily, "not-a-date".to_owned(), Vec::new()).is_err());
    }

    /// Opening an attachment whose file is no longer there must come back as a
    /// capturable error, never a panic or a silent success — that is what lets
    /// the view layer degrade a moved-or-deleted file to a message rather than
    /// crash (任务管理/11 acceptance). The command is a one-line wrapper over
    /// `tauri_plugin_opener::open_path`, whose no-`with` path checks `metadata()`
    /// before launching anything, so a missing path is an `Err` and this test
    /// opens no program.
    #[test]
    fn open_path_reports_a_missing_file_instead_of_launching() {
        let missing = std::env::temp_dir().join(format!(
            "todo-attachment-missing-{}-{:?}.txt",
            std::process::id(),
            std::thread::current().id()
        ));
        // Make certain it really is absent before asking to open it.
        let _ = std::fs::remove_file(&missing);

        let result = open_path(missing.to_string_lossy().into_owned());
        assert!(
            result.is_err(),
            "opening a file that is not there must be a capturable error, got {result:?}"
        );
    }

    fn frontend_bindings_path(file_name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../src/bindings")
            .join(file_name)
    }

    #[test]
    fn export_type_bindings() {
        let models_config = Config::new().with_out_dir(frontend_bindings_path("models"));

        Todo::export(&models_config).expect("failed to export Todo type");
        TodoStatus::export(&models_config).expect("failed to export TodoStatus type");
        TodoPatch::export(&models_config).expect("failed to export TodoPatch type");
        Subtask::export(&models_config).expect("export Subtask type");
        Attachment::export(&models_config).expect("export Attachment type");
        AttachmentKind::export(&models_config).expect("export AttachmentKind type");
        RecurrenceRule::export(&models_config).expect("export RecurrenceRule type");
        RecurrenceFrequency::export(&models_config).expect("export RecurrenceFrequency type");
        RecurrenceCalendar::export(&models_config).expect("export RecurrenceCalendar type");
        Weekday::export(&models_config).expect("export Weekday type");
        SyncOperationKind::export(&models_config).expect("export SyncOperationKind type");
        TodoSyncOperation::export(&models_config).expect("export TodoSyncOperation type");
        TodoSyncChange::export(&models_config).expect("export TodoSyncChange type");
        SyncRequest::export(&models_config).expect("export SyncRequest type");
        SyncResponse::export(&models_config).expect("export SyncResponse type");
        HealthResponse::export(&models_config).expect("export HealthResponse type");
        RuntimeInfo::export(&models_config).expect("export RuntimeInfo type");
        CalendarExport::export(&models_config).expect("export CalendarExport type");
        HabitProgress::export(&models_config).expect("export HabitProgress type");
        ipc_builder()
            .export(Typescript::default(), frontend_bindings_path("commands.ts"))
            .expect("export tauri-specta bindings");
    }
}
