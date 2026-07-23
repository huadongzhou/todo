import { ref } from "vue";

/** A form result the user should hear about, and how loud it is. */
export interface FormAlert {
  readonly tone: "warn" | "error";
  readonly message: string;
}

/**
 * The single channel form results travel on.
 *
 * The page already owns exactly one `role="status"` region, and a second one
 * would interrupt the first, so a rejected submit cannot announce itself by
 * growing its own live region next to the field. It writes here instead, and
 * the page renders it through the region it already had.
 *
 * It is a module-level ref rather than a store because it is not domain state:
 * it is a sentence about the action the user just took, it lives for one
 * keystroke, and nothing outside the form has anything to say about it. Being a
 * singleton is the point — one page, one thing being said at a time.
 */
export const formAlert = ref<FormAlert | null>(null);

/** Says one thing, replacing whatever the form was saying before. */
export function announceFormAlert(tone: FormAlert["tone"], message: string): void {
  formAlert.value = { tone, message };
}

/** Drops the form's line, letting a longer-lived page alert show through. */
export function clearFormAlert(): void {
  formAlert.value = null;
}
