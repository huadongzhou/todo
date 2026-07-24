import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { isTauri } from "@tauri-apps/api/core";
import { applyTheme, onSystemThemeChange, type ThemePreference } from "@/lib/appearance";
import { writeDiagnostic } from "@/lib/diagnostics";
import {
  loadBooleanPreference,
  loadStringPreference,
  loadThemePreference,
  saveBooleanPreference,
  saveStringPreference,
  saveThemePreference,
} from "@/lib/settings-storage";

export const useSettingsStore = defineStore("settings", () => {
  const theme = ref<ThemePreference>("system");
  const closeToTray = ref(true);
  const showTodayCard = ref(false);
  /**
   * Whether each new day pulls overdue tasks forward to it.
   *
   * Off by default: rewriting a deadline the user set by hand is a change with
   * consequences, and nothing records what the date used to be, so an app that
   * did it unasked would leave no way back.
   */
  const rolloverOverdue = ref(false);
  /**
   * Which quick "snooze" durations the reminder bar offers. All on by default:
   * the four cover the common cases, and a user who wants fewer buttons can turn
   * some off. "今晚"/"明天" resolve to fixed local times (20:00 / 09:00) inside
   * the bar; those hours are not configurable here on purpose, so they can later
   * follow the shared "默认提醒时间" / "晨间摘要时间" settings without a second
   * copy drifting out of step.
   */
  const snooze5m = ref(true);
  const snooze1h = ref(true);
  const snoozeTonight = ref(true);
  const snoozeTomorrow = ref(true);
  /**
   * Whether an overdue task keeps reminding until it is done. Off by default:
   * renag raises repeat system notifications, more intrusive than the silent
   * rollover, so it stays off until the user asks for it. Unlike the snooze
   * durations above (which the reminder bar reads here in the webview), this is
   * read natively by the reminder scheduler (`reminder_prefs.rs`) — the nag has
   * to fire while the app is only in the tray — through the same `settings.json`
   * this key is written to.
   */
  const renagOverdue = ref(false);
  /**
   * Whether reminders are held back during a nightly quiet window. Off by
   * default: a silent window suppresses system notifications until it ends, a
   * change with consequences like the rollover, so the user opts in. Read
   * natively by the reminder scheduler (`reminder_prefs.rs`) — the window has to
   * gate a reminder that fires while the app is only in the tray — through the
   * same `settings.json` these three keys are written to.
   */
  const quietHoursEnabled = ref(false);
  /**
   * The quiet window's start and end, as `"HH:mm"` local times. Stored even
   * while the switch is off (so turning it on shows 22:00–08:00 rather than a
   * blank), and read back the same by the scheduler. A window whose start equals
   * its end names no span and silences nothing — the native side fails open on
   * it — so the view does not block the user from setting one.
   */
  const quietHoursStart = ref("22:00");
  const quietHoursEnd = ref("08:00");
  /**
   * Whether the daily morning summary is on. Off by default: it is a new
   * every-morning system notification — an unasked-for interruption like the
   * renag — so the user opts in. Read natively by the reminder scheduler
   * (`reminder_prefs.rs`), which builds the summary while the app is only in the
   * tray, through the same `settings.json` these keys are written to.
   */
  const morningSummaryEnabled = ref(false);
  /**
   * The morning summary's push time, as an `"HH:mm"` local time. Stored even
   * while the switch is off (so turning it on shows 09:00 rather than a blank),
   * and read back the same by the scheduler. 09:00 is the shared "早晨" anchor —
   * the same hour the "明天" snooze resolves to — so the two do not drift.
   */
  const morningSummaryTime = ref("09:00");
  /**
   * Whether reminders fire at their fixed absolute instant rather than floating to
   * the device's wall clock (提醒通知/07/08). Off by default: a reminder set to
   * "9:00" is meant to stay 9:00 wherever the user is, so it floats unless they ask
   * for a fixed moment. Read natively by the reminder scheduler
   * (`reminder_prefs.rs`), which reads a reminder's stored offset to keep its wall
   * clock while the app is only in the tray, through the same `settings.json` this
   * key is written to.
   */
  const reminderTzAbsolute = ref(false);
  const isReady = ref(false);
  const persistenceError = ref<string | null>(null);
  let unsubscribeSystemTheme: (() => void) | undefined;
  const themeLabel = computed(
    () => ({ light: "浅色", dark: "深色", system: "跟随系统" })[theme.value],
  );

  const isDesktop = computed(() => isTauri());

  function watchSystemTheme(): void {
    unsubscribeSystemTheme?.();
    unsubscribeSystemTheme = onSystemThemeChange(() => {
      if (theme.value === "system") {
        void applyTheme("system");
      }
    });
  }

  async function bootstrap(): Promise<void> {
    try {
      theme.value = await loadThemePreference();
      closeToTray.value = await loadBooleanPreference("behavior.closeToTray", true);
      showTodayCard.value = await loadBooleanPreference("ui.showTodayCard", false);
      rolloverOverdue.value = await loadBooleanPreference("behavior.rolloverOverdue", false);
      snooze5m.value = await loadBooleanPreference("reminder.snooze.5m", true);
      snooze1h.value = await loadBooleanPreference("reminder.snooze.1h", true);
      snoozeTonight.value = await loadBooleanPreference("reminder.snooze.tonight", true);
      snoozeTomorrow.value = await loadBooleanPreference("reminder.snooze.tomorrow", true);
      renagOverdue.value = await loadBooleanPreference("reminder.renag", false);
      quietHoursEnabled.value = await loadBooleanPreference("reminder.quiet.enabled", false);
      quietHoursStart.value = await loadStringPreference("reminder.quiet.start", "22:00");
      quietHoursEnd.value = await loadStringPreference("reminder.quiet.end", "08:00");
      morningSummaryEnabled.value = await loadBooleanPreference("reminder.summary.enabled", false);
      morningSummaryTime.value = await loadStringPreference("reminder.summary.time", "09:00");
      reminderTzAbsolute.value = await loadBooleanPreference("reminder.tz.absolute", false);
      await applyTheme(theme.value);
      watchSystemTheme();
      writeDiagnostic("info", "Settings restored", {
        theme: theme.value,
        closeToTray: closeToTray.value,
        showTodayCard: showTodayCard.value,
        rolloverOverdue: rolloverOverdue.value,
        snooze5m: snooze5m.value,
        snooze1h: snooze1h.value,
        snoozeTonight: snoozeTonight.value,
        snoozeTomorrow: snoozeTomorrow.value,
        renagOverdue: renagOverdue.value,
        quietHoursEnabled: quietHoursEnabled.value,
        quietHoursStart: quietHoursStart.value,
        quietHoursEnd: quietHoursEnd.value,
        morningSummaryEnabled: morningSummaryEnabled.value,
        morningSummaryTime: morningSummaryTime.value,
        reminderTzAbsolute: reminderTzAbsolute.value,
      });
    } catch {
      theme.value = "system";
      closeToTray.value = true;
      showTodayCard.value = false;
      rolloverOverdue.value = false;
      snooze5m.value = true;
      snooze1h.value = true;
      snoozeTonight.value = true;
      snoozeTomorrow.value = true;
      renagOverdue.value = false;
      quietHoursEnabled.value = false;
      quietHoursStart.value = "22:00";
      quietHoursEnd.value = "08:00";
      morningSummaryEnabled.value = false;
      morningSummaryTime.value = "09:00";
      reminderTzAbsolute.value = false;
      persistenceError.value = "设置恢复失败，已使用系统默认值。";
      // The fallback itself must never throw: `main.ts` awaits `bootstrap()`
      // before mounting, so a second failure here would take the whole UI down
      // instead of degrading to defaults. `applyTheme` writes the DOM theme
      // before touching the native window, so the document stays styled even
      // when the native call is the part that fails.
      try {
        await applyTheme(theme.value);
      } catch {
        /* best effort: the DOM theme is already applied */
      }
      writeDiagnostic("warn", "Settings bootstrap failed; using defaults");
    } finally {
      isReady.value = true;
    }
  }

  async function setTheme(preference: ThemePreference): Promise<void> {
    theme.value = preference;
    persistenceError.value = null;

    try {
      await applyTheme(preference);
      await saveThemePreference(preference);
      writeDiagnostic("info", "Theme preference updated", { theme: preference });
    } catch {
      persistenceError.value = "主题已应用，但暂时无法保存设置。";
      writeDiagnostic("warn", "Theme preference could not be fully applied", { theme: preference });
    }
  }

  async function setCloseToTray(enabled: boolean): Promise<void> {
    closeToTray.value = enabled;
    persistenceError.value = null;

    try {
      await saveBooleanPreference("behavior.closeToTray", enabled);
      writeDiagnostic("info", "Close-to-tray preference updated", { closeToTray: enabled });
    } catch {
      persistenceError.value = "设置暂时无法保存。";
      writeDiagnostic("warn", "Close-to-tray preference could not be persisted", {
        closeToTray: enabled,
      });
    }
  }

  async function setShowTodayCard(enabled: boolean): Promise<void> {
    showTodayCard.value = enabled;
    persistenceError.value = null;

    try {
      await saveBooleanPreference("ui.showTodayCard", enabled);
      writeDiagnostic("info", "Show-today-card preference updated", { showTodayCard: enabled });
    } catch {
      persistenceError.value = "设置暂时无法保存。";
      writeDiagnostic("warn", "Show-today-card preference could not be persisted", {
        showTodayCard: enabled,
      });
    }
  }

  async function setRolloverOverdue(enabled: boolean): Promise<void> {
    rolloverOverdue.value = enabled;
    persistenceError.value = null;

    try {
      await saveBooleanPreference("behavior.rolloverOverdue", enabled);
      writeDiagnostic("info", "Rollover-overdue preference updated", {
        rolloverOverdue: enabled,
      });
    } catch {
      persistenceError.value = "设置暂时无法保存。";
      writeDiagnostic("warn", "Rollover-overdue preference could not be persisted", {
        rolloverOverdue: enabled,
      });
    }
  }

  /**
   * The write half the four snooze setters share. Each setter flips its own ref
   * (so the checkbox reflects the choice at once) and hands the key and value
   * here; keeping the persistence in one place is what stops four copies of the
   * same try/catch from drifting apart.
   */
  async function persistSnoozePreference(key: string, enabled: boolean): Promise<void> {
    persistenceError.value = null;

    try {
      await saveBooleanPreference(key, enabled);
      writeDiagnostic("info", "Snooze preference updated", { key, enabled });
    } catch {
      persistenceError.value = "设置暂时无法保存。";
      writeDiagnostic("warn", "Snooze preference could not be persisted", { key, enabled });
    }
  }

  async function setSnooze5m(enabled: boolean): Promise<void> {
    snooze5m.value = enabled;
    await persistSnoozePreference("reminder.snooze.5m", enabled);
  }

  async function setSnooze1h(enabled: boolean): Promise<void> {
    snooze1h.value = enabled;
    await persistSnoozePreference("reminder.snooze.1h", enabled);
  }

  async function setSnoozeTonight(enabled: boolean): Promise<void> {
    snoozeTonight.value = enabled;
    await persistSnoozePreference("reminder.snooze.tonight", enabled);
  }

  async function setSnoozeTomorrow(enabled: boolean): Promise<void> {
    snoozeTomorrow.value = enabled;
    await persistSnoozePreference("reminder.snooze.tomorrow", enabled);
  }

  async function setRenagOverdue(enabled: boolean): Promise<void> {
    renagOverdue.value = enabled;
    persistenceError.value = null;

    try {
      await saveBooleanPreference("reminder.renag", enabled);
      writeDiagnostic("info", "Renag-overdue preference updated", { renagOverdue: enabled });
    } catch {
      persistenceError.value = "设置暂时无法保存。";
      writeDiagnostic("warn", "Renag-overdue preference could not be persisted", {
        renagOverdue: enabled,
      });
    }
  }

  async function setQuietHoursEnabled(enabled: boolean): Promise<void> {
    quietHoursEnabled.value = enabled;
    persistenceError.value = null;

    try {
      await saveBooleanPreference("reminder.quiet.enabled", enabled);
      writeDiagnostic("info", "Quiet-hours preference updated", { quietHoursEnabled: enabled });
    } catch {
      persistenceError.value = "设置暂时无法保存。";
      writeDiagnostic("warn", "Quiet-hours preference could not be persisted", {
        quietHoursEnabled: enabled,
      });
    }
  }

  /**
   * The write half the two quiet-time setters share. Each flips its own ref (so
   * the input reflects the change at once) and hands the key and value here;
   * keeping the persistence in one place is what stops two copies of the same
   * try/catch from drifting apart, as `persistSnoozePreference` does for snooze.
   */
  async function persistQuietPreference(key: string, value: string): Promise<void> {
    persistenceError.value = null;

    try {
      await saveStringPreference(key, value);
      writeDiagnostic("info", "Quiet-hours window updated", { key, value });
    } catch {
      persistenceError.value = "设置暂时无法保存。";
      writeDiagnostic("warn", "Quiet-hours window could not be persisted", { key, value });
    }
  }

  async function setQuietHoursStart(value: string): Promise<void> {
    quietHoursStart.value = value;
    await persistQuietPreference("reminder.quiet.start", value);
  }

  async function setQuietHoursEnd(value: string): Promise<void> {
    quietHoursEnd.value = value;
    await persistQuietPreference("reminder.quiet.end", value);
  }

  async function setMorningSummaryEnabled(enabled: boolean): Promise<void> {
    morningSummaryEnabled.value = enabled;
    persistenceError.value = null;

    try {
      await saveBooleanPreference("reminder.summary.enabled", enabled);
      writeDiagnostic("info", "Morning-summary preference updated", {
        morningSummaryEnabled: enabled,
      });
    } catch {
      persistenceError.value = "设置暂时无法保存。";
      writeDiagnostic("warn", "Morning-summary preference could not be persisted", {
        morningSummaryEnabled: enabled,
      });
    }
  }

  async function setMorningSummaryTime(value: string): Promise<void> {
    morningSummaryTime.value = value;
    persistenceError.value = null;

    try {
      await saveStringPreference("reminder.summary.time", value);
      writeDiagnostic("info", "Morning-summary time updated", { morningSummaryTime: value });
    } catch {
      persistenceError.value = "设置暂时无法保存。";
      writeDiagnostic("warn", "Morning-summary time could not be persisted", {
        morningSummaryTime: value,
      });
    }
  }

  async function setReminderTzAbsolute(enabled: boolean): Promise<void> {
    reminderTzAbsolute.value = enabled;
    persistenceError.value = null;

    try {
      await saveBooleanPreference("reminder.tz.absolute", enabled);
      writeDiagnostic("info", "Reminder-timezone preference updated", {
        reminderTzAbsolute: enabled,
      });
    } catch {
      persistenceError.value = "设置暂时无法保存。";
      writeDiagnostic("warn", "Reminder-timezone preference could not be persisted", {
        reminderTzAbsolute: enabled,
      });
    }
  }

  return {
    theme,
    themeLabel,
    closeToTray,
    showTodayCard,
    rolloverOverdue,
    snooze5m,
    snooze1h,
    snoozeTonight,
    snoozeTomorrow,
    renagOverdue,
    quietHoursEnabled,
    quietHoursStart,
    quietHoursEnd,
    morningSummaryEnabled,
    morningSummaryTime,
    reminderTzAbsolute,
    isDesktop,
    isReady,
    persistenceError,
    bootstrap,
    setTheme,
    setCloseToTray,
    setShowTodayCard,
    setRolloverOverdue,
    setSnooze5m,
    setSnooze1h,
    setSnoozeTonight,
    setSnoozeTomorrow,
    setRenagOverdue,
    setQuietHoursEnabled,
    setQuietHoursStart,
    setQuietHoursEnd,
    setMorningSummaryEnabled,
    setMorningSummaryTime,
    setReminderTzAbsolute,
  };
});
