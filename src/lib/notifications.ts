import { isTauri } from "@tauri-apps/api/core";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { writeDiagnostic } from "@/lib/diagnostics";

export interface Reminder {
  readonly id: string;
  readonly title: string;
  readonly dueDate?: string | null;
  readonly reminderAt?: string | null;
}

let permissionState: "unknown" | "granted" | "denied" = "unknown";

/**
 * Desktop reminder platform layer.
 *
 * Reminder *timing* now lives natively. On Tauri desktop a background Rust
 * scheduler (`src-tauri/src/reminder_scheduler.rs`) polls the local database and
 * raises each reminder at its moment, so a reminder fires even while the app is
 * only in the tray, and a reminder missed while the app was closed is re-raised —
 * marked overdue — on the next launch. The webview could do neither: its
 * `setTimeout` only runs while the window is alive.
 *
 * So the scheduling entry points below are deliberately inert. The store still
 * calls them on every reminder change (create, edit, complete, lock / unlock,
 * startup) to state the intent in one platform-agnostic place; the native
 * scheduler realises it by reading the same rows the store just wrote and
 * re-deriving what is due — a completed, archived or prerequisite-locked task
 * raises nothing — rather than being told. Running them in the webview too would
 * raise every reminder twice.
 *
 * Other runtimes have no native scheduler and so no background reminders: browser
 * development never had them (there is no OS toast to raise), and mobile local
 * notifications are their own platform work (TODO.md 2.3). Both stay a no-op here.
 */

async function ensurePermission(): Promise<boolean> {
  if (!isTauri()) return false;

  try {
    if (permissionState === "granted") return true;
    if (permissionState === "denied") return false;

    let granted = await isPermissionGranted();
    if (!granted) {
      const permission = await requestPermission();
      granted = permission === "granted";
    }

    permissionState = granted ? "granted" : "denied";
    if (!granted) {
      writeDiagnostic("warn", "Notification permission not granted");
    }
    return granted;
  } catch (error) {
    permissionState = "denied";
    writeDiagnostic("warn", "Failed to check notification permission", {
      error: String(error),
    });
    return false;
  }
}

/**
 * Sends an immediate notification (used for ad-hoc messages / tests).
 *
 * Permission is requested silently on first use; if denied the call is a
 * no-op and a warning is logged, never a thrown error. This is the one-off toast
 * path, unrelated to reminder scheduling (which is native — see the module
 * comment).
 */
export async function notify(title: string, body?: string): Promise<void> {
  if (!isTauri()) return;

  try {
    if (!(await ensurePermission())) return;
    sendNotification({ title, body });
  } catch (error) {
    writeDiagnostic("warn", "Failed to send notification", { error: String(error) });
  }
}

/**
 * The store's platform-agnostic "arm this reminder" call. Inert by design: the
 * native scheduler owns timing on desktop and re-derives what to raise from the
 * stored rows, so nothing is armed in the webview (which would raise it twice).
 * Returns `false` — this layer scheduled nothing. See the module comment.
 */
export function scheduleReminder(_reminder: Reminder): Promise<boolean> {
  return Promise.resolve(false);
}

/**
 * The store's "stand this reminder down" call. Inert: the native scheduler reads
 * cancellation from the stored rows (a completed, deleted or re-locked task raises
 * nothing), so there is no webview timer to clear.
 */
export function cancelReminder(_id: string): void {}

/** Inert counterpart to {@link cancelReminder} for a full reschedule. */
export function cancelAllReminders(): void {}
