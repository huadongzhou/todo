import { watch } from "vue";
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { todayLocalDay } from "@/lib/dueDate";
import { useTodoStore } from "@/stores/todos";
import type { Todo } from "@/types/todo";

/**
 * Cross-window sync for the today card.
 *
 * The today card is a separate `WebviewWindow` with its own JS context and its
 * own (empty) Pinia store, so it cannot read the main window's todos directly.
 * Following develop.md's communication model (lightweight UI sync → event), the
 * two windows coordinate over three Tauri events:
 *
 *   - `today-card:todos`  — main → card, the live list of today's open todos
 *   - `today-card:request`— card → main, ask for a fresh push (cold start)
 *   - `today-card:toggle` — card → main, toggle a todo by id
 *   - `today-card:remove` — card → main, remove a todo by id
 */

export const EVENT_TODOS = "today-card:todos";
export const EVENT_REQUEST = "today-card:request";
export const EVENT_TOGGLE = "today-card:toggle";
export const EVENT_REMOVE = "today-card:remove";

/** Minimal todo shape the card needs to render a row. */
export interface TodayCardTodo {
  readonly id: string;
  readonly title: string;
  readonly dueDate: string | null;
  readonly status: "open" | "completed";
}

/**
 * Today's open todos: unfinished items whose deadline is today or earlier.
 * `YYYY-MM-DD` strings compare lexicographically in chronological order, so a
 * plain `<=` correctly captures overdue + due-today.
 *
 * Archived todos are left out for the same reason the main list leaves them
 * out. This device never produces an archived todo that is still open, but
 * another one can send one in, and a card showing a task the list does not is
 * two answers to the same question.
 */
export function selectTodayOpenTodos(items: Todo[]): TodayCardTodo[] {
  const today = todayLocalDay();
  // The filter guarantees `dueDate` is a non-empty string; map it to the
  // card's `string | null` shape so the optional `Todo.dueDate` is narrowed.
  return items
    .filter(
      (todo) => todo.status === "open" && !todo.archivedAt && todo.dueDate && todo.dueDate <= today,
    )
    .map((todo) => ({
      id: todo.id,
      title: todo.title,
      dueDate: todo.dueDate ?? null,
      status: todo.status,
    }));
}

/** Main window side: push today's todos on every change + serve cold-start requests. */
export function setupTodayCardSync(): UnlistenFn | null {
  const store = useTodoStore();
  const unlistenFns: UnlistenFn[] = [];

  // Push whenever the store changes (deep so toggles/title edits propagate).
  const stopWatch = watch(
    () => store.items,
    (items) => {
      void emit(EVENT_TODOS, selectTodayOpenTodos([...items]));
    },
    { deep: true, immediate: true },
  );

  // Serve cold-start requests from a freshly opened card.
  void listen(EVENT_REQUEST, () => {
    void emit(EVENT_TODOS, selectTodayOpenTodos([...store.items]));
  })
    .then((unlisten) => unlistenFns.push(unlisten))
    .catch(() => {
      /* best effort */
    });

  // Apply card-initiated mutations back onto the main store.
  void listen<string>(EVENT_TOGGLE, (event) => store.toggle(event.payload))
    .then((unlisten) => unlistenFns.push(unlisten))
    .catch(() => {
      /* best effort */
    });

  void listen<string>(EVENT_REMOVE, (event) => store.remove(event.payload))
    .then((unlisten) => unlistenFns.push(unlisten))
    .catch(() => {
      /* best effort */
    });

  return () => {
    stopWatch();
    for (const unlisten of unlistenFns) {
      try {
        unlisten();
      } catch {
        /* best effort */
      }
    }
  };
}

/**
 * Card window side: request a fresh list on mount, then keep applying pushes.
 * Returns an unlisten function that detaches the `today-card:todos` listener.
 */
export function setupTodayCardReceiver(onTodos: (todos: TodayCardTodo[]) => void): () => void {
  let unlistenTodos: UnlistenFn | undefined;
  let cancelled = false;

  void listen<TodayCardTodo[]>(EVENT_TODOS, (event) => {
    if (!cancelled) onTodos(event.payload);
  })
    .then((unlisten) => {
      unlistenTodos = unlisten;
    })
    .catch(() => {
      /* best effort */
    });

  // Ask the main window for the current list (cold start).
  void emit(EVENT_REQUEST).catch(() => {
    /* main window may not be running; ignore */
  });

  return () => {
    cancelled = true;
    if (unlistenTodos) {
      try {
        unlistenTodos();
      } catch {
        /* best effort */
      }
    }
  };
}

/** Card → main: ask the main window to toggle a todo. */
export function emitToggle(id: string): void {
  void emit(EVENT_TOGGLE, id).catch(() => {
    /* main window may not be running; ignore */
  });
}

/** Card → main: ask the main window to remove a todo. */
export function emitRemove(id: string): void {
  void emit(EVENT_REMOVE, id).catch(() => {
    /* main window may not be running; ignore */
  });
}
