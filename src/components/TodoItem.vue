<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { Check, Trash2 } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { toInstant, toLocalDateTimeInput } from "@/lib/datetime";
import { dueDateTone, formatDueDate, TONE_LABEL_CLASS } from "@/lib/dueDate";
import { MAX_TITLE_CHARS } from "@/lib/native";
import { useTodoStore } from "@/stores/todos";
import type { Todo } from "@/types/todo";

/**
 * One todo row, in either of its two forms: the display row, and the inline
 * form the title button opens.
 *
 * The draft lives here because it belongs to a single row; *which* row is open
 * lives in the page, because at most one may be — two rows editing at once turn
 * the list into a wall of forms and scatter the tab order.
 */
const props = defineProps<{ todo: Todo; editing: boolean }>();
const emit = defineEmits<{ edit: []; close: [] }>();

const todoStore = useTodoStore();

const titleButtonRef = ref<HTMLButtonElement | null>(null);
const titleInputRef = ref<HTMLInputElement | null>(null);

const draftTitle = ref("");
const draftDueDate = ref("");
const draftReminderAt = ref("");
const titleError = ref(false);
/**
 * The title the form was opened on. The form keeps it as its accessible name
 * after a save, so the name does not change under a screen reader whose focus
 * is still inside the form.
 */
const openedTitle = ref("");

const completed = computed(() => props.todo.status === "completed");

/**
 * A reminder in the past is a legitimate intent — it fires once, immediately —
 * so this only warns; it never blocks the save.
 */
const reminderInPast = computed(() => {
  const instant = toInstant(draftReminderAt.value);
  return instant !== null && new Date(instant).getTime() < Date.now();
});

/** Fills the draft from the stored todo whenever this row opens for editing. */
watch(
  () => props.editing,
  (editing) => {
    if (!editing) return;
    draftTitle.value = props.todo.title;
    draftDueDate.value = props.todo.dueDate ?? "";
    draftReminderAt.value = toLocalDateTimeInput(props.todo.reminderAt);
    titleError.value = false;
    openedTitle.value = props.todo.title;
    void focusTitleInput();
  },
);

// The error is raised on blur and on submit, never per keystroke; but once the
// user has typed something it has nothing left to report, so it goes at once.
watch(draftTitle, (value) => {
  if (titleError.value && value.trim()) titleError.value = false;
});

async function focusTitleInput(): Promise<void> {
  await nextTick();
  const input = titleInputRef.value;
  if (!input) return;
  input.focus();
  // Caret at the end rather than a full selection: a selected title would be
  // wiped by the first keystroke of someone who only meant to append.
  const end = input.value.length;
  input.setSelectionRange(end, end);
}

/**
 * Hands focus back to the row's title button. The form is an expanding layer,
 * and without this the focus falls to `<body>` on close, which makes a keyboard
 * user tab in again from the top of the page.
 */
async function returnFocus(): Promise<void> {
  await nextTick();
  titleButtonRef.value?.focus();
}

function validateTitle(): boolean {
  const valid = draftTitle.value.trim().length > 0;
  titleError.value = !valid;
  return valid;
}

function finishEditing(): void {
  emit("close");
  void returnFocus();
}

function save(): void {
  // The store silently ignores an empty title, so a UI that let one through
  // would leave the user with a save that did nothing and said nothing.
  if (!validateTitle()) {
    void focusTitleInput();
    return;
  }

  const title = draftTitle.value.trim();
  const dueDate = draftDueDate.value || null;
  const reminderAt = toInstant(draftReminderAt.value);
  // A save that changes nothing writes nothing: an empty change would still
  // queue an outbound sync operation, and later an undo-stack entry that undoes
  // nothing visible.
  const unchanged =
    title === props.todo.title &&
    dueDate === (props.todo.dueDate ?? null) &&
    reminderAt === (props.todo.reminderAt ?? null);
  if (!unchanged) todoStore.update(props.todo.id, { title, dueDate, reminderAt });

  finishEditing();
}

/** Discards the draft. It never reached storage, so there is nothing to undo. */
function cancel(): void {
  finishEditing();
}
</script>

<template>
  <article
    class="border-b border-slate-100 px-4 py-3 last:border-0 dark:border-slate-800"
    :class="editing ? '' : 'flex items-center gap-3'"
  >
    <!--
      Editing hides the completion circle and the delete button: a destructive
      action must not sit next to an open form, and the tab order inside the row
      should be the form and nothing else.
    -->
    <form
      v-if="editing"
      class="grid gap-3"
      :aria-label="`编辑 ${openedTitle}`"
      @submit.prevent="save"
      @keydown.esc="cancel"
    >
      <label class="block">
        <span class="mb-1 block text-xs font-medium text-slate-500 dark:text-slate-400">标题</span>
        <input
          ref="titleInputRef"
          v-model="draftTitle"
          :maxlength="MAX_TITLE_CHARS"
          class="min-h-11 w-full rounded-lg border-0 bg-slate-100 px-3 py-2 text-base focus:ring-2 focus:ring-sky-500 sm:text-sm dark:bg-slate-900"
          :aria-invalid="titleError ? 'true' : undefined"
          :aria-describedby="titleError ? `todo-title-error-${todo.id}` : undefined"
          @blur="validateTitle()"
        />
      </label>
      <p
        v-if="titleError"
        :id="`todo-title-error-${todo.id}`"
        class="m-0 text-xs text-red-600 dark:text-red-400"
      >
        标题不能为空，请输入内容，或按 Esc 取消编辑。
      </p>

      <div class="grid gap-3 sm:grid-cols-2">
        <label class="block">
          <span class="mb-1 block text-xs font-medium text-slate-500 dark:text-slate-400"
            >截止日</span
          >
          <input
            v-model="draftDueDate"
            type="date"
            class="min-h-11 w-full rounded-lg border-0 bg-slate-100 px-3 py-2 text-base focus:ring-2 focus:ring-sky-500 sm:text-sm dark:bg-slate-900"
          />
        </label>
        <label class="block">
          <span class="mb-1 block text-xs font-medium text-slate-500 dark:text-slate-400"
            >提醒时间</span
          >
          <input
            v-model="draftReminderAt"
            type="datetime-local"
            class="min-h-11 w-full rounded-lg border-0 bg-slate-100 px-3 py-2 text-base focus:ring-2 focus:ring-sky-500 sm:text-sm dark:bg-slate-900"
          />
        </label>
      </div>
      <p v-if="reminderInPast" class="m-0 text-xs text-amber-700 dark:text-amber-300">
        提醒时间已过，保存后会立即提醒一次。
      </p>

      <div class="flex flex-wrap items-center justify-between gap-3">
        <p class="m-0 text-xs text-slate-500 dark:text-slate-400">Enter 保存 · Esc 取消</p>
        <div class="flex gap-2">
          <Button variant="ghost" class="min-h-11" @click="cancel">取消</Button>
          <!--
            Deliberately never disabled: an editor that empties the title would
            read a greyed-out button as a stuck screen, and a disabled submit
            also kills the implicit Enter submission.
          -->
          <Button type="submit" class="min-h-11">保存</Button>
        </div>
      </div>
    </form>

    <template v-else>
      <button
        class="flex h-6 w-6 shrink-0 items-center justify-center rounded-full border border-slate-300 text-white focus-ring dark:border-slate-600"
        :class="completed ? 'border-sky-500 bg-sky-500' : 'bg-white dark:bg-slate-950'"
        :aria-label="completed ? '标记为未完成' : '标记为完成'"
        @click="todoStore.toggle(todo.id)"
      >
        <Check v-if="completed" :size="15" />
      </button>
      <div class="min-w-0 flex-1">
        <!--
          The title is the editor's trigger: it is the thing being edited, so
          pointing at it is the whole instruction. Hover shows an underline
          instead of a background block, which keeps the row's geometry — and
          the 12px gap to its neighbours — exactly as it was.

          `border-0 bg-transparent p-0` is what makes that true here: the
          project ships no CSS reset, so a bare <button> still carries the
          browser's grey face, 2px outset border and 6px inline padding, which
          would turn the title into a chrome box and indent it away from the
          due-date label underneath.

          The two colour sets are exclusive branches rather than a base plus an
          override: `text-slate-400` and `text-slate-700` weigh the same, so
          which one won would come down to the order UnoCSS happened to emit
          them in, and the completed row would quietly keep the active colour.
        -->
        <button
          ref="titleButtonRef"
          class="m-0 block min-h-11 w-full rounded-md border-0 bg-transparent p-0 text-left hover:underline focus-visible:focus-ring sm:min-h-0"
          :class="
            completed
              ? 'text-slate-400 line-through hover:text-slate-400 dark:text-slate-500 dark:hover:text-slate-500'
              : 'text-slate-700 hover:text-slate-950 dark:text-slate-200 dark:hover:text-slate-100'
          "
          :aria-label="`编辑 ${todo.title}`"
          @click="emit('edit')"
        >
          {{ todo.title }}
        </button>
        <p
          v-if="todo.dueDate"
          class="mb-0 mt-1 inline-block rounded-md px-1.5 py-0.5 text-xs font-medium"
          :class="TONE_LABEL_CLASS[dueDateTone(todo.dueDate, completed)]"
        >
          {{ formatDueDate(todo.dueDate) }}
          <span v-if="todo.reminderAt"> · 已设提醒</span>
        </p>
      </div>
      <Button
        variant="ghost"
        class="!p-2 text-slate-400 hover:text-red-600"
        :aria-label="`删除 ${todo.title}`"
        @click="todoStore.remove(todo.id)"
      >
        <Trash2 :size="17" />
      </Button>
    </template>
  </article>
</template>
