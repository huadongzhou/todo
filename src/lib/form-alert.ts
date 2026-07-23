import { ref, toRaw } from "vue";
import type { AlertLifetime, AlertTone, PageAlert } from "@/lib/page-alert";

/**
 * The single channel form results travel on.
 *
 * The page already owns exactly one `role="status"` region, and a second one
 * would interrupt the first, so a rejected submit cannot announce itself by
 * growing its own live region next to the field. It writes here instead, and
 * the page renders it through the region it already had.
 *
 * It is a module-level ref rather than a store because it is not domain state:
 * it is a sentence about the action the user just took, and nothing outside the
 * form has anything to say about it. Being a singleton is the point — one page,
 * one thing being said at a time.
 */
export const formAlert = ref<PageAlert | null>(null);

/**
 * Says one thing, replacing whatever the form was saying before.
 *
 * The lifetime is the caller's to state, because only the caller knows how the
 * line ends: a rejected submit goes on the next keystroke whatever it says
 * ("transient"), while "the title stops here" goes only when the title is short
 * again ("standing"). Getting that wrong is what let a standing amber line sit
 * on top of "some changes did not reach this machine" indefinitely.
 */
export function announceFormAlert(
  tone: AlertTone,
  message: string,
  lifetime: AlertLifetime,
): PageAlert {
  const alert: PageAlert = { tone, message, lifetime, source: "form" };
  formAlert.value = alert;
  return alert;
}

/** Drops the form's line, letting a longer-lived page alert show through. */
export function clearFormAlert(): void {
  formAlert.value = null;
}

/**
 * Takes back one particular line, and only while it is still the one being said.
 *
 * This channel is a singleton, so "the condition I reported is over" is not the
 * same statement as "the form has nothing to say": between the two moments
 * something else may have taken the line over. Whoever wants their own line back
 * hands in the alert `announceFormAlert` gave them, and identity decides — which
 * is what keeps a title dropping back under its limit from also silencing the
 * estimate's rejected submit.
 */
export function retractFormAlert(alert: PageAlert | null): void {
  // `toRaw`, because a ref hands back a reactive proxy of whatever was written
  // into it, and that proxy is never the object the announcer was handed.
  if (alert !== null && toRaw(formAlert.value) === alert) formAlert.value = null;
}
