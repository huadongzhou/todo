import { isTauri } from "@tauri-apps/api/core";
import { nativeCommands } from "@/lib/native";
import { writeDiagnostic } from "@/lib/diagnostics";
import type { Todo } from "@/types/todo";

/**
 * Where a write ended up.
 *
 * Callers need this to tell "the todo is in the database" from "the database
 * refused it and the only copy is the pending-write journal" — the second is a
 * write the app still owes, and the difference is what the caller surfaces to
 * the user and what the startup replay settles.
 */
export type WriteOutcome =
  /** Landed in this runtime's primary store: SQLite on desktop, `localStorage` in the browser. */
  | "stored"
  /** SQLite refused the write; it is recorded in the journal and replayed on the next start. */
  | "degraded"
  /** Nothing kept the write, not even the journal. */
  | "failed";

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
  save(todo: Todo): Promise<WriteOutcome>;
  /** Removes the todo; removing an unknown id succeeds. */
  remove(id: string): Promise<WriteOutcome>;
}

/**
 * The pending-write journal: every write SQLite still owes the user.
 *
 * `todos.items` holds the todos it would not take, `todos.deletions` the ids
 * whose removal it would not take. Two keys rather than one flag inside the
 * list, because a deletion has no todo to hang a flag on — and the shape of
 * `todos.items` stays the plain todo list it has always been, so a profile
 * written by an earlier build needs no conversion.
 *
 * One invariant makes the journal safe to replay: **a successful native write
 * clears that id from both keys**. An entry therefore only survives while
 * SQLite holds nothing newer for that id, which is what settles the question
 * the journal used to be unable to answer — is this copy newer than the stored
 * row, or an older leftover? It is always the newer one, so replaying it
 * overwrites, and a pending deletion always wins over the stored row.
 *
 * In the browser there is no database behind the journal, so `todos.items`
 * simply is the store and `todos.deletions` is never written.
 */
const PENDING_TODOS_KEY = "todos.items";
const PENDING_DELETIONS_KEY = "todos.deletions";

/**
 * Entries no build can read back as a todo. They are parked here instead of
 * being retried forever or thrown away: SQLite cannot take them, the UI cannot
 * render them, but they are still the user's bytes.
 */
const QUARANTINE_KEY = "todos.quarantine";

function readJson(key: string): unknown {
  try {
    const raw = window.localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as unknown) : null;
  } catch {
    writeDiagnostic("warn", "Unable to read a local todo storage key", { key });
    return null;
  }
}

function writeJson(key: string, value: unknown): boolean {
  try {
    window.localStorage.setItem(key, JSON.stringify(value));
    return true;
  } catch {
    writeDiagnostic("warn", "Unable to write a local todo storage key", { key });
    return false;
  }
}

function removeKey(key: string): boolean {
  try {
    window.localStorage.removeItem(key);
    return true;
  } catch {
    writeDiagnostic("warn", "Unable to clear a local todo storage key", { key });
    return false;
  }
}

const TODO_FIELDS: readonly string[] = [
  "id",
  "title",
  "status",
  "createdAt",
  "completedAt",
  "dueDate",
  "reminderAt",
];

function isOptionalString(value: unknown): boolean {
  return value === undefined || value === null || typeof value === "string";
}

/**
 * Mirrors the Rust `Todo` contract — `deny_unknown_fields`, `completedAt`
 * always present — so every entry this accepts survives the IPC boundary.
 * Without it a single malformed entry fails the whole command, which is how one
 * bad row used to keep every other pending write out of the database.
 */
function isTodo(value: unknown): value is Todo {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as Record<string, unknown>;
  if (Object.keys(candidate).some((key) => !TODO_FIELDS.includes(key))) return false;
  return (
    typeof candidate.id === "string" &&
    typeof candidate.title === "string" &&
    (candidate.status === "open" || candidate.status === "completed") &&
    typeof candidate.createdAt === "string" &&
    (candidate.completedAt === null || typeof candidate.completedAt === "string") &&
    isOptionalString(candidate.dueDate) &&
    isOptionalString(candidate.reminderAt)
  );
}

interface PendingWrites {
  /** Todos SQLite has not accepted; each one is newer than any row it shares an id with. */
  readonly upserts: Todo[];
  /** Ids whose deletion SQLite has not accepted. */
  readonly deletions: string[];
  /** Stored entries that are not readable as todos. */
  readonly malformed: unknown[];
}

function readPendingWrites(): PendingWrites {
  const storedTodos = readJson(PENDING_TODOS_KEY);
  const upserts: Todo[] = [];
  const malformed: unknown[] = [];

  if (Array.isArray(storedTodos)) {
    for (const entry of storedTodos) {
      if (isTodo(entry)) upserts.push(entry);
      else malformed.push(entry);
    }
  } else if (storedTodos !== null) {
    malformed.push(storedTodos);
  }

  const storedDeletions = readJson(PENDING_DELETIONS_KEY);
  const deletions = Array.isArray(storedDeletions)
    ? storedDeletions.filter((id): id is string => typeof id === "string")
    : [];

  return { upserts, deletions, malformed };
}

function writePendingTodos(todos: readonly Todo[]): boolean {
  return todos.length === 0 ? removeKey(PENDING_TODOS_KEY) : writeJson(PENDING_TODOS_KEY, todos);
}

function writePendingDeletions(ids: readonly string[]): boolean {
  return ids.length === 0
    ? removeKey(PENDING_DELETIONS_KEY)
    : writeJson(PENDING_DELETIONS_KEY, ids);
}

/** Records a todo SQLite refused; it supersedes any pending deletion of that id. */
function recordPendingUpsert(todo: Todo): boolean {
  const { upserts, deletions } = readPendingWrites();
  const index = upserts.findIndex((item) => item.id === todo.id);
  if (index === -1) upserts.unshift(todo);
  else upserts[index] = todo;

  if (deletions.includes(todo.id)) {
    writePendingDeletions(deletions.filter((id) => id !== todo.id));
  }
  return writePendingTodos(upserts);
}

/**
 * Records a deletion SQLite refused. Without this tombstone the row is still in
 * the database, so the next read — and the next replay — would bring the todo
 * the user deleted straight back.
 */
function recordPendingDeletion(id: string): boolean {
  const { upserts, deletions } = readPendingWrites();
  const remaining = upserts.filter((todo) => todo.id !== id);
  if (remaining.length !== upserts.length) writePendingTodos(remaining);

  if (deletions.includes(id)) return true;
  return writePendingDeletions([...deletions, id]);
}

/**
 * Drops every pending write for an id. Called whenever a native write succeeds:
 * the stored row is the newest copy from that moment on, so anything still
 * parked would be a stale write waiting to overwrite it.
 */
function clearPendingWrites(id: string): boolean {
  const { upserts, deletions } = readPendingWrites();
  let cleared = true;

  const remaining = upserts.filter((todo) => todo.id !== id);
  if (remaining.length !== upserts.length) cleared = writePendingTodos(remaining) && cleared;
  if (deletions.includes(id)) {
    cleared = writePendingDeletions(deletions.filter((item) => item !== id)) && cleared;
  }
  return cleared;
}

/** The todos a journal stands for on its own: pending upserts minus tombstones. */
function materialise(pending: PendingWrites): Todo[] {
  const deleted = new Set(pending.deletions);
  return pending.upserts.filter((todo) => !deleted.has(todo.id));
}

const browserRepository: TodoRepository = {
  load: () => Promise.resolve(materialise(readPendingWrites())),
  save: (todo) => Promise.resolve(recordPendingUpsert(todo) ? "stored" : "failed"),
  // No database sits behind the browser store, so a removal is a removal — a
  // tombstone would have nothing to be replayed against.
  remove: (id) => Promise.resolve(clearPendingWrites(id) ? "stored" : "failed"),
};

/** Narrows an unknown caught value to a loggable string. */
function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** The list order SQLite returns (`created_at DESC, id`), applied to merged rows. */
function newestFirst(left: Todo, right: Todo): number {
  if (left.createdAt !== right.createdAt) return left.createdAt < right.createdAt ? 1 : -1;
  if (left.id === right.id) return 0;
  return left.id < right.id ? -1 : 1;
}

/** Reads SQLite, or `null` when the native side is unusable. */
async function listNativeTodos(): Promise<Todo[] | null> {
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
  return null;
}

/**
 * The native database is the source of truth on desktop, but a broken database
 * must not cost the user their todos, so every failure logs and falls back to
 * the browser path — the same degradation strategy `settings-storage` uses.
 */
const sqliteRepository: TodoRepository = {
  load: async () => {
    const native = await listNativeTodos();
    if (native === null) return browserRepository.load();

    const pending = readPendingWrites();
    const outstanding = pending.upserts.length + pending.deletions.length;
    if (outstanding === 0) return native;

    // Pending writes are the newer copy of whatever they touch, so the view has
    // to show them the way the database will once the replay succeeds:
    // tombstoned rows are gone, parked edits win over the stored row, and
    // parked todos SQLite never saw are added.
    const deleted = new Set(pending.deletions);
    const parked = new Map(pending.upserts.map((todo) => [todo.id, todo]));
    const stored = new Set(native.map((todo) => todo.id));
    const merged = native
      .filter((todo) => !deleted.has(todo.id))
      .map((todo) => parked.get(todo.id) ?? todo);
    const unstored = pending.upserts.filter(
      (todo) => !stored.has(todo.id) && !deleted.has(todo.id),
    );

    writeDiagnostic("warn", "Showing todo writes SQLite has not accepted yet", {
      count: outstanding,
    });
    return [...merged, ...unstored].sort(newestFirst);
  },
  save: async (todo) => {
    try {
      const result = await nativeCommands.saveTodo(todo);
      if (result.status === "ok") {
        // The stored row is now the newest copy of this todo, so nothing about
        // it is owed any more. Leaving an older parked copy behind is exactly
        // how a replay would overwrite a good row with a stale one.
        clearPendingWrites(todo.id);
        return "stored";
      }
      writeDiagnostic("warn", "Unable to save a todo to SQLite; using browser fallback", {
        error: result.error,
      });
    } catch (error) {
      writeDiagnostic("warn", "Unable to save a todo to SQLite; using browser fallback", {
        error: errorMessage(error),
      });
    }
    return recordPendingUpsert(todo) ? "degraded" : "failed";
  },
  remove: async (id) => {
    try {
      const result = await nativeCommands.deleteTodo(id);
      if (result.status === "ok") {
        // An outage may have left a parked copy; drop it too, otherwise the
        // next replay would resurrect the deleted todo.
        clearPendingWrites(id);
        return "stored";
      }
      writeDiagnostic("warn", "Unable to delete a todo from SQLite; using browser fallback", {
        error: result.error,
      });
    } catch (error) {
      writeDiagnostic("warn", "Unable to delete a todo from SQLite; using browser fallback", {
        error: errorMessage(error),
      });
    }
    return recordPendingDeletion(id) ? "degraded" : "failed";
  },
};

/** What the startup replay did, for diagnostics and for the caller to act on. */
export type PendingWriteFlush =
  | { readonly status: "skipped"; readonly reason: "browser-runtime" | "nothing-pending" }
  | {
      readonly status: "flushed";
      readonly applied: number;
      readonly rejected: number;
      readonly quarantined: number;
    }
  | { readonly status: "failed"; readonly error: string; readonly outstanding: number };

/** Parks entries that are not todos, so the journal can drain past them. */
function quarantine(entries: readonly unknown[]): void {
  const parked = readJson(QUARANTINE_KEY);
  const kept = Array.isArray(parked) ? parked : [];
  const at = new Date().toISOString();
  writeJson(QUARANTINE_KEY, [...kept, ...entries.map((entry) => ({ at, entry }))]);
  writeDiagnostic("warn", "Parked local todo entries that are not readable as todos", {
    count: entries.length,
    key: QUARANTINE_KEY,
  });
}

/** Removes the entries SQLite confirmed; anything it refused stays pending. */
function dropApplied(applied: readonly string[]): void {
  if (applied.length === 0) return;
  const done = new Set(applied);
  const { upserts, deletions } = readPendingWrites();
  writePendingTodos(upserts.filter((todo) => !done.has(todo.id)));
  writePendingDeletions(deletions.filter((id) => !done.has(id)));
}

/**
 * Settles everything the database still owes, once, at startup — this is the
 * Store-to-SQLite migration: whatever todo data lives outside SQLite (data an
 * earlier build persisted, plus every write parked while the database was
 * unavailable) is moved in here, and the app runs on one store afterwards.
 *
 * Nothing is dropped on a guess. An entry leaves the journal only because
 * SQLite named it in `applied`, so a refused row, a failed command or a crash
 * mid-way all end the same way: the data is still there, still shown by `load`,
 * still retried on the next start. Entries that are not todos at all cannot be
 * retried into existence, so they move to the quarantine key instead of
 * blocking every other pending write behind them.
 */
export async function flushPendingWrites(): Promise<PendingWriteFlush> {
  // In the browser `localStorage` is the store, not a journal to drain.
  if (!isTauri()) return { status: "skipped", reason: "browser-runtime" };

  const pending = readPendingWrites();
  if (pending.malformed.length > 0) {
    quarantine(pending.malformed);
    writePendingTodos(pending.upserts);
  }

  const outstanding = pending.upserts.length + pending.deletions.length;
  if (outstanding === 0) {
    return pending.malformed.length === 0
      ? { status: "skipped", reason: "nothing-pending" }
      : { status: "flushed", applied: 0, rejected: 0, quarantined: pending.malformed.length };
  }

  let error: string;
  try {
    const result = await nativeCommands.replayPendingWrites(pending.upserts, pending.deletions);
    if (result.status === "ok") {
      const { applied, rejected } = result.data;
      dropApplied(applied);
      for (const entry of rejected) {
        writeDiagnostic("warn", "SQLite refused a pending todo write; keeping it for a retry", {
          id: entry.id,
          error: entry.error,
        });
      }
      writeDiagnostic("info", "Pending todo writes replayed into SQLite", {
        applied: applied.length,
        rejected: rejected.length,
        quarantined: pending.malformed.length,
      });
      return {
        status: "flushed",
        applied: applied.length,
        rejected: rejected.length,
        quarantined: pending.malformed.length,
      };
    }
    error = result.error;
  } catch (caught) {
    error = errorMessage(caught);
  }

  // The database itself is unusable, so nothing was applied: the journal is
  // untouched, `load` keeps merging it in, and the next start tries again.
  writeDiagnostic("warn", "Unable to replay pending todo writes; keeping them in local storage", {
    outstanding,
    error,
  });
  return { status: "failed", error, outstanding };
}

/** The repository backing this runtime: SQLite on Tauri, `localStorage` elsewhere. */
export function todoRepository(): TodoRepository {
  return isTauri() ? sqliteRepository : browserRepository;
}
