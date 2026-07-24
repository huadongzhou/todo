<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { BellRing, X } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { currentZoneOffsetSeconds } from "@/lib/datetime";
import type { Todo } from "@/types/todo";
import { useSettingsStore } from "@/stores/settings";
import { useTodoStore } from "@/stores/todos";

/**
 * The in-app half of "snooze": one bar above the list that gathers the reminders
 * that have come due and are still waiting, and offers to push each one to a
 * later moment.
 *
 * Snooze is not its own mechanism here — the native scheduler (提醒通知/01) polls
 * the database and raises whatever reminder is due and not yet delivered, so
 * "remind me later" is just moving the due reminder point to a future instant: the
 * next poll finds the new `(id, at)` key and raises it then, while the key it
 * already delivered stays quiet. A task carries a list of points (提醒通知/08); the
 * bar acts on the one that has most recently come due, moving that point and
 * leaving the task's other points untouched. So every action on this bar is an
 * ordinary `todoStore.update`/`toggle`, and nothing new has to be persisted.
 *
 * The bar announces a snooze through the page's single live region rather than a
 * region of its own; it hands the sentence up to `App.vue` (the region's owner)
 * as the `announce` event, and hands up `null` when the line should be dropped.
 */
const emit = defineEmits<{ (e: "announce", message: string | null): void }>();

const todoStore = useTodoStore();
const settingsStore = useSettingsStore();

/**
 * "今晚" and "明天" are fixed local times for now. They are constants rather than
 * settings on purpose (see the settings store): once 默认提醒时间 (外观与设置/02)
 * and 晨间摘要时间 (提醒通知/06) land, "明天" should follow the morning time so a
 * second copy does not drift — this is the single place to wire that.
 */
const TONIGHT_HOUR = 20;
const TOMORROW_HOUR = 9;

/**
 * A clock the derivation can react to. A reminder becomes due as time passes with
 * nothing else changing, so without a ticking `now` the bar would only notice on
 * the next unrelated store write. The cadence matches the native poll (提醒通知/01):
 * a task reminder does not need finer than that.
 */
const POLL_INTERVAL_MS = 30_000;
const now = ref(Date.now());
let ticker: ReturnType<typeof setInterval> | undefined;

/**
 * Reminders the user has set aside this session — a skipped item, or every item
 * present when the bar was closed. Kept only in memory: skipping and closing are
 * "not now, and do not touch the data", so they must not write the task's
 * reminders. A reminder that comes due later (a different task, or a snooze that
 * expires) is a new id-or-instant and is not in here, so the bar returns for it.
 */
const dismissed = ref<ReadonlySet<string>>(new Set());

const actionsRef = ref<HTMLElement | null>(null);

/**
 * The instant of the task's most-recently-due reminder point at `cutoff`, or
 * `null` when none of its points has come due yet. This is the point the bar
 * shows and acts on: a task with several points surfaces once, for whichever has
 * most recently arrived, and the earlier ones it already passed are behind it.
 */
function dueReminderIso(todo: Todo, cutoff: number): string | null {
  let bestIso: string | null = null;
  let bestAt = Number.NEGATIVE_INFINITY;
  for (const reminder of todo.reminders) {
    const at = Date.parse(reminder.at);
    if (Number.isNaN(at) || at > cutoff) continue;
    if (at > bestAt) {
      bestAt = at;
      bestIso = reminder.at;
    }
  }
  return bestIso;
}

/**
 * The tasks with a reminder point due and still waiting, most-recently-due first.
 *
 * The gate is the same one the native scheduler applies (提醒通知/01
 * `due_reminders`): open, not archived, not locked by a prerequisite, with a
 * reminder point whose instant has passed. `visibleItems` already carries the
 * single reading of "archived", so this filters from it rather than repeating that
 * test.
 */
const matches = computed<Todo[]>(() => {
  const cutoff = now.value;
  return todoStore.visibleItems
    .filter((todo) => {
      if (todo.status !== "open") return false;
      if (dismissed.value.has(todo.id)) return false;
      if (dueReminderIso(todo, cutoff) === null) return false;
      return !todoStore.dependencyLock(todo.id).locked;
    })
    .sort(
      (left, right) =>
        Date.parse(dueReminderIso(right, cutoff)!) - Date.parse(dueReminderIso(left, cutoff)!),
    );
});

/** The one task shown at a time; the rest wait their turn behind it. */
const current = computed<Todo | null>(() => matches.value[0] ?? null);
const remaining = computed(() => Math.max(0, matches.value.length - 1));

const currentTitle = computed(() => current.value?.title ?? "");
const currentSubline = computed(() => {
  const target = current.value;
  const at = target ? dueReminderIso(target, now.value) : null;
  return at ? `已到点 · ${relativeSince(at, now.value)}` : "";
});

interface SnoozeOption {
  readonly key: string;
  readonly label: string;
  readonly targetIso: string;
}

/**
 * The duration buttons to show: the ones the user enabled that resolve to an
 * instant strictly after now. "今晚" drops out once 20:00 has passed — snoozing
 * to a moment already gone has no meaning, and rolling it to tomorrow night
 * would make the label lie.
 */
const options = computed<SnoozeOption[]>(() => {
  const base = now.value;
  const result: SnoozeOption[] = [];
  if (settingsStore.snooze5m) {
    result.push({
      key: "5m",
      label: "5 分钟",
      targetIso: new Date(base + 5 * 60_000).toISOString(),
    });
  }
  if (settingsStore.snooze1h) {
    result.push({
      key: "1h",
      label: "1 小时",
      targetIso: new Date(base + 60 * 60_000).toISOString(),
    });
  }
  if (settingsStore.snoozeTonight) {
    const tonight = todayAt(base, TONIGHT_HOUR);
    if (tonight > base) {
      result.push({ key: "tonight", label: "今晚", targetIso: new Date(tonight).toISOString() });
    }
  }
  if (settingsStore.snoozeTomorrow) {
    result.push({
      key: "tomorrow",
      label: "明天",
      targetIso: new Date(tomorrowAt(base, TOMORROW_HOUR)).toISOString(),
    });
  }
  return result;
});

/** Today at `hour`:00 local. */
function todayAt(reference: number, hour: number): number {
  const date = new Date(reference);
  date.setHours(hour, 0, 0, 0);
  return date.getTime();
}

/** The next local day at `hour`:00. Stepped by calendar day, not +24h, so it is
 * the right day the hour a daylight-saving change shifts the clock. */
function tomorrowAt(reference: number, hour: number): number {
  const date = new Date(reference);
  date.setDate(date.getDate() + 1);
  date.setHours(hour, 0, 0, 0);
  return date.getTime();
}

/** How long ago an instant passed, in words, for the "已到点 · …" subline. */
function relativeSince(iso: string, reference: number): string {
  const then = Date.parse(iso);
  if (Number.isNaN(then)) return "";
  const minutes = Math.max(0, Math.round((reference - then) / 60_000));
  if (minutes < 1) return "刚刚";
  if (minutes < 60) return `${minutes} 分钟前`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} 小时前`;
  return `${Math.round(hours / 24)} 天前`;
}

/** The target instant as a short local label, for the snooze confirmation. */
function formatTarget(iso: string, reference: number): string {
  const target = new Date(iso);
  if (Number.isNaN(target.getTime())) return "";
  const time = target.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  const dayDiff = calendarDayDiff(reference, target.getTime());
  if (dayDiff <= 0) return time;
  if (dayDiff === 1) return `明天 ${time}`;
  return `${target.getMonth() + 1}月${target.getDate()}日 ${time}`;
}

/** Whole local calendar days between two instants. */
function calendarDayDiff(from: number, to: number): number {
  const start = new Date(from);
  start.setHours(0, 0, 0, 0);
  const end = new Date(to);
  end.setHours(0, 0, 0, 0);
  return Math.round((end.getTime() - start.getTime()) / 86_400_000);
}

function dismiss(id: string): void {
  const next = new Set(dismissed.value);
  next.add(id);
  dismissed.value = next;
}

function snooze(option: SnoozeOption): void {
  const target = current.value;
  if (!target) return;
  const dueIso = dueReminderIso(target, now.value);
  if (!dueIso) return;
  // Moving the due point into the future is the whole of "snooze": that point
  // leaves the due set at once and the native poll raises it again at the new
  // instant, while the task's other points are left where they are. The snoozed
  // point is re-stamped with the current device offset, since the user is
  // re-choosing its time here and now.
  const reminders = target.reminders.map((reminder) =>
    reminder.at === dueIso
      ? { at: option.targetIso, offset: currentZoneOffsetSeconds() }
      : reminder,
  );
  todoStore.update(target.id, { reminders });
  emit(
    "announce",
    `已稍后：${target.title} 将在 ${formatTarget(option.targetIso, now.value)} 再次提醒。`,
  );
  void advanceFocus();
}

function complete(): void {
  const target = current.value;
  if (!target) return;
  todoStore.toggle(target.id);
  // A completion is not a snooze, so any lingering "已稍后 …" line is now stale.
  emit("announce", null);
  void advanceFocus();
}

function skip(): void {
  const target = current.value;
  if (!target) return;
  dismiss(target.id);
  emit("announce", null);
  void advanceFocus();
}

function close(): void {
  // Set the whole current batch aside for the session; the bar returns when a
  // new reminder comes due.
  const next = new Set(dismissed.value);
  for (const todo of matches.value) next.add(todo.id);
  dismissed.value = next;
  emit("announce", null);
}

/**
 * After an action the bar advances to the next reminder; move focus onto its
 * first control so a keyboard user is not dropped. When there is no next one the
 * section unmounts and focus falls to the body, which is the intended fallback.
 */
async function advanceFocus(): Promise<void> {
  await nextTick();
  if (!current.value) return;
  actionsRef.value?.querySelector<HTMLElement>("button")?.focus();
}

/**
 * When the bar empties, retract this source's line. A snooze confirmation is
 * transient/success, and while the bar still has a next reminder it clears on the
 * next action (snooze/skip/complete/close). But snoozing the *last* due reminder,
 * or having the current one leave for a reason outside the bar (a sync from
 * another device completes it), takes `current` to null: the section hides yet
 * the component stays mounted (its tag in `App.vue` has no `v-if`), so neither
 * "next action" nor `onBeforeUnmount` ever fires to drop the line. Left standing
 * it would cover a storage/sync warning — a standing line the page must keep
 * showing — so the invariant in `page-alert.ts` (a transient cover has a definite
 * end) would not hold at this edge. `emit("announce", null)` withdraws only the
 * reminder source; the page re-picks and any storage line returns. Guarded on the
 * has→empty edge so advancing to the next reminder never retracts a fresh line.
 */
watch(current, (value, previous) => {
  if (!value && previous) emit("announce", null);
});

onMounted(() => {
  ticker = setInterval(() => {
    now.value = Date.now();
  }, POLL_INTERVAL_MS);
});

onBeforeUnmount(() => {
  if (ticker !== undefined) clearInterval(ticker);
  emit("announce", null);
});
</script>

<template>
  <section
    v-if="settingsStore.isDesktop && current"
    class="surface-card mb-6 p-4"
    aria-labelledby="reminder-bar-heading"
  >
    <div class="flex items-start justify-between gap-3">
      <div class="flex items-center gap-3">
        <span
          class="flex h-11 w-11 items-center justify-center rounded-xl bg-sky-100 text-sky-600"
          aria-hidden="true"
        >
          <BellRing :size="20" />
        </span>
        <h2
          id="reminder-bar-heading"
          class="m-0 text-base font-semibold text-slate-800 dark:text-slate-100"
        >
          提醒待处理
        </h2>
      </div>
      <Button
        variant="ghost"
        class="min-h-11 min-w-11 !p-2"
        aria-label="关闭提醒快捷条"
        @click="close"
      >
        <X :size="18" />
      </Button>
    </div>

    <div class="mt-3 min-w-0">
      <p class="m-0 truncate font-medium text-slate-800 dark:text-slate-100">{{ currentTitle }}</p>
      <p class="mb-0 mt-1 text-xs tabular-nums text-slate-500 dark:text-slate-400">
        {{ currentSubline }}
      </p>
    </div>

    <div ref="actionsRef" class="mt-3 flex flex-wrap items-center gap-2">
      <template v-if="options.length">
        <Button
          v-for="option in options"
          :key="option.key"
          variant="ghost"
          class="min-h-11 !px-3 !py-1 text-xs"
          @click="snooze(option)"
        >
          {{ option.label }}
        </Button>
      </template>
      <span v-else class="text-xs text-slate-500 dark:text-slate-400"
        >未启用快捷时长，可在设置中开启</span
      >
      <Button variant="ghost" class="min-h-11" @click="complete">完成</Button>
      <Button variant="ghost" class="min-h-11" @click="skip">跳过</Button>
      <span v-if="remaining > 0" class="text-xs text-slate-500 dark:text-slate-400"
        >还有 {{ remaining }} 项</span
      >
    </div>
  </section>
</template>
