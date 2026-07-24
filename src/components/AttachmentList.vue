<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import { Link as LinkIcon, Paperclip, Plus, Trash2 } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import {
  attachmentLabel,
  isRejectedScheme,
  kindWord,
  MAX_ATTACHMENTS,
  MAX_URL_CHARS,
  normalizeUrl,
  openAttachment,
  pickAttachmentFile,
} from "@/lib/attachments";
import { announceFormAlert, retractFormAlert } from "@/lib/form-alert";
import type { PageAlert } from "@/lib/page-alert";
import { useTodoStore } from "@/stores/todos";
import type { Attachment, Todo } from "@/types/todo";

/**
 * The files and links hanging off one todo, shown only in that todo's editor and
 * a sibling of `SubtaskList` rather than a field in `TodoFields`: anything inside
 * the field block would also appear in the create form, which attachments must
 * not (they belong to an existing task — mounting a file needs a row to hang it
 * on). Like the checklist it owns its whole region and commits every change
 * straight to the store, so it rides neither the editor's Enter-to-save nor its
 * Esc-to-cancel: an attachment is an immediate, discrete, individually-undoable
 * action.
 */
const props = defineProps<{ todo: Todo }>();
const todoStore = useTodoStore();

/** The counter appears only once this many code points are left. */
const URL_COUNTER_FROM = 40;

const attachments = computed(() => props.todo.attachments);

/** The link input's text, kept here so it can be cleared and kept focused after each add. */
const newUrl = ref("");
/**
 * Whether the link input is showing. The two triggers (`链接`/`文件`) are always
 * there; clicking `链接` reveals the URL input beside them and keeps it revealed
 * so several links can be typed in a run, mirroring `SubtaskList`'s add box.
 */
const linkRevealed = ref(false);
/**
 * Whether the last submit was refused for its scheme (`javascript:`/`data:`).
 * Shown inline beneath the input and cleared on the next keystroke, so the
 * message names the fix without growing a second live region (规格 d/2, e).
 */
const schemeRejected = ref(false);

const sectionRef = ref<HTMLElement | null>(null);
const urlInputRef = ref<HTMLInputElement | null>(null);

/**
 * The "at the limit" line the URL input last put up, so holding a key at the cap
 * does not stutter the page's one live region and dropping back under it takes
 * only its own line back — the single-input form of `SubtaskList`'s cap map.
 */
let urlCapAlert: PageAlert | null = null;

/** Code points, matching the contract's `chars().count()`. */
function countCodePoints(value: string): number {
  return Array.from(value).length;
}

/** Caps a string at the URL limit, counted in code points, and says whether it cut. */
function capCodePoints(raw: string): { capped: string; truncated: boolean } {
  const points = Array.from(raw);
  if (points.length <= MAX_URL_CHARS) return { capped: raw, truncated: false };
  return { capped: points.slice(0, MAX_URL_CHARS).join(""), truncated: true };
}

/** What the counter says, or nothing at all while the end is still far off. */
const urlCounter = computed<{ text: string; atLimit: boolean } | null>(() => {
  const remaining = MAX_URL_CHARS - countCodePoints(newUrl.value);
  if (remaining > URL_COUNTER_FROM) return null;
  return remaining <= 0
    ? { text: `已达上限 ${MAX_URL_CHARS} 字`, atLimit: true }
    : { text: `还可输入 ${remaining} 字`, atLimit: false };
});

/**
 * Caps the URL input at the contract's limit and reports it through the page's
 * single live region, so an over-long paste keeps its first N code points
 * instead of vanishing whole. Mirrors `SubtaskList.applyTitleCap`.
 */
function onUrlInput(event: Event): void {
  schemeRejected.value = false;
  const control = event.target as HTMLInputElement;
  const { capped, truncated } = capCodePoints(control.value);
  // Vue does not repaint a control whose bound value came back unchanged, so an
  // overflow has to be written back by hand or the box goes on showing it.
  if (truncated) control.value = capped;
  newUrl.value = capped;

  if (countCodePoints(capped) < MAX_URL_CHARS) {
    retractFormAlert(urlCapAlert);
    urlCapAlert = null;
    return;
  }
  const pasted = event instanceof InputEvent && event.inputType === "insertFromPaste";
  if (truncated && pasted) {
    urlCapAlert = announceFormAlert(
      "warn",
      `链接地址已达上限，最多 ${MAX_URL_CHARS} 字；粘贴内容超出的部分未加入。`,
      "standing",
    );
  } else if (!urlCapAlert) {
    urlCapAlert = announceFormAlert(
      "warn",
      `链接地址已达上限，最多 ${MAX_URL_CHARS} 字。`,
      "standing",
    );
  }
}

/** Drops the URL input's cap line once it is cleared or the input is left. */
function clearUrlCap(): void {
  retractFormAlert(urlCapAlert);
  urlCapAlert = null;
}

/** A fresh copy of the list as plain rows, ready to be reshaped and stored. */
function copyOfList(): Attachment[] {
  return attachments.value.map((attachment) => ({ ...attachment }));
}

/** Reveals the link input on click and puts the cursor in it. */
function revealLink(): void {
  linkRevealed.value = true;
  void nextTick(() => urlInputRef.value?.focus());
}

/** True when the list is already at the contract ceiling; announces it once. */
function atCapacity(): boolean {
  if (attachments.value.length < MAX_ATTACHMENTS) return false;
  announceFormAlert("warn", `附件数量已达上限 ${MAX_ATTACHMENTS} 项，无法再添加。`, "transient");
  return true;
}

/**
 * Mounts a link. A blank URL is a no-op; a `javascript:`/`data:` one is refused
 * inline without storing; anything else is stored after the `https://` paste
 * convenience. The input is cleared and kept focused so links can be typed in a
 * run.
 */
function addLink(): void {
  const url = normalizeUrl(newUrl.value);
  if (!url) return;
  if (isRejectedScheme(url)) {
    schemeRejected.value = true;
    return;
  }
  if (atCapacity()) return;
  const attachment: Attachment = { id: crypto.randomUUID(), kind: "link", url };
  todoStore.editAttachments(props.todo.id, [...copyOfList(), attachment]);
  newUrl.value = "";
  schemeRejected.value = false;
  clearUrlCap();
  void nextTick(() => urlInputRef.value?.focus());
}

/**
 * Mounts a file through the native picker. Outside the desktop runtime there is
 * no dialog, so the click announces the degradation instead of doing nothing
 * silently; a cancelled pick is a plain no-op.
 */
async function addFile(): Promise<void> {
  const result = await pickAttachmentFile();
  if (result.kind === "unsupported") {
    announceFormAlert("warn", "此环境无法挂载本地文件（仅桌面应用支持）。", "transient");
    return;
  }
  if (result.kind === "cancelled") return;
  if (atCapacity()) return;
  const attachment: Attachment = { id: crypto.randomUUID(), kind: "file", url: result.path };
  todoStore.editAttachments(props.todo.id, [...copyOfList(), attachment]);
}

/**
 * Opens an attachment and, on any failure or unsupported runtime, says so
 * through the page's one live region rather than throwing — a file that has been
 * moved or deleted must degrade, not crash (规格 b, acceptance).
 */
async function open(attachment: Attachment): Promise<void> {
  const result = await openAttachment(attachment);
  if (result.kind === "opened") return;
  if (result.kind === "unsupported-file") {
    announceFormAlert("warn", "此环境无法打开本地文件（仅桌面应用支持）。", "transient");
    return;
  }
  if (result.kind === "rejected-scheme") {
    announceFormAlert("warn", "无法打开链接：仅支持网页或应用链接。", "transient");
    return;
  }
  const message =
    attachment.kind === "file"
      ? "无法打开文件：文件可能已被移动或删除。"
      : "无法打开链接，请稍后重试。";
  announceFormAlert("warn", message, "transient");
}

/**
 * Removes one attachment, then hands focus to the row that slid into its place —
 * or to the `链接` trigger when it was the last one — so a keyboard user is never
 * dropped onto the page body (规格 5, mirroring `SubtaskList.focusAfterDelete`).
 */
function remove(index: number): void {
  const next = copyOfList().filter((_, position) => position !== index);
  todoStore.editAttachments(props.todo.id, next);
  void nextTick(() => focusAfterRemove(index));
}

function focusAfterRemove(removedIndex: number): void {
  const remaining = attachments.value.length;
  if (remaining === 0) {
    sectionRef.value?.querySelector<HTMLElement>('[data-role="link-trigger"]')?.focus();
    return;
  }
  const targetIndex = Math.min(removedIndex, remaining - 1);
  const buttons = sectionRef.value?.querySelectorAll<HTMLElement>('[data-role="remove"]');
  buttons?.[targetIndex]?.focus();
}
</script>

<template>
  <section
    ref="sectionRef"
    class="grid gap-3 border-t border-slate-100 pt-4 dark:border-slate-800"
    aria-label="附件"
  >
    <p v-if="attachments.length" class="m-0 text-xs font-medium text-slate-500 dark:text-slate-400">
      附件 <span class="tabular-nums">{{ attachments.length }}</span>
    </p>

    <ul v-if="attachments.length" class="m-0 grid list-none gap-2 p-0">
      <li
        v-for="(attachment, index) in attachments"
        :key="attachment.id"
        class="flex items-center gap-2"
      >
        <!-- kind icon: link vs paperclip, so the two kinds read apart at a
             glance. Decorative — the kind is carried by the button's text and
             accessible name, never the icon alone. -->
        <LinkIcon
          v-if="attachment.kind === 'link'"
          :size="16"
          class="shrink-0 text-slate-400 dark:text-slate-500"
          aria-hidden="true"
        />
        <Paperclip
          v-else
          :size="16"
          class="shrink-0 text-slate-400 dark:text-slate-500"
          aria-hidden="true"
        />

        <!-- The label is the open trigger: a bare button (the project ships no
             CSS reset, so `border-0 bg-transparent p-0` is what keeps it from
             carrying the browser's grey box). `min-w-0 flex-1 truncate` lets a
             long path shrink to an ellipsis instead of pushing the row into a
             horizontal scroll. -->
        <button
          type="button"
          class="m-0 min-h-11 min-w-0 flex-1 truncate rounded-md border-0 bg-transparent p-0 text-left text-slate-700 hover:text-slate-950 hover:underline focus:ring-2 focus:ring-sky-500 sm:min-h-0 dark:text-slate-200 dark:hover:text-slate-100"
          :aria-label="`打开${kindWord(attachment.kind)} ${attachmentLabel(attachment)}`"
          :title="attachment.url"
          @click="open(attachment)"
        >
          <span class="sr-only">{{ kindWord(attachment.kind) }}：</span
          >{{ attachmentLabel(attachment) }}
        </button>

        <Button
          variant="ghost"
          class="min-h-11 min-w-11 shrink-0 !p-2 text-slate-400 hover:text-red-600 dark:hover:text-red-400"
          data-role="remove"
          :aria-label="`移除${kindWord(attachment.kind)} ${attachmentLabel(attachment)}`"
          @click="remove(index)"
        >
          <Trash2 :size="20" />
        </Button>
      </li>
    </ul>

    <div v-if="linkRevealed" class="grid gap-1">
      <div class="flex items-center gap-2">
        <input
          ref="urlInputRef"
          :value="newUrl"
          placeholder="粘贴链接…"
          aria-label="链接地址"
          :aria-invalid="schemeRejected ? 'true' : undefined"
          :aria-describedby="schemeRejected ? 'attachment-scheme-error' : undefined"
          class="min-h-11 min-w-0 flex-1 rounded-lg border-0 bg-slate-100 px-3 py-2 text-base placeholder:text-slate-400 focus:ring-2 focus:ring-sky-500 sm:text-sm dark:bg-slate-900"
          @input="onUrlInput"
          @keydown.enter.prevent="addLink"
          @blur="clearUrlCap"
        />
        <Button
          variant="ghost"
          class="min-h-11 min-w-11 shrink-0 !p-2"
          aria-label="添加链接"
          @click="addLink"
        >
          <Plus :size="20" />
        </Button>
      </div>
      <span
        v-if="urlCounter"
        class="block text-right text-xs tabular-nums"
        :class="
          urlCounter.atLimit
            ? 'text-amber-700 dark:text-amber-300'
            : 'text-slate-500 dark:text-slate-400'
        "
        >{{ urlCounter.text }}</span
      >
      <p
        v-if="schemeRejected"
        id="attachment-scheme-error"
        class="m-0 text-xs text-red-600 dark:text-red-400"
      >
        仅支持网页或应用链接。
      </p>
    </div>

    <div class="flex flex-wrap gap-2">
      <Button
        variant="ghost"
        class="min-h-11 !justify-start !px-2 text-sm text-slate-500 dark:text-slate-400"
        data-role="link-trigger"
        aria-label="添加链接"
        @click="revealLink"
      >
        <LinkIcon :size="16" /><span class="ml-1">链接</span>
      </Button>
      <Button
        variant="ghost"
        class="min-h-11 !justify-start !px-2 text-sm text-slate-500 dark:text-slate-400"
        aria-label="添加文件"
        @click="addFile"
      >
        <Paperclip :size="16" /><span class="ml-1">文件</span>
      </Button>
    </div>
  </section>
</template>
