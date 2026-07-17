import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

export const themePreferences = ["light", "dark", "system"] as const;
export type ThemePreference = (typeof themePreferences)[number];

function prefersDarkMode(): boolean {
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? false;
}

export function resolveTheme(preference: ThemePreference): "light" | "dark" {
  return preference === "system" ? (prefersDarkMode() ? "dark" : "light") : preference;
}

export async function applyTheme(preference: ThemePreference): Promise<void> {
  const resolved = resolveTheme(preference);
  document.documentElement.dataset.theme = resolved;
  document.documentElement.classList.toggle("dark", resolved === "dark");

  if (!isTauri()) return;

  const nativeTheme: "light" | "dark" | null = preference === "system" ? null : preference;
  await getCurrentWindow().setTheme(nativeTheme);
}

export function onSystemThemeChange(callback: () => void): () => void {
  const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
  mediaQuery.addEventListener("change", callback);
  return () => mediaQuery.removeEventListener("change", callback);
}
