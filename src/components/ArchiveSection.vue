<script setup lang="ts">
import { nextTick, ref } from "vue";
import { ArchiveRestore, ChevronDown, Trash2 } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { daysSinceLocalDay } from "@/lib/dueDate";
import { useTodoStore } from "@/stores/todos";

/**
 * The archive, as a collapsed block under the list.
 *
 * It takes no props and emits nothing: everything it needs is in the store.
 * That is what lets it become a page of its own the day there is a navigation
 * shell to hold one — the line that renders it moves, and nothing in here does.
 */
const store = useTodoStore();

const open = ref(false);
const toggleRef = ref<HTMLButtonElement | null>(null);

/**
 * When a todo was archived, in the terms this block is read in.
 *
 * Deliberately not `formatDueDate`: that vocabulary is about how long is left
 * before a deadline, and applied to a past instant it says things like "overdue
 * by 3 days", which is not a sentence about when something was filed away.
 */
function archivedLabel(instant: string | null | undefined): string {
  if (!instant) return "";
  const parsed = new Date(instant);
  if (Number.isNaN(parsed.getTime())) return "";
  if (daysSinceLocalDay(instant) === 0) return "今天归档";
  return `${parsed.getMonth() + 1}月${parsed.getDate()}日归档`;
}

/**
 * Puts the focus back on the header button after a row leaves.
 *
 * The row the focus was on is gone, and without this it falls to `<body>`,
 * which drops a keyboard user back to the top of the page. The header is always
 * there to receive it — which is why this block still renders when it is empty.
 */
async function returnFocus(): Promise<void> {
  await nextTick();
  toggleRef.value?.focus();
}

function restore(id: string): void {
  store.unarchive(id);
  void returnFocus();
}

function remove(id: string): void {
  store.remove(id);
  void returnFocus();
}
</script>

<template>
  <!--
    Shown as soon as there is any task at all, even with nothing archived yet:
    an empty archive still explains itself, and a block that appeared out of
    nowhere on the seventh day would be a surprise rather than a feature. With
    no tasks whatsoever there is nothing to explain, and the page stays as it
    was.
  -->
  <section
    v-if="store.items.length"
    class="surface-card mt-6 overflow-hidden"
    aria-labelledby="archive-heading"
  >
    <h2 class="m-0">
      <button
        id="archive-heading"
        ref="toggleRef"
        type="button"
        class="flex min-h-11 w-full items-center justify-between gap-3 border-0 bg-transparent px-4 py-3 text-left focus-visible:focus-ring"
        :aria-expanded="open"
        aria-controls="archive-list"
        @click="open = !open"
      >
        <span class="text-sm font-medium text-slate-800 dark:text-slate-200"
          >已归档（{{ store.archivedItems.length }}）</span
        >
        <ChevronDown
          :size="20"
          class="shrink-0 text-slate-400 transition-transform duration-200 motion-reduce:transition-none"
          :class="open ? 'rotate-180' : ''"
          aria-hidden="true"
        />
      </button>
    </h2>

    <!--
      `v-show` rather than `v-if`: `aria-controls` above has to point at an
      element that is there whether or not it is showing.
    -->
    <div id="archive-list" v-show="open" class="border-t border-slate-100 dark:border-slate-800">
      <!--
        Said before the button that does it, because "还原" on its own does not
        say that the task also stops being completed.
      -->
      <p class="m-0 px-4 pt-3 text-xs text-slate-500 dark:text-slate-400">
        还原会把任务放回待办列表，并标记为未完成。
      </p>

      <p
        v-if="store.archivedItems.length === 0"
        class="mb-0 px-4 py-3 text-sm text-slate-500 dark:text-slate-400"
      >
        暂无已归档任务。已完成超过 7 天的任务会自动移到这里。
      </p>

      <article
        v-for="todo in store.archivedItems"
        :key="todo.id"
        class="flex items-center gap-3 border-b border-slate-100 px-4 py-3 last:border-0 dark:border-slate-800"
      >
        <div class="min-w-0 flex-1">
          <!--
            The title is text, not a button: an archived task cannot be edited
            in place (restore it first), so there is nothing for a click to do.
            Two exclusive colour sets rather than a base and an override, for
            the reason the list row gives. A row that is archived but not
            completed can only come from another device, and must not be shown
            as though it were finished.
          -->
          <p
            class="m-0"
            :class="
              todo.status === 'completed'
                ? 'text-slate-400 line-through dark:text-slate-500'
                : 'text-slate-700 dark:text-slate-200'
            "
          >
            {{ todo.title }}
          </p>
          <p class="mb-0 mt-1 text-xs text-slate-500 dark:text-slate-400">
            {{ archivedLabel(todo.archivedAt) }}
          </p>
        </div>
        <Button
          variant="ghost"
          class="min-h-11 min-w-11 !p-2 text-slate-400 dark:hover:text-slate-100"
          :aria-label="`还原 ${todo.title} 到待办列表`"
          @click="restore(todo.id)"
        >
          <ArchiveRestore :size="20" />
        </Button>
        <!--
          Kept here too: a task that could only be archived and never removed
          would be one the user can never get rid of.
        -->
        <Button
          variant="ghost"
          class="min-h-11 min-w-11 !p-2 text-slate-400 hover:text-red-600"
          :aria-label="`删除 ${todo.title}`"
          @click="remove(todo.id)"
        >
          <Trash2 :size="20" />
        </Button>
      </article>
    </div>
  </section>
</template>
