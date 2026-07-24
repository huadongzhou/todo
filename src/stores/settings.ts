import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { isTauri } from "@tauri-apps/api/core";
import { applyTheme, onSystemThemeChange, type ThemePreference } from "@/lib/appearance";
import { writeDiagnostic } from "@/lib/diagnostics";
import {
  loadBooleanPreference,
  loadThemePreference,
  saveBooleanPreference,
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
  };
});
