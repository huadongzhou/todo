<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import { Check, Circle, Plus, Search, Trash2 } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { announceFormAlert } from "@/lib/form-alert";
import { useTodoStore } from "@/stores/todos";
import type { Todo } from "@/types/todo";

/**
 * The prerequisites of one todo — the tasks that must be done before it becomes
 * actionable — shown only in that todo's editor and the fourth sibling of the
 * field block after the checklist and the attachments, never a field in
 * `TodoFields`: a dependency is a link to *another existing task*, so it belongs
 * to a task already in the graph, not to the unified create form. Like its
 * siblings it owns its whole region and commits every change straight to the
 * store (an immediate, individually-undoable action), so it rides neither the
 * editor's Enter-to-save nor its Esc-to-cancel.
 */
const props = defineProps<{ todo: Todo }>();
const todoStore = useTodoStore();

/**
 * The contract's per-list ceiling (`MAX_LIST_ENTRIES`), private to the Rust
 * crate and mirrored here as `MAX_SUBTASKS` / `MAX_ATTACHMENTS` are, so the add
 * path can refuse the 201st prerequisite before it builds a list the database
 * would reject and then lose on restart.
 */
const MAX_DEPENDENCIES = 200;
/**
 * How many matches the result list shows at once. A short, one-glance list with
 * a bounded set of Tab stops — the whole point of a search box over a long
 * native `<select>` — so the tab order never swallows the entire task list.
 */
const MAX_RESULTS = 8;

const dependsOn = computed(() => props.todo.dependsOn);

/** The search box's text, kept so it can drive the live filter. */
const query = ref("");
/**
 * Whether the search box is showing. The "＋ 添加前置任务" trigger is always
 * there; clicking it reveals the box and keeps it revealed so several
 * prerequisites can be added in a run.
 */
const searchRevealed = ref(false);
/**
 * Whether the last add was refused for closing a cycle. Shown inline beneath the
 * box and cleared on the next keystroke, the authoritative announcement always
 * going through the page's one live region (规格 c) rather than a second one.
 */
const cycleRejected = ref(false);

const sectionRef = ref<HTMLElement | null>(null);
const searchInputRef = ref<HTMLInputElement | null>(null);

/**
 * The selected prerequisites as display rows: each id resolved to its task, or
 * marked unresolvable when the task was deleted or has not synced to this device
 * yet — that row still shows and can still be removed, so a dependency on a task
 * this device cannot see never traps the editor.
 */
const selected = computed(() =>
  dependsOn.value.map((id) => {
    const dep = todoStore.items.find((item) => item.id === id);
    if (!dep) {
      return { id, title: null as string | null, done: false, resolved: false };
    }
    return { id, title: dep.title, done: dep.status === "completed", resolved: true };
  }),
);

/** The screen-reader label for a row's status icon (never carried by colour alone). */
function statusLabel(row: { resolved: boolean; done: boolean }): string {
  if (!row.resolved) return "已删除或未同步";
  return row.done ? "已完成" : "未完成";
}

/**
 * The candidate tasks a prerequisite can be chosen from, before the result cap.
 *
 * Self, already-selected tasks and any task that would close a dependency cycle
 * are culled here so the illegal choices are never offered (规格 a/③, c). An
 * empty query lists recent open tasks — a task must still be open to be a useful
 * thing to wait on — while a typed query matches any task's title, so a
 * completed task can still be added deliberately by searching for it.
 */
const filtered = computed(() => {
  const selfId = props.todo.id;
  const selectedIds = new Set(dependsOn.value);
  const eligible = todoStore.items.filter(
    (item) =>
      item.id !== selfId &&
      !selectedIds.has(item.id) &&
      !todoStore.wouldCreateDependencyCycle(selfId, item.id),
  );
  const needle = query.value.trim().toLowerCase();
  if (!needle) return eligible.filter((item) => item.status === "open");
  return eligible.filter((item) => item.title.toLowerCase().includes(needle));
});

/** The results actually shown, capped so the Tab stops stay bounded. */
const results = computed(() => filtered.value.slice(0, MAX_RESULTS));
/** How many matches were cut by the cap, for the overflow hint. */
const overflowCount = computed(() => Math.max(0, filtered.value.length - MAX_RESULTS));

/** Reveals the search box on click and puts the cursor in it. */
function revealSearch(): void {
  searchRevealed.value = true;
  void nextTick(() => searchInputRef.value?.focus());
}

/** Clears the transient cycle hint as soon as the query changes. */
function onQueryInput(event: Event): void {
  cycleRejected.value = false;
  query.value = (event.target as HTMLInputElement).value;
}

/**
 * Adds one prerequisite. The list stops at the contract ceiling (said once
 * through the live region); a candidate that would close a cycle is refused the
 * same way — pre-culling keeps it out of the results, but a race could still
 * surface one, and the store refuses it for good measure. The search box and its
 * focus are kept so prerequisites can be added in a run.
 */
function addPrerequisite(id: string): void {
  if (dependsOn.value.length >= MAX_DEPENDENCIES) {
    announceFormAlert(
      "warn",
      `前置任务数量已达上限 ${MAX_DEPENDENCIES} 项，无法再添加。`,
      "transient",
    );
    return;
  }
  if (!todoStore.editDependsOn(props.todo.id, [...dependsOn.value, id])) {
    cycleRejected.value = true;
    announceFormAlert("error", "无法添加：这会让任务互相依赖（成环）。", "transient");
    return;
  }
  void nextTick(() => searchInputRef.value?.focus());
}

/**
 * Removes one prerequisite, then hands focus to the row that slid into its place
 * — or to the "＋ 添加前置任务" trigger when it was the last one — so a keyboard
 * user is never dropped onto the page body (规格 5, mirroring
 * `AttachmentList.focusAfterRemove`).
 */
function remove(index: number): void {
  const next = dependsOn.value.filter((_, position) => position !== index);
  todoStore.editDependsOn(props.todo.id, next);
  void nextTick(() => focusAfterRemove(index));
}

function focusAfterRemove(removedIndex: number): void {
  const remaining = dependsOn.value.length;
  if (remaining === 0) {
    sectionRef.value?.querySelector<HTMLElement>('[data-role="add-trigger"]')?.focus();
    return;
  }
  const targetIndex = Math.min(removedIndex, remaining - 1);
  const buttons = sectionRef.value?.querySelectorAll<HTMLElement>('[data-role="remove"]');
  buttons?.[targetIndex]?.focus();
}
</script>

<template>
  <section
    ref="sectionRef"
    class="grid gap-3 border-t border-slate-100 pt-4 dark:border-slate-800"
    aria-label="前置任务"
  >
    <p v-if="dependsOn.length" class="m-0 text-xs font-medium text-slate-500 dark:text-slate-400">
      前置任务 <span class="tabular-nums">{{ dependsOn.length }}</span>
    </p>

    <ul v-if="dependsOn.length" class="m-0 grid list-none gap-2 p-0">
      <li v-for="(row, index) in selected" :key="row.id" class="flex items-center gap-2">
        <!-- status icon: a completed prerequisite reads as satisfied (emerald
             Check), anything else as pending (neutral Circle). Decorative — the
             state is carried by the row's sr-only text, never the icon or colour
             alone. -->
        <Check
          v-if="row.resolved && row.done"
          :size="16"
          class="shrink-0 text-emerald-600 dark:text-emerald-300"
          aria-hidden="true"
        />
        <Circle
          v-else
          :size="16"
          class="shrink-0 text-slate-400 dark:text-slate-500"
          aria-hidden="true"
        />

        <span
          class="min-w-0 flex-1 truncate text-sm"
          :class="
            row.resolved
              ? 'text-slate-700 dark:text-slate-200'
              : 'text-slate-400 dark:text-slate-500'
          "
          :title="row.title ?? undefined"
        >
          <span class="sr-only">{{ statusLabel(row) }}：</span
          >{{ row.title ?? "（已删除或未同步的任务）" }}
        </span>

        <Button
          variant="ghost"
          class="min-h-11 min-w-11 shrink-0 !p-2 text-slate-400 hover:text-red-600 dark:hover:text-red-400"
          data-role="remove"
          :aria-label="`移除前置任务 ${row.title ?? '（已删除或未同步的任务）'}`"
          @click="remove(index)"
        >
          <Trash2 :size="20" />
        </Button>
      </li>
    </ul>

    <Button
      variant="ghost"
      class="min-h-11 w-fit !justify-start !px-2 text-sm text-slate-500 dark:text-slate-400"
      data-role="add-trigger"
      aria-label="添加前置任务"
      @click="revealSearch"
    >
      <Plus :size="16" /><span class="ml-1">添加前置任务</span>
    </Button>

    <div v-if="searchRevealed" class="grid gap-1">
      <div class="flex items-center gap-2">
        <Search :size="16" class="shrink-0 text-slate-400 dark:text-slate-500" aria-hidden="true" />
        <input
          ref="searchInputRef"
          type="search"
          :value="query"
          placeholder="搜索任务标题…"
          aria-label="搜索任务标题"
          :aria-invalid="cycleRejected ? 'true' : undefined"
          :aria-describedby="cycleRejected ? 'dependency-cycle-error' : undefined"
          class="min-h-11 min-w-0 flex-1 rounded-lg border-0 bg-slate-100 px-3 py-2 text-base placeholder:text-slate-400 focus:ring-2 focus:ring-sky-500 sm:text-sm dark:bg-slate-900"
          @input="onQueryInput"
        />
      </div>

      <ul class="m-0 grid list-none gap-1 p-0" aria-label="搜索结果">
        <li v-for="candidate in results" :key="candidate.id">
          <button
            type="button"
            class="m-0 flex min-h-11 w-full items-center gap-2 rounded-lg border-0 bg-transparent px-2 py-2 text-left text-sm text-slate-600 hover:bg-slate-100 hover:text-slate-950 focus:ring-2 focus:ring-sky-500 sm:min-h-0 dark:text-slate-300 dark:hover:bg-slate-800 dark:hover:text-slate-100"
            :aria-label="`添加前置任务 ${candidate.title}`"
            :title="candidate.title"
            @click="addPrerequisite(candidate.id)"
          >
            <Plus :size="14" class="shrink-0" aria-hidden="true" />
            <span class="min-w-0 flex-1 truncate">{{ candidate.title }}</span>
          </button>
        </li>
        <li
          v-if="overflowCount > 0"
          class="px-2 py-1 text-xs text-slate-400 dark:text-slate-500"
          aria-hidden="true"
        >
          还有 {{ overflowCount }} 项，继续输入以缩小范围
        </li>
        <li
          v-else-if="results.length === 0"
          class="px-2 py-1 text-xs text-slate-400 dark:text-slate-500"
        >
          {{ query.trim() ? "没有匹配的任务" : "暂无可作前置的任务" }}
        </li>
      </ul>

      <p
        v-if="cycleRejected"
        id="dependency-cycle-error"
        class="m-0 text-xs text-red-600 dark:text-red-400"
      >
        这会让任务互相依赖（成环），无法添加。
      </p>
    </div>
  </section>
</template>
