import { commands } from "@/bindings/commands";

/** Typed entry point for native commands; do not call `invoke` with string names directly. */
export const nativeCommands = commands;

/**
 * The contract's title limit, generated from Rust alongside the commands. It is
 * re-exported here so the view layer caps title inputs at the same number the
 * database enforces, keeping the two from drifting into "the field accepts what
 * storage then refuses".
 */
export { MAX_TITLE_CHARS } from "@/bindings/commands";
