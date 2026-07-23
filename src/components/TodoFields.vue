<script lang="ts">
import { toInstant, toLocalDateTimeInput } from "@/lib/datetime";
import { MAX_NOTES_CHARS } from "@/lib/native";
import { normalizeNotes, normalizeTitle } from "@/lib/todo-normalize";
import type { Todo } from "@/types/todo";

/**
 * What a create form or an inline editor is holding while the user fills it in.
 *
 * Values are kept in the shape storage speaks, not the shape an input speaks,
 * so a container can hand the whole object to the store without naming a single
 * field. Turning a stored value into an input value and back is a per-field
 * concern, and lives with the field (`codec` below).
 *
 * Every key is always present and `null` always means "the user wants this
 * empty" — the draft has no "not provided" state. That is what lets a container
 * submit a whole draft and have a cleared field actually clear, here and on
 * every other device.
 */
export interface TodoDraft {
  title: string;
  notes: string | null;
  dueDate: string | null;
  reminderAt: string | null;
  startDate: string | null;
  startsAt: string | null;
  endsAt: string | null;
  estimatedMinutes: number | null;
}

/** A blank draft. Containers reset by replacing the object, never field by field. */
export function createEmptyDraft(): TodoDraft {
  return {
    title: "",
    notes: null,
    dueDate: null,
    reminderAt: null,
    startDate: null,
    startsAt: null,
    endsAt: null,
    estimatedMinutes: null,
  };
}

/** The draft an editor opens on, taken from the stored todo. */
export function draftFromTodo(todo: Todo): TodoDraft {
  return {
    title: todo.title,
    notes: todo.notes ?? null,
    dueDate: todo.dueDate ?? null,
    reminderAt: todo.reminderAt ?? null,
    startDate: todo.startDate ?? null,
    startsAt: todo.startsAt ?? null,
    endsAt: todo.endsAt ?? null,
    estimatedMinutes: todo.estimatedMinutes ?? null,
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
 * The draft as it would be stored, rather than as it was typed.
 *
 * "Did the user change anything" has to be asked of the values the store will
 * actually write, which is why the two text fields go through the very functions
 * it uses. Asked of the raw text instead, a title with a space added to the end
 * reads as an edit: it writes back the identical string, sends a sync operation
 * carrying nothing, and files an undo entry that undoes nothing the user can
 * see — press Ctrl+Z once and the screen does not move.
 */
export function normalizeDraft(draft: TodoDraft): TodoDraft {
  return {
    ...draft,
    title: normalizeTitle(draft.title),
    notes: normalizeNotes(draft.notes),
  };
}

/**
 * Whether anything outside the first row is set.
 *
 * The inline editor opens its details region on this rather than on a list of
 * field names, which is what keeps the container ignorant of the fields: adding
 * one changes the answer without changing the question.
 */
export function draftHasDetails(draft: TodoDraft): boolean {
  return (Object.keys(draft) as Array<keyof TodoDraft>).some(
    (key) => key !== "title" && draft[key] !== null && draft[key] !== "",
  );
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

/**
 * One draft key seen as a control's string value, in both directions.
 *
 * Built by `codec` so the two halves are written against the same key and the
 * same value type; what comes out has forgotten which key it was, which is what
 * lets fields of different value types sit in one table.
 */
interface FieldCodec {
  readonly key: FieldKey;
  readonly read: (draft: TodoDraft) => string;
  readonly write: (draft: TodoDraft, raw: string) => TodoDraft;
}

function codec<K extends FieldKey>(
  key: K,
  toInput: (value: TodoDraft[K]) => string,
  fromInput: (raw: string) => TodoDraft[K],
): FieldCodec {
  return {
    key,
    read: (draft) => toInput(draft[key]),
    write: (draft, raw) => {
      const next = { ...draft };
      next[key] = fromInput(raw);
      return next;
    },
  };
}

/** One half of a `range`: its own draft key, its own label, one control. */
interface RangePart {
  readonly label: string;
  readonly type: "datetime-local";
  readonly codec: FieldCodec;
}

/**
 * The shapes a control can take.
 *
 * The template branches on `kind` and nothing else, so the number of branches
 * grows with the number of *control shapes* — a small closed set — and never
 * with the number of fields. A new field is a row in `FIELDS`; a new shape is
 * one more variant here and one more branch in the template. The next ones are
 * already known:
 *
 *   | { kind: "select"; options: readonly { value: string; label: string }[] }
 *   | { kind: "checkbox"; triState?: boolean }
 */
type FieldControl =
  | {
      readonly kind: "input";
      readonly type: "date" | "datetime-local" | "number";
      readonly inputMode?: "numeric";
      readonly codec: FieldCodec;
    }
  | { readonly kind: "textarea"; readonly rows: number; readonly codec: FieldCodec }
  | { readonly kind: "range"; readonly parts: readonly [RangePart, RangePart] };

/**
 * A length limit and when to start warning about it.
 *
 * Two numbers rather than one because the point at which a counter stops being
 * help and starts being noise depends on the limit: twenty code points is "one
 * more sentence" for a title, and invisible against twenty thousand for a note.
 */
interface CharLimit {
  /** Longest accepted value, in code points — the unit the contract counts in. */
  readonly max: number;
  /** The counter appears once this many code points are left. */
  readonly counterFrom: number;
}

interface FieldDefinition {
  /** Names the field in the table and in ids; a range takes its first key. */
  readonly key: string;
  readonly label: string;
  readonly group: GroupKey;
  /**
   * "full" for a control that grows with its content or is built of several
   * sub-controls; "half" for a single scalar control. Declared by the field so
   * the group can change how many columns it lays out without every field
   * having to be revisited.
   */
  readonly span: "full" | "half";
  readonly control: FieldControl;
  /** A hint that belongs to this field; it takes a full row under the control. */
  readonly note?: (draft: TodoDraft) => string | null;
  /**
   * What makes this field unstorable, said in the terms of the fix. Reads the
   * whole draft, so a rule spanning two of its keys — an end before its start —
   * is written with the field it belongs to rather than in the container.
   */
  readonly error?: (draft: TodoDraft) => string | null;
  readonly maxChars?: CharLimit;
}

/** Empty text is an empty field; anything else is kept as the user typed it. */
function textOrNull(raw: string): string | null {
  return raw.trim() ? raw : null;
}

/**
 * Minutes as the input gives them. Blank and unreadable both read as "not set";
 * a number that is not a whole positive count is kept as typed so the field can
 * say what is wrong with it instead of silently discarding the input.
 */
function minutesFromInput(raw: string): number | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;
  const minutes = Number(trimmed);
  return Number.isNaN(minutes) ? null : minutes;
}

function rangeError(draft: TodoDraft): string | null {
  // The contract refuses an end without a start outright, so letting it through
  // would only turn into a failed write later.
  if (draft.endsAt && !draft.startsAt) return "先填开始时刻，或清空结束时刻。";
  if (!draft.startsAt || !draft.endsAt) return null;
  const start = new Date(draft.startsAt).getTime();
  const end = new Date(draft.endsAt).getTime();
  if (Number.isNaN(start) || Number.isNaN(end)) return null;
  // Blocking, not a hint: the two halves are one field, and a block that ends
  // before it starts has no reading at all — while the time-block view and the
  // .ics export both consume these two values directly.
  return end > start ? null : "结束时刻需要晚于开始时刻。";
}

function estimateError(draft: TodoDraft): string | null {
  const minutes = draft.estimatedMinutes;
  if (minutes === null) return null;
  return Number.isInteger(minutes) && minutes > 0 ? null : "估时请填写大于 0 的整数分钟，或留空。";
}

/**
 * Every optional field the create界面 and the inline editor offer, in the order
 * the contract declares them. Adding one here is the whole change: the groups,
 * the layout, the reset, the submit and the validation all follow from it.
 */
const FIELDS: readonly FieldDefinition[] = [
  {
    key: "notes",
    label: "备注",
    group: "content",
    span: "full",
    control: {
      kind: "textarea",
      rows: 3,
      codec: codec("notes", (value) => value ?? "", textOrNull),
    },
    maxChars: { max: MAX_NOTES_CHARS, counterFrom: 200 },
  },
  {
    key: "dueDate",
    label: "截止日",
    group: "time",
    span: "half",
    control: {
      kind: "input",
      type: "date",
      codec: codec(
        "dueDate",
        (value) => value ?? "",
        (raw) => raw || null,
      ),
    },
  },
  {
    key: "reminderAt",
    label: "提醒时间",
    group: "time",
    span: "half",
    control: {
      kind: "input",
      type: "datetime-local",
      codec: codec("reminderAt", toLocalDateTimeInput, toInstant),
    },
    // A reminder in the past is a legitimate intent — it fires once, at once —
    // so this only says so; it never blocks the submit.
    note: (draft) => {
      const instant = draft.reminderAt;
      if (!instant) return null;
      const at = new Date(instant).getTime();
      return !Number.isNaN(at) && at < Date.now() ? "提醒时间已过，保存后会立即提醒一次。" : null;
    },
  },
  {
    key: "startDate",
    label: "开始日期",
    group: "time",
    span: "half",
    control: {
      kind: "input",
      type: "date",
      codec: codec(
        "startDate",
        (value) => value ?? "",
        (raw) => raw || null,
      ),
    },
  },
  {
    // One field, two draft keys. Split into two half-width fields they would be
    // torn apart by the two-column grid — the start ending one row above the end
    // — which is precisely what "several sub-controls means a full row" is for.
    key: "startsAt",
    label: "时间段",
    group: "time",
    span: "full",
    control: {
      kind: "range",
      parts: [
        {
          label: "开始时刻",
          type: "datetime-local",
          codec: codec("startsAt", toLocalDateTimeInput, toInstant),
        },
        {
          label: "结束时刻",
          type: "datetime-local",
          codec: codec("endsAt", toLocalDateTimeInput, toInstant),
        },
      ],
    },
    error: rangeError,
  },
  {
    // The unit is in the label rather than in a second control: one control
    // fewer, and a screen reader reads the unit as part of the field's name.
    key: "estimatedMinutes",
    label: "估时（分钟）",
    group: "time",
    span: "half",
    control: {
      kind: "input",
      type: "number",
      inputMode: "numeric",
      codec: codec(
        "estimatedMinutes",
        (value) => (value === null ? "" : String(value)),
        minutesFromInput,
      ),
    },
    error: estimateError,
  },
];

/** The draft keys one field owns: both halves for a range, one otherwise. */
function keysOf(field: FieldDefinition): FieldKey[] {
  return field.control.kind === "range"
    ? field.control.parts.map((part) => part.codec.key)
    : [field.control.codec.key];
}

/** `dueDate` -> `<prefix>-due-date`, so an id reads like the label it belongs to. */
function fieldId(prefix: string, key: string): string {
  return `${prefix}-${key.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)}`;
}
</script>

<script setup lang="ts">
import { computed, nextTick, ref, toRaw, watch } from "vue";
import { announceFormAlert, clearFormAlert, retractFormAlert } from "@/lib/form-alert";
import { MAX_TITLE_CHARS } from "@/lib/native";
import type { PageAlert } from "@/lib/page-alert";

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
  /** Whether the optional fields are showing. */
  detailsOpen: boolean;
  /** Id of the details region, for the container's expander to point at. */
  detailsId: string;
}>();

/**
 * Asks the container to open the details region.
 *
 * An error inside a collapsed region can be neither seen nor focused, and the
 * open state belongs to the container — so the field block can only ask. This
 * is asked once, by the validation, for every field there will ever be.
 */
const emit = defineEmits<{ "expand-details": [] }>();

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
let pendingDraft: TodoDraft = toRaw(draft.value);

const TITLE_LIMIT: CharLimit = { max: MAX_TITLE_CHARS, counterFrom: 20 };
const TITLE_REQUIRED_MESSAGE = "标题不能为空，请输入内容。";
const CONTROL_CLASS =
  "min-h-11 w-full rounded-lg border-0 bg-slate-100 px-3 py-2 text-base focus:ring-2 focus:ring-sky-500 sm:text-sm dark:bg-slate-900";
const LABEL_CLASS = "text-xs font-medium text-slate-500 dark:text-slate-400";
const COUNTER_CLASS = "text-slate-500 dark:text-slate-400";
const COUNTER_LIMIT_CLASS = "text-amber-700 dark:text-amber-300";

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
 * The "at the limit" line this field last said, or `null` if it has not said one
 * for the current run at the limit. Without it, holding a key down turns the
 * alert into a stutter. Falling back under the limit re-arms it.
 *
 * The line itself is kept, not just the fact that there was one, because taking
 * it back later is only allowed while it is still the line being said — see
 * `retractFormAlert`.
 */
interface CapState {
  announced: PageAlert | null;
}
const capStates = new Map<string, CapState>();
function capState(key: string): CapState {
  let state = capStates.get(key);
  if (!state) {
    state = { announced: null };
    capStates.set(key, state);
  }
  return state;
}

/**
 * What a rejected submit was about: the keys it named, the draft it named them
 * in, and the line it put on the page. That line is an answer to that submit, so
 * it goes as soon as one of those keys is touched — and stays while none of them
 * is.
 *
 * The line itself is kept for the same reason a cap keeps its own (see
 * `retractFormAlert`): by the time the user fixes the field, the channel may be
 * saying something else, and that sentence is not this one's to clear.
 */
let rejection: { keys: Array<keyof TodoDraft>; draft: TodoDraft; alert: PageAlert } | null = null;

/** Every control in the details region, by id, so validation can focus one. */
const controls: Record<string, HTMLElement> = {};
function registerControl(id: string, element: unknown): void {
  if (element instanceof HTMLElement) controls[id] = element;
}

function commitDraft(next: TodoDraft): void {
  pendingDraft = next;
  draft.value = next;
}

/**
 * A draft the container replaced is a different form: whatever this one was
 * saying was about the previous one. Our own writes come back through here too,
 * and are told apart by identity — they are already `pendingDraft`.
 */
watch(
  draft,
  (value) => {
    // `toRaw`, because a draft handed back through `v-model` arrives as a
    // reactive proxy of the very object that was written out — comparing the
    // proxy would read every keystroke as a container replacing the draft, and
    // reset the form's own state under the user on each one.
    const replacement = toRaw(value);
    if (replacement === pendingDraft) return;
    pendingDraft = replacement;
    titleError.value = false;
    titleEverFilled.value = false;
    rejection = null;
    for (const state of capStates.values()) state.announced = null;
    clearFormAlert();
  },
  { flush: "sync" },
);

watch(
  () => draft.value.title,
  (value) => {
    if (value.trim()) titleEverFilled.value = true;
  },
  { immediate: true },
);

const titleId = computed(() => `${props.idPrefix}-title`);
const titleCounterId = computed(() => `${props.idPrefix}-title-count`);
const titleErrorId = computed(() => `${props.idPrefix}-title-error`);

const titleCounter = computed(() => counterFor(draft.value.title, TITLE_LIMIT));
/**
 * The counter and the error are described-by, never part of the label: inside
 * the `<label>` they would join the input's accessible name and change it on
 * every keystroke.
 */
const titleDescribedBy = computed(() =>
  describedBy(
    titleCounter.value ? titleCounterId.value : null,
    null,
    titleError.value ? titleErrorId.value : null,
  ),
);

/** Code points, matching the contract's `chars().count()`. */
function countChars(value: string): number {
  return Array.from(value).length;
}

/** What the counter says, or nothing at all while the end is still far off. */
function counterFor(value: string, limit: CharLimit): { text: string; atLimit: boolean } | null {
  const remaining = limit.max - countChars(value);
  if (remaining > limit.counterFrom) return null;
  return remaining <= 0
    ? { text: `已达上限 ${limit.max} 字`, atLimit: true }
    : { text: `还可输入 ${remaining} 字`, atLimit: false };
}

/** The ids describing one control, in a fixed order: counter, hint, error. */
function describedBy(
  counterId: string | null,
  noteId: string | null,
  errorId: string | null,
): string | undefined {
  const ids = [counterId, noteId, errorId].filter((id): id is string => id !== null);
  return ids.length > 0 ? ids.join(" ") : undefined;
}

/**
 * Caps a text control at its limit, counted the way the contract counts it, and
 * says so. Typing and pasting arrive here alike, which is what makes an
 * over-long paste keep its first N code points instead of being thrown away
 * whole. The line it puts up is `standing`: nothing but a shorter value ends it.
 */
function applyCharCap(event: Event, label: string, limit: CharLimit, key: string): string {
  const control = event.target as HTMLInputElement | HTMLTextAreaElement;
  const raw = control.value;
  const points = Array.from(raw);
  const truncated = points.length > limit.max;
  const capped = truncated ? points.slice(0, limit.max).join("") : raw;
  // Vue does not repaint a control whose bound value came back unchanged, so
  // the overflow has to be written back by hand or the box goes on showing it.
  if (truncated) control.value = capped;

  const state = capState(key);
  if (points.length < limit.max) {
    // Only ever takes back what this cap said, and only while it is still the
    // line on the page: a rejected submit, or another field's own cap, may have
    // taken it over since, and neither is ours to clear here.
    retractFormAlert(state.announced);
    state.announced = null;
    return capped;
  }

  const pasted = event instanceof InputEvent && event.inputType === "insertFromPaste";
  if (truncated && pasted) {
    state.announced = announceFormAlert(
      "warn",
      `${label}已达上限，最多 ${limit.max} 字；粘贴内容超出的部分未加入。`,
      "standing",
    );
  } else if (!state.announced) {
    state.announced = announceFormAlert(
      "warn",
      `${label}已达上限，最多 ${limit.max} 字。`,
      "standing",
    );
  }
  return capped;
}

/** Drops a rejected submit's line once one of the fields it named has moved. */
function clearRejectionIfTouched(): void {
  const rejected = rejection;
  if (!rejected) return;
  if (!rejected.keys.some((key) => pendingDraft[key] !== rejected.draft[key])) return;
  rejection = null;
  // Takes back this rejection's own line, and only while it is still the one
  // being said: a field that has hit its limit since may have taken the channel
  // over, and silencing that would leave a filled-up field with nothing said
  // about it and no way to have it said again.
  retractFormAlert(rejected.alert);
}

function onTitleInput(event: Event): void {
  const capped = applyCharCap(event, "标题", TITLE_LIMIT, "title");
  commitDraft({ ...pendingDraft, title: capped });
  // Typing is never validated per keystroke, but content leaves the error with
  // nothing left to report, and it goes at once.
  if (capped.trim()) titleError.value = false;
  clearRejectionIfTouched();
}

function onFieldInput(field: FieldDefinition, fieldCodec: FieldCodec, event: Event): void {
  const control = event.target as HTMLInputElement | HTMLTextAreaElement;
  const raw = field.maxChars
    ? applyCharCap(event, field.label, field.maxChars, field.key)
    : control.value;
  commitDraft(fieldCodec.write(pendingDraft, raw));
  clearRejectionIfTouched();
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

/** One thing standing between the draft and storage, and where to fix it. */
interface Failure {
  readonly keys: Array<keyof TodoDraft>;
  readonly message: string;
  readonly controlId: string;
  readonly inDetails: boolean;
}

function failures(): Failure[] {
  const found: Failure[] = [];
  if (!pendingDraft.title.trim()) {
    found.push({
      keys: ["title"],
      message: TITLE_REQUIRED_MESSAGE,
      controlId: titleId.value,
      inDetails: false,
    });
  }
  for (const field of FIELDS) {
    const message = field.error?.(pendingDraft);
    if (!message) continue;
    const keys = keysOf(field);
    found.push({
      keys,
      message,
      controlId: fieldId(props.idPrefix, keys[0]),
      inDetails: true,
    });
  }
  return found;
}

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
 * Checks the draft on submit, and takes the user to whatever needs doing: opens
 * the details region if the problem is in it, says the first one through the
 * page's single live region, and puts the focus on the control at fault.
 *
 * The container only reads the boolean. Which fields exist and what makes them
 * valid is this component's business, which is what keeps a new field from
 * having to be written into two submit paths.
 */
function validate(): boolean {
  const found = failures();
  titleError.value = found.some((failure) => failure.keys.includes("title"));

  if (found.length === 0) {
    clearFormAlert();
    rejection = null;
    // An accepted submit ends this draft: either the container replaces it with
    // a blank one, or the editor closes. Either way the next empty title is a
    // fresh form again, not one the user emptied.
    titleEverFilled.value = false;
    return true;
  }

  // A field the user cannot see is a field they cannot fix.
  if (found.some((failure) => failure.inDetails)) emit("expand-details");
  // The rejection has to reach a screen reader on the Enter path too, where
  // focus never moves and a newly described-by error goes unread.
  const alert = announceFormAlert("error", found[0].message, "transient");
  rejection = { keys: found.flatMap((failure) => failure.keys), draft: pendingDraft, alert };

  const target = found[0].controlId;
  if (target === titleId.value) {
    void focusTitleInput();
  } else {
    void nextTick().then(() => {
      const control = controls[target];
      if (control?.isConnected) control.focus();
    });
  }
  return false;
}

/**
 * The only focus entry a container gets; every other field is this one's
 * business.
 *
 * It stands aside after a rejected submit: `validate` has just put the focus on
 * the control at fault, and both containers call this straight afterwards. The
 * guard is on the rejection rather than on "is anything invalid" so that opening
 * an editor on a todo synced from elsewhere — one whose values this build would
 * refuse — still lands the focus in the title instead of nowhere.
 */
async function focusTitle(): Promise<void> {
  if (rejection) return;
  await focusTitleInput();
}

/**
 * Only groups that actually have a field, each carrying everything the template
 * needs to draw it. A group with nothing in it renders nothing, so the details
 * region gains its headings by itself on the day a second group gets a field.
 */
const visibleGroups = computed(() =>
  GROUPS.map((group) => ({
    key: group.key,
    label: group.label,
    entries: FIELDS.filter((field) => field.group === group.key).map((field) => {
      const control = field.control;
      const id = fieldId(props.idPrefix, field.key);
      const note = field.note?.(draft.value) ?? null;
      const error = field.error?.(draft.value) ?? null;
      const noteId = note ? `${id}-note` : null;
      const errorId = error ? `${id}-error` : null;
      const single = control.kind === "range" ? null : control.codec;
      const value = single ? single.read(draft.value) : "";
      const counter = field.maxChars ? counterFor(value, field.maxChars) : null;
      const counterId = counter ? `${id}-count` : null;
      return {
        field,
        id,
        // Flattened rather than left on the control: the template then reads
        // plain optional values and branches on `kind` alone, instead of asking
        // the type checker to narrow a union through a `v-if`.
        kind: control.kind,
        inputType: control.kind === "input" ? control.type : undefined,
        inputMode: control.kind === "input" ? control.inputMode : undefined,
        rows: control.kind === "textarea" ? control.rows : undefined,
        value,
        // Absent for a range, which has one handler per half instead.
        onInput: single ? (event: Event) => onFieldInput(field, single, event) : undefined,
        parts:
          control.kind === "range"
            ? control.parts.map((part) => ({
                label: part.label,
                type: part.type,
                id: fieldId(props.idPrefix, part.codec.key),
                value: part.codec.read(draft.value),
                onInput: (event: Event) => onFieldInput(field, part.codec, event),
              }))
            : [],
        note,
        noteId,
        error,
        errorId,
        counter,
        counterId,
        describedBy: describedBy(counterId, noteId, errorId),
      };
    }),
  })).filter((group) => group.entries.length > 0),
);

defineExpose({ validate, focusTitle });
</script>

<template>
  <div class="grid gap-4">
    <!-- The one required field, and the only one outside the details region. -->
    <div class="grid gap-2">
      <div class="flex flex-wrap items-end gap-2">
        <div class="min-w-0 flex-1 basis-full sm:basis-0">
          <div class="mb-1 flex items-baseline justify-between gap-2">
            <label :for="titleId" :class="LABEL_CLASS">标题</label>
            <span
              v-if="titleCounter"
              :id="titleCounterId"
              class="text-xs tabular-nums"
              :class="titleCounter.atLimit ? COUNTER_LIMIT_CLASS : COUNTER_CLASS"
              >{{ titleCounter.text }}</span
            >
          </div>
          <!--
            No `maxlength`: it counts UTF-16 units while the contract counts code
            points, so an emoji title would be cut in half. The cap is applied in
            `applyCharCap` instead, where typing and pasting share one rule.
          -->
          <input
            :id="titleId"
            ref="titleInputRef"
            :value="draft.title"
            placeholder="输入待办标题…"
            :class="`${CONTROL_CLASS} placeholder:text-slate-400`"
            :aria-invalid="titleError ? 'true' : undefined"
            :aria-describedby="titleDescribedBy"
            @input="onTitleInput"
            @blur="onTitleBlur"
          />
        </div>
        <!-- The one place the two shells differ: create puts its buttons here. -->
        <slot name="actions" />
      </div>
      <p v-if="titleError" :id="titleErrorId" class="m-0 text-xs text-red-600 dark:text-red-400">
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
              <!--
                A range is one field made of two controls, so it is one nested
                <fieldset>: a screen reader entering either half hears 时间段
                first, which is how "these two go together" is said in HTML.
              -->
              <fieldset v-if="entry.kind === 'range'" class="m-0 border-0 p-0">
                <legend class="mb-1" :class="LABEL_CLASS">{{ entry.field.label }}</legend>
                <div class="grid gap-3 sm:grid-cols-2">
                  <div v-for="part in entry.parts" :key="part.id">
                    <label :for="part.id" class="mb-1 block" :class="LABEL_CLASS">{{
                      part.label
                    }}</label>
                    <input
                      :id="part.id"
                      :ref="(element) => registerControl(part.id, element)"
                      :type="part.type"
                      :value="part.value"
                      :class="CONTROL_CLASS"
                      :aria-invalid="entry.error ? 'true' : undefined"
                      :aria-describedby="entry.describedBy"
                      @input="part.onInput($event)"
                    />
                  </div>
                </div>
              </fieldset>
              <template v-else>
                <div class="mb-1 flex items-baseline justify-between gap-2">
                  <label :for="entry.id" :class="LABEL_CLASS">{{ entry.field.label }}</label>
                  <span
                    v-if="entry.counter"
                    :id="entry.counterId"
                    class="text-xs tabular-nums"
                    :class="entry.counter.atLimit ? COUNTER_LIMIT_CLASS : COUNTER_CLASS"
                    >{{ entry.counter.text }}</span
                  >
                </div>
                <!--
                  `resize-y`: the user may make a note taller, but never wider —
                  a wider one would push the page into horizontal scrolling.
                -->
                <textarea
                  v-if="entry.kind === 'textarea'"
                  :id="entry.id"
                  :ref="(element) => registerControl(entry.id, element)"
                  :rows="entry.rows"
                  :value="entry.value"
                  :class="`${CONTROL_CLASS} resize-y`"
                  :aria-invalid="entry.error ? 'true' : undefined"
                  :aria-describedby="entry.describedBy"
                  @input="entry.onInput?.($event)"
                ></textarea>
                <!--
                  No `min` / `max` / `required`: native constraints are enforced
                  by the browser before the submit event, in a bubble of its own
                  that bypasses the page's single live region — and they turn the
                  implicit Enter submit into nothing happening at all.
                -->
                <input
                  v-else
                  :id="entry.id"
                  :ref="(element) => registerControl(entry.id, element)"
                  :type="entry.inputType"
                  :inputmode="entry.inputMode"
                  :value="entry.value"
                  :class="CONTROL_CLASS"
                  :aria-invalid="entry.error ? 'true' : undefined"
                  :aria-describedby="entry.describedBy"
                  @input="entry.onInput?.($event)"
                />
              </template>
            </div>
            <p
              v-if="entry.note"
              :id="entry.noteId"
              class="col-span-full m-0 text-xs text-amber-700 dark:text-amber-300"
            >
              {{ entry.note }}
            </p>
            <p
              v-if="entry.error"
              :id="entry.errorId"
              class="col-span-full m-0 text-xs text-red-600 dark:text-red-400"
            >
              {{ entry.error }}
            </p>
          </template>
        </div>
      </fieldset>
    </div>
  </div>
</template>
