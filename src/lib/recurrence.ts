import { isTauri } from "@tauri-apps/api/core";
import { nativeCommands } from "@/lib/native";
import { writeDiagnostic } from "@/lib/diagnostics";
import type { HabitProgress } from "@/bindings/commands";
import type { RecurrenceFrequency, RecurrenceRule, Weekday } from "@/types/todo";

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

/** The seven weekdays in the engine's Monday-first order, for a stable display. */
const WEEKDAY_ORDER: readonly Weekday[] = [
  "monday",
  "tuesday",
  "wednesday",
  "thursday",
  "friday",
  "saturday",
  "sunday",
];

/** One character per weekday — 一…日 — matching the editor's chip faces. */
const WEEKDAY_SUMMARY_CHAR: Record<Weekday, string> = {
  monday: "一",
  tuesday: "二",
  wednesday: "三",
  thursday: "四",
  friday: "五",
  saturday: "六",
  sunday: "日",
};

/**
 * The whole rule as one readable line for the editor's inline echo — "每周一、
 * 三、五，共 10 次", "农历每年，永不结束".
 *
 * Where {@link recurrenceLabel} says only the frequency band the list pill needs,
 * this says everything the user set: it borrows that frequency band, then appends
 * the weekday set, the day-of-month position and the end condition so the editor
 * can show back exactly what the controls hold. Pure text like its neighbour, and
 * for the same reason — UnoCSS does not scan `.ts`, so no colour or `dark:` class
 * string lives here; the echo row's tone is a literal utility in the component.
 *
 * It is defensive about the two in-progress values a control can hold before it
 * is a storable rule: an empty end date (kept as `""` so its radio stays chosen)
 * reads as "永不结束", and a blank day-of-month drops the day rather than printing
 * a `NaN`. The frequency-and-count blocking states are the field block's to
 * report; the editor hides this line while one is showing, so it is never asked
 * of an unreadable interval or count.
 */
export function recurrenceSummary(rule: RecurrenceRule): string {
  let base = recurrenceLabel(rule);

  if (rule.frequency === "weekly" && rule.weekdays.length > 0) {
    base += weekdaySummary(rule.weekdays);
  } else if (rule.frequency === "monthly") {
    if (rule.onLastDay) {
      base += "最后一天";
    } else if (isMonthDay(rule.monthDay)) {
      base += ` ${rule.monthDay} 日`;
    }
  }

  return base + endSummary(rule);
}

/** "一、三、五" for a weekday set, always read in Monday-first order. */
function weekdaySummary(weekdays: readonly Weekday[]): string {
  return WEEKDAY_ORDER.filter((day) => weekdays.includes(day))
    .map((day) => WEEKDAY_SUMMARY_CHAR[day])
    .join("、");
}

/** The end clause: "，共 N 次" / "，截止 YYYY-MM-DD" / "，永不结束". */
function endSummary(rule: RecurrenceRule): string {
  if (typeof rule.count === "number" && Number.isInteger(rule.count) && rule.count >= 1) {
    return `，共 ${rule.count} 次`;
  }
  if (typeof rule.until === "string" && rule.until.trim()) {
    return `，截止 ${rule.until}`;
  }
  return "，永不结束";
}

/** A day-of-month a summary can print: a whole number in 1–31, never a blank NaN. */
function isMonthDay(day: number | null | undefined): day is number {
  return typeof day === "number" && Number.isInteger(day) && day >= 1 && day <= 31;
}

/**
 * Whether a monthly or yearly rule anchored on `anchor` names a position the
 * calendar never brings round again — a lunar leap month, a lunar 30th of a short
 * month, a Gregorian February 31st. Answered by asking the engine, with the end
 * condition stripped off (`until`/`count` removed) so a series that has simply run
 * out of its counted occurrences is not mistaken for one whose position never
 * recurs: `null` back from the engine then means only "no such date ever again".
 *
 * A non-Tauri runtime (browser dev) has no engine and cannot know, so it answers
 * `false` — the same degrade as {@link nextOccurrence}, and the reason the caller
 * must gate on the engine rather than reading a bare `null` as "never recurs".
 */
export async function recurrenceNeverRecursAgain(
  rule: RecurrenceRule,
  anchor: string,
): Promise<boolean> {
  if (!isTauri()) return false;
  const probe: RecurrenceRule = { ...rule, until: null, count: null };
  return (await nextOccurrence(probe, anchor, anchor)) === null;
}

/**
 * Whether a repeat rule is a habit — a daily or weekly one, shown as a check-in
 * row rather than a task that leaves a completed instance behind each time
 * (任务管理/09). The frequency alone decides it; there is no separate "is a habit"
 * flag on the contract.
 */
export function isHabitRule(rule: RecurrenceRule): boolean {
  return rule.frequency === "daily" || rule.frequency === "weekly";
}

/**
 * The streak pill's words — "连续 5 天" / "连续 3 周".
 *
 * Pure text, kept out of the component alongside `recurrenceLabel` and away from
 * any colour class the way the rest of this file is (UnoCSS does not scan `.ts`).
 * Only daily and weekly reach here — they are the only habits — so the unit is
 * days unless the rule counts weeks.
 */
export function streakLabel(count: number, frequency: RecurrenceFrequency): string {
  const unit = frequency === "weekly" ? "周" : "天";
  return `连续 ${count} ${unit}`;
}

/**
 * A make-up button's short face — "补 周一" for a weekly habit, "补 3日" for a
 * daily one. The kind of day is what tells the missed occurrences apart at a
 * glance: which weekday for a weekly rule, which day of the month for a daily
 * one. An unreadable date degrades to a bare "补卡" rather than throwing.
 */
export function makeupShortLabel(date: string, frequency: RecurrenceFrequency): string {
  const parsed = new Date(`${date}T00:00:00`);
  if (Number.isNaN(parsed.getTime())) return "补卡";
  if (frequency === "weekly") return `补 ${WEEKDAY_ZH[parsed.getDay()]}`;
  return `补 ${parsed.getDate()}日`;
}

/** The missed day as "M月D日", for a make-up button's full accessible name. */
export function isoMonthDay(date: string): string {
  const parsed = new Date(`${date}T00:00:00`);
  if (Number.isNaN(parsed.getTime())) return date;
  return `${parsed.getMonth() + 1}月${parsed.getDate()}日`;
}

/** Sunday-first, matching `Date.prototype.getDay`. */
const WEEKDAY_ZH = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"] as const;

/**
 * A habit's streak and the recent days it could still be checked in on, from the
 * Rust engine — the app's single reading of "when does this repeat", so the pill
 * and the make-up buttons never count the schedule a second time in TypeScript.
 *
 * `dueDate` is the task's current due date (its next occurrence) and `checkIns`
 * the scheduled dates already checked. A non-Tauri runtime (browser dev) has no
 * engine, so it answers `null` and the row simply shows no streak or make-up —
 * the same way `nextOccurrence` degrades.
 */
export async function habitProgress(
  rule: RecurrenceRule,
  dueDate: string,
  checkIns: readonly string[],
): Promise<HabitProgress | null> {
  if (!isTauri()) return null;

  try {
    const result = await nativeCommands.habitProgress(rule, dueDate, [...checkIns]);
    if (result.status === "ok") return result.data;
    writeDiagnostic("warn", "Habit progress refused by the engine", { error: result.error });
    return null;
  } catch (error) {
    writeDiagnostic("error", "Habit progress lookup failed", {
      error: error instanceof Error ? error.message : String(error),
    });
    return null;
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
