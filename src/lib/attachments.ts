import { isTauri } from "@tauri-apps/api/core";
import { nativeCommands } from "@/lib/native";
import type { Attachment } from "@/types/todo";

/**
 * The contract's per-list ceiling (`MAX_LIST_ENTRIES`) and per-URL code-point
 * ceiling (`MAX_URL_CHARS`). Both are private to the Rust crate, so they are
 * mirrored here rather than imported; the numbers are the ones
 * `validate_attachments` enforces, so the add path can refuse an entry the
 * database would then reject and lose on restart — the same posture
 * `SubtaskList` keeps with its mirrored `MAX_SUBTASKS`.
 */
export const MAX_ATTACHMENTS = 200;
export const MAX_URL_CHARS = 2048;

/**
 * Schemes that are never "a link to open later" and are classic injection
 * vectors, refused at the interface layer before anything is stored or opened
 * (规格 d/2). Everything else — http/https, `mailto:`, app deep links — is a
 * legitimate link a user may mount, so this is a two-item block list rather than
 * an http/https allow list that would misfire on power users.
 */
const REJECTED_SCHEMES = new Set(["javascript", "data"]);

/**
 * The scheme of a URL (lowercased, no trailing colon) if it carries one, else
 * `null`. Delegates to the browser's own WHATWG URL parser rather than a regex
 * so scheme detection matches exactly how `window.open`/`<a href>` resolve the
 * string when it is opened: the parser trims leading C0-control/space and then
 * strips every embedded ASCII tab (U+0009), newline (U+000A) and carriage
 * return (U+000D) before reading the scheme. A raw-string regex misses those,
 * so `java\tscript:`, `java\nscript:`, `da\tta:` and a leading `\0` slip past it
 * while the browser still reconstructs and executes `javascript:`/`data:` (规格
 * d/2, defense in depth). A string with no parseable absolute scheme
 * (relative/schemeless links such as `example.com`) makes `new URL` throw; that
 * is not a rejected scheme, so it maps to `null` — the same "no scheme →
 * allowed" posture the regex had, and what `normalizeUrl` relies on to add the
 * `https://` prefix.
 */
function schemeOf(url: string): string | null {
  try {
    // `protocol` is always lowercased and ends with ":".
    return new URL(url).protocol.slice(0, -1);
  } catch {
    return null;
  }
}

/** Whether a URL uses a scheme the interface layer refuses to mount or open. */
export function isRejectedScheme(url: string): boolean {
  const scheme = schemeOf(url);
  return scheme !== null && REJECTED_SCHEMES.has(scheme);
}

/**
 * Prefixes a bare, host-like string with `https://` as a paste convenience — a
 * user commonly pastes `example.com` (规格 a/3, optional). Conservative: only
 * when there is no scheme, no whitespace, at least one dot, and it does not begin
 * with a slash (a path). Anything already carrying a scheme — including a
 * rejected one — is left untouched, so the scheme check still sees it.
 */
export function normalizeUrl(raw: string): string {
  const trimmed = raw.trim();
  if (!trimmed) return trimmed;
  if (schemeOf(trimmed) !== null) return trimmed;
  if (/\s/.test(trimmed) || trimmed.startsWith("/") || !trimmed.includes(".")) return trimmed;
  return `https://${trimmed}`;
}

/**
 * The last path segment, split on both separators — a Windows file dialog
 * returns backslash paths, so `/` alone would return the whole path. Falls back
 * to the path itself if the split leaves nothing (a trailing separator).
 */
export function fileBasename(path: string): string {
  const segments = path.split(/[/\\]/);
  const last = segments[segments.length - 1];
  return last && last.length > 0 ? last : path;
}

/**
 * Whether a code point can make a displayed label lie about what it opens:
 * bidirectional overrides/isolates (U+202A–202E, U+2066–2069) can flip
 * `evil-txt.exe` to read as `evil-exe.txt`, and C0/C1 control characters can
 * scramble a line. Tested by code point rather than a control-character regex so
 * the source carries no literal control bytes.
 */
function isDisplayUnsafeCodePoint(codePoint: number): boolean {
  return (
    (codePoint >= 0x202a && codePoint <= 0x202e) ||
    (codePoint >= 0x2066 && codePoint <= 0x2069) ||
    codePoint <= 0x1f ||
    (codePoint >= 0x7f && codePoint <= 0x9f)
  );
}

/**
 * Strips the display-unsafe code points from a label. Applied to what is *shown*
 * only — storage keeps the original value (规格 d/双向, low-cost display
 * hardening on top of Vue's own HTML escaping).
 */
function stripDisplayUnsafe(value: string): string {
  let out = "";
  for (const character of value) {
    const codePoint = character.codePointAt(0);
    if (codePoint === undefined || !isDisplayUnsafeCodePoint(codePoint)) out += character;
  }
  return out;
}

/**
 * The text shown for an attachment: its name if set, else the file's basename or
 * the link's URL (规格 c) — basename rather than the whole path so the file name
 * survives even when a long path is truncated. Bidi/control characters are
 * stripped so a crafted name cannot masquerade as another file type.
 */
export function attachmentLabel(attachment: Attachment): string {
  const name = attachment.name?.trim();
  if (name) return stripDisplayUnsafe(name);
  const fallback = attachment.kind === "file" ? fileBasename(attachment.url) : attachment.url;
  return stripDisplayUnsafe(fallback);
}

/** The word for an attachment kind, for accessible names (打开链接/打开文件). */
export function kindWord(kind: Attachment["kind"]): string {
  return kind === "file" ? "文件" : "链接";
}

/** What picking a file resolved to. */
export type FilePickResult =
  | { readonly kind: "picked"; readonly path: string }
  | { readonly kind: "cancelled" }
  | { readonly kind: "unsupported" };

/**
 * Opens the native file picker and hands back the chosen path. Outside the Tauri
 * desktop runtime (browser dev, mobile web) there is no dialog, so it resolves
 * `unsupported` for the caller to announce rather than crashing (规格 a/4). A
 * dialog that fails to open inside Tauri is treated as a cancel — a silent no-op
 * is less misleading than claiming the environment is unsupported.
 */
export async function pickAttachmentFile(): Promise<FilePickResult> {
  if (!isTauri()) return { kind: "unsupported" };
  try {
    const path = await nativeCommands.pickAttachmentFile();
    return path ? { kind: "picked", path } : { kind: "cancelled" };
  } catch {
    return { kind: "cancelled" };
  }
}

/** What opening an attachment resolved to. */
export type OpenResult =
  | { readonly kind: "opened" }
  | { readonly kind: "failed" }
  | { readonly kind: "unsupported-file" }
  | { readonly kind: "rejected-scheme" };

/**
 * Opens an attachment: a link in the system default browser, a file with the
 * system default program.
 *
 * Runtime degradation (规格 b):
 *   - Tauri desktop: both go through the native opener, so a link lands in the
 *     user's real default browser rather than the embedded webview.
 *   - Non-Tauri (browser dev / mobile web): a link falls back to `window.open`
 *     with `noopener,noreferrer` (a browser can open it, and the flags block
 *     opener/referrer leakage); a file cannot be opened from the browser sandbox,
 *     so it resolves `unsupported-file` for the caller to announce.
 *   - A file that has been moved or deleted comes back `failed` because the
 *     native opener reports it — never an uncaught throw (the acceptance case).
 *   - A `javascript:`/`data:` link comes back `rejected-scheme`: the add path
 *     refuses those schemes, but an attachment that reached the store another way
 *     (sync/import/a future shared list) never passed that guard, so the scheme is
 *     re-checked here — the one chokepoint every open funnels through — before a
 *     non-Tauri `window.open` could otherwise run it (规格 d/2, defense in depth).
 */
export async function openAttachment(attachment: Attachment): Promise<OpenResult> {
  if (attachment.kind === "link") {
    if (isRejectedScheme(attachment.url)) return { kind: "rejected-scheme" };
    if (!isTauri()) {
      window.open(attachment.url, "_blank", "noopener,noreferrer");
      return { kind: "opened" };
    }
    return runOpener(() => nativeCommands.openUrl(attachment.url));
  }
  if (!isTauri()) return { kind: "unsupported-file" };
  return runOpener(() => nativeCommands.openPath(attachment.url));
}

/**
 * Runs a native opener command and folds its result into `opened`/`failed`,
 * catching a thrown IPC error too so an open never escapes as an unhandled
 * rejection (规格 b: never throw to uncaught, never white-screen).
 */
async function runOpener(
  call: () => Promise<{ status: "ok"; data: null } | { status: "error"; error: string }>,
): Promise<OpenResult> {
  try {
    const result = await call();
    return result.status === "ok" ? { kind: "opened" } : { kind: "failed" };
  } catch {
    return { kind: "failed" };
  }
}
