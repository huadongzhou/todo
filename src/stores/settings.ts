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
      await applyTheme(theme.value);
      watchSystemTheme();
      writeDiagnostic("info", "Settings restored", {
        theme: theme.value,
        closeToTray: closeToTray.value,
        showTodayCard: showTodayCard.value,
      });
    } catch {
      theme.value = "system";
      closeToTray.value = true;
      showTodayCard.value = false;
      persistenceError.value = "设置恢复失败，已使用系统默认值。";
      await applyTheme(theme.value);
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

  return {
    theme,
    themeLabel,
    closeToTray,
    showTodayCard,
    isDesktop,
    isReady,
    persistenceError,
    bootstrap,
    setTheme,
    setCloseToTray,
    setShowTodayCard,
  };
});
