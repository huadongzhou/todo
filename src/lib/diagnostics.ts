import { isTauri } from "@tauri-apps/api/core";
import { debug, error, info, trace, warn } from "@tauri-apps/plugin-log";
import { readonly, ref } from "vue";

export type DiagnosticLevel = "trace" | "debug" | "info" | "warn" | "error";
type DiagnosticValue = string | number | boolean | undefined;

export interface DiagnosticEntry {
  readonly at: string;
  readonly level: DiagnosticLevel;
  readonly message: string;
  readonly context?: Readonly<Record<string, DiagnosticValue>>;
}

const MAX_ENTRIES = 100;
const entries = ref<DiagnosticEntry[]>([]);
const nativeWriters = { trace, debug, info, warn, error };

function formatMessage(
  message: string,
  context?: Readonly<Record<string, DiagnosticValue>>,
): string {
  if (!context) return message;

  const values = Object.entries(context).filter(([, value]) => value !== undefined);
  return values.length ? `${message} ${JSON.stringify(Object.fromEntries(values))}` : message;
}

export function writeDiagnostic(
  level: DiagnosticLevel,
  message: string,
  context?: Readonly<Record<string, DiagnosticValue>>,
): void {
  const entry: DiagnosticEntry = { at: new Date().toISOString(), level, message, context };
  entries.value = [entry, ...entries.value].slice(0, MAX_ENTRIES);

  const formatted = formatMessage(message, context);
  console[level === "trace" ? "debug" : level](formatted);

  if (isTauri()) {
    void nativeWriters[level](formatted).catch(() => undefined);
  }
}

export function useDiagnosticEntries() {
  return readonly(entries);
}
