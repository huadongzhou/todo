<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { Check, ClipboardList, Trash2 } from "lucide-vue-next";
import {
  type TodayCardTodo,
  emitRemove,
  emitToggle,
  setupTodayCardReceiver,
} from "@/lib/today-card-sync";
import { dueDateTone, formatDueDate, TONE_LABEL_CLASS } from "@/lib/dueDate";

/**
 * Today-card view. Rendered inside the card `WebviewWindow`
 * (`index.html?card=today`). Receives its todo list from the main window over
 * the `today-card:todos` event and writes toggle/remove intents back the same
 * way, keeping the two windows in sync without shared state.
 */
const todos = ref<TodayCardTodo[]>([]);
let unlisten: (() => void) | undefined;

onMounted(() => {
  unlisten = setupTodayCardReceiver((next) => {
    todos.value = next;
  });
});

onBeforeUnmount(() => {
  unlisten?.();
});

function onToggle(id: string): void {
  void emitToggle(id);
}

function onRemove(id: string): void {
  void emitRemove(id);
}
</script>

<template>
  <main class="flex h-screen flex-col bg-white dark:bg-slate-950">
    <header
      class="flex items-center gap-3 border-b border-slate-100 px-5 py-4 dark:border-slate-800"
    >
      <div class="flex h-9 w-9 items-center justify-center rounded-lg bg-sky-100 text-sky-600">
        <ClipboardList :size="18" />
      </div>
      <div class="min-w-0 flex-1">
        <h1 class="m-0 text-sm font-semibold text-slate-950 dark:text-slate-100">今日待办</h1>
        <p class="m-0 text-xs text-slate-500 dark:text-slate-400">{{ todos.length }} 项待完成</p>
      </div>
    </header>

    <section class="flex-1 overflow-y-auto">
      <div v-if="todos.length" class="px-3 py-2">
        <article
          v-for="todo in todos"
          :key="todo.id"
          class="mb-1 flex items-center gap-3 rounded-lg px-2 py-2 hover:bg-slate-50 dark:hover:bg-slate-900"
        >
          <button
            class="flex h-6 w-6 shrink-0 items-center justify-center rounded-full border border-slate-300 bg-white text-white focus-ring dark:border-slate-600 dark:bg-slate-950"
            :aria-label="`完成 ${todo.title}`"
            @click="onToggle(todo.id)"
          >
            <Check :size="14" class="text-slate-400" />
          </button>
          <div class="min-w-0 flex-1">
            <p class="m-0 truncate text-sm text-slate-700 dark:text-slate-200">{{ todo.title }}</p>
            <p
              v-if="todo.dueDate"
              class="mb-0 mt-0.5 inline-block rounded-md px-1.5 py-0.5 text-[11px] font-medium"
              :class="TONE_LABEL_CLASS[dueDateTone(todo.dueDate, false)]"
            >
              {{ formatDueDate(todo.dueDate) }}
            </p>
          </div>
          <button
            class="!p-1.5 text-slate-400 hover:text-red-600"
            :aria-label="`删除 ${todo.title}`"
            @click="onRemove(todo.id)"
          >
            <Trash2 :size="15" />
          </button>
        </article>
      </div>
      <div v-else class="grid h-full place-items-center px-6 text-center">
        <div>
          <div
            class="mx-auto mb-3 flex h-10 w-10 items-center justify-center rounded-full bg-sky-100 text-sky-600"
          >
            <Check :size="20" />
          </div>
          <p class="m-0 text-sm font-medium text-slate-800 dark:text-slate-100">今日暂无待办</p>
          <p class="mb-0 mt-1 text-xs text-slate-500 dark:text-slate-400">享受你的清闲时光。</p>
        </div>
      </div>
    </section>
  </main>
</template>
