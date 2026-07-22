import { computed, ref } from "vue";
import { defineStore } from "pinia";
import type { Todo } from "@/types/todo";
import type { TodoPatch } from "@/bindings/models/TodoPatch";
import { cancelAllReminders, cancelReminder, scheduleReminder } from "@/lib/notifications";
import { writeDiagnostic } from "@/lib/diagnostics";
import { buildDeleteOperation, buildUpsertOperation, recordOperation } from "@/lib/sync-engine";
import { todoRepository } from "@/lib/todo-repository";

export interface NewTodoInput {
  readonly title: string;
  readonly dueDate?: string | null;
  readonly reminderAt?: string | null;
}

const newId = () => crypto.randomUUID();

export const useTodoStore = defineStore("todos", () => {
  const items = ref<Todo[]>([]);
  const activeItems = computed(() => items.value.filter((todo) => todo.status === "open"));
  const completedItems = computed(() => items.value.filter((todo) => todo.status === "completed"));
  const repository = todoRepository();

  /** Writes a todo back to storage; failures degrade inside the repository. */
  function persist(todo: Todo): void {
    void repository.save(todo);
  }

  /** Loads persisted todos into memory. Called once on app startup. */
  async function hydrate(): Promise<void> {
    items.value = await repository.load();
    writeDiagnostic("info", "Todos restored from local storage", { count: items.value.length });
  }

  function add(input: string | NewTodoInput) {
    const normalized = typeof input === "string" ? input.trim() : input.title.trim();
    if (!normalized) return false;

    const todo: Todo = {
      id: newId(),
      title: normalized,
      status: "open",
      createdAt: new Date().toISOString(),
      completedAt: null,
      dueDate: typeof input === "string" ? null : (input.dueDate ?? null),
      reminderAt: typeof input === "string" ? null : (input.reminderAt ?? null),
    };

    items.value.unshift(todo);
    persist(todo);
    void scheduleReminder(todo);
    void recordOperation(
      buildUpsertOperation(
        todo.id,
        {
          title: todo.title,
          status: todo.status,
          dueDate: todo.dueDate,
          completedAt: todo.completedAt,
        },
        todo.createdAt,
      ),
    );
    return true;
  }

  function update(id: string, patch: Partial<Pick<Todo, "title" | "dueDate" | "reminderAt">>) {
    const todo = items.value.find((item) => item.id === id);
    if (!todo) return;

    if (patch.title !== undefined) {
      const normalized = patch.title.trim();
      if (normalized) todo.title = normalized;
    }
    if (patch.dueDate !== undefined) todo.dueDate = patch.dueDate;
    if (patch.reminderAt !== undefined) todo.reminderAt = patch.reminderAt;

    persist(todo);
    void scheduleReminder(todo);
    void recordOperation(
      buildUpsertOperation(
        todo.id,
        {
          title: patch.title,
          dueDate: patch.dueDate,
        },
        new Date().toISOString(),
      ),
    );
  }

  function toggle(id: string) {
    const todo = items.value.find((item) => item.id === id);
    if (!todo) return;

    const completed = todo.status === "open";
    todo.status = completed ? "completed" : "open";
    todo.completedAt = completed ? new Date().toISOString() : null;
    const occurredAt = new Date().toISOString();

    persist(todo);

    if (completed) {
      cancelReminder(id);
    } else {
      void scheduleReminder(todo);
    }

    void recordOperation(
      buildUpsertOperation(
        todo.id,
        { status: todo.status, completedAt: todo.completedAt },
        occurredAt,
      ),
    );
  }

  function remove(id: string) {
    items.value = items.value.filter((todo) => todo.id !== id);
    void repository.remove(id);
    cancelReminder(id);
    void recordOperation(buildDeleteOperation(id, new Date().toISOString()));
  }

  /**
   * Applies a remote upsert change (server-wins, field-level). Creates the todo
   * if it is unknown locally. Does NOT record a sync operation — this is the
   * inbound half of sync, so recording would loop.
   */
  function applyRemoteUpsert(
    todoId: string,
    patch: TodoPatch | undefined,
    occurredAt: string,
  ): void {
    const existing = items.value.find((item) => item.id === todoId);
    if (existing) {
      if (patch?.title !== undefined) existing.title = patch.title;
      if (patch?.status !== undefined) existing.status = patch.status;
      if (patch?.dueDate !== undefined) existing.dueDate = patch.dueDate;
      if (patch?.completedAt !== undefined) existing.completedAt = patch.completedAt;
      persist(existing);
      return;
    }

    // Unknown todo: materialise it from the patch. A valid upsert patch always
    // carries a title (enforced by the server contract).
    if (patch?.title) {
      const created: Todo = {
        id: todoId,
        title: patch.title,
        status: patch.status ?? "open",
        createdAt: occurredAt,
        completedAt: patch.completedAt ?? null,
        dueDate: patch.dueDate ?? null,
        reminderAt: null,
      };
      items.value.unshift(created);
      persist(created);
    }
  }

  /** Applies a remote delete change. Does NOT record a sync operation. */
  function applyRemoteDelete(todoId: string): void {
    items.value = items.value.filter((todo) => todo.id !== todoId);
    void repository.remove(todoId);
  }

  /** Reschedules every open todo's reminder. Called once on app startup. */
  async function rescheduleAll(): Promise<void> {
    cancelAllReminders();
    let scheduled = 0;
    for (const todo of items.value) {
      if (todo.status === "open" && todo.reminderAt) {
        if (await scheduleReminder(todo)) scheduled += 1;
      }
    }
    writeDiagnostic("info", "Reminders rescheduled on startup", { scheduled });
  }

  return {
    items,
    activeItems,
    completedItems,
    hydrate,
    add,
    update,
    toggle,
    remove,
    applyRemoteUpsert,
    applyRemoteDelete,
    rescheduleAll,
  };
});
