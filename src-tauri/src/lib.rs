mod todo_db;

use serde::Serialize;
use specta::Type;
#[cfg(any(debug_assertions, test))]
use specta_typescript::Typescript;
use tauri::Manager;
use tauri_plugin_store::StoreExt;
use tauri_specta::{collect_commands, Builder};
use todo_contracts::Todo;
#[cfg(test)]
use todo_contracts::{
    Attachment, AttachmentKind, HealthResponse, RecurrenceCalendar, RecurrenceFrequency,
    RecurrenceRule, Subtask, SyncOperationKind, SyncRequest, SyncResponse, TodoPatch, TodoStatus,
    TodoSyncChange, TodoSyncOperation, Weekday,
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

fn ipc_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            get_runtime_info,
            list_todos,
            save_todo,
            delete_todo,
            replay_pending_writes
        ])
        // The title is the one contract limit a user can reach by typing today,
        // so the view layer caps its input at the same number the contract
        // enforces. Sharing the constant keeps the two from drifting apart and
        // recreating "the field accepts input the database then refuses".
        .constant("MAX_TITLE_CHARS", todo_contracts::MAX_TITLE_CHARS)
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
        .setup(|app| {
            app.manage(setup_todo_db(app.handle()));
            setup_tray(app.handle());
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
        ipc_builder()
            .export(Typescript::default(), frontend_bindings_path("commands.ts"))
            .expect("export tauri-specta bindings");
    }
}
