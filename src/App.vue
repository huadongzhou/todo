<script setup lang="ts">
import { ref } from "vue";
import { Check, ClipboardList, Plus, Trash2 } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { useTodoStore } from "@/stores/todos";

const todoStore = useTodoStore();
const draft = ref("");

function submit() {
  if (todoStore.add(draft.value)) draft.value = "";
}
</script>

<template>
  <main class="mx-auto min-h-screen max-w-2xl px-5 py-12 sm:py-20">
    <header class="mb-10 flex items-start justify-between gap-4">
      <div>
        <p class="mb-2 text-sm font-semibold tracking-wide text-sky-600">TODAY</p>
        <h1 class="m-0 text-3xl font-bold tracking-tight text-slate-950">我的待办</h1>
        <p class="mb-0 mt-2 text-slate-500">{{ todoStore.activeItems.length }} 项待完成</p>
      </div>
      <div
        class="flex h-11 w-11 items-center justify-center rounded-xl bg-sky-100 text-sky-600"
        aria-hidden="true"
      >
        <ClipboardList :size="22" />
      </div>
    </header>

    <form class="surface-card mb-6 flex gap-2 p-2" @submit.prevent="submit">
      <input
        v-model="draft"
        class="min-w-0 flex-1 rounded-lg border-0 bg-transparent px-3 outline-none placeholder:text-slate-400 focus:ring-2 focus:ring-sky-500"
        placeholder="添加一项待办…"
        aria-label="待办标题"
      />
      <Button type="submit" :disabled="!draft.trim()"
        ><Plus :size="18" /><span class="ml-1">添加</span></Button
      >
    </form>

    <section aria-labelledby="todo-list-heading">
      <h2 id="todo-list-heading" class="sr-only">待办列表</h2>
      <div v-if="todoStore.items.length" class="surface-card overflow-hidden">
        <article
          v-for="todo in todoStore.items"
          :key="todo.id"
          class="flex items-center gap-3 border-b border-slate-100 px-4 py-3 last:border-0"
        >
          <button
            class="flex h-6 w-6 shrink-0 items-center justify-center rounded-full border border-slate-300 text-white focus-ring"
            :class="todo.status === 'completed' ? 'border-sky-500 bg-sky-500' : 'bg-white'"
            :aria-label="todo.status === 'completed' ? '标记为未完成' : '标记为完成'"
            @click="todoStore.toggle(todo.id)"
          >
            <Check v-if="todo.status === 'completed'" :size="15" />
          </button>
          <p
            class="m-0 min-w-0 flex-1 text-slate-700"
            :class="{ 'text-slate-400 line-through': todo.status === 'completed' }"
          >
            {{ todo.title }}
          </p>
          <Button
            variant="ghost"
            class="!p-2 text-slate-400 hover:text-red-600"
            :aria-label="`删除 ${todo.title}`"
            @click="todoStore.remove(todo.id)"
            ><Trash2 :size="17"
          /></Button>
        </article>
      </div>
      <div v-else class="surface-card grid place-items-center px-6 py-16 text-center">
        <div
          class="mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-sky-100 text-sky-600"
        >
          <Check :size="24" />
        </div>
        <p class="m-0 font-medium text-slate-800">暂无待办</p>
        <p class="mb-0 mt-1 text-sm text-slate-500">从上方添加第一项任务吧。</p>
      </div>
    </section>
  </main>
</template>
