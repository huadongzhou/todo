import { computed, ref } from "vue";
import { defineStore } from "pinia";
import type { Todo } from "@/types/todo";
import type { TodoPatch } from "@/bindings/models/TodoPatch";
import type { PageAlert } from "@/lib/page-alert";
import { MAX_NOTES_CHARS, MAX_TITLE_CHARS } from "@/lib/native";
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

/**
 * Trims a title and caps it at the contract's limit, counted in code points to
 * match the Rust `chars().count()` check. The draft input caps at the same
 * number, so on the typed path this only trims whitespace; the cap here guards
 * the paths that build a title without going through that input (quick add,
 * future callers) so none of them can create a todo the database would refuse
 * and then lose on the next restart.
 */
function normalizeTitle(raw: string): string {
  const trimmed = raw.trim();
  // `Array.from` walks code points, which is exactly what `chars()` counts on
  // the Rust side; counting UTF-16 units instead would reject titles the
  // contract accepts.
  const points = Array.from(trimmed);
  return points.length > MAX_TITLE_CHARS ? points.slice(0, MAX_TITLE_CHARS).join("") : trimmed;
}

/**
 * Caps a note at the contract's limit, for the same reason the title is capped
 * and with more at stake: an over-long note is refused by the server for the
 * whole sync request it travels in, and a refused operation is never
 * acknowledged — so one of them stops this device syncing at all, not just
 * itself. The notes input caps at the same number, so on the typed path this
 * does nothing.
 */
function normalizeNotes(raw: string | null | undefined): string | null {
  if (raw === null || raw === undefined) return null;
  const points = Array.from(raw);
  return points.length > MAX_NOTES_CHARS ? points.slice(0, MAX_NOTES_CHARS).join("") : raw;
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

  function update(id: string, patch: TodoEdit) {
    const todo = items.value.find((item) => item.id === id);
    if (!todo) return;

    // The outbound operation is built from what was actually written locally,
    // never from the raw patch. Two reasons, both fatal:
    //   - the server validates a sync request as one batch, so a single raw
    //     title it refuses (blank, or longer than the contract allows) rejects
    //     every operation in the request, and a rejected operation is never
    //     acknowledged — it stays in the queue and re-poisons every later
    //     request, which stops this device syncing for good;
    //   - a title accepted in its raw form would still leave the other devices
    //     showing a different string than this one does.
    //
    // `undefined` is "the edit did not touch this"; `null` is "the user emptied
    // it", and it travels as a literal `null` so the other devices empty it too.
    const outbound: TodoPatch = {};

    if (patch.title !== undefined) {
      const normalized = normalizeTitle(patch.title);
      if (normalized) {
        todo.title = normalized;
        outbound.title = normalized;
      }
    }
    if (patch.dueDate !== undefined) {
      todo.dueDate = patch.dueDate;
      outbound.dueDate = patch.dueDate;
    }
    // The reminder travels too: it is an editable field, so leaving it out of
    // the operation means a reminder changed here never reaches another device.
    if (patch.reminderAt !== undefined) {
      todo.reminderAt = patch.reminderAt;
      outbound.reminderAt = patch.reminderAt;
    }
    if (patch.notes !== undefined) {
      todo.notes = normalizeNotes(patch.notes);
      outbound.notes = todo.notes;
    }
    if (patch.startDate !== undefined) {
      todo.startDate = patch.startDate;
      outbound.startDate = patch.startDate;
    }
    if (patch.startsAt !== undefined) {
      todo.startsAt = patch.startsAt;
      outbound.startsAt = patch.startsAt;
    }
    if (patch.endsAt !== undefined) {
      todo.endsAt = patch.endsAt;
      outbound.endsAt = patch.endsAt;
    }
    if (patch.estimatedMinutes !== undefined) {
      todo.estimatedMinutes = patch.estimatedMinutes;
      outbound.estimatedMinutes = patch.estimatedMinutes;
    }

    persist(todo);
    void scheduleReminder(todo);
    void recordOperation(buildUpsertOperation(todo.id, outbound, new Date().toISOString()));
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
    forget(id);
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
    applyRemoteUpsert,
    applyRemoteDelete,
    rescheduleAll,
  };
});
