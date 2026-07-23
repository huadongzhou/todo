import { isTauri } from "@tauri-apps/api/core";
import { nativeCommands } from "@/lib/native";
import { writeDiagnostic } from "@/lib/diagnostics";
import type { RecurrenceRule } from "@/types/todo";

/**
 * Recurrence view + generation helpers.
 *
 * Two jobs, both kept out of the components and the store body:
 *
 *   - `recurrenceLabel` turns a rule into the frequency words the list shows. It
 *     is pure text: no colour or `dark:` class string lives here, because UnoCSS
 *     does not scan `.ts` and a class string it never sees is a class it never
 *     generates (the `dueDate.ts` dark-mode trap). The pill's colours are literal
 *     utilities in `TodoItem.vue`.
 *   - `nextOccurrence` reaches the Rust rule engine — the app's single reading of
 *     "when does this repeat" — for a recurring task's next date. The engine is
 *     not reimplemented here; a runtime with no native side (browser dev) simply
 *     has no next date, and the caller completes the task without a successor.
 *
 * The date-shift helpers move the rest of a task's dates along when it advances to
 * its next occurrence, so a reminder or a timed block travels with the task
 * rather than staying on the date that has passed.
 */

const MS_PER_DAY = 24 * 60 * 60 * 1000;

/**
 * The frequency words for a rule — "每周", "每 3 天", "农历每月".
 *
 * Only the frequency band (how often), never the whole rule: the specific
 * weekday, day of month, end date or count belong to the editor and a detail
 * view, not to a list pill. Lunar only prefixes monthly and yearly, because the
 * engine reads a lunar daily or weekly rule the same as a Gregorian one.
 */
export function recurrenceLabel(rule: RecurrenceRule): string {
  const n = rule.interval;
  const lunar = rule.calendar === "lunar";
  switch (rule.frequency) {
    case "daily":
      return n === 1 ? "每日" : `每 ${n} 天`;
    case "weekly":
      return n === 1 ? "每周" : `每 ${n} 周`;
    case "monthly":
      if (lunar) return n === 1 ? "农历每月" : `农历每 ${n} 个月`;
      return n === 1 ? "每月" : `每 ${n} 个月`;
    case "yearly":
      if (lunar) return n === 1 ? "农历每年" : `农历每 ${n} 年`;
      return n === 1 ? "每年" : `每 ${n} 年`;
    case "workday":
      return n === 1 ? "每个工作日" : `每 ${n} 个工作日`;
    default:
      // A frequency a newer build wrote and this one has no word for: the pill
      // says "it repeats" rather than going blank.
      return "重复";
  }
}

/**
 * The next date a recurring task lands on strictly after `after`, or `null` when
 * the series has ended or this runtime has no engine to ask.
 *
 * `anchor` and `after` are local `YYYY-MM-DD` dates. The engine is Rust-only, so
 * a non-Tauri runtime (browser dev, a mobile shell without the command) answers
 * `null` and the caller degrades to "complete without generating a successor".
 */
export async function nextOccurrence(
  rule: RecurrenceRule,
  anchor: string,
  after: string,
): Promise<string | null> {
  if (!isTauri()) return null;

  try {
    const result = await nativeCommands.nextOccurrence(rule, anchor, after);
    if (result.status === "ok") return result.data;
    writeDiagnostic("warn", "Next occurrence refused by the engine", { error: result.error });
    return null;
  } catch (error) {
    writeDiagnostic("error", "Next occurrence lookup failed", {
      error: error instanceof Error ? error.message : String(error),
    });
    return null;
  }
}

/**
 * Whole local calendar days from `from` to `to`, both `YYYY-MM-DD`; positive when
 * `to` is later. This is how far every other date on the task shifts when it
 * moves to its next occurrence.
 */
export function daysBetween(from: string, to: string): number {
  const start = new Date(`${from}T00:00:00`);
  const end = new Date(`${to}T00:00:00`);
  return Math.round((end.getTime() - start.getTime()) / MS_PER_DAY);
}

/**
 * A local date (`YYYY-MM-DD`) moved `days` days on, keeping the shape; `null`
 * passes through so a task without a start date generates one without a start
 * date. An unreadable value is left as it is rather than dropped.
 */
export function shiftLocalDate(date: string | null, days: number): string | null {
  if (!date) return null;
  const parsed = new Date(`${date}T00:00:00`);
  if (Number.isNaN(parsed.getTime())) return date;
  parsed.setDate(parsed.getDate() + days);
  const month = String(parsed.getMonth() + 1).padStart(2, "0");
  const day = String(parsed.getDate()).padStart(2, "0");
  return `${parsed.getFullYear()}-${month}-${day}`;
}

/**
 * An absolute instant moved `days` local days on, keeping its wall-clock time.
 *
 * `setDate` shifts in local time, so a reminder set for eight in the evening is
 * still eight in the evening on the new date even across a daylight-saving
 * change. `null` and unreadable values pass through unchanged.
 */
export function shiftInstant(instant: string | null, days: number): string | null {
  if (!instant) return null;
  const parsed = new Date(instant);
  if (Number.isNaN(parsed.getTime())) return instant;
  parsed.setDate(parsed.getDate() + days);
  return parsed.toISOString();
}
