/**
 * When the day-start rules run.
 *
 * Three moments, and no polling: a timer that wakes once a minute to wait for
 * something that happens once a day is a cost paid 1439 times for nothing.
 *
 *   - **now**, on registration — the app was closed when the last few midnights
 *     went past, and that is the common case;
 *   - **the next local midnight**, through a single timer that re-arms itself
 *     afterwards, for a window left open across the night;
 *   - **coming back to the foreground**, because a timer is not kept while the
 *     system is asleep or the app is suspended, which on mobile is the normal
 *     way a day passes.
 *
 * Shaped like `registerUndoShortcut`: hand it what to do, get back the function
 * that stops it. The one listener it adds is on `document`, and it is removed
 * again by that function.
 */

/**
 * Milliseconds until just after the next local midnight.
 *
 * A few seconds past, not on the stroke: a timer that fires a moment early
 * would read the calendar on the old day and decide there was nothing to do,
 * and then not look again until the following midnight.
 */
function millisUntilNextLocalDay(from: Date): number {
  const next = new Date(from.getFullYear(), from.getMonth(), from.getDate() + 1, 0, 0, 5, 0);
  // A clock moved backwards can put "the next midnight" behind us; a second is
  // then the shortest honest wait, and the re-armed timer works it out again.
  return Math.max(next.getTime() - from.getTime(), 1_000);
}

/** Runs `handler` now, at every local midnight, and on return to the foreground. */
export function registerDayRollover(handler: () => void): () => void {
  let timer: ReturnType<typeof setTimeout> | undefined;

  function arm(): void {
    // Re-armed from the moment it actually fired rather than by adding 24
    // hours, so a late or early wake-up does not carry its error forward, and
    // days that are not 24 hours long (daylight saving) land correctly.
    timer = setTimeout(() => {
      handler();
      arm();
    }, millisUntilNextLocalDay(new Date()));
  }

  function onVisibilityChange(): void {
    if (document.visibilityState === "visible") handler();
  }

  handler();
  arm();
  document.addEventListener("visibilitychange", onVisibilityChange);

  return () => {
    if (timer !== undefined) clearTimeout(timer);
    document.removeEventListener("visibilitychange", onVisibilityChange);
  };
}
