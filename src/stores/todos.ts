import { computed, ref } from "vue";
import { defineStore } from "pinia";
import type { Todo } from "@/types/todo";

const newId = () => crypto.randomUUID();

export const useTodoStore = defineStore("todos", () => {
  const items = ref<Todo[]>([]);
  const activeItems = computed(() => items.value.filter((todo) => todo.status === "open"));
  const completedItems = computed(() => items.value.filter((todo) => todo.status === "completed"));

  function add(title: string) {
    const normalizedTitle = title.trim();
    if (!normalizedTitle) return false;

    items.value.unshift({
      id: newId(),
      title: normalizedTitle,
      status: "open",
      createdAt: new Date().toISOString(),
      completedAt: null,
    });
    return true;
  }

  function toggle(id: string) {
    const todo = items.value.find((item) => item.id === id);
    if (!todo) return;

    const completed = todo.status === "open";
    todo.status = completed ? "completed" : "open";
    todo.completedAt = completed ? new Date().toISOString() : null;
  }

  function remove(id: string) {
    items.value = items.value.filter((todo) => todo.id !== id);
  }

  return { items, activeItems, completedItems, add, toggle, remove };
});
