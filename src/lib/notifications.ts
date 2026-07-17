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
 * The Tauri notification plugin's JS API lets us send an immediate toast but
 * does not expose a front-end scheduler. So the frontend owns timing: on every
 * reminder change it computes the delay to `reminderAt` and uses `setTimeout`
 * to fire the toast when the app is running. Past-due reminders fire
 * immediately on the next launch.
 *
 * Native background scheduling (the Rust `NotificationBuilder::schedule` API)
 * can be layered on later without changing this module's surface.
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

const MAX_DELAY_MS = 2 ** 31 - 1; // ~24.8 days, the setTimeout ceiling.
const timers = new Map<string, ReturnType<typeof setTimeout>>();

function dueBody(reminder: Reminder): string {
  if (!reminder.dueDate) return "提醒时间到了";
  return `截止日：${reminder.dueDate}`;
}

function clearTimer(id: string): void {
  const existing = timers.get(id);
  if (existing !== undefined) {
    clearTimeout(existing);
    timers.delete(id);
  }
}

/**
 * Sends an immediate notification (used for ad-hoc messages / tests).
 *
 * Permission is requested silently on first use; if denied the call is a
 * no-op and a warning is logged, never a thrown error.
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
 * Schedules a reminder to fire at its `reminderAt` instant.
 *
 * Implements the product rule: a due date without an explicit reminder time
 * does not fire an active toast. Returns `true` if a toast was scheduled or
 * fired immediately (past-due); `false` if there is nothing to schedule.
 */
export async function scheduleReminder(reminder: Reminder): Promise<boolean> {
  if (!isTauri()) return false;
  clearTimer(reminder.id);

  if (!reminder.reminderAt) return false;

  const fireAt = new Date(reminder.reminderAt);
  if (Number.isNaN(fireAt.getTime())) {
    writeDiagnostic("warn", "Invalid reminderAt; skipping reminder", {
      id: reminder.id,
      reminderAt: reminder.reminderAt,
    });
    return false;
  }

  try {
    if (!(await ensurePermission())) return false;
  } catch {
    return false;
  }

  const delay = fireAt.getTime() - Date.now();

  if (delay <= 0) {
    // Past-due: fire once on launch so late reminders are not silently lost.
    fire(reminder);
    return true;
  }

  const clampedDelay = Math.min(delay, MAX_DELAY_MS);
  const timer = setTimeout(() => {
    timers.delete(reminder.id);
    fire(reminder);
  }, clampedDelay);
  timers.set(reminder.id, timer);

  if (clampedDelay < delay) {
    writeDiagnostic("warn", "Reminder delay clamped to setTimeout ceiling", {
      id: reminder.id,
      reminderAt: reminder.reminderAt,
    });
  }
  return true;
}

/** Cancels any pending reminder for the given id. */
export function cancelReminder(id: string): void {
  clearTimer(id);
}

/** Cancels every pending reminder (used on teardown / full reschedule). */
export function cancelAllReminders(): void {
  for (const timer of timers.values()) clearTimeout(timer);
  timers.clear();
}

function fire(reminder: Reminder): void {
  try {
    sendNotification({ title: reminder.title, body: dueBody(reminder) });
  } catch (error) {
    writeDiagnostic("warn", "Failed to fire reminder", {
      id: reminder.id,
      error: String(error),
    });
  }
}
