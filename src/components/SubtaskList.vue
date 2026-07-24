<script setup lang="ts">
import { computed, nextTick, reactive, ref } from "vue";
import { ChevronDown, ChevronUp, GripVertical, Plus, Trash2 } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { announceFormAlert, retractFormAlert } from "@/lib/form-alert";
import { MAX_TITLE_CHARS } from "@/lib/native";
import type { PageAlert } from "@/lib/page-alert";
import { useTodoStore } from "@/stores/todos";
import type { Subtask, Todo } from "@/types/todo";

/**
 * The checklist inside one todo, shown only in that todo's editor.
 *
 * It owns the whole subtask region — its rows, its columns, its add box — so the
 * row that renders it (`TodoItem`) learns nothing about the fields inside, the
 * same posture it keeps toward `TodoFields`. Every discrete edit is committed
 * straight to the store (memory, storage, sync, one undo entry each) rather than
 * into the parent draft: subtasks are immediate, and do not ride the editor's
 * Enter-to-save / Esc-to-cancel.
 */
const props = defineProps<{ todo: Todo }>();
const todoStore = useTodoStore();

/**
 * The contract's per-list ceiling (`MAX_LIST_ENTRIES`). It is private to the
 * Rust crate, so it is mirrored here rather than imported; the number is the one
 * `validate_subtasks` enforces, so the add path can refuse the 201st entry
 * before it builds a list the database would reject and then lose on restart.
 */
const MAX_SUBTASKS = 200;
/** The counter appears only once this many code points are left, as the title field does. */
const TITLE_COUNTER_FROM = 20;

const subtasks = computed(() => props.todo.subtasks);
const doneCount = computed(() => subtasks.value.filter((subtask) => subtask.done).length);

/** The add box's text, kept here so it can be cleared and kept focused after each add. */
const newTitle = ref("");
/**
 * Whether the add box is showing while the list is empty. An empty checklist
 * shows only a "＋ 添加子任务" trigger, so the editor of a plain task stays a
 * plain task; the trigger reveals the box. With ≥1 step the box is always there.
 */
const addRevealed = ref(false);
const showAddInput = computed(() => subtasks.value.length > 0 || addRevealed.value);

const addInputRef = ref<HTMLInputElement | null>(null);
const listRef = ref<HTMLUListElement | null>(null);

/**
 * Each row's title while it is being edited, keyed by subtask id. A rename is
 * committed on blur or Enter, not per keystroke, so one edit is one undo entry;
 * until then the text lives here and the stored title is untouched. A key absent
 * here means the row shows its stored title.
 */
const rowDraft = reactive<Record<string, string>>({});

/**
 * The "at the limit" line each input last put up, keyed the same way ("add" for
 * the add box, the subtask id for a row), so holding a key at the cap does not
 * stutter the one live region and dropping back under it takes only its own line
 * back. Mirrors `TodoFields`' cap bookkeeping onto the page's single alert channel.
 */
const capAnnounced = new Map<string, PageAlert | null>();
/** The index a grip drag started on, or `null` when nothing is being dragged. */
const dragIndex = ref<number | null>(null);

/** Code points, matching the contract's `chars().count()`. */
function countCodePoints(value: string): number {
  return Array.from(value).length;
}

/** Caps a string at the title limit, counted in code points, and says whether it cut. */
function capCodePoints(raw: string): { capped: string; truncated: boolean } {
  const points = Array.from(raw);
  if (points.length <= MAX_TITLE_CHARS) return { capped: raw, truncated: false };
  return { capped: points.slice(0, MAX_TITLE_CHARS).join(""), truncated: true };
}

/** What the counter says, or nothing at all while the end is still far off. */
function counterFor(value: string): { text: string; atLimit: boolean } | null {
  const remaining = MAX_TITLE_CHARS - countCodePoints(value);
  if (remaining > TITLE_COUNTER_FROM) return null;
  return remaining <= 0
    ? { text: `已达上限 ${MAX_TITLE_CHARS} 字`, atLimit: true }
    : { text: `还可输入 ${remaining} 字`, atLimit: false };
}

/**
 * Caps one text input at the title limit and reports it through the page's single
 * live region — the same rule the title field uses, so an over-long paste keeps
 * its first N code points instead of vanishing whole, and the report never grows
 * a second live region.
 */
function applyTitleCap(event: Event, key: string): string {
  const control = event.target as HTMLInputElement;
  const { capped, truncated } = capCodePoints(control.value);
  // Vue does not repaint a control whose bound value came back unchanged, so an
  // overflow has to be written back by hand or the box goes on showing it.
  if (truncated) control.value = capped;

  const previous = capAnnounced.get(key) ?? null;
  if (countCodePoints(capped) < MAX_TITLE_CHARS) {
    retractFormAlert(previous);
    capAnnounced.set(key, null);
    return capped;
  }

  const pasted = event instanceof InputEvent && event.inputType === "insertFromPaste";
  if (truncated && pasted) {
    capAnnounced.set(
      key,
      announceFormAlert(
        "warn",
        `子任务标题已达上限，最多 ${MAX_TITLE_CHARS} 字；粘贴内容超出的部分未加入。`,
        "standing",
      ),
    );
  } else if (!previous) {
    capAnnounced.set(
      key,
      announceFormAlert("warn", `子任务标题已达上限，最多 ${MAX_TITLE_CHARS} 字。`, "standing"),
    );
  }
  return capped;
}

/** Drops one input's cap line once it stops being edited. */
function clearCap(key: string): void {
  retractFormAlert(capAnnounced.get(key) ?? null);
  capAnnounced.set(key, null);
}

/** A row's text: the in-progress edit if there is one, else the stored title. */
function rowValue(subtask: Subtask): string {
  return subtask.id in rowDraft ? rowDraft[subtask.id] : subtask.title;
}

/** The counter for a row's current text, shown only near the limit. */
function rowCounter(subtask: Subtask): { text: string; atLimit: boolean } | null {
  return counterFor(rowValue(subtask));
}

const addCounter = computed(() => counterFor(newTitle.value));

/** A fresh copy of the checklist as plain rows, ready to be reshaped and stored. */
function copyOfList(): Subtask[] {
  return subtasks.value.map((subtask) => ({ ...subtask }));
}

function onAddInput(event: Event): void {
  newTitle.value = applyTitleCap(event, "add");
}

/** Reveals the add box on an empty list and puts the cursor in it. */
function revealAdd(): void {
  addRevealed.value = true;
  void nextTick(() => addInputRef.value?.focus());
}

/**
 * Appends a step. A blank title is a no-op, and so is a title that would be the
 * 201st: the checklist stops at the contract's ceiling, said once through the
 * live region. The box is cleared and kept focused so steps can be typed in a run.
 */
function addSubtask(): void {
  const title = newTitle.value.trim();
  if (!title) return;
  if (subtasks.value.length >= MAX_SUBTASKS) {
    announceFormAlert("warn", `子任务数量已达上限 ${MAX_SUBTASKS} 项，无法再添加。`, "transient");
    return;
  }
  const next = [...copyOfList(), { id: crypto.randomUUID(), title, done: false }];
  todoStore.editSubtasks(props.todo.id, next);
  newTitle.value = "";
  clearCap("add");
  void nextTick(() => addInputRef.value?.focus());
}

function onRowInput(subtask: Subtask, event: Event): void {
  rowDraft[subtask.id] = applyTitleCap(event, subtask.id);
}

/**
 * Commits a row's rename on blur or Enter. An empty title reverts to the stored
 * one (the contract forbids a blank step); an unchanged one writes nothing. The
 * key is dropped either way, so the row falls back to the stored title and a
 * later stray blur cannot commit a second time.
 */
function commitRename(subtask: Subtask): void {
  if (!(subtask.id in rowDraft)) return;
  const title = rowDraft[subtask.id].trim();
  delete rowDraft[subtask.id];
  clearCap(subtask.id);
  if (!title || title === subtask.title) return;
  const next = copyOfList().map((row) => (row.id === subtask.id ? { ...row, title } : row));
  todoStore.editSubtasks(props.todo.id, next);
}

function toggleSubtask(subtask: Subtask): void {
  const next = copyOfList().map((row) =>
    row.id === subtask.id ? { ...row, done: !row.done } : row,
  );
  todoStore.editSubtasks(props.todo.id, next);
}

/**
 * Drops a step, then hands focus to the row that slid into its place — or to the
 * add box if it was the last one — so a keyboard user is never dropped onto the
 * page body. Deleting the last step reveals the add box for that landing.
 */
function deleteSubtask(index: number): void {
  const next = copyOfList().filter((_, position) => position !== index);
  if (next.length === 0) addRevealed.value = true;
  todoStore.editSubtasks(props.todo.id, next);
  void nextTick(() => focusAfterDelete(index));
}

function focusAfterDelete(removedIndex: number): void {
  const remaining = subtasks.value.length;
  if (remaining === 0) {
    addInputRef.value?.focus();
    return;
  }
  const targetIndex = Math.min(removedIndex, remaining - 1);
  const buttons = listRef.value?.querySelectorAll<HTMLElement>('[data-role="delete"]');
  buttons?.[targetIndex]?.focus();
}

/** Moves the up/down button of a row into view for focus after a reorder. */
function moveButton(id: string, direction: "up" | "down"): HTMLButtonElement | null {
  return (
    listRef.value?.querySelector<HTMLButtonElement>(
      `[data-move="${direction}"][data-id="${CSS.escape(id)}"]`,
    ) ?? null
  );
}

/**
 * Reorders by one step through the up/down buttons — the channel keyboard, screen
 * reader and touch all share, since native drag does not work on touch. Focus
 * stays on the same button as it rides its row to the new place; when that button
 * lands at an edge and disables, focus falls to its partner so it never drops to
 * the body. The new position is spoken through the one live region.
 */
function move(index: number, direction: -1 | 1): void {
  const target = index + direction;
  if (target < 0 || target >= subtasks.value.length) return;
  const next = copyOfList();
  const [moved] = next.splice(index, 1);
  next.splice(target, 0, moved);
  todoStore.editSubtasks(props.todo.id, next);
  void nextTick(() => afterMove(moved.id, target, direction));
}

function afterMove(id: string, newIndex: number, direction: -1 | 1): void {
  const total = subtasks.value.length;
  const title = subtasks.value[newIndex]?.title ?? "";
  announceFormAlert(
    "success",
    `“${title}”已移到第 ${newIndex + 1} 项，共 ${total} 项。`,
    "transient",
  );
  const same = direction === -1 ? "up" : "down";
  const partner = direction === -1 ? "down" : "up";
  const button = moveButton(id, same);
  if (button && !button.disabled) button.focus();
  else moveButton(id, partner)?.focus();
}

function onDragStart(index: number): void {
  dragIndex.value = index;
}

function onDragEnd(): void {
  dragIndex.value = null;
}

/**
 * Drops a dragged step onto another row's place — the pointer-only enhancement
 * on top of the up/down buttons, offered on the grip that only shows from `sm:`
 * up (touch cannot fire it reliably).
 */
function onDrop(targetIndex: number): void {
  const from = dragIndex.value;
  dragIndex.value = null;
  if (from === null || from === targetIndex) return;
  const next = copyOfList();
  const [moved] = next.splice(from, 1);
  next.splice(targetIndex, 0, moved);
  todoStore.editSubtasks(props.todo.id, next);
}
</script>

<template>
  <section
    class="grid gap-3 border-t border-slate-100 pt-4 dark:border-slate-800"
    aria-label="子任务"
  >
    <p v-if="subtasks.length" class="m-0 text-xs font-medium text-slate-500 dark:text-slate-400">
      子任务 <span class="tabular-nums">{{ doneCount }}/{{ subtasks.length }}</span>
    </p>

    <ul v-if="subtasks.length" ref="listRef" class="m-0 grid list-none gap-2 p-0">
      <li
        v-for="(subtask, index) in subtasks"
        :key="subtask.id"
        class="flex items-center gap-2"
        :class="dragIndex === index ? 'opacity-50' : ''"
        @dragover.prevent
        @drop.prevent="onDrop(index)"
      >
        <!-- Square native checkbox, padded to a 44px hit area: it reads and keys
             like a checkbox for free, and its shape sets it apart from the round
             completion circle of the task itself (a step is not the task). -->
        <label class="flex min-h-11 min-w-11 shrink-0 cursor-pointer items-center justify-center">
          <input
            type="checkbox"
            class="h-4 w-4 accent-sky-500"
            :checked="subtask.done"
            :aria-label="subtask.title"
            @change="toggleSubtask(subtask)"
          />
        </label>

        <div class="min-w-0 flex-1">
          <input
            :value="rowValue(subtask)"
            aria-label="子任务标题"
            class="min-h-11 w-full rounded-lg border-0 bg-transparent px-2 py-2 text-base focus:ring-2 focus:ring-sky-500 sm:min-h-0 sm:text-sm"
            :class="
              subtask.done
                ? 'text-slate-400 line-through dark:text-slate-500'
                : 'text-slate-700 dark:text-slate-200'
            "
            @input="onRowInput(subtask, $event)"
            @keydown.enter.prevent="commitRename(subtask)"
            @blur="commitRename(subtask)"
          />
          <span
            v-if="rowCounter(subtask)"
            class="mt-0.5 block text-right text-xs tabular-nums"
            :class="
              rowCounter(subtask)!.atLimit
                ? 'text-amber-700 dark:text-amber-300'
                : 'text-slate-500 dark:text-slate-400'
            "
            >{{ rowCounter(subtask)!.text }}</span
          >
        </div>

        <!-- Up/down stacked into one 44px-wide slot: the reorder channel that
             works for keyboard, screen reader and touch alike. -->
        <div class="flex w-11 shrink-0 flex-col">
          <Button
            variant="ghost"
            class="w-full !px-0 !py-1"
            :disabled="index === 0"
            data-move="up"
            :data-id="subtask.id"
            :aria-label="`上移子任务 ${subtask.title}`"
            @click="move(index, -1)"
          >
            <ChevronUp :size="18" />
          </Button>
          <Button
            variant="ghost"
            class="w-full !px-0 !py-1"
            :disabled="index === subtasks.length - 1"
            data-move="down"
            :data-id="subtask.id"
            :aria-label="`下移子任务 ${subtask.title}`"
            @click="move(index, 1)"
          >
            <ChevronDown :size="18" />
          </Button>
        </div>

        <Button
          variant="ghost"
          class="min-h-11 min-w-11 shrink-0 !p-2 text-slate-400 hover:text-red-600 dark:hover:text-red-400"
          data-role="delete"
          :aria-label="`删除子任务 ${subtask.title}`"
          @click="deleteSubtask(index)"
        >
          <Trash2 :size="20" />
        </Button>

        <!-- Drag handle, desktop only: touch cannot fire native drag, and on a
             narrow row it would crowd the up/down buttons. Not a keyboard entry —
             reordering by key goes through the buttons. -->
        <span
          class="hidden shrink-0 cursor-grab touch-none items-center px-1 text-slate-400 sm:flex sm:min-h-11"
          draggable="true"
          aria-hidden="true"
          @dragstart="onDragStart(index)"
          @dragend="onDragEnd"
        >
          <GripVertical :size="18" />
        </span>
      </li>
    </ul>

    <div v-if="showAddInput" class="grid gap-1">
      <div class="flex items-center gap-2">
        <input
          ref="addInputRef"
          :value="newTitle"
          placeholder="添加子任务…"
          aria-label="新子任务标题"
          class="min-h-11 min-w-0 flex-1 rounded-lg border-0 bg-slate-100 px-3 py-2 text-base placeholder:text-slate-400 focus:ring-2 focus:ring-sky-500 sm:text-sm dark:bg-slate-900"
          @input="onAddInput"
          @keydown.enter.prevent="addSubtask"
        />
        <Button
          variant="ghost"
          class="min-h-11 min-w-11 shrink-0 !p-2"
          aria-label="添加子任务"
          @click="addSubtask"
        >
          <Plus :size="20" />
        </Button>
      </div>
      <span
        v-if="addCounter"
        class="block text-right text-xs tabular-nums"
        :class="
          addCounter.atLimit
            ? 'text-amber-700 dark:text-amber-300'
            : 'text-slate-500 dark:text-slate-400'
        "
        >{{ addCounter.text }}</span
      >
    </div>
    <Button
      v-else
      variant="ghost"
      class="min-h-11 w-fit !justify-start !px-2 text-sm text-slate-500 dark:text-slate-400"
      @click="revealAdd"
    >
      <Plus :size="16" /><span class="ml-1">添加子任务</span>
    </Button>
  </section>
</template>
