<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { Check, Copy, SlidersHorizontal, Trash2 } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import TodoFields, {
  createEmptyDraft,
  draftFromTodo,
  draftHasDetails,
  isSameDraft,
  normalizeDraft,
  type TodoDraft,
} from "@/components/TodoFields.vue";
import { clearFormAlert } from "@/lib/form-alert";
import { dueDateTone, formatDueDate, isNotStarted, TONE_LABEL_CLASS } from "@/lib/dueDate";
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
const emit = defineEmits<{ edit: []; close: []; duplicate: [] }>();

const todoStore = useTodoStore();

const titleButtonRef = ref<HTMLButtonElement | null>(null);
const fieldsRef = ref<InstanceType<typeof TodoFields> | null>(null);

const draft = ref<TodoDraft>(createEmptyDraft());
/**
 * The draft as it was when the form opened. Comparing against it — rather than
 * against a list of fields — is what keeps "changed nothing, wrote nothing"
 * true when a field is added.
 */
const openedDraft = ref<TodoDraft>(createEmptyDraft());
/**
 * The title the form was opened on. The form keeps it as its accessible name
 * after a save, so the name does not change under a screen reader whose focus
 * is still inside the form.
 */
const openedTitle = ref("");
/**
 * Whether the optional fields are showing in this row's form.
 *
 * Decided once, when the form opens, from whether the todo has anything to show
 * there — a row with nothing but a title opens as a title and two buttons, which
 * is what keeps "click the title to fix a typo" from unfolding most of a screen.
 * After that it is the user's to toggle: recomputing it would collapse the form
 * under someone who has just emptied the last optional field.
 */
const detailsOpen = ref(false);

const completed = computed(() => props.todo.status === "completed");
/** A task whose start date has not arrived yet: set, but nothing to do about. */
const notStarted = computed(() => isNotStarted(props.todo.startDate, completed.value));

/** Fills the draft from the stored todo whenever this row opens for editing. */
watch(
  () => props.editing,
  (editing) => {
    if (!editing) {
      // Closing by any route — save, cancel, Esc, or another row taking over —
      // ends whatever the form had to say.
      clearFormAlert();
      return;
    }
    openedDraft.value = draftFromTodo(props.todo);
    draft.value = { ...openedDraft.value };
    // Asked of the draft, never of a list of field names: the container stays
    // ignorant of which fields exist.
    detailsOpen.value = draftHasDetails(openedDraft.value);
    openedTitle.value = props.todo.title;
    void focusTitle();
  },
);

/** Focuses the title field, once the form has actually been rendered. */
async function focusTitle(): Promise<void> {
  await nextTick();
  await fieldsRef.value?.focusTitle();
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

function finishEditing(): void {
  emit("close");
  void returnFocus();
}

function save(): void {
  // A save that changes nothing writes nothing: an empty change would still
  // queue an outbound sync operation, and an undo-stack entry that undoes
  // nothing visible. Compared as the two would be *stored* rather than as they
  // were typed — a title differing only by a trailing space is written as the
  // same string, so counting it as a change is counting nothing as something.
  // Asked before validating, because there is nothing to validate — and because
  // a todo synced in from a build with other rules would otherwise trap its
  // editor open, unable even to close unchanged.
  if (isSameDraft(normalizeDraft(draft.value), normalizeDraft(openedDraft.value))) {
    finishEditing();
    return;
  }

  // The store silently ignores an empty title, so a UI that let one through
  // would leave the user with a save that did nothing and said nothing. What
  // counts as valid is the field block's business; this only reads the verdict.
  if (!fieldsRef.value?.validate()) {
    void focusTitle();
    return;
  }

  todoStore.update(props.todo.id, draft.value);
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
      <!--
        The same field block the create form renders. Editing is "see all of it,
        change one of them", so its details region is always open and it has no
        expander to put in the `actions` slot.
      -->
      <TodoFields
        ref="fieldsRef"
        v-model="draft"
        :id-prefix="`todo-${todo.id}`"
        :details-open="detailsOpen"
        :details-id="`todo-${todo.id}-details`"
        @expand-details="detailsOpen = true"
      >
        <template #actions>
          <Button
            type="button"
            variant="ghost"
            class="ml-auto min-h-11 min-w-11 !p-2"
            :aria-label="detailsOpen ? '收起更多选项' : '展开更多选项'"
            :aria-expanded="detailsOpen"
            :aria-controls="`todo-${todo.id}-details`"
            @click="detailsOpen = !detailsOpen"
          >
            <SlidersHorizontal :size="20" />
          </Button>
        </template>
      </TodoFields>

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
        <!--
          The badge row shows on a start date too, not only a due date: a start
          date the user cannot see anywhere reads as a field that did nothing.
        -->
        <p v-if="todo.dueDate || notStarted" class="mb-0 mt-1 flex flex-wrap items-center gap-2">
          <span
            v-if="todo.dueDate"
            class="inline-block rounded-md px-1.5 py-0.5 text-xs font-medium"
            :class="TONE_LABEL_CLASS[dueDateTone(todo.dueDate, completed)]"
          >
            {{ formatDueDate(todo.dueDate) }}
            <span v-if="todo.reminderAt"> · 已设提醒</span>
          </span>
          <span
            v-if="notStarted"
            class="inline-block rounded-md px-1.5 py-0.5 text-xs font-medium"
            :class="TONE_LABEL_CLASS.normal"
          >
            {{ formatDueDate(todo.startDate) }}开始
          </span>
        </p>
      </div>
      <!--
        Duplicating is the one of the three new abilities that has to be one
        click away — its entire value is "make another one like this", and
        hiding it behind opening the editor would cost more clicks than typing
        the task again. Archiving has no button at all (a rule does it) and the
        rollover is a single setting, so the row gains exactly one control.
      -->
      <Button
        variant="ghost"
        class="min-h-11 min-w-11 !p-2 text-slate-400 dark:hover:text-slate-100"
        :aria-label="`复制 ${todo.title}`"
        @click="emit('duplicate')"
      >
        <Copy :size="20" />
      </Button>
      <!--
        Brought to the same 44×44 as the button beside it: it was 33×33, under
        the touch-target floor, and two neighbouring icon buttons of different
        sizes read as a mistake.
      -->
      <Button
        variant="ghost"
        class="min-h-11 min-w-11 !p-2 text-slate-400 hover:text-red-600"
        :aria-label="`删除 ${todo.title}`"
        @click="todoStore.remove(todo.id)"
      >
        <Trash2 :size="20" />
      </Button>
    </template>
  </article>
</template>
