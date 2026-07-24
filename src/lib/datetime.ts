/**
 * Conversions between the `datetime-local` input value and the stored instant.
 *
 * Storage keeps an ISO 8601 instant (UTC); the input speaks local wall-clock
 * time without a zone. The two functions here are each other's inverse and must
 * stay together: an editor that prefills with one and submits with the other
 * would otherwise shift the reminder by the timezone offset on every save.
 */

/** Converts a `datetime-local` input value to an ISO8601 instant (local tz). */
export function toInstant(value: string): string | null {
  if (!value) return null;
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? null : parsed.toISOString();
}

/**
 * Where this device sits, in seconds east of UTC — the `offset` a reminder point
 * stores beside its instant (提醒通知/07/08).
 *
 * `getTimezoneOffset()` counts the other way (minutes west), so it is negated and
 * scaled to seconds here, the same one-place conversion the calendar export makes.
 * Captured when a reminder is set so its wall clock can be recovered: the native
 * scheduler reads it to keep a floating reminder on the same wall clock as the
 * device travels, and ignores it when the "fixed time" switch is on.
 */
export function currentZoneOffsetSeconds(): number {
  return -new Date().getTimezoneOffset() * 60;
}

function pad(value: number): string {
  return String(value).padStart(2, "0");
}

/**
 * Converts a stored instant back to a `datetime-local` input value in local
 * time. Returns "" for an absent or unparsable instant, which is what an empty
 * input reads as — so a reminder the app cannot understand shows as "not set"
 * rather than silently keeping a value the user cannot see or correct.
 */
export function toLocalDateTimeInput(instant: string | null | undefined): string {
  if (!instant) return "";
  const parsed = new Date(instant);
  if (Number.isNaN(parsed.getTime())) return "";
  const date = `${parsed.getFullYear()}-${pad(parsed.getMonth() + 1)}-${pad(parsed.getDate())}`;
  return `${date}T${pad(parsed.getHours())}:${pad(parsed.getMinutes())}`;
}
