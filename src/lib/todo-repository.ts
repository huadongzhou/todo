import { isTauri } from "@tauri-apps/api/core";
import { nativeCommands } from "@/lib/native";
import { writeDiagnostic } from "@/lib/diagnostics";
import type { Todo } from "@/types/todo";

/**
 * Todo persistence layer.
 *
 * The store talks to this interface only, so components never learn where a
 * todo is stored. On the Tauri desktop runtime it is SQLite (owned by the Rust
 * side, reached through the generated commands); everywhere else (browser dev
 * server, any runtime where the native call fails) it degrades to
 * `localStorage`, keeping the app fully usable offline.
 */
export interface TodoRepository {
  /** Every persisted todo, newest first. */
  load(): Promise<Todo[]>;
  /** Inserts the todo or replaces the stored one with the same id. */
  save(todo: Todo): Promise<void>;
  /** Removes the todo; removing an unknown id succeeds. */
  remove(id: string): Promise<void>;
}

const BROWSER_STORAGE_KEY = "todos.items";

function readBrowserTodos(): Todo[] {
  try {
    const raw = window.localStorage.getItem(BROWSER_STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    return Array.isArray(parsed) ? (parsed as Todo[]) : [];
  } catch {
    writeDiagnostic("warn", "Unable to read todos from browser storage");
    return [];
  }
}

function writeBrowserTodos(todos: Todo[]): void {
  try {
    window.localStorage.setItem(BROWSER_STORAGE_KEY, JSON.stringify(todos));
  } catch {
    writeDiagnostic("warn", "Unable to persist todos to browser storage");
  }
}

const browserRepository: TodoRepository = {
  load: () => Promise.resolve(readBrowserTodos()),
  save: (todo) => {
    const todos = readBrowserTodos();
    const index = todos.findIndex((item) => item.id === todo.id);
    if (index === -1) {
      todos.unshift(todo);
    } else {
      todos[index] = todo;
    }
    writeBrowserTodos(todos);
    return Promise.resolve();
  },
  remove: (id) => {
    writeBrowserTodos(readBrowserTodos().filter((todo) => todo.id !== id));
    return Promise.resolve();
  },
};

/** Narrows an unknown caught value to a loggable string. */
function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/**
 * The native database is the source of truth on desktop, but a broken database
 * must not cost the user their todos, so every failure logs and falls back to
 * the browser path — the same degradation strategy `settings-storage` uses.
 */
const sqliteRepository: TodoRepository = {
  load: async () => {
    try {
      const result = await nativeCommands.listTodos();
      if (result.status === "ok") return result.data;
      writeDiagnostic("warn", "Unable to load todos from SQLite; using browser fallback", {
        error: result.error,
      });
    } catch (error) {
      writeDiagnostic("warn", "Unable to load todos from SQLite; using browser fallback", {
        error: errorMessage(error),
      });
    }
    return browserRepository.load();
  },
  save: async (todo) => {
    try {
      const result = await nativeCommands.saveTodo(todo);
      if (result.status === "ok") return;
      writeDiagnostic("warn", "Unable to save a todo to SQLite; using browser fallback", {
        error: result.error,
      });
    } catch (error) {
      writeDiagnostic("warn", "Unable to save a todo to SQLite; using browser fallback", {
        error: errorMessage(error),
      });
    }
    await browserRepository.save(todo);
  },
  remove: async (id) => {
    try {
      const result = await nativeCommands.deleteTodo(id);
      if (result.status === "ok") return;
      writeDiagnostic("warn", "Unable to delete a todo from SQLite; using browser fallback", {
        error: result.error,
      });
    } catch (error) {
      writeDiagnostic("warn", "Unable to delete a todo from SQLite; using browser fallback", {
        error: errorMessage(error),
      });
    }
    await browserRepository.remove(id);
  },
};

/** The repository backing this runtime: SQLite on Tauri, `localStorage` elsewhere. */
export function todoRepository(): TodoRepository {
  return isTauri() ? sqliteRepository : browserRepository;
}
