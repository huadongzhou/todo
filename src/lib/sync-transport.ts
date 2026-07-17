import { isTauri } from "@tauri-apps/api/core";
import type { SyncRequest } from "@/bindings/models/SyncRequest";
import type { SyncResponse } from "@/bindings/models/SyncResponse";

/**
 * Sync transport platform layer.
 *
 * Talks to the Axum sync server (`GET /health`, `POST /v1/sync`) over the
 * WebView's standard `fetch`. The shared `todo-contracts` DTOs (exported to TS
 * via ts-rs) are the wire format — no hand-rolled request/response shapes.
 *
 * Outside the Tauri desktop runtime (browser dev, mobile) every function here
 * is a no-op so the app keeps working without a server to talk to.
 */

export const DEFAULT_SYNC_SERVER_URL = "http://127.0.0.1:3000";

/** Raised when the transport call fails (network error, non-2xx, bad JSON). */
export class SyncTransportError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "SyncTransportError";
  }
}

function trimBaseUrl(baseUrl: string): string {
  return baseUrl.trim().replace(/\/+$/, "");
}

/** Tells the caller whether the sync server is reachable. */
export async function checkHealth(baseUrl: string = DEFAULT_SYNC_SERVER_URL): Promise<boolean> {
  if (!isTauri()) return false;

  try {
    const response = await fetch(`${trimBaseUrl(baseUrl)}/health`, { method: "GET" });
    return response.ok;
  } catch {
    return false;
  }
}

/**
 * Pushes local operations and pulls remote changes from the sync server.
 * Throws `SyncTransportError` on any failure so the engine can keep the
 * pending queue and retry later.
 */
export async function syncWithServer(baseUrl: string, request: SyncRequest): Promise<SyncResponse> {
  if (!isTauri()) {
    throw new SyncTransportError("Sync transport is only available in the Tauri desktop runtime");
  }

  let response: Response;
  try {
    response = await fetch(`${trimBaseUrl(baseUrl)}/v1/sync`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request),
    });
  } catch (error) {
    throw new SyncTransportError(`Network error while contacting sync server: ${String(error)}`);
  }

  if (!response.ok) {
    let detail = `${response.status} ${response.statusText}`;
    try {
      const body = (await response.json()) as { error?: string };
      if (body?.error) detail = body.error;
    } catch {
      /* response had no JSON body; keep the status text */
    }
    throw new SyncTransportError(`Sync server returned an error: ${detail}`);
  }

  try {
    return (await response.json()) as SyncResponse;
  } catch (error) {
    throw new SyncTransportError(`Failed to parse sync server response: ${String(error)}`);
  }
}
