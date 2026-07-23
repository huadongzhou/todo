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
      await applyTheme(theme.value);
      watchSystemTheme();
      writeDiagnostic("info", "Settings restored", {
        theme: theme.value,
        closeToTray: closeToTray.value,
        showTodayCard: showTodayCard.value,
        rolloverOverdue: rolloverOverdue.value,
      });
    } catch {
      theme.value = "system";
      closeToTray.value = true;
      showTodayCard.value = false;
      rolloverOverdue.value = false;
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

  return {
    theme,
    themeLabel,
    closeToTray,
    showTodayCard,
    rolloverOverdue,
    isDesktop,
    isReady,
    persistenceError,
    bootstrap,
    setTheme,
    setCloseToTray,
    setShowTodayCard,
    setRolloverOverdue,
  };
});
