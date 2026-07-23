import { MAX_NOTES_CHARS, MAX_TITLE_CHARS } from "@/lib/native";

/**
 * What a todo's free text becomes on its way into storage.
 *
 * These live here rather than in the store because two places need the same
 * answer: the store, which writes the value, and the editor, which has to decide
 * whether the user changed anything at all. Asking that question of the typed
 * value instead of the stored one makes a title with a trailing space look like
 * an edit — one that writes the identical string, queues an outbound sync
 * operation with nothing in it, and leaves an undo entry that undoes nothing the
 * user can see. One definition, so the two can never drift apart.
 */

/**
 * Trims a title and caps it at the contract's limit, counted in code points to
 * match the Rust `chars().count()` check. The draft input caps at the same
 * number, so on the typed path this only trims whitespace; the cap here guards
 * the paths that build a title without going through that input (quick add,
 * future callers) so none of them can create a todo the database would refuse
 * and then lose on the next restart.
 */
export function normalizeTitle(raw: string): string {
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
export function normalizeNotes(raw: string | null | undefined): string | null {
  if (raw === null || raw === undefined) return null;
  const points = Array.from(raw);
  return points.length > MAX_NOTES_CHARS ? points.slice(0, MAX_NOTES_CHARS).join("") : raw;
}
