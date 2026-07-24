<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { recurrenceNeverRecursAgain, recurrenceSummary } from "@/lib/recurrence";
import { todayLocalDay } from "@/lib/dueDate";
import type { RecurrenceFrequency, RecurrenceRule, Weekday } from "@/types/todo";

/**
 * The repeat rule of a todo — the one field in the block whose value is a whole
 * object rather than a string, so it does not go through the block's string
 * `codec` and lives here instead.
 *
 * It is a pure function of `modelValue`: every control derives its state from the
 * rule and every change emits a whole new rule, so there is no second copy of the
 * rule to keep in step. Two things that cannot be read straight off the contract —
 * "the end-date radio is chosen but no date is typed yet" and "the day-of-month is
 * being cleared to be retyped" — are held as an empty string and a `NaN` inside
 * the rule rather than as separate UI state: both read back as "still that mode"
 * (a value that is present but not yet storable), so the radio and the select do
 * not snap back under the user mid-edit. The field block reports the two blocking
 * states (interval and count) from the same rule; the export and the engine both
 * treat an empty `until` as "no end", so a rule left mid-edit is still safe to store.
 */
const props = defineProps<{
  modelValue: RecurrenceRule | null;
  /** The task's due date — the anchor a repeat is counted from. */
  anchor: string | null;
  /** Id of the frequency `<select>`, so the block can focus it on a blocking error. */
  fieldId: string;
  /** Id of the block's blocking-error line, to describe the frequency control. */
  errorId: string | null;
  /** The block's control registry, so the frequency `<select>` can be focused. */
  registerControl: (id: string, element: unknown) => void;
}>();

const emit = defineEmits<{ "update:modelValue": [RecurrenceRule | null] }>();

// Copied verbatim from TodoFields rather than shared through a new module: the
// spec calls for the identical strings, not a second token system.
const CONTROL_CLASS =
  "min-h-11 w-full rounded-lg border-0 bg-slate-100 px-3 py-2 text-base focus:ring-2 focus:ring-sky-500 sm:text-sm dark:bg-slate-900";
const NUMBER_CLASS =
  "min-h-11 w-24 rounded-lg border-0 bg-slate-100 px-3 py-2 text-base focus:ring-2 focus:ring-sky-500 sm:text-sm dark:bg-slate-900";
const LABEL_CLASS = "text-xs font-medium text-slate-500 dark:text-slate-400";

const FREQUENCY_OPTIONS: readonly { value: "none" | RecurrenceFrequency; label: string }[] = [
  { value: "none", label: "不重复" },
  { value: "daily", label: "每日" },
  { value: "weekly", label: "每周" },
  { value: "monthly", label: "每月" },
  { value: "yearly", label: "每年" },
  { value: "workday", label: "工作日" },
];

/** The unit "每 N ___" counts in, per frequency. */
const INTERVAL_UNIT: Record<RecurrenceFrequency, string> = {
  daily: "天",
  weekly: "周",
  monthly: "个月",
  yearly: "年",
  workday: "个工作日",
};

/** Monday-first, matching the `Weekday` enum and the engine's week. */
const WEEKDAYS: readonly { value: Weekday; short: string; label: string }[] = [
  { value: "monday", short: "一", label: "周一" },
  { value: "tuesday", short: "二", label: "周二" },
  { value: "wednesday", short: "三", label: "周三" },
  { value: "thursday", short: "四", label: "周四" },
  { value: "friday", short: "五", label: "周五" },
  { value: "saturday", short: "六", label: "周六" },
  { value: "sunday", short: "日", label: "周日" },
];

const MONTH_MODES = [
  { value: "same", label: "与截止日同一天" },
  { value: "day", label: "指定某日" },
  { value: "last", label: "每月最后一天" },
] as const;

const rule = computed(() => props.modelValue);
const isRecurring = computed(() => rule.value !== null);
const frequency = computed<RecurrenceFrequency | null>(() => rule.value?.frequency ?? null);
const frequencyValue = computed<"none" | RecurrenceFrequency>(
  () => rule.value?.frequency ?? "none",
);

const showWeekdays = computed(() => frequency.value === "weekly");
const showLunar = computed(() => frequency.value === "monthly" || frequency.value === "yearly");
const showMonthPosition = computed(() => frequency.value === "monthly");

const intervalUnit = computed(() => (frequency.value ? INTERVAL_UNIT[frequency.value] : ""));
const intervalDisplay = computed(() => {
  const value = rule.value?.interval;
  return typeof value === "number" && Number.isFinite(value) ? String(value) : "";
});

const isLunar = computed(() => rule.value?.calendar === "lunar");
const selectedWeekdays = computed(() => new Set(rule.value?.weekdays ?? []));

const monthMode = computed<"same" | "day" | "last">(() => {
  const value = rule.value;
  if (!value) return "same";
  if (value.onLastDay) return "last";
  return value.monthDay != null ? "day" : "same";
});
const monthDayDisplay = computed(() => {
  const value = rule.value?.monthDay;
  return typeof value === "number" && Number.isFinite(value) ? String(value) : "";
});

const endMode = computed<"never" | "until" | "count">(() => {
  const value = rule.value;
  if (!value) return "never";
  if (value.count != null) return "count";
  if (value.until != null) return "until";
  return "never";
});
const untilDisplay = computed(() => rule.value?.until ?? "");
const countDisplay = computed(() => {
  const value = rule.value?.count;
  return typeof value === "number" && Number.isFinite(value) ? String(value) : "";
});

/** The two blocking states, read off the rule the field block validates against. */
function intervalValid(value: RecurrenceRule): boolean {
  return Number.isInteger(value.interval) && value.interval >= 1 && value.interval <= 999;
}
function countValid(value: RecurrenceRule): boolean {
  return value.count == null || (Number.isInteger(value.count) && value.count >= 1);
}

/**
 * The inline echo of the whole rule, hidden while a blocking state is showing:
 * the red error line under the control speaks then, and a summary built on an
 * unreadable interval or count would only echo the mistake.
 */
const summary = computed(() => {
  const value = rule.value;
  if (!value || !intervalValid(value) || !countValid(value)) return null;
  return recurrenceSummary(value);
});

function emitRule(next: RecurrenceRule | null): void {
  emit("update:modelValue", next);
}

/** Writes part of the rule, leaving the rest as it stands. */
function patch(part: Partial<RecurrenceRule>): void {
  const current = rule.value;
  if (!current) return;
  emitRule({ ...current, ...part });
}

function onFrequencyChange(event: Event): void {
  const value = (event.target as HTMLSelectElement).value;
  if (value === "none") {
    emitRule(null);
    return;
  }
  const freq = value as RecurrenceFrequency;
  const current = rule.value;
  // Interval and end condition carry across a frequency change — they mean the
  // same in every frequency; the frequency-specific parts reset, because a
  // weekday set or a day-of-month has no reading under the new frequency.
  const interval =
    current && Number.isFinite(current.interval) && current.interval >= 1 ? current.interval : 1;
  const lunarCapable = freq === "monthly" || freq === "yearly";
  emitRule({
    frequency: freq,
    interval,
    weekdays: [],
    monthDay: null,
    onLastDay: false,
    calendar: lunarCapable && current?.calendar === "lunar" ? "lunar" : "gregorian",
    until: current?.until ?? null,
    count: current?.count ?? null,
  });
}

function onIntervalInput(event: Event): void {
  const raw = (event.target as HTMLInputElement).value.trim();
  patch({ interval: raw ? Number(raw) : Number.NaN });
}

function toggleWeekday(day: Weekday): void {
  const current = rule.value;
  if (!current) return;
  const chosen = new Set(current.weekdays);
  if (chosen.has(day)) chosen.delete(day);
  else chosen.add(day);
  // Rebuilt in Monday-first order so the stored set and its echo never depend on
  // the order the chips were tapped in.
  patch({
    weekdays: WEEKDAYS.map((weekday) => weekday.value).filter((value) => chosen.has(value)),
  });
}

function onLunarChange(event: Event): void {
  patch({ calendar: (event.target as HTMLInputElement).checked ? "lunar" : "gregorian" });
}

/** The day-of-month a fresh "指定某日" starts on: the anchor's own day, else the 1st. */
function defaultMonthDay(): number {
  const anchor = props.anchor;
  if (anchor) {
    const parsed = new Date(`${anchor}T00:00:00`);
    if (!Number.isNaN(parsed.getTime())) return parsed.getDate();
  }
  return 1;
}

function onMonthModeChange(event: Event): void {
  const mode = (event.target as HTMLSelectElement).value;
  if (mode === "last") patch({ monthDay: null, onLastDay: true });
  else if (mode === "day") patch({ monthDay: defaultMonthDay(), onLastDay: false });
  else patch({ monthDay: null, onLastDay: false });
}

function onMonthDayInput(event: Event): void {
  const raw = (event.target as HTMLInputElement).value.trim();
  patch({ monthDay: raw ? Number(raw) : Number.NaN });
}

function onEndModeChange(mode: "never" | "until" | "count"): void {
  const current = rule.value;
  if (!current) return;
  if (mode === "never") {
    patch({ until: null, count: null });
  } else if (mode === "until") {
    const kept =
      typeof current.until === "string" && current.until.trim()
        ? current.until
        : (props.anchor ?? todayLocalDay());
    patch({ until: kept, count: null });
  } else {
    const kept =
      typeof current.count === "number" && Number.isInteger(current.count) && current.count >= 1
        ? current.count
        : 1;
    patch({ count: kept, until: null });
  }
}

function onUntilInput(event: Event): void {
  // Empty is kept as "" rather than dropped to null: "" is still the "到某天"
  // mode with no date yet, while null would snap the radio back to 永不.
  patch({ until: (event.target as HTMLInputElement).value });
}

function onCountInput(event: Event): void {
  const raw = (event.target as HTMLInputElement).value.trim();
  patch({ count: raw ? Number(raw) : Number.NaN });
}

/**
 * A repeat with no due date never generates its next occurrence (the store
 * anchors on the due date), so it is said here — a note, not a block: the rule is
 * storable and a due date can be added afterwards.
 */
const missingAnchorWarning = computed(() =>
  isRecurring.value && !props.anchor
    ? "重复需要截止日作为起点：先设置截止日，这条重复才会生成下一次。"
    : null,
);

/**
 * Whether the chosen monthly/yearly position never comes round again from this
 * anchor (a lunar leap month, a short month's 30th, a February 31st). Asked of
 * the engine, so it is absent on a runtime with none (browser dev) and while the
 * interval is unreadable — the probe is only meaningful for a storable rule.
 */
const neverRecurs = ref(false);
let probeToken = 0;
watch(
  [rule, () => props.anchor],
  async () => {
    const value = rule.value;
    const anchor = props.anchor;
    const monthlyOrYearly = value?.frequency === "monthly" || value?.frequency === "yearly";
    if (!value || !anchor || !monthlyOrYearly || !intervalValid(value)) {
      neverRecurs.value = false;
      return;
    }
    const token = ++probeToken;
    const result = await recurrenceNeverRecursAgain(value, anchor);
    if (token === probeToken) neverRecurs.value = result;
  },
  { immediate: true },
);
const neverRecursWarning = computed(() =>
  neverRecurs.value ? "此重复位置在可预见的年份内不会再出现，保存后只会发生这一次。" : null,
);

const missingAnchorId = computed(() => `${props.fieldId}-missing-anchor`);
const neverRecursId = computed(() => `${props.fieldId}-never-recurs`);
/** Warnings first, then the block's blocking error — the block's own note order. */
const frequencyDescribedBy = computed(() => {
  const ids = [
    missingAnchorWarning.value ? missingAnchorId.value : null,
    neverRecursWarning.value ? neverRecursId.value : null,
    props.errorId,
  ].filter((id): id is string => id !== null && id !== undefined);
  return ids.length > 0 ? ids.join(" ") : undefined;
});
</script>

<template>
  <fieldset class="m-0 border-0 p-0">
    <legend class="mb-1" :class="LABEL_CLASS">重复</legend>
    <div class="grid gap-3">
      <div>
        <label :for="fieldId" class="mb-1 block" :class="LABEL_CLASS">频率</label>
        <select
          :id="fieldId"
          :ref="(element) => registerControl(fieldId, element)"
          :value="frequencyValue"
          :class="CONTROL_CLASS"
          :aria-describedby="frequencyDescribedBy"
          @change="onFrequencyChange"
        >
          <option v-for="option in FREQUENCY_OPTIONS" :key="option.value" :value="option.value">
            {{ option.label }}
          </option>
        </select>
      </div>

      <template v-if="isRecurring">
        <!--
          Interval: one number box carries "每 N", the unit read from the
          frequency. No `min` / `max`: like the block's own number field, native
          constraints bypass the page's single live region and break the implicit
          Enter submit — the 1–999 rule is the field block's blocking error instead.
        -->
        <div>
          <label :for="`${fieldId}-interval`" class="mb-1 block" :class="LABEL_CLASS"
            >重复间隔</label
          >
          <div class="flex items-center gap-2">
            <span class="text-sm text-slate-500 dark:text-slate-400">每</span>
            <input
              :id="`${fieldId}-interval`"
              type="number"
              inputmode="numeric"
              :value="intervalDisplay"
              :class="NUMBER_CLASS"
              @input="onIntervalInput"
            />
            <span class="text-sm text-slate-500 dark:text-slate-400">{{ intervalUnit }}</span>
          </div>
        </div>

        <div v-if="showWeekdays">
          <span :id="`${fieldId}-weekdays-label`" class="mb-1 block" :class="LABEL_CLASS"
            >重复的星期</span
          >
          <div
            role="group"
            :aria-labelledby="`${fieldId}-weekdays-label`"
            class="flex flex-wrap gap-2"
          >
            <button
              v-for="weekday in WEEKDAYS"
              :key="weekday.value"
              type="button"
              :aria-pressed="selectedWeekdays.has(weekday.value)"
              :aria-label="weekday.label"
              class="min-h-11 min-w-11 rounded-lg text-sm focus-visible:ring-2 focus-visible:ring-sky-500 focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-slate-950"
              :class="
                selectedWeekdays.has(weekday.value)
                  ? 'bg-sky-500 font-semibold text-white hover:bg-sky-600'
                  : 'bg-slate-100 font-medium text-slate-600 hover:bg-slate-200 dark:bg-slate-800 dark:text-slate-300 dark:hover:bg-slate-700'
              "
              @click="toggleWeekday(weekday.value)"
            >
              {{ weekday.short }}
            </button>
          </div>
        </div>

        <label
          v-if="showLunar"
          class="flex min-h-11 cursor-pointer items-center gap-3 rounded-lg px-3 py-2 hover:bg-slate-50 dark:hover:bg-slate-900"
        >
          <input
            type="checkbox"
            :checked="isLunar"
            class="h-5 w-5 rounded border-slate-300 text-sky-500 focus:ring-2 focus:ring-sky-500 dark:border-slate-600"
            @change="onLunarChange"
          />
          <span class="text-sm text-slate-700 dark:text-slate-200">按农历重复</span>
        </label>

        <div v-if="showMonthPosition">
          <label :for="`${fieldId}-month-mode`" class="mb-1 block" :class="LABEL_CLASS"
            >月内位置</label
          >
          <select
            :id="`${fieldId}-month-mode`"
            :value="monthMode"
            :class="CONTROL_CLASS"
            @change="onMonthModeChange"
          >
            <option v-for="option in MONTH_MODES" :key="option.value" :value="option.value">
              {{ option.label }}
            </option>
          </select>
          <div v-if="monthMode === 'day'" class="mt-2">
            <label :for="`${fieldId}-month-day`" class="mb-1 block" :class="LABEL_CLASS"
              >第几日</label
            >
            <input
              :id="`${fieldId}-month-day`"
              type="number"
              inputmode="numeric"
              :value="monthDayDisplay"
              :class="CONTROL_CLASS"
              @input="onMonthDayInput"
            />
          </div>
        </div>

        <fieldset class="m-0 border-0 p-0">
          <legend class="mb-1" :class="LABEL_CLASS">结束</legend>
          <div class="grid gap-1">
            <label
              class="flex min-h-11 cursor-pointer items-center gap-3 rounded-lg px-3 py-2 hover:bg-slate-50 dark:hover:bg-slate-900"
            >
              <input
                type="radio"
                :name="`${fieldId}-end`"
                :checked="endMode === 'never'"
                @change="onEndModeChange('never')"
              />
              <span class="text-sm text-slate-700 dark:text-slate-200">永不结束</span>
            </label>
            <label
              class="flex min-h-11 cursor-pointer items-center gap-3 rounded-lg px-3 py-2 hover:bg-slate-50 dark:hover:bg-slate-900"
            >
              <input
                type="radio"
                :name="`${fieldId}-end`"
                :checked="endMode === 'until'"
                @change="onEndModeChange('until')"
              />
              <span class="text-sm text-slate-700 dark:text-slate-200">到某天停止</span>
            </label>
            <div v-if="endMode === 'until'" class="px-3">
              <label :for="`${fieldId}-until`" class="mb-1 block" :class="LABEL_CLASS"
                >结束日期</label
              >
              <input
                :id="`${fieldId}-until`"
                type="date"
                :value="untilDisplay"
                :class="CONTROL_CLASS"
                @input="onUntilInput"
              />
            </div>
            <label
              class="flex min-h-11 cursor-pointer items-center gap-3 rounded-lg px-3 py-2 hover:bg-slate-50 dark:hover:bg-slate-900"
            >
              <input
                type="radio"
                :name="`${fieldId}-end`"
                :checked="endMode === 'count'"
                @change="onEndModeChange('count')"
              />
              <span class="text-sm text-slate-700 dark:text-slate-200">重复次数后停止</span>
            </label>
            <div v-if="endMode === 'count'" class="px-3">
              <label :for="`${fieldId}-count`" class="mb-1 block" :class="LABEL_CLASS"
                >重复次数</label
              >
              <input
                :id="`${fieldId}-count`"
                type="number"
                inputmode="numeric"
                :value="countDisplay"
                :class="CONTROL_CLASS"
                @input="onCountInput"
              />
            </div>
          </div>
        </fieldset>

        <p
          v-if="missingAnchorWarning"
          :id="missingAnchorId"
          class="m-0 text-xs text-amber-700 dark:text-amber-300"
        >
          {{ missingAnchorWarning }}
        </p>
        <p
          v-if="neverRecursWarning"
          :id="neverRecursId"
          class="m-0 text-xs text-amber-700 dark:text-amber-300"
        >
          {{ neverRecursWarning }}
        </p>
        <p v-if="summary" class="m-0 text-xs text-slate-500 dark:text-slate-400">{{ summary }}</p>
      </template>
    </div>
  </fieldset>
</template>
