import { isTauri } from "@tauri-apps/api/core";
import { nativeCommands } from "@/lib/native";
import { writeDiagnostic } from "@/lib/diagnostics";
import type { Attachment, RecurrenceRule, Subtask, Todo } from "@/types/todo";

/**
 * Where a write ended up.
 *
 * Callers need this to tell "storage and the app agree about this todo" from
 * "the app still owes storage a write" — the second is what the caller
 * surfaces to the user and what the startup replay settles.
 */
export type WriteOutcome =
  /** Landed in this runtime's primary store and nothing about it is still owed. */
  | "stored"
  /**
   * The write survives, but storage is not settled: SQLite refused it and the
   * journal holds the only copy, or the row landed and the journal entry it
   * supersedes could not be cleared. Either way the next replay has work to do.
   */
  | "degraded"
  /**
   * The write is not saved and no retry will change that: either the contract
   * refused it for good (parking it would only promise a retry that can never
   * land) or even the journal would not take it. The caller keeps the todo in
   * view and tells the user, rather than letting it vanish on the next start.
   */
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
  /**
   * How many writes storage still owes right now. Zero means the journal is
   * empty, which is the only honest basis for clearing a "not saved yet"
   * warning: a single successful write says nothing about the other entries.
   */
  outstanding(): number;
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

/** How one field of a stored entry is read back. */
interface TodoFieldRule {
  /** Whether a value found under this key can be used as it is. */
  readonly accepts: (value: unknown) => boolean;
  /**
   * The Rust field has no serde default, so an entry without the key is not a
   * todo in any build and cannot be repaired.
   */
  readonly required?: true;
  /**
   * The Rust field has a serde default, so an entry written before the field
   * existed is read back with this value instead of being rejected.
   */
  readonly fallback?: () => unknown;
}

function isString(value: unknown): boolean {
  return typeof value === "string";
}

/** Optional in the contract: the key may be absent, `null`, or a string. */
function isOptionalString(value: unknown): boolean {
  return value === undefined || value === null || typeof value === "string";
}

function isOptionalNumber(value: unknown): boolean {
  return (
    value === undefined || value === null || (typeof value === "number" && Number.isFinite(value))
  );
}

function isOptionalBoolean(value: unknown): boolean {
  return value === undefined || value === null || typeof value === "boolean";
}

function isArrayOf(value: unknown, item: (entry: unknown) => boolean): boolean {
  return Array.isArray(value) && value.every(item);
}

function isStringArray(value: unknown): boolean {
  return isArrayOf(value, isString);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/**
 * Field-for-field mirror of the `Subtask` contract, unknown keys included.
 *
 * The nested contracts are checked strictly — every key the generated type
 * declares must be there — because nothing predates them: no build ever wrote a
 * subtask without `done`, so a missing key means the entry was not written by
 * this app.
 */
function isSubtask(value: unknown): boolean {
  if (!isRecord(value)) return false;
  const fields: Record<keyof Subtask, true> = { id: true, title: true, done: true };
  if (Object.keys(value).some((key) => !(key in fields))) return false;
  return (
    typeof value.id === "string" &&
    typeof value.title === "string" &&
    typeof value.done === "boolean"
  );
}

/** Field-for-field mirror of the `Attachment` contract. */
function isAttachment(value: unknown): boolean {
  if (!isRecord(value)) return false;
  const fields: Record<keyof Attachment, true> = { id: true, kind: true, url: true, name: true };
  if (Object.keys(value).some((key) => !(key in fields))) return false;
  return (
    typeof value.id === "string" &&
    (value.kind === "link" || value.kind === "file") &&
    typeof value.url === "string" &&
    isOptionalString(value.name)
  );
}

/** Field-for-field mirror of the `RecurrenceRule` contract. */
function isRecurrenceRule(value: unknown): boolean {
  if (!isRecord(value)) return false;
  const fields: Record<keyof RecurrenceRule, true> = {
    frequency: true,
    interval: true,
    weekdays: true,
    monthDay: true,
    onLastDay: true,
    calendar: true,
    until: true,
    count: true,
  };
  if (Object.keys(value).some((key) => !(key in fields))) return false;
  const frequencies = ["daily", "weekly", "monthly", "yearly", "workday"];
  const weekdays = ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"];
  return (
    typeof value.frequency === "string" &&
    frequencies.includes(value.frequency) &&
    typeof value.interval === "number" &&
    isArrayOf(value.weekdays, (day) => typeof day === "string" && weekdays.includes(day)) &&
    isOptionalNumber(value.monthDay) &&
    typeof value.onLastDay === "boolean" &&
    (value.calendar === "gregorian" || value.calendar === "lunar") &&
    isOptionalString(value.until) &&
    isOptionalNumber(value.count)
  );
}

function isOptionalRecurrenceRule(value: unknown): boolean {
  return value === undefined || value === null || isRecurrenceRule(value);
}

/**
 * The `Todo` contract as a runtime check, one rule per field.
 *
 * `satisfies Record<keyof Todo, TodoFieldRule>` is the point of the shape: a
 * field added to the Rust contract regenerates `Todo`, and this object stops
 * satisfying it, so `pnpm run check` fails instead of the app quietly deciding
 * that every entry it wrote yesterday is unreadable. The rules mirror serde —
 * the key set (`deny_unknown_fields`), which keys may be absent (`default`) and
 * what each one accepts — but not the contract's semantic rules; those are
 * enforced once, in Rust, and a violation comes back from the replay marked
 * `permanent`.
 */
const TODO_FIELDS = {
  id: { accepts: isString, required: true },
  title: { accepts: isString, required: true },
  status: {
    accepts: (value: unknown) => value === "open" || value === "completed",
    required: true,
  },
  createdAt: { accepts: isString, required: true },
  completedAt: {
    accepts: (value: unknown) => value === null || typeof value === "string",
    required: true,
  },
  dueDate: { accepts: isOptionalString },
  reminderAt: { accepts: isOptionalString },
  notes: { accepts: isOptionalString },
  startDate: { accepts: isOptionalString },
  startsAt: { accepts: isOptionalString },
  endsAt: { accepts: isOptionalString },
  estimatedMinutes: { accepts: isOptionalNumber },
  recurrence: { accepts: isOptionalRecurrenceRule },
  listId: { accepts: isOptionalString },
  important: { accepts: isOptionalBoolean },
  urgent: { accepts: isOptionalBoolean },
  sortOrder: { accepts: isOptionalNumber },
  tagIds: { accepts: isStringArray, fallback: (): string[] => [] },
  subtasks: {
    accepts: (value: unknown) => isArrayOf(value, isSubtask),
    fallback: (): Subtask[] => [],
  },
  attachments: {
    accepts: (value: unknown) => isArrayOf(value, isAttachment),
    fallback: (): Attachment[] => [],
  },
  dependsOn: { accepts: isStringArray, fallback: (): string[] => [] },
} satisfies Record<keyof Todo, TodoFieldRule>;

/**
 * Reads a stored entry back as a todo, filling in the fields it predates.
 *
 * Returning a todo (rather than a boolean) is what makes the field extension
 * lossless: an entry written before `tagIds` existed is not malformed, it is a
 * todo with no tags, exactly as the Rust `#[serde(default)]` reads it. Only an
 * entry no build could ever read comes back `null`.
 */
function readTodo(value: unknown): Todo | null {
  if (!isRecord(value)) return null;
  if (Object.keys(value).some((key) => !(key in TODO_FIELDS))) return null;

  const rules: Record<string, TodoFieldRule> = TODO_FIELDS;
  const normalised: Record<string, unknown> = { ...value };
  for (const [key, rule] of Object.entries(rules)) {
    if (key in value) {
      if (!rule.accepts(value[key])) return null;
      continue;
    }
    if (rule.required) return null;
    if (rule.fallback) normalised[key] = rule.fallback();
  }

  // Every key was checked against the rule the contract generated, so this is
  // the one place the shape is asserted rather than proved.
  return normalised as Todo;
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
      const todo = readTodo(entry);
      if (todo) upserts.push(todo);
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

/** Parks entries that are not todos, so the journal can drain past them. */
function quarantine(entries: readonly unknown[]): boolean {
  const parked = readJson(QUARANTINE_KEY);
  const kept = Array.isArray(parked) ? parked : [];
  const at = new Date().toISOString();
  const written = writeJson(QUARANTINE_KEY, [...kept, ...entries.map((entry) => ({ at, entry }))]);
  writeDiagnostic(written ? "warn" : "error", "Parked local todo entries that are not readable", {
    count: entries.length,
    key: QUARANTINE_KEY,
    parked: written,
  });
  return written;
}

/**
 * Moves unreadable entries to the quarantine key and answers with the ones the
 * journal must keep carrying.
 *
 * Every rewrite of the journal goes through this, which is what closes the hole
 * where an entry the reader could not parse simply vanished on the next write —
 * on the browser runtime, where `todos.items` is the store rather than a
 * journal, that was the user's data. If the quarantine write fails the entries
 * come back and stay in the journal: unreadable is not a reason to delete.
 */
function sequester(malformed: readonly unknown[]): unknown[] {
  if (malformed.length === 0) return [];
  return quarantine(malformed) ? [] : [...malformed];
}

function writePendingTodos(entries: readonly unknown[]): boolean {
  return entries.length === 0
    ? removeKey(PENDING_TODOS_KEY)
    : writeJson(PENDING_TODOS_KEY, entries);
}

function writePendingDeletions(ids: readonly string[]): boolean {
  return ids.length === 0
    ? removeKey(PENDING_DELETIONS_KEY)
    : writeJson(PENDING_DELETIONS_KEY, ids);
}

/** Records a todo SQLite refused; it supersedes any pending deletion of that id. */
function recordPendingUpsert(todo: Todo): boolean {
  const { upserts, deletions, malformed } = readPendingWrites();

  // Drop the tombstone first and only continue if it is provably gone: a
  // journal holding both an upsert and a deletion for one id replays as "write
  // it, then delete it" (`replay_pending` applies upserts before deletions), so
  // recording the write on top of a surviving tombstone would delete the todo
  // the user just edited.
  if (deletions.includes(todo.id)) {
    if (!writePendingDeletions(deletions.filter((id) => id !== todo.id))) {
      writeDiagnostic("error", "Unable to drop the pending deletion of an edited todo", {
        id: todo.id,
      });
      return false;
    }
  }

  const index = upserts.findIndex((item) => item.id === todo.id);
  if (index === -1) upserts.unshift(todo);
  else upserts[index] = todo;

  return writePendingTodos([...upserts, ...sequester(malformed)]);
}

/**
 * Records a deletion SQLite refused. Without this tombstone the row is still in
 * the database, so the next read — and the next replay — would bring the todo
 * the user deleted straight back.
 */
function recordPendingDeletion(id: string): boolean {
  const { upserts, deletions, malformed } = readPendingWrites();

  if (!deletions.includes(id) && !writePendingDeletions([...deletions, id])) return false;

  // The tombstone is what preserves the deletion. A stale upsert left behind
  // replays as "write, then delete", which still ends deleted, so failing here
  // is worth saying out loud but does not undo the removal.
  const remaining = upserts.filter((todo) => todo.id !== id);
  if (remaining.length !== upserts.length || malformed.length > 0) {
    if (!writePendingTodos([...remaining, ...sequester(malformed)])) {
      writeDiagnostic("error", "Unable to drop the pending write of a deleted todo", { id });
    }
  }
  return true;
}

/**
 * Drops every pending write for an id. Called whenever a native write succeeds:
 * the stored row is the newest copy from that moment on, so anything still
 * parked would be a stale write waiting to overwrite it.
 */
function clearPendingWrites(id: string): boolean {
  const { upserts, deletions, malformed } = readPendingWrites();
  let cleared = true;

  const remaining = upserts.filter((todo) => todo.id !== id);
  if (remaining.length !== upserts.length || malformed.length > 0) {
    cleared = writePendingTodos([...remaining, ...sequester(malformed)]) && cleared;
  }
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
  // `localStorage` is the store here, so a write that landed owes nothing.
  outstanding: () => 0,
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
        // how a replay would overwrite a good row with a stale one, which is
        // why a failure here is reported rather than swallowed.
        if (clearPendingWrites(todo.id)) return "stored";
        writeDiagnostic("error", "Stored a todo but could not clear its journal entry", {
          id: todo.id,
        });
        return "degraded";
      }
      // A permanent refusal is the contract rejecting the todo, and every
      // future build rejects it the same way. Parking it in the journal would
      // promise a retry that can never land — the banner even says so — and
      // then drop it on the next start when the replay parks it. So it is not
      // journalled: the caller keeps it in view and reports the failure now.
      if (result.error.permanent) {
        writeDiagnostic("error", "SQLite will never accept this todo; reporting it as failed", {
          id: todo.id,
          error: result.error.error,
        });
        return "failed";
      }
      writeDiagnostic("warn", "Unable to save a todo to SQLite; using browser fallback", {
        error: result.error.error,
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
        if (clearPendingWrites(id)) return "stored";
        writeDiagnostic("error", "Deleted a todo but could not clear its journal entry", { id });
        return "degraded";
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
  outstanding: () => {
    const pending = readPendingWrites();
    return pending.upserts.length + pending.deletions.length;
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

/** What settling the journal actually accomplished, for a truthful report. */
interface SettleResult {
  /** How many entries reached the quarantine key (not merely how many it tried). */
  readonly quarantined: number;
}

/**
 * Settles the journal against what SQLite actually did.
 *
 * An entry leaves only for a reason the database gave: it was applied, or it
 * was refused in a way no retry can fix (`permanent`), in which case it is
 * parked rather than deleted. Everything else stays pending.
 *
 * The count it returns is what `sequester` truly parked — entries it could not
 * park come back and stay in the journal, so reporting the attempt rather than
 * the result would overstate how much was quarantined and understate the
 * backlog. Callers derive the remaining backlog from `outstanding()`, which
 * reads the journal these writes just rewrote.
 */
function settle(applied: readonly string[], refused: readonly string[]): SettleResult {
  const done = new Set(applied);
  const parked = new Set(refused);
  const { upserts, deletions, malformed } = readPendingWrites();
  const unstorable = upserts.filter((todo) => parked.has(todo.id));

  const toPark = [...malformed, ...unstorable];
  const retained = sequester(toPark);
  const quarantined = toPark.length - retained.length;

  const wroteTodos = writePendingTodos([
    ...upserts.filter((todo) => !done.has(todo.id) && !parked.has(todo.id)),
    ...retained,
  ]);
  // A deletion has no contract to violate, so only the applied ones go.
  const wroteDeletions = writePendingDeletions(deletions.filter((id) => !done.has(id)));

  // Both writes were swallowed before, which hid two different problems: entries
  // SQLite had already applied stayed in the journal (so the backlog the user is
  // shown disagreed with the one the next write recomputes), and an entry moved
  // to the quarantine key stayed in the journal as well, leaving two copies and
  // no record of it. `outstanding()` reports the first; this reports the second.
  if (!wroteTodos || !wroteDeletions) {
    writeDiagnostic("error", "Settled the replay but could not rewrite the journal", {
      applied: applied.length,
      quarantined,
      todos: wroteTodos,
      deletions: wroteDeletions,
    });
  }

  return { quarantined };
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
 * still retried on the next start. Entries that are not todos at all, and
 * entries the contract will refuse however often they are retried, move to the
 * quarantine key instead of blocking every other pending write behind them.
 */
export async function flushPendingWrites(): Promise<PendingWriteFlush> {
  // In the browser `localStorage` is the store, not a journal to drain.
  if (!isTauri()) return { status: "skipped", reason: "browser-runtime" };

  const pending = readPendingWrites();
  const outstanding = pending.upserts.length + pending.deletions.length;
  if (outstanding === 0) {
    // Nothing to replay, but any unreadable entry still has to be parked before
    // a later write can overwrite it; `settle` does exactly that and reports
    // how many it actually managed to move.
    if (pending.malformed.length === 0) return { status: "skipped", reason: "nothing-pending" };
    const { quarantined } = settle([], []);
    return { status: "flushed", applied: 0, rejected: 0, quarantined };
  }

  let error: string;
  try {
    const result = await nativeCommands.replayPendingWrites(pending.upserts, pending.deletions);
    if (result.status === "ok") {
      const { applied, rejected } = result.data;
      const unstorable = rejected.filter((entry) => entry.permanent);
      const { quarantined } = settle(
        applied,
        unstorable.map((entry) => entry.id),
      );
      for (const entry of rejected) {
        writeDiagnostic(
          "warn",
          entry.permanent
            ? "SQLite will never accept a pending todo write; parking it"
            : "SQLite refused a pending todo write; keeping it for a retry",
          { id: entry.id, error: entry.error },
        );
      }
      // The transient rejections are the ones still owed; the permanent ones
      // were quarantined, so `quarantined` — what `settle` truly parked, not
      // what it was handed — is the honest figure for both the log and the
      // report.
      const transient = rejected.length - unstorable.length;
      writeDiagnostic("info", "Pending todo writes replayed into SQLite", {
        applied: applied.length,
        rejected: transient,
        quarantined,
      });
      return {
        status: "flushed",
        applied: applied.length,
        rejected: transient,
        quarantined,
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
