import { isTauri } from "@tauri-apps/api/core";
import { nativeCommands } from "@/lib/native";
import { writeDiagnostic } from "@/lib/diagnostics";
import type { Todo } from "@/types/todo";

/**
 * Calendar export platform layer.
 *
 * The .ics document itself is a pure rule owned by the Rust side (`todo-domain`),
 * reached through the generated command; this module only decides whether the
 * current runtime can produce a file at all and turns the command's answer into
 * the three outcomes the UI has words for.
 */

/** What an export attempt ended up doing, in the terms the UI reports. */
export type CalendarExportOutcome =
  /**
   * A file was written; `path` is where the user will find it.
   *
   * `unrepeatableRecurrences` counts the exported tasks whose repeat rule the
   * .ics standard cannot express (a lunar rule, "every N working days"), which
   * therefore sit on the calendar once. The UI reports the number so a dropped
   * repeat is not something the user has to discover in their calendar.
   */
  | {
      readonly kind: "exported";
      readonly eventCount: number;
      readonly path: string;
      readonly unrepeatableRecurrences: number;
    }
  /** No task carried a date, so nothing was written. Not an error. */
  | { readonly kind: "empty" }
  /** Nothing was written and something went wrong; `reason` may be unavailable. */
  | { readonly kind: "failed"; readonly reason: string | null };

/**
 * Whether this runtime can export at all.
 *
 * Only the Tauri runtime can: the document is built in Rust and written to a
 * real folder. In the browser dev server there is no command to call and no
 * folder to write to, so the entry is hidden rather than shown as a control that
 * always fails.
 */
export function canExportCalendar(): boolean {
  return isTauri();
}

/** Narrows an unknown caught value to a reportable string. */
function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** Writes the tasks that carry a date to an .ics file on this device. */
export async function exportCalendar(todos: readonly Todo[]): Promise<CalendarExportOutcome> {
  if (!canExportCalendar()) return { kind: "failed", reason: null };

  try {
    const result = await nativeCommands.exportCalendar([...todos]);
    if (result.status === "ok") {
      const { path, eventCount, unrepeatableRecurrences } = result.data;
      if (path === null) {
        writeDiagnostic("info", "Calendar export found no dated task");
        return { kind: "empty" };
      }
      writeDiagnostic("info", "Calendar exported", { eventCount, unrepeatableRecurrences });
      return { kind: "exported", eventCount, path, unrepeatableRecurrences };
    }
    writeDiagnostic("warn", "Calendar export refused", { error: result.error });
    return { kind: "failed", reason: result.error };
  } catch (error) {
    const reason = errorMessage(error);
    writeDiagnostic("error", "Calendar export failed", { error: reason });
    return { kind: "failed", reason };
  }
}
