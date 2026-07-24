<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import { Plus, Trash2 } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { announceFormAlert } from "@/lib/form-alert";
import { currentZoneOffsetSeconds, toInstant, toLocalDateTimeInput } from "@/lib/datetime";
import type { Reminder } from "@/types/todo";

/**
 * The reminder points of a todo — the one field in the block whose value is a
 * list of objects rather than a string, so it does not go through the block's
 * string `codec` and lives here, the twin of `RecurrenceField`.
 *
 * It rides the draft like recurrence does (not the store like the subtask/
 * attachment/dependency lists): a reminder is part of a task's schedule, set at
 * creation time and saved with the task, so every change goes out as a whole new
 * list through `update:modelValue` and lands only when the create Enter or the
 * editor's 保存 fires. It borrows `SubtaskList`'s "rows + add box" shape but not
 * its immediate-commit behaviour.
 *
 * A point is a wall clock plus the device offset it was set at: the input speaks
 * the wall clock (`datetime-local`), and the offset is captured beside it so the
 * native scheduler can float or fix the reminder later (提醒通知/07/08). Order is
 * insertion order and carries no meaning — each point fires at its own time — so
 * there is no sorting and no reorder control, one less than the checklist has.
 */
const props = defineProps<{
  modelValue: readonly Reminder[];
  /** Id namespace, so a row's describing note has a stable id of its own. */
  fieldId: string;
}>();

const emit = defineEmits<{ "update:modelValue": [Reminder[]] }>();

/**
 * The contract's per-list ceiling (`MAX_LIST_ENTRIES`), mirrored here as
 * `SubtaskList` mirrors it: the number is private to the Rust crate, and this is
 * the last guard before a list the database would refuse and then lose on
 * restart. A task's real reminder count is far below it; this is only a backstop.
 */
const MAX_REMINDERS = 200;

// Copied verbatim from TodoFields rather than shared through a new module: the
// spec calls for the identical strings, not a second token system (as
// RecurrenceField does).
const CONTROL_CLASS =
  "min-h-11 w-full rounded-lg border-0 bg-slate-100 px-3 py-2 text-base focus:ring-2 focus:ring-sky-500 sm:text-sm dark:bg-slate-900";
const LABEL_CLASS = "text-xs font-medium text-slate-500 dark:text-slate-400";

const reminders = computed(() => props.modelValue);

/**
 * Whether the add input is showing while the list is empty. An empty task shows
 * only a "＋ 添加提醒" trigger, so a plain task's editor stays a plain task; the
 * trigger reveals the input. With ≥1 point the input is always there.
 */
const addRevealed = ref(false);
const showAddInput = computed(() => reminders.value.length > 0 || addRevealed.value);
/** The add input's pending value, cleared after each add so points can be added in a run. */
const newValue = ref("");

const listRef = ref<HTMLElement | null>(null);
const addInputRef = ref<HTMLInputElement | null>(null);
const addTriggerRef = ref<HTMLElement | null>(null);

/** Each point as the template needs it: the input value, whether it has passed, its ids. */
const rows = computed(() =>
  reminders.value.map((reminder, index) => {
    const at = new Date(reminder.at).getTime();
    return {
      index,
      value: toLocalDateTimeInput(reminder.at),
      label: readableTime(reminder.at),
      past: !Number.isNaN(at) && at < Date.now(),
      pastId: `${props.fieldId}-past-${index}`,
    };
  }),
);

/** A reminder's instant as a short local label, for the delete button's name. */
function readableTime(at: string): string {
  const parsed = new Date(at);
  return Number.isNaN(parsed.getTime()) ? "该提醒" : parsed.toLocaleString();
}

/** A point built from an input value, or `null` when the value is not a time. */
function reminderFrom(value: string): Reminder | null {
  const at = toInstant(value);
  if (!at) return null;
  return { at, offset: currentZoneOffsetSeconds() };
}

/** Reveals the add input on an empty list and puts the cursor in it. */
function revealAdd(): void {
  addRevealed.value = true;
  void nextTick(() => addInputRef.value?.focus());
}

/**
 * Appends a point from the add input. An empty or unreadable value is a no-op,
 * and so is a point that would overflow the contract's ceiling — said once
 * through the page's single live region. The input is cleared and kept focused so
 * points can be added in a run, exactly as the checklist's add box behaves.
 */
function addReminder(event: Event): void {
  const reminder = reminderFrom((event.target as HTMLInputElement).value);
  if (!reminder) {
    newValue.value = "";
    return;
  }
  if (reminders.value.length >= MAX_REMINDERS) {
    announceFormAlert("warn", `提醒数量已达上限 ${MAX_REMINDERS} 项，无法再添加。`, "transient");
    newValue.value = "";
    return;
  }
  emit("update:modelValue", [...reminders.value, reminder]);
  newValue.value = "";
  void nextTick(() => addInputRef.value?.focus());
}

/**
 * Edits a point in place. Clearing it to an unreadable value drops the row —
 * an empty `datetime-local` is not a reminder, the same reading the single field
 * had ("空 datetime = null").
 */
function onRowChange(index: number, event: Event): void {
  const reminder = reminderFrom((event.target as HTMLInputElement).value);
  if (!reminder) {
    removeReminder(index);
    return;
  }
  const next = reminders.value.map((existing, position) =>
    position === index ? reminder : existing,
  );
  emit("update:modelValue", next);
}

/**
 * Drops a point, then hands focus to the row that slid into its place — or to the
 * add trigger/input when it was the last — so a keyboard user is never dropped
 * onto the page body, one for one with `SubtaskList.focusAfterDelete`.
 */
function removeReminder(index: number): void {
  const next = reminders.value.filter((_, position) => position !== index);
  if (next.length === 0) addRevealed.value = true;
  emit("update:modelValue", next);
  void nextTick(() => focusAfterDelete(index));
}

function focusAfterDelete(removedIndex: number): void {
  const remaining = reminders.value.length;
  if (remaining === 0) {
    (addInputRef.value ?? addTriggerRef.value?.querySelector<HTMLElement>("button"))?.focus();
    return;
  }
  const targetIndex = Math.min(removedIndex, remaining - 1);
  const buttons = listRef.value?.querySelectorAll<HTMLElement>('[data-role="delete"]');
  buttons?.[targetIndex]?.focus();
}
</script>

<template>
  <fieldset class="m-0 border-0 p-0">
    <legend class="mb-1" :class="LABEL_CLASS">提醒</legend>
    <div class="grid gap-2">
      <div v-if="rows.length" ref="listRef" class="grid gap-2">
        <template v-for="row in rows" :key="row.index">
          <div class="flex items-center gap-2">
            <input
              type="datetime-local"
              :value="row.value"
              :aria-label="`提醒时间（第 ${row.index + 1} 项）`"
              :aria-describedby="row.past ? row.pastId : undefined"
              :class="CONTROL_CLASS"
              @change="onRowChange(row.index, $event)"
            />
            <Button
              variant="ghost"
              class="min-h-11 min-w-11 shrink-0 !p-2 text-slate-400 hover:text-red-600 dark:hover:text-red-400"
              data-role="delete"
              :aria-label="`删除提醒 ${row.label}`"
              @click="removeReminder(row.index)"
            >
              <Trash2 :size="20" />
            </Button>
          </div>
          <p
            v-if="row.past"
            :id="row.pastId"
            class="m-0 text-xs text-amber-700 dark:text-amber-300"
          >
            提醒时间已过，保存后会立即提醒一次。
          </p>
        </template>
      </div>

      <input
        v-if="showAddInput"
        ref="addInputRef"
        type="datetime-local"
        :value="newValue"
        aria-label="新提醒时间"
        :class="CONTROL_CLASS"
        @change="addReminder"
      />
      <span v-else ref="addTriggerRef">
        <Button
          variant="ghost"
          class="min-h-11 w-fit !justify-start !px-2 text-sm text-slate-500 dark:text-slate-400"
          @click="revealAdd"
        >
          <Plus :size="16" /><span class="ml-1">添加提醒</span>
        </Button>
      </span>
    </div>
  </fieldset>
</template>
