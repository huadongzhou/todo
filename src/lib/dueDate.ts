import { computed, type Ref } from "vue";

/**
 * Due-date presentation helpers.
 *
 * The product rule (see develop.md milestones) is that deadlines are local
 * calendar dates (`YYYY-MM-DD`), while reminders are absolute instants
 * (ISO8601 with timezone). These helpers convert a calendar-date string into
 * the user-facing label and the overdue flag used by the UI.
 */

const MS_PER_DAY = 24 * 60 * 60 * 1000;

/** Calendar-day difference between `dueDate` key and today's local day key. */
function dayDifference(dueDate: string): number {
  const due = new Date(`${dueDate}T00:00:00`);
  const today = new Date();
  const todayLocal = new Date(today.getFullYear(), today.getMonth(), today.getDate());
  return Math.round((due.getTime() - todayLocal.getTime()) / MS_PER_DAY);
}

export function isOverdue(dueDate: string | null | undefined, completed: boolean): boolean {
  if (!dueDate || completed) return false;
  return dayDifference(dueDate) < 0;
}

/**
 * Whether a task has not become actionable yet: its start date is a later
 * calendar day than today's.
 *
 * It lives beside the due-date helpers rather than in a file of its own so it
 * uses the same `dayDifference` they do. A second reading of "which day is it"
 * would disagree with this one across a month boundary, a daylight-saving
 * change, or the minute before midnight — and then a task would be in the
 * future by one measure and not by the other.
 *
 * A finished task is never "not started yet", and a task without a start date
 * has nothing to wait for.
 */
export function isNotStarted(startDate: string | null | undefined, completed: boolean): boolean {
  if (!startDate || completed) return false;
  return dayDifference(startDate) > 0;
}

export function formatDueDate(dueDate: string | null | undefined): string {
  if (!dueDate) return "";

  const diff = dayDifference(dueDate);
  if (diff === 0) return "今天";
  if (diff === 1) return "明天";
  if (diff === -1) return "昨天";

  const date = new Date(`${dueDate}T00:00:00`);
  const month = date.getMonth() + 1;
  const day = date.getDate();

  if (diff < -1) return `逾期 ${-diff} 天`;
  if (diff > 1 && diff <= 7) return `${diff} 天后`;
  return `${month}月${day}日`;
}

export type DueDateTone = "overdue" | "soon" | "normal" | "none";

export function dueDateTone(dueDate: string | null | undefined, completed: boolean): DueDateTone {
  if (!dueDate || completed) return "none";

  const diff = dayDifference(dueDate);
  if (diff < 0) return "overdue";
  if (diff <= 1) return "soon";
  return "normal";
}

export const TONE_LABEL_CLASS: Record<DueDateTone, string> = {
  overdue: "bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-300",
  soon: "bg-amber-100 text-amber-700 dark:bg-amber-900/40 dark:text-amber-300",
  normal: "bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-300",
  none: "",
};

/** Vue composable wrapping the due-date labels for a reactive `dueDate` ref. */
export function useDueLabels(dueDate: Ref<string | null | undefined>, completed: Ref<boolean>) {
  const label = computed(() => formatDueDate(dueDate.value));
  const tone = computed(() => dueDateTone(dueDate.value, completed.value));
  const overdue = computed(() => isOverdue(dueDate.value, completed.value));
  return { label, tone, overdue };
}
