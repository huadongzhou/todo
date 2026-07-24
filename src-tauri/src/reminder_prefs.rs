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
use todo_domain::quiet::QuietWindow;

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

/// Reads a string preference, degrading to `default` the same way [`read_bool`]
/// does: a store that will not open, a missing key, or a value of the wrong type
/// all fall back rather than drag a reminder behaviour down. The quiet-window
/// times are read through this.
fn read_string(app: &tauri::AppHandle, key: &str, default: &str) -> String {
    let store = match app.store(SETTINGS_FILE) {
        Ok(store) => store,
        Err(error) => {
            log::warn!("Unable to open settings store to read `{key}`: {error}");
            return default.to_owned();
        }
    };
    store
        .get(key)
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| default.to_owned())
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

/// Whether the do-not-disturb window is on (提醒通知/05).
///
/// Off by default, matching the view layer's `reminder.quiet.enabled` default:
/// a silent window holds back system notifications — a change with consequences,
/// like the rollover — so it stays off until the user turns it on. When it is off
/// the scheduler never reads the window, so a reminder is delivered exactly as it
/// is today.
pub fn quiet_hours_enabled(app: &tauri::AppHandle) -> bool {
    read_bool(app, "reminder.quiet.enabled", false)
}

/// The do-not-disturb window, parsed from the two `"HH:mm"` times the view layer
/// stores, or `None` when either is missing, unreadable, or equal to the other.
///
/// The defaults (22:00–08:00) match what the settings store seeds, so a user who
/// turns the switch on before ever touching the times gets that window. A `None`
/// is the scheduler's cue to fail open — deliver rather than silence — which is
/// what a blank or start-equals-end window must never do to a reminder (母任务
/// "a reminder is never lost"). Reading the switch is left to
/// [`quiet_hours_enabled`]; this only reads the span.
pub fn quiet_hours_window(app: &tauri::AppHandle) -> Option<QuietWindow> {
    let start = read_string(app, "reminder.quiet.start", "22:00");
    let end = read_string(app, "reminder.quiet.end", "08:00");
    QuietWindow::parse(&start, &end)
}
