import { commands } from "@/bindings/commands";

/** Typed entry point for native commands; do not call `invoke` with string names directly. */
export const nativeCommands = commands;

/**
 * The contract's text limits, generated from Rust alongside the commands. They
 * are re-exported here so the view layer caps its inputs at the same numbers the
 * database enforces, keeping the two from drifting into "the field accepts what
 * storage then refuses".
 */
export { MAX_NOTES_CHARS, MAX_TITLE_CHARS } from "@/bindings/commands";
