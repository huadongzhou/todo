//! Reading the user's reminder preferences from the settings store — the native
//! side of a preference that has to gate work the webview is not there for.
//!
//! The reminder scheduler runs in a background thread (提醒通知/01), so a
//! reminder it raises while the app is only in the tray cannot ask the webview
//! whether a preference is on. Settings live in the `tauri-plugin-store` file
//! `settings.json` — the same file the view layer writes through
//! `saveBooleanPreference`, and the same one the close-to-tray check already
//! reads from Rust — so this reads them the same way, through `app.store()`. That
//! call hands back the one in-memory store the view layer also holds, so a toggle
//! the user flips takes effect on the scheduler's next poll with no channel of
//! its own to keep in step.
//!
//! This is the shared native reader the rest of 提醒通知 hangs off: 勿扰时段
//! (05, a silent window), 晨间摘要 (06, a daily time) and 时区策略 (07, the
//! device's offset) each add their own accessor here rather than re-opening the
//! store, so there is one place that knows reminders take their preferences from
//! `settings.json` and one place each key's default lives.

use tauri_plugin_store::StoreExt;

/// The settings-store file the view layer persists preferences to.
const SETTINGS_FILE: &str = "settings.json";

/// Reads a boolean preference, degrading to `default` when the store will not
/// open or the key is missing or the wrong type — the same tolerance the
/// close-to-tray check applies, so a settings file that cannot be read never
/// drags a reminder behaviour down with it. The reader 05/06/07 build their own
/// typed accessors on.
fn read_bool(app: &tauri::AppHandle, key: &str, default: bool) -> bool {
    let store = match app.store(SETTINGS_FILE) {
        Ok(store) => store,
        Err(error) => {
            log::warn!("Unable to open settings store to read `{key}`: {error}");
            return default;
        }
    };
    store
        .get(key)
        .and_then(|value| value.as_bool())
        .unwrap_or(default)
}

/// Whether "keep reminding while a task is overdue" is on.
///
/// Off by default, matching the view layer's `reminder.renag` default: renag
/// raises repeat system notifications — more intrusive than the silent rollover —
/// so it stays off until the user turns it on. The key is the one
/// `useSettingsStore` writes.
pub fn renag_overdue_enabled(app: &tauri::AppHandle) -> bool {
    read_bool(app, "reminder.renag", false)
}
