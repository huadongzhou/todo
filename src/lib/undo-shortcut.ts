/**
 * Undo and redo on the keyboard, inside the window.
 *
 * This is a plain `keydown` listener on the webview, not an OS-level hotkey.
 * The chords in `shortcuts.ts` go through the global-shortcut plugin because
 * they mean "summon this app from wherever I am"; undo means "take back what I
 * just did in the list I am looking at", which is only meaningful while the
 * window has the keyboard. Registering it globally would take Ctrl+Z away from
 * every other application on the machine. It therefore neither uses nor is
 * affected by that plugin or its permissions.
 */

export interface UndoShortcutHandlers {
  readonly undo: () => void;
  readonly redo: () => void;
  /**
   * Whether something else currently owns the chord — an open editor, above all.
   * Asked at the moment of the keypress rather than passed in once, because the
   * answer changes while the listener stays.
   */
  readonly isSuspended: () => boolean;
}

/**
 * Whether the key is being pressed inside something that edits text.
 *
 * There, Ctrl+Z belongs to the browser: it means "take back what I just typed",
 * and stealing it would undo an unrelated task while the user was trying to
 * fix a word. This covers the date and time inputs too, where the browser does
 * nothing with it — one rule that can be stated in a sentence beats a list of
 * input types that would have to be revisited with every new field.
 */
function isTextEntry(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  return target.tagName === "INPUT" || target.tagName === "TEXTAREA";
}

/**
 * Listens for undo and redo until the returned function is called.
 *
 * Redo answers to both spellings: Ctrl+Y, which is what Windows apps use and
 * what the product asked for, and Ctrl+Shift+Z, which is what everything on
 * macOS uses and what a good half of Windows apps accept as well.
 */
export function registerUndoShortcut(handlers: UndoShortcutHandlers): () => void {
  function onKeydown(event: KeyboardEvent): void {
    // Meta for macOS, where the chord is Cmd+Z. Alt is somebody else's chord.
    if (!(event.ctrlKey || event.metaKey) || event.altKey) return;

    const key = event.key.toLowerCase();
    const redo = (key === "z" && event.shiftKey) || (key === "y" && !event.shiftKey);
    const undo = key === "z" && !event.shiftKey;
    if (!undo && !redo) return;

    if (handlers.isSuspended() || isTextEntry(event.target)) return;

    event.preventDefault();
    if (redo) {
      handlers.redo();
    } else {
      handlers.undo();
    }
  }

  window.addEventListener("keydown", onKeydown);
  return () => window.removeEventListener("keydown", onKeydown);
}
