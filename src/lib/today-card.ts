import { isTauri } from "@tauri-apps/api/core";
import {
  currentMonitor,
  primaryMonitor,
  type Monitor,
  type WindowOptions,
} from "@tauri-apps/api/window";
import type { WebviewOptions } from "@tauri-apps/api/webview";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

/**
 * Options accepted by the `WebviewWindow` constructor: webview options minus
 * the position/size fields (which live on `WindowOptions`), combined with the
 * window options.
 */
type CardWindowOptions = Omit<WebviewOptions, "x" | "y" | "width" | "height"> & WindowOptions;
import { writeDiagnostic } from "@/lib/diagnostics";

/** Narrows an unknown caught value to a loggable string. */
function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/**
 * Today-card platform layer.
 *
 * Implements the "today desktop card" as a separate lightweight Tauri
 * `WebviewWindow` (see develop.md milestone 5). Per develop.md L797 the card
 * does NOT depend on any plugin — it is built entirely on Tauri Core's
 * multi-window API plus monitor info for DPI-aware positioning.
 *
 * Outside the Tauri desktop runtime (browser dev, mobile) every function here
 * is a no-op so the app keeps working.
 *
 * The card window lives at `index.html?card=today`; `App.vue` switches to the
 * compact card view when it sees that query parameter.
 */

export const TODAY_CARD_LABEL = "today";
const TODAY_CARD_URL = "index.html?card=today";
const CARD_WIDTH = 360;
const CARD_HEIGHT = 480;
const CARD_MARGIN = 24;

/** Resolves the monitor to anchor the card on, with graceful fallbacks. */
async function resolveMonitor(): Promise<Monitor | null> {
  try {
    return (await currentMonitor()) ?? (await primaryMonitor());
  } catch (error) {
    writeDiagnostic("warn", "Failed to resolve monitor for today card", {
      error: errorMessage(error),
    });
    return null;
  }
}

/**
 * Computes the bottom-right anchor (in logical pixels) for the card.
 * Falls back to `null` when no monitor is available, in which case the
 * WebviewWindow is created centered.
 */
async function computeCardPosition(): Promise<{ x: number; y: number } | null> {
  const monitor = await resolveMonitor();
  if (!monitor) return null;

  const logicalWidth = monitor.size.width / monitor.scaleFactor;
  const logicalHeight = monitor.size.height / monitor.scaleFactor;
  const x = logicalWidth - CARD_WIDTH - CARD_MARGIN;
  const y = logicalHeight - CARD_HEIGHT - CARD_MARGIN;
  return { x, y };
}

function cardOptions(position: { x: number; y: number } | null): CardWindowOptions {
  return {
    url: TODAY_CARD_URL,
    title: "今日待办",
    width: CARD_WIDTH,
    height: CARD_HEIGHT,
    resizable: false,
    decorations: false,
    shadow: true,
    alwaysOnTop: true,
    skipTaskbar: true,
    focus: false,
    focusable: true,
    visible: true,
    preventOverflow: true,
    ...(position ? { x: position.x, y: position.y } : { center: true }),
  };
}

/** Whether a today-card window currently exists. */
export async function isTodayCardOpen(): Promise<boolean> {
  if (!isTauri()) return false;
  try {
    return (await WebviewWindow.getByLabel(TODAY_CARD_LABEL)) !== null;
  } catch {
    return false;
  }
}

/**
 * Opens the today card. Reuses and focuses an existing card instead of
 * creating a duplicate. Returns the live window instance, or `null` when the
 * card cannot be created (non-desktop runtime or a runtime error).
 */
export async function openTodayCard(): Promise<WebviewWindow | null> {
  if (!isTauri()) return null;

  try {
    const existing = await WebviewWindow.getByLabel(TODAY_CARD_LABEL);
    if (existing) {
      await existing.show();
      await existing.setFocus();
      writeDiagnostic("info", "Today card focused", { label: TODAY_CARD_LABEL });
      return existing;
    }
  } catch {
    /* no existing window — fall through to create */
  }

  try {
    const position = await computeCardPosition();
    const card = new WebviewWindow(TODAY_CARD_LABEL, cardOptions(position));
    await card.once("tauri://created", () => {
      writeDiagnostic("info", "Today card created", { label: TODAY_CARD_LABEL });
    });
    await card.once("tauri://error", (error) => {
      writeDiagnostic("warn", "Today card creation failed", {
        label: TODAY_CARD_LABEL,
        error: errorMessage(error),
      });
    });
    return card;
  } catch (error) {
    writeDiagnostic("warn", "Failed to open today card", { error: errorMessage(error) });
    return null;
  }
}

/** Closes and destroys the today card if it exists. */
export async function closeTodayCard(): Promise<void> {
  if (!isTauri()) return;

  try {
    const existing = await WebviewWindow.getByLabel(TODAY_CARD_LABEL);
    if (existing) {
      await existing.close();
      writeDiagnostic("info", "Today card closed", { label: TODAY_CARD_LABEL });
    }
  } catch (error) {
    writeDiagnostic("warn", "Failed to close today card", { error: errorMessage(error) });
  }
}
