import { ref } from "vue";
import type { TodoPatch } from "@/bindings/models/TodoPatch";
import type { SyncOperationKind } from "@/bindings/models/SyncOperationKind";
import type { TodoSyncChange } from "@/bindings/models/TodoSyncChange";
import type { TodoSyncOperation } from "@/bindings/models/TodoSyncOperation";
import { DEFAULT_SYNC_SERVER_URL, SyncTransportError, syncWithServer } from "@/lib/sync-transport";
import { loadStringPreference, saveStringPreference } from "@/lib/settings-storage";
import { writeDiagnostic } from "@/lib/diagnostics";
import { useTodoStore } from "@/stores/todos";

/**
 * Sync engine (platform layer).
 *
 * Owns the client side of the op-based sync protocol against the Axum server:
 *   - records every local todo mutation as a `TodoSyncOperation`
 *   - on `sync()` pushes unacknowledged ops + pulls remote `TodoSyncChange`s
 *   - applies remote changes into the todos store (server-wins, field-level)
 *
 * State (device id, cursor, pending ops, server URL) is persisted through
 * `settings-storage` so unacknowledged operations survive a restart. The todos
 * store itself stays in-memory; the server is the source of truth and the
 * client rebuilds its state from it on startup.
 */

const DEVICE_ID_KEY = "sync.deviceId";
const CURSOR_KEY = "sync.cursor";
const PENDING_KEY = "sync.pending";
const SERVER_URL_KEY = "sync.serverUrl";

type SyncStatus = "idle" | "syncing" | "error";

let device_id = "";
let cursor = 0;
let pending: TodoSyncOperation[] = [];
let serverUrl = DEFAULT_SYNC_SERVER_URL;
let initialized = false;

/** Debounce timer for auto-sync after local changes. */
let autoSyncTimer: ReturnType<typeof setTimeout> | undefined;

export const syncStatus = ref<SyncStatus>("idle");
export const lastError = ref<string | null>(null);
export const lastSyncedAt = ref<string | null>(null);
export const deviceId = ref("");
export const serverUrlRef = ref(DEFAULT_SYNC_SERVER_URL);

/** Builds the wire-format operation describing a local upsert. */
export function buildUpsertOperation(
  todoId: string,
  patch: TodoPatch,
  occurredAt: string,
  operationId: string = crypto.randomUUID(),
): TodoSyncOperation {
  return {
    operationId,
    todoId,
    kind: "upsert" as SyncOperationKind,
    occurredAt,
    patch,
  };
}

/** Builds the wire-format operation describing a local delete. */
export function buildDeleteOperation(
  todoId: string,
  occurredAt: string,
  operationId: string = crypto.randomUUID(),
): TodoSyncOperation {
  return {
    operationId,
    todoId,
    kind: "delete" as SyncOperationKind,
    occurredAt,
    patch: undefined,
  };
}

/** Restores persisted state and generates a device id on first run. */
export async function initSyncEngine(): Promise<void> {
  if (initialized) return;

  device_id = await loadStringPreference(DEVICE_ID_KEY, "");
  if (!device_id) {
    device_id = crypto.randomUUID();
    await saveStringPreference(DEVICE_ID_KEY, device_id);
    writeDiagnostic("info", "Generated sync device id", { deviceId: device_id });
  }

  const cursorValue = await loadStringPreference(CURSOR_KEY, "0");
  cursor = Number.isFinite(Number(cursorValue)) ? Number(cursorValue) : 0;

  serverUrl = await loadStringPreference(SERVER_URL_KEY, DEFAULT_SYNC_SERVER_URL);

  try {
    const pendingJson = await loadStringPreference(PENDING_KEY, "[]");
    const parsed = JSON.parse(pendingJson) as unknown;
    if (Array.isArray(parsed)) pending = parsed as TodoSyncOperation[];
  } catch {
    pending = [];
  }

  deviceId.value = device_id;
  serverUrlRef.value = serverUrl;
  initialized = true;
}

async function persistPending(): Promise<void> {
  try {
    await saveStringPreference(PENDING_KEY, JSON.stringify(pending));
  } catch {
    /* best effort */
  }
}

async function persistCursor(): Promise<void> {
  try {
    await saveStringPreference(CURSOR_KEY, String(cursor));
  } catch {
    /* best effort */
  }
}

/**
 * Records a local mutation so it is pushed on the next sync. Called by the
 * todos store after each successful mutation.
 */
export async function recordOperation(operation: TodoSyncOperation): Promise<void> {
  pending.push(operation);
  await persistPending();
  scheduleAutoSync();
}

function scheduleAutoSync(): void {
  if (autoSyncTimer) clearTimeout(autoSyncTimer);
  autoSyncTimer = setTimeout(() => {
    void syncNow();
  }, 1000);
}

/** Updates the configured sync server URL and persists it. */
export async function setSyncServerUrl(url: string): Promise<void> {
  serverUrl = url.trim() || DEFAULT_SYNC_SERVER_URL;
  serverUrlRef.value = serverUrl;
  await saveStringPreference(SERVER_URL_KEY, serverUrl);
}

/**
 * Applies a single remote change into the todos store. Server-wins at the
 * field level; never records a new operation (so it never loops).
 */
export function applyRemoteChange(change: TodoSyncChange): void {
  const store = useTodoStore();
  if (change.kind === "upsert") {
    store.applyRemoteUpsert(change.todoId, change.patch, change.occurredAt);
  } else {
    store.applyRemoteDelete(change.todoId);
  }
}

/**
 * Runs one sync cycle: push pending ops, pull remote changes, apply them in
 * revision order, advance the cursor, and clear acknowledged ops. Safe to call
 * when there is nothing to push — it still pulls, which rebuilds state on
 * startup.
 */
export async function syncNow(): Promise<void> {
  if (syncStatus.value === "syncing") return;
  if (!initialized) await initSyncEngine();

  syncStatus.value = "syncing";
  lastError.value = null;

  try {
    const response = await syncWithServer(serverUrl, {
      deviceId: device_id,
      cursor,
      operations: pending,
    });

    const acknowledged = new Set(response.acknowledgedOperationIds);
    pending = pending.filter((operation) => !acknowledged.has(operation.operationId));
    await persistPending();

    // Apply in revision order (the server returns them sorted, but be explicit).
    const changes = [...response.changes].sort((a, b) => a.revision - b.revision);
    for (const change of changes) {
      applyRemoteChange(change);
    }

    cursor = response.nextCursor;
    await persistCursor();

    lastSyncedAt.value = new Date().toISOString();
    syncStatus.value = "idle";
    writeDiagnostic("info", "Sync completed", {
      acknowledged: acknowledged.size,
      applied: changes.length,
      cursor,
      pending: pending.length,
    });
  } catch (error) {
    const message = error instanceof SyncTransportError ? error.message : String(error);
    lastError.value = message;
    syncStatus.value = "error";
    writeDiagnostic("warn", "Sync failed", { error: message });
  }
}
