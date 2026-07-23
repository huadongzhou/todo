<script lang="ts">
import { toInstant, toLocalDateTimeInput } from "@/lib/datetime";
import type { Todo } from "@/types/todo";

/**
 * What a create form or an inline editor is holding while the user fills it in.
 *
 * Values are kept in the shape storage speaks, not the shape an input speaks,
 * so a container can hand the whole object to the store without naming a single
 * field. Turning a stored value into an input value and back is a per-field
 * concern, and lives with the field (`toInput` / `fromInput` below).
 */
export interface TodoDraft {
  title: string;
  dueDate: string | null;
  reminderAt: string | null;
}

/** A blank draft. Containers reset by replacing the object, never field by field. */
export function createEmptyDraft(): TodoDraft {
  return { title: "", dueDate: null, reminderAt: null };
}

/** The draft an editor opens on, taken from the stored todo. */
export function draftFromTodo(todo: Todo): TodoDraft {
  return {
    title: todo.title,
    dueDate: todo.dueDate ?? null,
    reminderAt: todo.reminderAt ?? null,
  };
}

/**
 * Whether two drafts carry the same values. Written over the draft's own keys so
 * "did the user change anything" keeps working when a field is added, instead of
 * quietly comparing everything but the new one.
 */
export function isSameDraft(a: TodoDraft, b: TodoDraft): boolean {
  return (Object.keys(a) as Array<keyof TodoDraft>).every((key) => a[key] === b[key]);
}

/**
 * The four questions a field can answer. A field that answers none of them is a
 * change of interface shape, not a new field: stop and raise it rather than
 * opening a fifth group.
 */
const GROUPS = [
  { key: "content", label: "内容" },
  { key: "time", label: "时间" },
  { key: "category", label: "归类" },
  { key: "relation", label: "关联" },
] as const;

type GroupKey = (typeof GROUPS)[number]["key"];
/** Every draft key except the title, which is the one field that is required. */
type FieldKey = Exclude<keyof TodoDraft, "title">;
type FieldValue = TodoDraft[FieldKey];

interface FieldDefinition {
  readonly key: FieldKey;
  readonly label: string;
  readonly group: GroupKey;
  /**
   * "full" for a control that grows with its content or is built of several
   * sub-controls; "half" for a single scalar control. Declared by the field so
   * the group can change how many columns it lays out without every field
   * having to be revisited.
   */
  readonly span: "full" | "half";
  readonly inputType: "date" | "datetime-local";
  readonly toInput: (value: FieldValue) => string;
  readonly fromInput: (raw: string) => FieldValue;
  /** A hint that belongs to this field; it takes a full row under the control. */
  readonly note?: (draft: TodoDraft) => string | null;
}

/**
 * Every optional field the create界面 and the inline editor offer, in the order
 * the contract declares them. Adding one here is the whole change: the groups,
 * the layout, the reset, the submit and the validation all follow from it.
 */
const FIELDS: readonly FieldDefinition[] = [
  {
    key: "dueDate",
    label: "截止日",
    group: "time",
    span: "half",
    inputType: "date",
    toInput: (value) => value ?? "",
    fromInput: (raw) => raw || null,
  },
  {
    key: "reminderAt",
    label: "提醒时间",
    group: "time",
    span: "half",
    inputType: "datetime-local",
    toInput: (value) => toLocalDateTimeInput(value),
    fromInput: (raw) => toInstant(raw),
    // A reminder in the past is a legitimate intent — it fires once, at once —
    // so this only says so; it never blocks the submit.
    note: (draft) => {
      const instant = draft.reminderAt;
      if (!instant) return null;
      const at = new Date(instant).getTime();
      return !Number.isNaN(at) && at < Date.now() ? "提醒时间已过，保存后会立即提醒一次。" : null;
    },
  },
];

/** `dueDate` -> `<prefix>-due-date`, so an id reads like the label it belongs to. */
function fieldId(prefix: string, key: string): string {
  return `${prefix}-${key.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)}`;
}
</script>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { announceFormAlert, clearFormAlert } from "@/lib/form-alert";
import { MAX_TITLE_CHARS } from "@/lib/native";

/**
 * The fields of a todo, and nothing else — no shell, no buttons, no submit.
 *
 * Both places that edit a todo render this: the create form at the top of the
 * page, and the row that opens inline. They keep their own shells (one is a
 * standing form, the other an expanding row) and differ only in what they put
 * in the `actions` slot, which is why this component knows nothing about either.
 */
const props = defineProps<{
  /** Id namespace, so two of these on one page never collide. */
  idPrefix: string;
  /** Whether the optional fields are showing. Always true for the editor. */
  detailsOpen: boolean;
  /** Id of the details region, for the container's expander to point at. */
  detailsId: string;
}>();

/** The draft being edited. A new field is a new key, nothing more. */
const draft = defineModel<TodoDraft>({ required: true });

/**
 * The draft including a change the parent has not handed back yet.
 *
 * `v-model` is a round trip: a write leaves as an event and only returns as a
 * new value on the parent's next render. A draft is replaced whole on every
 * write, so two fields changed inside one tick would each be built on the value
 * from before the other, and one of the two changes would be lost. Writes are
 * therefore built from here; everything that renders reads `draft`, which is
 * what the parent actually holds.
 */
let pendingDraft: TodoDraft = draft.value;
watch(draft, (value) => (pendingDraft = value), { flush: "sync" });

function commitDraft(next: TodoDraft): void {
  pendingDraft = next;
  draft.value = next;
}

/**
 * The counter appears only once the end is in sight. A 500-code-point limit is
 * one no ordinary title ever reaches, so a permanent counter is pure noise;
 * twenty code points is "one more sentence", which is enough warning.
 */
const COUNTER_THRESHOLD = 20;
const TITLE_REQUIRED_MESSAGE = "标题不能为空，请输入内容。";

const titleInputRef = ref<HTMLInputElement | null>(null);
const titleError = ref(false);
/**
 * Whether the title has ever held content in this draft. A title that was never
 * filled in is a blank form, not a mistake, so blurring an untouched create form
 * must not turn it red; a title the user has just emptied is a mistake, and says
 * so at once.
 */
const titleEverFilled = ref(false);
/**
 * Whether the "at the limit" line has already been said for the current run at
 * the limit. Without it, holding a key down turns the alert into a stutter.
 * Falling back under the limit re-arms it.
 */
let limitAnnounced = false;

// Counted in code points, matching the contract's `chars().count()`. UTF-16
// units — what `maxlength` counts — would take away half of an emoji title.
const titleLength = computed(() => Array.from(draft.value.title).length);
const remaining = computed(() => MAX_TITLE_CHARS - titleLength.value);
const showCounter = computed(() => remaining.value <= COUNTER_THRESHOLD);
const atLimit = computed(() => remaining.value <= 0);
const counterText = computed(() =>
  atLimit.value ? `已达上限 ${MAX_TITLE_CHARS} 字` : `还可输入 ${remaining.value} 字`,
);

const titleId = computed(() => `${props.idPrefix}-title`);
const counterId = computed(() => `${props.idPrefix}-title-count`);
const errorId = computed(() => `${props.idPrefix}-title-error`);
/**
 * The counter and the error are described-by, never part of the label: inside
 * the `<label>` they would join the input's accessible name and change it on
 * every keystroke.
 */
const titleDescribedBy = computed(() => {
  const ids = [
    showCounter.value ? counterId.value : null,
    titleError.value ? errorId.value : null,
  ].filter((id): id is string => id !== null);
  return ids.length > 0 ? ids.join(" ") : undefined;
});

/**
 * Only groups that actually have a field, each carrying whatever notes its
 * fields want to show right now. A group with nothing in it renders nothing, so
 * today's details region is one unnamed group — exactly the shape it had before
 * — and gains its heading by itself on the day a second group gets a field.
 */
const visibleGroups = computed(() =>
  GROUPS.map((group) => ({
    key: group.key,
    label: group.label,
    entries: FIELDS.filter((field) => field.group === group.key).map((field) => ({
      field,
      id: fieldId(props.idPrefix, field.key),
      value: field.toInput(draft.value[field.key]),
      note: field.note?.(draft.value) ?? null,
    })),
  })).filter((group) => group.entries.length > 0),
);

watch(
  () => draft.value.title,
  (value) => {
    if (value.trim()) titleEverFilled.value = true;
  },
  { immediate: true },
);

function updateField(field: FieldDefinition, raw: string): void {
  const next: TodoDraft = { ...pendingDraft };
  next[field.key] = field.fromInput(raw);
  commitDraft(next);
}

/**
 * Says that the title stops here. Only the first hit of a run is announced (see
 * `limitAnnounced`); a paste that had to be cut is announced every time, because
 * it is a different fact — some of what the user handed over was not taken.
 */
function announceLimit(length: number, truncated: boolean, event: Event): void {
  if (length < MAX_TITLE_CHARS) {
    limitAnnounced = false;
    return;
  }
  const pasted = event instanceof InputEvent && event.inputType === "insertFromPaste";
  if (truncated && pasted) {
    announceFormAlert(
      "warn",
      `标题已达上限，最多 ${MAX_TITLE_CHARS} 字；粘贴内容超出的部分未加入。`,
    );
  } else if (!limitAnnounced) {
    announceFormAlert("warn", `标题已达上限，最多 ${MAX_TITLE_CHARS} 字。`);
  }
  limitAnnounced = true;
}

/**
 * Caps the title at the contract's limit, counted the way the contract counts.
 * Typing and pasting arrive here alike, which is what makes an over-long paste
 * keep its first 500 code points instead of being thrown away whole.
 */
function onTitleInput(event: Event): void {
  const input = event.target as HTMLInputElement;
  const raw = input.value;
  const points = Array.from(raw);
  const truncated = points.length > MAX_TITLE_CHARS;
  const capped = truncated ? points.slice(0, MAX_TITLE_CHARS).join("") : raw;
  const length = truncated ? MAX_TITLE_CHARS : points.length;
  // Vue does not repaint an input whose bound value came back unchanged, so the
  // overflow has to be written back by hand or the box goes on showing it.
  if (truncated) input.value = capped;

  commitDraft({ ...pendingDraft, title: capped });

  // Typing is never validated per keystroke, but content leaves the error with
  // nothing left to report, and it goes at once.
  if (capped.trim()) titleError.value = false;
  // Anything the form was saying was about the previous keystroke. It goes as
  // soon as the title is back under the limit — including when the title is
  // emptied, which would otherwise leave "已达上限" standing over an empty box.
  // At the limit it stays, or holding a key down would blink it in and out.
  if (length < MAX_TITLE_CHARS) clearFormAlert();

  announceLimit(length, truncated, event);
}

/**
 * Validates on blur, but only once the field has held something: the create form
 * opens empty and blurring it is not a mistake. Nothing is announced here — the
 * user is moving focus, and a screen reader is already busy reading wherever
 * they landed.
 */
function onTitleBlur(): void {
  if (!titleEverFilled.value) return;
  titleError.value = !pendingDraft.title.trim();
}

/**
 * Checks the draft on submit. The container only reads the boolean: which fields
 * exist and what makes them valid is this component's business, which is what
 * keeps a new field from having to be written into two submit paths.
 */
function validate(): boolean {
  const valid = pendingDraft.title.trim().length > 0;
  titleError.value = !valid;
  if (!valid) {
    // The rejection has to reach a screen reader on the Enter path too, where
    // focus never moves and a newly described-by error goes unread.
    announceFormAlert("error", TITLE_REQUIRED_MESSAGE);
    return false;
  }
  clearFormAlert();
  // An accepted submit ends this draft: either the container replaces it with a
  // blank one, or the editor closes. Either way the next empty title is a fresh
  // form again, not one the user emptied.
  titleEverFilled.value = false;
  return true;
}

/** The only focus entry a container gets; every other field is this one's business. */
async function focusTitle(): Promise<void> {
  await nextTick();
  const input = titleInputRef.value;
  if (!input) return;
  input.focus();
  // Caret at the end rather than a full selection: a selected title would be
  // wiped by the first keystroke of someone who only meant to append.
  const end = input.value.length;
  input.setSelectionRange(end, end);
}

defineExpose({ validate, focusTitle });
</script>

<template>
  <div class="grid gap-4">
    <!-- The one required field, and the only one outside the details region. -->
    <div class="grid gap-2">
      <div class="flex flex-wrap items-end gap-2">
        <div class="min-w-0 flex-1 basis-full sm:basis-0">
          <div class="mb-1 flex items-baseline justify-between gap-2">
            <label :for="titleId" class="text-xs font-medium text-slate-500 dark:text-slate-400"
              >标题</label
            >
            <span
              v-if="showCounter"
              :id="counterId"
              class="text-xs tabular-nums"
              :class="
                atLimit
                  ? 'text-amber-700 dark:text-amber-300'
                  : 'text-slate-500 dark:text-slate-400'
              "
              >{{ counterText }}</span
            >
          </div>
          <!--
            No `maxlength`: it counts UTF-16 units while the contract counts code
            points, so an emoji title would be cut in half. The cap is applied in
            `onTitleInput` instead, where typing and pasting share one rule.
          -->
          <input
            :id="titleId"
            ref="titleInputRef"
            :value="draft.title"
            placeholder="输入待办标题…"
            class="min-h-11 w-full rounded-lg border-0 bg-slate-100 px-3 py-2 text-base placeholder:text-slate-400 focus:ring-2 focus:ring-sky-500 sm:text-sm dark:bg-slate-900"
            :aria-invalid="titleError ? 'true' : undefined"
            :aria-describedby="titleDescribedBy"
            @input="onTitleInput"
            @blur="onTitleBlur"
          />
        </div>
        <!-- The one place the two shells differ: create puts its buttons here. -->
        <slot name="actions" />
      </div>
      <p v-if="titleError" :id="errorId" class="m-0 text-xs text-red-600 dark:text-red-400">
        {{ TITLE_REQUIRED_MESSAGE }}
      </p>
    </div>

    <!--
      `v-show`, not `v-if`: the expander's `aria-controls` has to point at an
      element that is always there, and rebuilding the native date pickers on
      every open would be a waste besides. No height and no overflow of its own —
      more fields make the page taller, they do not make this scroll.
    -->
    <div
      :id="detailsId"
      v-show="detailsOpen"
      class="grid gap-4 border-t border-slate-100 pt-4 dark:border-slate-800"
    >
      <fieldset v-for="group in visibleGroups" :key="group.key" class="m-0 border-0 p-0">
        <legend
          v-if="visibleGroups.length > 1"
          class="mb-2 text-sm font-medium text-slate-800 dark:text-slate-200"
        >
          {{ group.label }}
        </legend>
        <div class="grid gap-3 sm:grid-cols-2">
          <template v-for="entry in group.entries" :key="entry.field.key">
            <div :class="entry.field.span === 'full' ? 'col-span-full' : ''">
              <label
                :for="entry.id"
                class="mb-1 block text-xs font-medium text-slate-500 dark:text-slate-400"
                >{{ entry.field.label }}</label
              >
              <input
                :id="entry.id"
                :type="entry.field.inputType"
                :value="entry.value"
                class="min-h-11 w-full rounded-lg border-0 bg-slate-100 px-3 py-2 text-base focus:ring-2 focus:ring-sky-500 sm:text-sm dark:bg-slate-900"
                @input="updateField(entry.field, ($event.target as HTMLInputElement).value)"
              />
            </div>
            <p
              v-if="entry.note"
              class="col-span-full m-0 text-xs text-amber-700 dark:text-amber-300"
            >
              {{ entry.note }}
            </p>
          </template>
        </div>
      </fieldset>
    </div>
  </div>
</template>
