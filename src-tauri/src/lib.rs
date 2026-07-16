use serde::Serialize;
use specta::Type;
#[cfg(any(debug_assertions, test))]
use specta_typescript::Typescript;
use tauri_specta::{collect_commands, Builder};
#[cfg(test)]
use todo_contracts::{
    HealthResponse, SyncOperationKind, SyncRequest, SyncResponse, Todo, TodoPatch, TodoStatus,
    TodoSyncChange, TodoSyncOperation,
};
use ts_rs::TS;
#[cfg(test)]
use ts_rs::Config;

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

fn ipc_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![get_runtime_info])
}

#[cfg(debug_assertions)]
fn export_type_bindings() {
    ipc_builder()
        .export(Typescript::default(), "../src/bindings/commands.ts")
        .expect("failed to export tauri-specta bindings");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(debug_assertions)]
    export_type_bindings();

    let ipc_builder = ipc_builder();

    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
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
        SyncOperationKind::export(&models_config).expect("failed to export SyncOperationKind type");
        TodoSyncOperation::export(&models_config).expect("failed to export TodoSyncOperation type");
        TodoSyncChange::export(&models_config).expect("failed to export TodoSyncChange type");
        SyncRequest::export(&models_config).expect("failed to export SyncRequest type");
        SyncResponse::export(&models_config).expect("failed to export SyncResponse type");
        HealthResponse::export(&models_config).expect("failed to export HealthResponse type");
        RuntimeInfo::export(&models_config).expect("failed to export RuntimeInfo type");
        ipc_builder()
            .export(Typescript::default(), frontend_bindings_path("commands.ts"))
            .expect("failed to export tauri-specta bindings");
    }
}
