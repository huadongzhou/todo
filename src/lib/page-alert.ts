/**
 * The one thing the page says at a time, and the rule for which of the
 * candidates gets to say it.
 *
 * The page owns a single `role="status"` region (see `App.vue`), so four
 * sources — the form, storage, the export and the reminder bar — compete for
 * one line. Ranking them here rather than in the page is what keeps the page
 * from guessing at properties only the producer knows: how long a line lives is
 * a fact about where it came from, not about where it is shown.
 */

export type AlertTone = "success" | "warn" | "error";

/**
 * How a line ends.
 *
 * `transient`: it clears on something unrelated to what it says — the next
 * keystroke, closing a panel — so it always gets out of the way by itself.
 * `standing`: only the condition it describes can clear it, so it stays for as
 * long as the condition does.
 */
export type AlertLifetime = "transient" | "standing";

export type AlertSource = "form" | "storage" | "export" | "reminder";

export interface PageAlert {
  readonly tone: AlertTone;
  readonly message: string;
  readonly lifetime: AlertLifetime;
  readonly source: AlertSource;
}

const LIFETIME_ORDER: Record<AlertLifetime, number> = { transient: 0, standing: 1 };
const TONE_ORDER: Record<AlertTone, number> = { error: 0, warn: 1, success: 2 };
const SOURCE_ORDER: Record<AlertSource, number> = { form: 0, storage: 1, export: 2, reminder: 3 };

/**
 * The line to show, out of everything that has something to say right now.
 *
 * Sorted by lifetime, then severity, then source, and the first one wins — the
 * page never shows two at once, because one `aria-atomic` region reading out two
 * unrelated sentences is worse than showing the more urgent one alone.
 *
 * Lifetime comes before severity, which looks backwards until you follow what
 * happens next: a transient line is the direct answer to what the user just did
 * and has a clearing moment of its own, so once it goes the standing line comes
 * back by itself (this is a plain derivation — nothing has to remember it). Two
 * standing lines are the case that actually needed fixing, and there severity
 * decides: "some changes did not reach this machine" outranks "the title stops
 * here", which loses nothing at all.
 *
 * The constraint that keeps this honest, checked as each source is added: a line
 * that can be covered must either be covered only by something with a definite
 * end, or have a second way of being seen. The reminder bar (the fourth source)
 * is transient/success, so it can cover a standing storage line — allowed only
 * because it has a definite end: the bar clears it on the next action (skip,
 * complete, close), when its last reminder leaves (the due set empties), or when
 * it unmounts, after which the storing line returns.
 */
export function pickPageAlert(
  ...candidates: ReadonlyArray<PageAlert | null | undefined>
): PageAlert | null {
  let best: PageAlert | null = null;
  for (const candidate of candidates) {
    if (!candidate) continue;
    if (best === null || rank(candidate) < rank(best)) best = candidate;
  }
  return best;
}

function rank(alert: PageAlert): number {
  return (
    LIFETIME_ORDER[alert.lifetime] * 100 + TONE_ORDER[alert.tone] * 10 + SOURCE_ORDER[alert.source]
  );
}
