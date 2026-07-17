import { isTauri } from "@tauri-apps/api/core";
import {
  isRegistered,
  register,
  unregister,
  unregisterAll,
  type ShortcutEvent,
} from "@tauri-apps/plugin-global-shortcut";
import { writeDiagnostic } from "@/lib/diagnostics";

export interface ShortcutHandlers {
  readonly toggleQuickAdd: () => void | Promise<void>;
}

/**
 * Global shortcut platform layer.
 *
 * Lets the user summon the quick-add flow from anywhere on the desktop, even
 * when the main window is hidden to the tray. Outside the Tauri desktop runtime
 * (browser dev, mobile) this is a no-op so the app keeps working.
 *
 * The default chord is `CommandOrControl+Shift+A`. It is deliberately chosen
 * to avoid clashing with the common `Ctrl+Shift+A` (many apps) and the
 * browser devtools shortcuts.
 */
export const DEFAULT_QUICK_ADD_SHORTCUT = "CommandOrControl+Shift+A";

let registeredShortcuts: string[] = [];

/**
 * Registers the global shortcuts. Safe to call once on startup; re-registers
 * if already registered. Returns the shortcuts that ended up registered.
 */
export async function registerShortcuts(
  handlers: ShortcutHandlers,
  shortcuts: string | string[] = DEFAULT_QUICK_ADD_SHORTCUT,
): Promise<string[]> {
  if (!isTauri()) return [];

  const shortcutList = Array.isArray(shortcuts) ? shortcuts : [shortcuts];

  try {
    for (const shortcut of shortcutList) {
      const already = await isRegistered(shortcut);
      await register(shortcut, (event: ShortcutEvent) => {
        if (event.state === "Pressed") {
          void handlers.toggleQuickAdd();
        }
      });
      writeDiagnostic(
        already ? "debug" : "info",
        already ? "Global shortcut re-registered" : "Global shortcut registered",
        { shortcut, state: "ok" },
      );
    }
    registeredShortcuts = shortcutList;
    return shortcutList;
  } catch (error) {
    writeDiagnostic("warn", "Failed to register global shortcut", {
      shortcuts: shortcutList.join(","),
      error: String(error),
    });
    return [];
  }
}

/** Unregisters every shortcut this app previously registered. */
export async function unregisterShortcuts(): Promise<void> {
  if (!isTauri()) return;
  if (registeredShortcuts.length === 0) return;

  try {
    await unregister(registeredShortcuts);
    writeDiagnostic("info", "Global shortcuts unregistered", {
      shortcuts: registeredShortcuts.join(","),
    });
  } catch (error) {
    writeDiagnostic("warn", "Failed to unregister global shortcuts", {
      error: String(error),
    });
    // Fall back to unregisterAll so we never leak OS grabs on shutdown.
    try {
      await unregisterAll();
    } catch {
      /* best effort */
    }
  } finally {
    registeredShortcuts = [];
  }
}
