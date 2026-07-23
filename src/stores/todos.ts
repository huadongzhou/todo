import { computed, ref, toRaw } from "vue";
import { defineStore } from "pinia";
import type { Todo } from "@/types/todo";
import type { TodoPatch } from "@/bindings/models/TodoPatch";
import type { PageAlert } from "@/lib/page-alert";
import { normalizeNotes, normalizeTitle } from "@/lib/todo-normalize";
import { cancelAllReminders, cancelReminder, scheduleReminder } from "@/lib/notifications";
import { writeDiagnostic } from "@/lib/diagnostics";
import { buildDeleteOperation, buildUpsertOperation, recordOperation } from "@/lib/sync-engine";
import type { PendingWriteFlush, WriteOutcome } from "@/lib/todo-repository";
import { todoRepository } from "@/lib/todo-repository";

/**
 * The fields a caller may set while creating a todo. Every one of them travels
 * on to storage and to the outbound sync operation; a field added here and
 * forgotten in either place is a field the user can fill in and then lose.
 */
export interface NewTodoInput {
  readonly title: string;
  readonly dueDate?: string | null;
  readonly reminderAt?: string | null;
  readonly notes?: string | null;
  readonly startDate?: string | null;
  readonly startsAt?: string | null;
  readonly endsAt?: string | null;
  readonly estimatedMinutes?: number | null;
}

/** The fields an edit may change. */
export type TodoEdit = Partial<
  Pick<
    Todo,
    | "title"
    | "dueDate"
    | "reminderAt"
    | "notes"
    | "startDate"
    | "startsAt"
    | "endsAt"
    | "estimatedMinutes"
  >
>;

const newId = () => crypto.randomUUID();

/** The fields one recorded change writes, in the shape the wire speaks. */
type FieldChange = Pick<
  TodoPatch,
  | "title"
  | "status"
  | "completedAt"
  | "dueDate"
  | "reminderAt"
  | "notes"
  | "startDate"
  | "startsAt"
  | "endsAt"
  | "estimatedMinutes"
>;

/**
 * One thing the user did, kept as data rather than as a pair of closures so it
 * can be inverted, replayed and read.
 *
 * `change` covers completing and editing alike — both are "these fields held
 * that, and now hold this" — which is what lets one apply path serve the two of
 * them and their undos. A deletion cannot be said that way (there is no todo
 * left to hold anything), so it carries the whole row and the place it sat.
 */
type HistoryEntry =
  | {
      readonly kind: "change";
      readonly id: string;
      readonly before: FieldChange;
      readonly after: FieldChange;
    }
  | { readonly kind: "delete"; readonly todo: Todo; readonly index: number }
  | { readonly kind: "insert"; readonly todo: Todo; readonly index: number };

/**
 * How many actions can be taken back.
 *
 * Deep enough to cover a burst of mistakes — a handful of stray completions, a
 * wrong delete, the edits around them — and shallow enough that the far end of
 * the stack is still something the user would recognise, and that the snapshots
 * a deletion keeps stay bounded (a todo carries up to 20 000 characters of
 * notes). The stack is session-only and never persisted: an undo entry is the
 * inverse of an action the user remembers taking, and nobody remembers across a
 * restart — meanwhile the other devices have gone on changing the same todos.
 */
const UNDO_DEPTH_LIMIT = 50;

/** Undoing a change means writing back what the fields held before it. */
function inverse(entry: HistoryEntry): HistoryEntry {
  switch (entry.kind) {
    case "change":
      return { kind: "change", id: entry.id, before: entry.after, after: entry.before };
    case "delete":
      return { kind: "insert", todo: entry.todo, index: entry.index };
    case "insert":
      return { kind: "delete", todo: entry.todo, index: entry.index };
  }
}

/** Every field of a todo, as the patch that would recreate it elsewhere. */
function patchFromTodo(todo: Todo): TodoPatch {
  return {
    title: todo.title,
    status: todo.status,
    completedAt: todo.completedAt,
    dueDate: todo.dueDate ?? null,
    reminderAt: todo.reminderAt ?? null,
    notes: todo.notes ?? null,
    startDate: todo.startDate ?? null,
    startsAt: todo.startsAt ?? null,
    endsAt: todo.endsAt ?? null,
    estimatedMinutes: todo.estimatedMinutes ?? null,
    recurrence: todo.recurrence ?? null,
    listId: todo.listId ?? null,
    important: todo.important ?? null,
    urgent: todo.urgent ?? null,
    sortOrder: todo.sortOrder ?? null,
    tagIds: todo.tagIds,
    subtasks: todo.subtasks,
    attachments: todo.attachments,
    dependsOn: todo.dependsOn,
  };
}

/** What an edit would actually write, and what those fields held before it. */
interface PlannedEdit {
  readonly before: FieldChange;
  readonly after: FieldChange;
}

/**
 * Notes one field of an edit — but only if the value would really change.
 *
 * The current value is handed in already read as `value ?? null` so a field that
 * was never set compares equal to one the user left empty; the two are the same
 * state, and telling them apart here would turn opening and saving an editor
 * into an edit.
 */
function planField<K extends keyof FieldChange>(
  plan: PlannedEdit,
  key: K,
  current: FieldChange[K],
  next: FieldChange[K],
): void {
  if (next === current) return;
  plan.after[key] = next;
  // Never `undefined`: that travels as "the patch does not mention this field"
  // and would leave the other devices on the value the undo just took back here.
  plan.before[key] = current;
}

export const useTodoStore = defineStore("todos", () => {
  const items = ref<Todo[]>([]);
  const activeItems = computed(() => items.value.filter((todo) => todo.status === "open"));
  const completedItems = computed(() => items.value.filter((todo) => todo.status === "completed"));
  const repository = todoRepository();

  // Where the writes this session issued actually landed. The repository can
  // only report one write at a time; keeping the ids here is what turns those
  // reports into a state the UI can show and the user can act on.
  const degradedIds = ref<ReadonlySet<string>>(new Set());
  const failedIds = ref<ReadonlySet<string>>(new Set());
  /** Whether the journal still holds writes the database has not taken. */
  const startupBacklog = ref(false);

  /**
   * What the user did, newest last, and what they have taken back.
   *
   * Plain arrays rather than refs: nothing renders them, and a reactive stack
   * would only wrap the snapshots a deletion keeps in proxies of a todo that is
   * no longer in the list.
   */
  const undoStack: HistoryEntry[] = [];
  const redoStack: HistoryEntry[] = [];

  /**
   * Both lines are `standing`: nothing but the condition itself clears them,
   * which is exactly what decides whether the page may let another line cover
   * them (see `pickPageAlert`). Stated here rather than guessed at by the page.
   */
  const storageAlert = computed<PageAlert | null>(() => {
    if (failedIds.value.size > 0) {
      // "failed" now covers two causes — storage would not take the write, or
      // the contract refuses this todo for good — so the wording names both
      // instead of asserting the disk is full.
      return {
        tone: "error",
        message: "有改动没能保存到本机，请检查内容或存储空间后重试。",
        lifetime: "standing",
        source: "storage",
      };
    }
    if (degradedIds.value.size > 0 || startupBacklog.value) {
      return {
        tone: "warn",
        message: "有改动暂存在本地缓存，尚未写入数据库，重启后会自动补写。",
        lifetime: "standing",
        source: "storage",
      };
    }
    return null;
  });

  function withoutId(ids: ReadonlySet<string>, id: string): Set<string> {
    const next = new Set(ids);
    next.delete(id);
    return next;
  }

  /** Consumes one write outcome: the tri-state decides both state and log level. */
  function noteWriteOutcome(id: string, outcome: WriteOutcome): void {
    const degraded = withoutId(degradedIds.value, id);
    const failed = withoutId(failedIds.value, id);
    if (outcome === "degraded") degraded.add(id);
    if (outcome === "failed") failed.add(id);
    degradedIds.value = degraded;
    failedIds.value = failed;

    // The backlog is whatever storage still owes *now*, not what it owed at
    // startup: once the journal is empty the warning has nothing left to warn
    // about, and saying "it will be written on the next restart" after it was
    // written is worse than saying nothing.
    startupBacklog.value = repository.outstanding() > 0;

    if (outcome === "stored") return;
    writeDiagnostic(outcome === "failed" ? "error" : "warn", "A todo write did not reach storage", {
      id,
      outcome,
    });
  }

  /** Records what the startup replay left behind, so the UI can say so. */
  function notePendingWrites(flush: PendingWriteFlush): void {
    // Read from the journal, the same source `noteWriteOutcome` uses, so the
    // two can never disagree. Deriving it from the flush summary let them: a
    // replay that applied everything but could not rewrite the journal still
    // owes those writes, which `rejected` does not count — the banner stayed
    // hidden at startup and then appeared out of nowhere on the next unrelated
    // write. A replay that failed outright is kept as a floor, because a
    // storage that cannot be replayed may also be unreadable to `outstanding`.
    startupBacklog.value = repository.outstanding() > 0 || flush.status === "failed";
  }

  /** Writes a todo back to storage; failures degrade inside the repository. */
  function persist(todo: Todo): void {
    void repository.save(todo).then((outcome) => noteWriteOutcome(todo.id, outcome));
  }

  /** Removes a todo from storage, recording where the removal landed. */
  function forget(id: string): void {
    void repository.remove(id).then((outcome) => noteWriteOutcome(id, outcome));
  }

  /** Loads persisted todos into memory. Called once on app startup. */
  async function hydrate(): Promise<void> {
    items.value = await repository.load();
    writeDiagnostic("info", "Todos restored from local storage", { count: items.value.length });
  }

  function add(input: string | NewTodoInput) {
    const normalized = normalizeTitle(typeof input === "string" ? input : input.title);
    if (!normalized) return false;

    const fields: Omit<NewTodoInput, "title"> = typeof input === "string" ? {} : input;
    const todo: Todo = {
      id: newId(),
      title: normalized,
      status: "open",
      createdAt: new Date().toISOString(),
      completedAt: null,
      dueDate: fields.dueDate ?? null,
      reminderAt: fields.reminderAt ?? null,
      notes: normalizeNotes(fields.notes),
      startDate: fields.startDate ?? null,
      startsAt: fields.startsAt ?? null,
      endsAt: fields.endsAt ?? null,
      estimatedMinutes: fields.estimatedMinutes ?? null,
      // The list-valued fields have one empty value rather than two (`[]` and
      // "not set"), so a new todo starts with the empty one; the optional
      // scalars stay absent until something sets them.
      tagIds: [],
      subtasks: [],
      attachments: [],
      dependsOn: [],
    };

    items.value.unshift(todo);
    persist(todo);
    void scheduleReminder(todo);
    void recordOperation(
      buildUpsertOperation(
        todo.id,
        // Every field the creation can carry travels with it. A field left out
        // here is one the user can set while adding a task and never see on
        // another device — the gap the reminder used to have on this path.
        {
          title: todo.title,
          status: todo.status,
          dueDate: todo.dueDate,
          reminderAt: todo.reminderAt,
          completedAt: todo.completedAt,
          notes: todo.notes,
          startDate: todo.startDate,
          startsAt: todo.startsAt,
          endsAt: todo.endsAt,
          estimatedMinutes: todo.estimatedMinutes,
        },
        todo.createdAt,
      ),
    );
    return true;
  }

  /**
   * A todo's pending reminder, brought in line with what it now says.
   *
   * One rule for every write: a completed todo holds no reminder. Completing one
   * used to cancel it while editing one re-armed it, so editing a task that was
   * already done put its reminder back on the clock.
   */
  function refreshReminder(todo: Todo): void {
    if (todo.status === "completed") {
      cancelReminder(todo.id);
      return;
    }
    void scheduleReminder(todo);
  }

  /**
   * Writes a set of fields onto one todo: memory, storage, reminder, sync.
   *
   * The outbound operation is built from what was actually written locally,
   * never from a caller's raw patch. Two reasons, both fatal:
   *   - the server validates a sync request as one batch, so a single raw title
   *     it refuses (blank, or longer than the contract allows) rejects every
   *     operation in the request, and a rejected operation is never
   *     acknowledged — it stays in the queue and re-poisons every later request,
   *     which stops this device syncing for good;
   *   - a title accepted in its raw form would still leave the other devices
   *     showing a different string than this one does.
   *
   * Undo comes back through here as well, and that is deliberate: taking a
   * change back is a new change forward, not the retraction of an operation that
   * has already been pushed. The log is append-only and has no "never mind".
   */
  function applyChange(id: string, change: FieldChange): boolean {
    const todo = items.value.find((item) => item.id === id);
    if (!todo) return false;

    Object.assign(todo, change);
    persist(todo);
    refreshReminder(todo);
    void recordOperation(buildUpsertOperation(id, change, new Date().toISOString()));
    return true;
  }

  /** Takes a todo out of the list, handing back what it was and where it sat. */
  function deleteTodo(id: string): { todo: Todo; index: number } | null {
    const index = items.value.findIndex((todo) => todo.id === id);
    const removed = items.value[index];
    if (!removed) return null;

    // A copy, taken raw and taken now: the live object is reactive and would go
    // on changing under a history entry meant to hold what was deleted.
    const snapshot: Todo = { ...toRaw(removed) };
    items.value.splice(index, 1);
    forget(id);
    cancelReminder(id);
    void recordOperation(buildDeleteOperation(id, new Date().toISOString()));
    return { todo: snapshot, index };
  }

  /** Puts a deleted todo back where it was, under the id it always had. */
  function insertTodo(todo: Todo, index: number): boolean {
    if (items.value.some((item) => item.id === todo.id)) return false;

    const restored: Todo = { ...todo };
    items.value.splice(Math.min(Math.max(index, 0), items.value.length), 0, restored);
    persist(restored);
    refreshReminder(restored);
    // Every field travels: on the other devices this todo is gone, so the
    // operation has to be able to build it again from nothing.
    void recordOperation(
      buildUpsertOperation(restored.id, patchFromTodo(restored), new Date().toISOString()),
    );
    return true;
  }

  /** Runs one entry forwards. `false` means it had nothing left to act on. */
  function applyEntry(entry: HistoryEntry): boolean {
    switch (entry.kind) {
      case "change":
        return applyChange(entry.id, entry.after);
      case "delete":
        return deleteTodo(entry.todo.id) !== null;
      case "insert":
        return insertTodo(entry.todo, entry.index);
    }
  }

  /**
   * Files one thing the user did.
   *
   * Only called once the change has actually landed, which is what makes every
   * entry in the stack correspond to a visible difference: press undo and
   * something changes, every time.
   */
  function recordHistory(entry: HistoryEntry): void {
    undoStack.push(entry);
    // A new action branches away from the state the redone future was written
    // against, so that future is no longer reachable.
    redoStack.length = 0;
    // Full means dropping the oldest, not refusing the newest: the action a user
    // reaches for is the one they just took, and a stack that stopped recording
    // would lose exactly that one.
    if (undoStack.length > UNDO_DEPTH_LIMIT) undoStack.shift();
  }

  /**
   * Takes back the last recorded action.
   *
   * Entries whose todo is gone — deleted on another device since — are dropped
   * and the one before it is tried instead: an entry that cannot change anything
   * would spend a keypress on nothing and read as a broken undo.
   */
  function undo(): boolean {
    while (undoStack.length > 0) {
      const entry = undoStack.pop();
      if (!entry) break;
      if (applyEntry(inverse(entry))) {
        redoStack.push(entry);
        return true;
      }
    }
    return false;
  }

  /** Does again what undo took back, under the same rule about stale entries. */
  function redo(): boolean {
    while (redoStack.length > 0) {
      const entry = redoStack.pop();
      if (!entry) break;
      if (applyEntry(entry)) {
        undoStack.push(entry);
        return true;
      }
    }
    return false;
  }

  function update(id: string, patch: TodoEdit) {
    const todo = items.value.find((item) => item.id === id);
    if (!todo) return;

    // `undefined` is "the edit did not touch this"; `null` is "the user emptied
    // it", and it travels as a literal `null` so the other devices empty it too.
    const plan: PlannedEdit = { before: {}, after: {} };

    if (patch.title !== undefined) {
      const normalized = normalizeTitle(patch.title);
      // An empty title is not an edit this store accepts: it keeps the old one.
      if (normalized) planField(plan, "title", todo.title, normalized);
    }
    if (patch.dueDate !== undefined) {
      planField(plan, "dueDate", todo.dueDate ?? null, patch.dueDate);
    }
    // The reminder travels too: it is an editable field, so leaving it out of
    // the operation means a reminder changed here never reaches another device.
    if (patch.reminderAt !== undefined) {
      planField(plan, "reminderAt", todo.reminderAt ?? null, patch.reminderAt);
    }
    if (patch.notes !== undefined) {
      planField(plan, "notes", todo.notes ?? null, normalizeNotes(patch.notes));
    }
    if (patch.startDate !== undefined) {
      planField(plan, "startDate", todo.startDate ?? null, patch.startDate);
    }
    if (patch.startsAt !== undefined) {
      planField(plan, "startsAt", todo.startsAt ?? null, patch.startsAt);
    }
    if (patch.endsAt !== undefined) {
      planField(plan, "endsAt", todo.endsAt ?? null, patch.endsAt);
    }
    if (patch.estimatedMinutes !== undefined) {
      planField(plan, "estimatedMinutes", todo.estimatedMinutes ?? null, patch.estimatedMinutes);
    }

    // An edit that writes the values already there is not an edit: writing it
    // anyway would queue an outbound operation carrying nothing and file an undo
    // entry that undoes nothing the user can see.
    if (Object.keys(plan.after).length === 0) return;
    if (!applyChange(id, plan.after)) return;
    recordHistory({ kind: "change", id, before: plan.before, after: plan.after });
  }

  function toggle(id: string) {
    const todo = items.value.find((item) => item.id === id);
    if (!todo) return;

    const completing = todo.status === "open";
    // The instant is kept on both sides, so undoing a completion puts back the
    // time it was originally completed at rather than inventing a new one.
    const before: FieldChange = { status: todo.status, completedAt: todo.completedAt };
    const after: FieldChange = {
      status: completing ? "completed" : "open",
      completedAt: completing ? new Date().toISOString() : null,
    };

    if (!applyChange(id, after)) return;
    recordHistory({ kind: "change", id, before, after });
  }

  function remove(id: string) {
    const removed = deleteTodo(id);
    if (!removed) return;
    recordHistory({ kind: "delete", todo: removed.todo, index: removed.index });
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
      // Field level, one field per line. `undefined` is "the remote change did
      // not touch this field", so the local value stands; `null` is "the user
      // emptied it over there", and it is applied like any other value — which
      // is why the test is `!== undefined` and never a truthiness check. The
      // contract now carries those two apart on the wire, so this reading is
      // finally the whole truth rather than half of it. Every field the
      // contract has is listed: one left out would silently stop syncing.
      if (patch?.title !== undefined) existing.title = patch.title;
      if (patch?.status !== undefined) existing.status = patch.status;
      if (patch?.dueDate !== undefined) existing.dueDate = patch.dueDate;
      if (patch?.completedAt !== undefined) existing.completedAt = patch.completedAt;
      if (patch?.reminderAt !== undefined) existing.reminderAt = patch.reminderAt;
      if (patch?.notes !== undefined) existing.notes = patch.notes;
      if (patch?.startDate !== undefined) existing.startDate = patch.startDate;
      if (patch?.startsAt !== undefined) existing.startsAt = patch.startsAt;
      if (patch?.endsAt !== undefined) existing.endsAt = patch.endsAt;
      if (patch?.estimatedMinutes !== undefined) existing.estimatedMinutes = patch.estimatedMinutes;
      if (patch?.recurrence !== undefined) existing.recurrence = patch.recurrence;
      if (patch?.listId !== undefined) existing.listId = patch.listId;
      if (patch?.important !== undefined) existing.important = patch.important;
      if (patch?.urgent !== undefined) existing.urgent = patch.urgent;
      if (patch?.sortOrder !== undefined) existing.sortOrder = patch.sortOrder;
      if (patch?.tagIds !== undefined) existing.tagIds = patch.tagIds;
      if (patch?.subtasks !== undefined) existing.subtasks = patch.subtasks;
      if (patch?.attachments !== undefined) existing.attachments = patch.attachments;
      if (patch?.dependsOn !== undefined) existing.dependsOn = patch.dependsOn;
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
        reminderAt: patch.reminderAt ?? null,
        notes: patch.notes ?? null,
        startDate: patch.startDate ?? null,
        startsAt: patch.startsAt ?? null,
        endsAt: patch.endsAt ?? null,
        estimatedMinutes: patch.estimatedMinutes ?? null,
        recurrence: patch.recurrence ?? null,
        listId: patch.listId ?? null,
        important: patch.important ?? null,
        urgent: patch.urgent ?? null,
        sortOrder: patch.sortOrder ?? null,
        tagIds: patch.tagIds ?? [],
        subtasks: patch.subtasks ?? [],
        attachments: patch.attachments ?? [],
        dependsOn: patch.dependsOn ?? [],
      };
      items.value.unshift(created);
      persist(created);
    }
  }

  /** Applies a remote delete change. Does NOT record a sync operation. */
  function applyRemoteDelete(todoId: string): void {
    items.value = items.value.filter((todo) => todo.id !== todoId);
    forget(todoId);
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
    storageAlert,
    notePendingWrites,
    hydrate,
    add,
    update,
    toggle,
    remove,
    undo,
    redo,
    applyRemoteUpsert,
    applyRemoteDelete,
    rescheduleAll,
  };
});
