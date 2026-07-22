import { isTauri } from "@tauri-apps/api/core";
import { load } from "@tauri-apps/plugin-store";
import { type ThemePreference, themePreferences } from "@/lib/appearance";
import { writeDiagnostic } from "@/lib/diagnostics";

const STORE_FILE = "settings.json";
const THEME_KEY = "appearance.theme";

function asBoolean(value: unknown, fallback: boolean): boolean {
  return typeof value === "boolean" ? value : fallback;
}

function isThemePreference(value: unknown): value is ThemePreference {
  return typeof value === "string" && themePreferences.includes(value as ThemePreference);
}

function readBrowserTheme(): ThemePreference | null {
  try {
    const value = window.localStorage.getItem(THEME_KEY);
    return isThemePreference(value) ? value : null;
  } catch {
    return null;
  }
}

function saveBrowserTheme(preference: ThemePreference): void {
  try {
    window.localStorage.setItem(THEME_KEY, preference);
  } catch {
    writeDiagnostic("warn", "Unable to persist browser theme preference");
  }
}

export async function loadThemePreference(): Promise<ThemePreference> {
  if (!isTauri()) return readBrowserTheme() ?? "system";

  try {
    const store = await load(STORE_FILE, { autoSave: true, defaults: {} });
    const value = await store.get<unknown>(THEME_KEY);
    return isThemePreference(value) ? value : "system";
  } catch {
    writeDiagnostic("warn", "Unable to load native theme preference; using fallback");
    return readBrowserTheme() ?? "system";
  }
}

export async function saveThemePreference(preference: ThemePreference): Promise<void> {
  if (!isTauri()) {
    saveBrowserTheme(preference);
    return;
  }

  try {
    const store = await load(STORE_FILE, { autoSave: true, defaults: {} });
    await store.set(THEME_KEY, preference);
    await store.save();
  } catch {
    saveBrowserTheme(preference);
    writeDiagnostic("warn", "Unable to persist native theme preference; using browser fallback");
  }
}

export async function loadBooleanPreference(key: string, fallback: boolean): Promise<boolean> {
  if (!isTauri()) {
    try {
      return asBoolean(window.localStorage.getItem(key), fallback);
    } catch {
      return fallback;
    }
  }

  try {
    const store = await load(STORE_FILE, { autoSave: true, defaults: {} });
    return asBoolean(await store.get<unknown>(key), fallback);
  } catch {
    writeDiagnostic("warn", "Unable to load native boolean preference; using fallback", { key });
    try {
      return asBoolean(window.localStorage.getItem(key), fallback);
    } catch {
      return fallback;
    }
  }
}

export async function loadStringPreference(key: string, fallback: string): Promise<string> {
  if (!isTauri()) {
    try {
      return window.localStorage.getItem(key) ?? fallback;
    } catch {
      return fallback;
    }
  }

  try {
    const store = await load(STORE_FILE, { autoSave: true, defaults: {} });
    const value = await store.get<unknown>(key);
    return typeof value === "string" ? value : fallback;
  } catch {
    writeDiagnostic("warn", "Unable to load native string preference; using fallback", { key });
    try {
      return window.localStorage.getItem(key) ?? fallback;
    } catch {
      return fallback;
    }
  }
}

export async function saveStringPreference(key: string, value: string): Promise<void> {
  if (!isTauri()) {
    try {
      window.localStorage.setItem(key, value);
    } catch {
      writeDiagnostic("warn", "Unable to persist browser string preference", { key });
    }
    return;
  }

  try {
    const store = await load(STORE_FILE, { autoSave: true, defaults: {} });
    await store.set(key, value);
    await store.save();
  } catch {
    try {
      window.localStorage.setItem(key, value);
    } catch {
      writeDiagnostic("warn", "Unable to persist native string preference; fallback failed", {
        key,
      });
    }
  }
}

export async function saveBooleanPreference(key: string, value: boolean): Promise<void> {
  if (!isTauri()) {
    try {
      window.localStorage.setItem(key, String(value));
    } catch {
      writeDiagnostic("warn", "Unable to persist browser boolean preference", { key });
    }
    return;
  }

  try {
    const store = await load(STORE_FILE, { autoSave: true, defaults: {} });
    await store.set(key, value);
    await store.save();
  } catch {
    try {
      window.localStorage.setItem(key, String(value));
    } catch {
      writeDiagnostic("warn", "Unable to persist native boolean preference; fallback failed", {
        key,
      });
    }
  }
}
