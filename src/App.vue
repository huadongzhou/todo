<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  watch,
  type ComponentPublicInstance,
} from "vue";
import { Check, ClipboardList, Plus, Settings2, SlidersHorizontal, X } from "lucide-vue-next";
import ArchiveSection from "@/components/ArchiveSection.vue";
import Button from "@/components/ui/button/Button.vue";
import TodayCard from "@/components/TodayCard.vue";
import ReminderSnoozeBar from "@/components/ReminderSnoozeBar.vue";
import TodoFields, { createEmptyDraft, type TodoDraft } from "@/components/TodoFields.vue";
import TodoItem from "@/components/TodoItem.vue";
import type { ThemePreference } from "@/lib/appearance";
import { canExportCalendar, exportCalendar } from "@/lib/calendar-export";
import { registerDayRollover } from "@/lib/day-rollover";
import { formAlert } from "@/lib/form-alert";
import { pickPageAlert, type PageAlert } from "@/lib/page-alert";
import {
  closeTodayCard,
  openTodayCard,
  isTodayCardOpen,
  isTodayCardWindow,
} from "@/lib/today-card";
import { setupTodayCardSync } from "@/lib/today-card-sync";
import {
  deviceId,
  lastError,
  lastSyncedAt,
  serverUrlRef,
  setSyncServerUrl,
  syncNow,
  syncStatus,
} from "@/lib/sync-engine";
import {
  DEFAULT_QUICK_ADD_SHORTCUT,
  registerShortcuts,
  unregisterShortcuts,
} from "@/lib/shortcuts";
import { registerUndoShortcut } from "@/lib/undo-shortcut";
import { useSettingsStore } from "@/stores/settings";
import { useTodoStore } from "@/stores/todos";

/**
 * The card window loads `index.html?card=today`; everything else is the main
 * window. The URL is fixed per webview, so a one-time read at setup is enough
 * to decide which surface to render.
 */
const isCard = isTodayCardWindow();

const todoStore = useTodoStore();
const settingsStore = useSettingsStore();
let stopTodayCardSync: (() => void) | undefined;
let stopUndoShortcut: (() => void) | undefined;
let stopDayRollover: (() => void) | undefined;
/**
 * The whole draft, as one object. The page never touches a field inside it: it
 * hands it to <TodoFields> to be filled in, hands it to the store to be saved,
 * and replaces it wholesale to reset — which is what lets a field be added
 * without this file changing at all.
 */
const draft = ref<TodoDraft>(createEmptyDraft());
const settingsOpen = ref(false);
const showDetails = ref(false);
const fieldsRef = ref<InstanceType<typeof TodoFields> | null>(null);
const activeShortcut = ref<string>(DEFAULT_QUICK_ADD_SHORTCUT);
/**
 * The row currently open for editing, or `null`. Mutual exclusion lives here
 * rather than in the rows because it is a property of the list: opening one row
 * is what closes another, and a row cannot know about its siblings.
 */
const editingId = ref<string | null>(null);

/**
 * Brings the main window to the user's attention and focuses the draft box. The
 * details region is left exactly as the user left it: the shortcut means "put
 * the window in front of me", not "put the interface back the way it shipped".
 */
async function toggleQuickAdd(): Promise<void> {
  settingsOpen.value = false;
  await new Promise((resolve) => requestAnimationFrame(resolve));
  void fieldsRef.value?.focusTitle();
}

onMounted(async () => {
  if (isCard) {
    // The card window only renders <TodayCard>, which owns its own receiver.
    return;
  }

  // Ctrl+Z / Ctrl+Y, inside the window. An open editor gets the chord instead:
  // there it is the input's own text undo, and taking it would undo somebody's
  // last delete while they were trying to take back a word (the boundary the
  // inline editor was built to, which is why nothing else on this page listens
  // for keys at the document level).
  stopUndoShortcut = registerUndoShortcut({
    undo: () => {
      todoStore.undo();
    },
    redo: () => {
      todoStore.redo();
    },
    isSuspended: () => editingId.value !== null,
  });

  // The rules a new day brings. Registered here rather than at startup because
  // the row being edited is the page's to know, and every rule steps over it.
  // `advanceRecurrences` is the recurring half — it walks the engine, so it is
  // async and fired without awaiting; `runDayStart` stays synchronous.
  stopDayRollover = registerDayRollover(() => {
    todoStore.runDayStart({
      rolloverOverdue: settingsStore.rolloverOverdue,
      skipId: editingId.value,
    });
    void todoStore.advanceRecurrences({ skipId: editingId.value });
  });

  const registered = await registerShortcuts({ toggleQuickAdd });
  if (registered.length > 0) activeShortcut.value = registered[0];

  // Start pushing today's todos to the card (no-op when no card is open).
  stopTodayCardSync = setupTodayCardSync() ?? undefined;
});

onBeforeUnmount(() => {
  if (isCard) return;
  void unregisterShortcuts();
  stopUndoShortcut?.();
  stopTodayCardSync?.();
  stopDayRollover?.();
});

// Opening the setting raises the card; closing it tears the card down. We check
// existence first so a redundant "on" flip does not recreate the window.
async function syncTodayCardWithSetting(enabled: boolean): Promise<void> {
  const isOpen = await isTodayCardOpen();
  if (enabled && !isOpen) {
    await openTodayCard();
  } else if (!enabled && isOpen) {
    await closeTodayCard();
  }
}

watch(
  () => settingsStore.showTodayCard,
  (enabled) => {
    if (isCard) return;
    void syncTodayCardWithSetting(enabled);
  },
);

const themeOptions: ReadonlyArray<{
  readonly value: ThemePreference;
  readonly label: string;
  readonly description: string;
}> = [
  { value: "light", label: "浅色", description: "始终使用浅色界面" },
  { value: "dark", label: "深色", description: "始终使用深色界面" },
  { value: "system", label: "跟随系统", description: "根据设备当前主题切换" },
];

/**
 * Tones for the page-level alert, following the semantic-colour formula in
 * DESIGN.md ("100 底 + 700 字" light, "900/40 底 + 300 字" dark).
 */
const ALERT_TONE_CLASS: Record<"success" | "warn" | "error", string> = {
  success: "bg-emerald-100 text-emerald-700 dark:bg-emerald-900/40 dark:text-emerald-300",
  warn: "bg-amber-100 text-amber-700 dark:bg-amber-900/40 dark:text-amber-300",
  error: "bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-300",
};

/** Whether this runtime can write an .ics file at all; fixed for the session. */
const canExport = canExportCalendar();
const exporting = ref(false);
const lastExportedAt = ref<string | null>(null);
const exportButtonRef = ref<ComponentPublicInstance | null>(null);
/**
 * The result of the last export, in the same shape as a storage alert so both
 * can share one alert element. It stays local to the page rather than going into
 * a store: it is not part of the todo domain, and nothing outside this view has
 * anything to say about it.
 */
const exportAlert = ref<PageAlert | null>(null);

/**
 * The snooze bar's line, in the same shape as the others so it shares the one
 * region. It is transient/success and lives here, next to `exportAlert`, because
 * it is a sentence about an action just taken on this page, not domain state; the
 * bar hands up the message (or `null` to drop it) and this file wraps it.
 */
const reminderAlert = ref<PageAlert | null>(null);
function announceReminder(message: string | null): void {
  reminderAlert.value = message
    ? { tone: "success", message, lifetime: "transient", source: "reminder" }
    : null;
}

/**
 * What the single page-level alert says.
 *
 * The four sources are handed over as they are and ranked by `pickPageAlert`;
 * the page does not decide which wins, because how long a line lives is known
 * only where it was produced. The rule and the reasoning are in that module.
 */
const pageAlert = computed(() =>
  pickPageAlert(formAlert.value, todoStore.storageAlert, exportAlert.value, reminderAlert.value),
);

// A result the user cannot see any more has nothing left to report, and leaving
// it behind would show a stale line the next time the panel is opened.
watch(settingsOpen, (open) => {
  if (!open) exportAlert.value = null;
});

/**
 * Disabling the button while the export runs can leave the focus on `<body>`,
 * which drops a keyboard or screen-reader user out of the settings panel. Only
 * that case is corrected — moving focus the user has since placed elsewhere
 * would be worse than losing it.
 */
function restoreExportFocus(): void {
  if (document.activeElement !== document.body) return;
  const element = exportButtonRef.value?.$el;
  if (element instanceof HTMLElement) element.focus();
}

/** Exports the todos that carry a date, reporting the result in the alert. */
async function runExport(): Promise<void> {
  // The button is disabled while this runs, but a keyboard can still fire the
  // handler twice before the re-render, which would write two files.
  if (exporting.value) return;
  exporting.value = true;
  exportAlert.value = null;

  try {
    // The archive is left out: "no longer worth seeing" and "put it in my
    // calendar" are opposite instructions, and everything in there is finished
    // history anyway.
    const outcome = await exportCalendar(todoStore.visibleItems);
    // Every export result is `transient`: closing the settings panel ends it,
    // whatever it says, so it never sits on top of a standing alert for long.
    let result: PageAlert;
    if (outcome.kind === "exported") {
      lastExportedAt.value = new Date().toISOString();
      // A repeat rule the .ics standard cannot say is exported as a single
      // event on purpose; saying nothing would leave the user believing their
      // lunar birthday still repeats in the calendar they just imported.
      const dropped = outcome.unrepeatableRecurrences;
      const droppedNote = dropped
        ? `其中 ${dropped} 项周期任务只导出为单次事件：农历与“每 N 个工作日”无法用日历标准表示。`
        : "";
      result = {
        tone: "success",
        message: `已导出 ${outcome.eventCount} 项任务：${outcome.path}。${droppedNote}`,
        lifetime: "transient",
        source: "export",
      };
    } else if (outcome.kind === "empty") {
      result = {
        tone: "warn",
        message: "没有可导出的任务：先给任务设置截止日或提醒时间，再试一次。",
        lifetime: "transient",
        source: "export",
      };
    } else {
      result = {
        tone: "error",
        message: outcome.reason
          ? `导出失败：${outcome.reason}。请重试或换一个保存位置。`
          : "导出失败，请重试。",
        lifetime: "transient",
        source: "export",
      };
    }
    // Closing the panel clears the result; a result that arrives after the
    // panel is already closed is the same case, and showing it anyway would
    // strand a line on the page with nothing left to clear it — the watcher
    // below only fires on the moment of closing.
    if (settingsOpen.value) exportAlert.value = result;
  } finally {
    exporting.value = false;
    await nextTick();
    restoreExportFocus();
  }
}

/** Formats an ISO timestamp into a short local time label for the UI. */
function formatSyncTime(iso: string): string {
  const parsed = new Date(iso);
  if (Number.isNaN(parsed.getTime())) return iso;
  return parsed.toLocaleString();
}

/**
 * Hands the draft to the store, whatever it happens to contain. Validation is
 * the field block's business — this only reads the verdict — and the reset is a
 * fresh draft object rather than a list of fields to blank out.
 */
function submit(): void {
  const fields = fieldsRef.value;
  if (!fields?.validate()) {
    void fields?.focusTitle();
    return;
  }
  if (todoStore.add(draft.value)) {
    draft.value = createEmptyDraft();
    // The details region stays as it is: opening it was a deliberate statement
    // of intent, and reopening it for every task in a batch is pure friction.
    void fields.focusTitle();
  }
}

function updateTheme(preference: ThemePreference): void {
  void settingsStore.setTheme(preference);
}

/**
 * Copies a row and opens the copy for editing.
 *
 * Opening the editor is what answers "which one is the new one" without a
 * "copy of" suffix in the title: the form is on it and the cursor is in it. It
 * also saves the click a copy usually needs anyway, and gives a screen reader
 * something to announce — a row quietly appearing in a list announces nothing.
 */
async function duplicateTodo(id: string): Promise<void> {
  const created = todoStore.duplicate(id);
  if (!created) return;
  // The row has to exist before it is told to open. A row that mounts with the
  // editor already on never sees the change that fills its draft, so it comes
  // up as an empty form with no title and no focus; one tick later it is an
  // ordinary row being opened, which is the path every other edit takes.
  await nextTick();
  editingId.value = created;
}
</script>

<template>
  <TodayCard v-if="isCard" />
  <main v-else class="mx-auto min-h-screen max-w-2xl px-5 py-12 sm:py-20">
    <header class="mb-10 flex items-start justify-between gap-4">
      <div>
        <p class="mb-2 text-sm font-semibold tracking-wide text-sky-600">TODAY</p>
        <h1 class="m-0 text-3xl font-bold tracking-tight text-slate-950 dark:text-slate-100">
          我的待办
        </h1>
        <p class="mb-0 mt-2 text-slate-500 dark:text-slate-400">
          {{ todoStore.activeItems.length }} 项待完成
        </p>
      </div>
      <div class="flex items-center gap-2">
        <div
          class="flex h-11 w-11 items-center justify-center rounded-xl bg-sky-100 text-sky-600"
          aria-hidden="true"
        >
          <ClipboardList :size="22" />
        </div>
        <Button
          variant="ghost"
          class="!p-3"
          aria-label="打开设置"
          :aria-expanded="settingsOpen"
          @click="settingsOpen = !settingsOpen"
        >
          <Settings2 :size="20" />
        </Button>
      </div>
    </header>

    <!--
      The one live region on the page, and the only one there may be. It stays
      mounted with no text rather than being created along with its first
      message: a status region that appears at the same moment as its content is
      a well-known way to have nothing announced at all. With nothing to say it
      is `sr-only`, so the page looks exactly as it did.
    -->
    <p
      class="text-sm"
      :class="
        pageAlert ? `mb-6 rounded-lg px-3 py-2 ${ALERT_TONE_CLASS[pageAlert.tone]}` : 'sr-only'
      "
      role="status"
      aria-live="polite"
      aria-atomic="true"
    >
      {{ pageAlert?.message ?? "" }}
    </p>

    <section v-if="settingsOpen" class="surface-card mb-6 p-5" aria-labelledby="settings-heading">
      <div class="mb-5 flex items-start justify-between gap-4">
        <div>
          <h2
            id="settings-heading"
            class="m-0 text-lg font-semibold text-slate-950 dark:text-slate-100"
          >
            设置
          </h2>
          <p class="mb-0 mt-1 text-sm text-slate-500 dark:text-slate-400">
            个性化应用在此设备上的显示方式。
          </p>
        </div>
        <Button variant="ghost" class="!p-2" aria-label="关闭设置" @click="settingsOpen = false">
          <X :size="18" />
        </Button>
      </div>

      <fieldset class="m-0 border-0 p-0">
        <legend class="mb-3 font-medium text-slate-800 dark:text-slate-200">外观</legend>
        <label
          v-for="option in themeOptions"
          :key="option.value"
          class="mb-2 flex min-h-10 cursor-pointer items-center gap-3 rounded-lg px-3 py-2 hover:bg-slate-50 dark:hover:bg-slate-900"
        >
          <input
            type="radio"
            name="theme"
            :value="option.value"
            :checked="settingsStore.theme === option.value"
            @change="updateTheme(option.value)"
          />
          <span class="min-w-0">
            <span class="block text-sm font-medium text-slate-800 dark:text-slate-100">{{
              option.label
            }}</span>
            <span class="block text-xs text-slate-500 dark:text-slate-400">{{
              option.description
            }}</span>
          </span>
        </label>
      </fieldset>
      <p class="mb-0 mt-3 text-xs text-slate-500 dark:text-slate-400" aria-live="polite">
        当前：{{ settingsStore.themeLabel }}
        <span v-if="settingsStore.persistenceError"> · {{ settingsStore.persistenceError }}</span>
      </p>

      <!--
        No `isDesktop` gate: how tasks behave is not a desktop capability, and
        the phone needs the rollover at least as much as the desktop does. It
        sits above the platform-specific groups for that reason.
      -->
      <fieldset class="m-0 mt-6 border-0 p-0" aria-labelledby="task-behavior-heading">
        <legend
          id="task-behavior-heading"
          class="mb-3 font-medium text-slate-800 dark:text-slate-200"
        >
          任务
        </legend>
        <label
          class="mb-2 flex min-h-10 cursor-pointer items-center gap-3 rounded-lg px-3 py-2 hover:bg-slate-50 dark:hover:bg-slate-900"
        >
          <input
            type="checkbox"
            :checked="settingsStore.rolloverOverdue"
            @change="settingsStore.setRolloverOverdue(($event.target as HTMLInputElement).checked)"
          />
          <span class="min-w-0">
            <span class="block text-sm font-medium text-slate-800 dark:text-slate-100"
              >逾期任务自动顺延到今天</span
            >
            <!--
              "顺延" sounds like the whole task moves; it does not, and saying
              which parts stay is cheaper than letting the user find out.
            -->
            <span class="block text-xs text-slate-500 dark:text-slate-400"
              >每天开始时，把逾期未完成任务的截止日改为今天。提醒时间与时间段不变。</span
            >
          </span>
        </label>
        <!--
          Archiving has no control of its own — a rule performs it — so this is
          where the user finds out that it happens and where the results went.
        -->
        <p class="mb-0 px-3 text-xs text-slate-500 dark:text-slate-400">
          已完成超过 7 天的任务会自动归档，可在列表下方“已归档”中查看与还原。
        </p>
      </fieldset>

      <!--
        No `isDesktop` gate, same as the "任务" group above it: which snooze
        durations to offer is a reminder-behaviour preference, not a desktop
        capability, and the phone reuses these once its local notifications land.
      -->
      <fieldset class="m-0 mt-6 border-0 p-0" aria-labelledby="reminder-settings-heading">
        <legend
          id="reminder-settings-heading"
          class="mb-3 font-medium text-slate-800 dark:text-slate-200"
        >
          提醒
        </legend>
        <!--
          Renag rides at the top of the group, above the snooze durations: it is
          the reminder behaviour a user reaches for first, and it stays off by
          default, so its description is where they learn what it does and how to
          stop it. No `isDesktop` gate, same as the group's other rows.
        -->
        <label
          class="mb-2 flex min-h-10 cursor-pointer items-center gap-3 rounded-lg px-3 py-2 hover:bg-slate-50 dark:hover:bg-slate-900"
        >
          <input
            type="checkbox"
            :checked="settingsStore.renagOverdue"
            @change="settingsStore.setRenagOverdue(($event.target as HTMLInputElement).checked)"
          />
          <span class="min-w-0">
            <span class="block text-sm font-medium text-slate-800 dark:text-slate-100"
              >逾期未完成时持续提醒</span
            >
            <span class="block text-xs text-slate-500 dark:text-slate-400"
              >任务过了截止日仍未完成时再次提醒：首日提醒数次，之后每天一次，完成任务即停止。</span
            >
          </span>
        </label>
        <p class="mb-3 px-3 text-xs text-slate-500 dark:text-slate-400">
          以下时长会作为“稍后提醒”的快捷选项。
        </p>
        <label
          class="mb-2 flex min-h-10 cursor-pointer items-center gap-3 rounded-lg px-3 py-2 hover:bg-slate-50 dark:hover:bg-slate-900"
        >
          <input
            type="checkbox"
            :checked="settingsStore.snooze5m"
            @change="settingsStore.setSnooze5m(($event.target as HTMLInputElement).checked)"
          />
          <span class="min-w-0">
            <span class="block text-sm font-medium text-slate-800 dark:text-slate-100">5 分钟</span>
            <span class="block text-xs text-slate-500 dark:text-slate-400"
              >延后 5 分钟后再次提醒。</span
            >
          </span>
        </label>
        <label
          class="mb-2 flex min-h-10 cursor-pointer items-center gap-3 rounded-lg px-3 py-2 hover:bg-slate-50 dark:hover:bg-slate-900"
        >
          <input
            type="checkbox"
            :checked="settingsStore.snooze1h"
            @change="settingsStore.setSnooze1h(($event.target as HTMLInputElement).checked)"
          />
          <span class="min-w-0">
            <span class="block text-sm font-medium text-slate-800 dark:text-slate-100">1 小时</span>
            <span class="block text-xs text-slate-500 dark:text-slate-400"
              >延后 1 小时后再次提醒。</span
            >
          </span>
        </label>
        <label
          class="mb-2 flex min-h-10 cursor-pointer items-center gap-3 rounded-lg px-3 py-2 hover:bg-slate-50 dark:hover:bg-slate-900"
        >
          <input
            type="checkbox"
            :checked="settingsStore.snoozeTonight"
            @change="settingsStore.setSnoozeTonight(($event.target as HTMLInputElement).checked)"
          />
          <span class="min-w-0">
            <span class="block text-sm font-medium text-slate-800 dark:text-slate-100">今晚</span>
            <span class="block text-xs text-slate-500 dark:text-slate-400"
              >延后到今天 20:00 再次提醒（已过 20:00 时当天不提供此项）。</span
            >
          </span>
        </label>
        <label
          class="mb-2 flex min-h-10 cursor-pointer items-center gap-3 rounded-lg px-3 py-2 hover:bg-slate-50 dark:hover:bg-slate-900"
        >
          <input
            type="checkbox"
            :checked="settingsStore.snoozeTomorrow"
            @change="settingsStore.setSnoozeTomorrow(($event.target as HTMLInputElement).checked)"
          />
          <span class="min-w-0">
            <span class="block text-sm font-medium text-slate-800 dark:text-slate-100">明天</span>
            <span class="block text-xs text-slate-500 dark:text-slate-400"
              >延后到明天 09:00 再次提醒。</span
            >
          </span>
        </label>
      </fieldset>

      <fieldset
        v-if="settingsStore.isDesktop"
        class="m-0 mt-6 border-0 p-0"
        aria-labelledby="desktop-behavior-heading"
      >
        <legend
          id="desktop-behavior-heading"
          class="mb-3 font-medium text-slate-800 dark:text-slate-200"
        >
          桌面行为
        </legend>
        <label
          class="mb-2 flex min-h-10 cursor-pointer items-center gap-3 rounded-lg px-3 py-2 hover:bg-slate-50 dark:hover:bg-slate-900"
        >
          <input
            type="checkbox"
            :checked="settingsStore.closeToTray"
            @change="settingsStore.setCloseToTray(($event.target as HTMLInputElement).checked)"
          />
          <span class="min-w-0">
            <span class="block text-sm font-medium text-slate-800 dark:text-slate-100"
              >关闭按钮隐藏到托盘</span
            >
            <span class="block text-xs text-slate-500 dark:text-slate-400"
              >关闭主界面时保留在系统托盘，而不是退出应用。</span
            >
          </span>
        </label>
        <label
          class="mb-2 flex min-h-10 cursor-pointer items-center gap-3 rounded-lg px-3 py-2 hover:bg-slate-50 dark:hover:bg-slate-900"
        >
          <input
            type="checkbox"
            :checked="settingsStore.showTodayCard"
            @change="settingsStore.setShowTodayCard(($event.target as HTMLInputElement).checked)"
          />
          <span class="min-w-0">
            <span class="block text-sm font-medium text-slate-800 dark:text-slate-100"
              >显示今日卡片</span
            >
            <span class="block text-xs text-slate-500 dark:text-slate-400"
              >在桌面打开独立的“今日待办”卡片窗口，随时查看当天任务。</span
            >
          </span>
        </label>
      </fieldset>

      <fieldset
        v-if="settingsStore.isDesktop"
        class="m-0 mt-6 border-0 p-0"
        aria-labelledby="sync-heading"
      >
        <legend id="sync-heading" class="mb-3 font-medium text-slate-800 dark:text-slate-200">
          同步
        </legend>
        <label class="mb-2 block">
          <span class="mb-1 block text-xs font-medium text-slate-500 dark:text-slate-400"
            >服务端地址</span
          >
          <input
            :value="serverUrlRef"
            type="url"
            placeholder="http://127.0.0.1:3000"
            class="w-full rounded-lg border-0 bg-slate-100 px-3 py-2 text-sm focus:ring-2 focus:ring-sky-500 dark:bg-slate-900"
            @change="setSyncServerUrl(($event.target as HTMLInputElement).value)"
          />
        </label>
        <div class="mb-2 flex items-center justify-between gap-3">
          <span class="text-xs text-slate-500 dark:text-slate-400">
            <span v-if="syncStatus === 'syncing'">同步中…</span>
            <span v-else-if="syncStatus === 'error'" class="text-red-600 dark:text-red-400"
              >同步失败{{ lastError ? `：${lastError}` : "" }}</span
            >
            <span v-else-if="lastSyncedAt">上次同步：{{ formatSyncTime(lastSyncedAt) }}</span>
            <span v-else>尚未同步</span>
          </span>
          <Button
            type="button"
            variant="ghost"
            class="!px-3 !py-1.5 text-sm"
            @click="void syncNow()"
          >
            立即同步
          </Button>
        </div>
        <p class="mb-0 text-xs text-slate-500 dark:text-slate-400">
          设备 ID：<code class="font-mono">{{ deviceId }}</code>
        </p>
      </fieldset>

      <!--
        Hidden rather than disabled where the runtime cannot produce a file: the
        document is built natively, so the browser dev server has nothing to
        call, and a control that can only fail explains nothing.
      -->
      <fieldset v-if="canExport" class="m-0 mt-6 border-0 p-0" aria-labelledby="calendar-heading">
        <legend id="calendar-heading" class="mb-3 font-medium text-slate-800 dark:text-slate-200">
          日历
        </legend>
        <p class="mb-3 text-xs text-slate-500 dark:text-slate-400">
          把设置了截止日或提醒时间的任务导出为 .ics 文件，可导入 Outlook、Google 日历或苹果日历。
        </p>
        <div class="flex flex-wrap items-center justify-between gap-3">
          <span v-if="lastExportedAt" class="text-xs text-slate-500 dark:text-slate-400"
            >上次导出：{{ formatSyncTime(lastExportedAt) }}</span
          >
          <Button
            ref="exportButtonRef"
            type="button"
            variant="ghost"
            class="min-h-11 !px-3 text-sm"
            :disabled="exporting"
            :aria-busy="exporting"
            @click="void runExport()"
          >
            {{ exporting ? "导出中…" : "导出 .ics" }}
          </Button>
        </div>
      </fieldset>
    </section>

    <!--
      The create shell knows two things about the form it holds: that there is a
      field block, and whether the details region is open. It never names a
      field, never lays one out, and never resets one — which is what makes
      adding a field a change to <TodoFields> and nothing else.
    -->
    <form class="surface-card mb-6 p-4" @submit.prevent="submit">
      <TodoFields
        ref="fieldsRef"
        v-model="draft"
        id-prefix="create"
        :details-open="showDetails"
        details-id="create-details"
        @expand-details="showDetails = true"
      >
        <template #actions>
          <div class="ml-auto flex gap-2">
            <Button
              type="button"
              variant="ghost"
              class="min-h-11 min-w-11 !p-2"
              :aria-label="showDetails ? '收起更多选项' : '展开更多选项'"
              :aria-expanded="showDetails"
              aria-controls="create-details"
              @click="showDetails = !showDetails"
            >
              <SlidersHorizontal :size="20" />
            </Button>
            <!--
              Never disabled. A greyed-out button cannot say why a submit was
              refused, and disabling a submit also kills the implicit Enter that
              the quick-add shortcut promises.
            -->
            <Button type="submit" class="min-h-11">
              <Plus :size="16" /><span class="ml-1">添加</span>
            </Button>
          </div>
        </template>
      </TodoFields>
    </form>

    <p
      v-if="activeShortcut"
      class="mb-6 flex items-center gap-2 text-xs text-slate-500 dark:text-slate-400"
    >
      <span>使用</span>
      <kbd
        class="rounded border border-slate-200 bg-slate-50 px-1.5 py-0.5 font-mono text-[11px] dark:border-slate-700 dark:bg-slate-900"
        >{{ activeShortcut }}</kbd
      >
      <span>随时打开快速新增。</span>
    </p>

    <!--
      Reminders that have come due and are still waiting sit here, above the list
      they are about: one at a time, each offering to be pushed to a later moment.
      It announces through the one page region rather than a region of its own —
      hence `@announce`, which this file wraps into the shared alert.
    -->
    <ReminderSnoozeBar @announce="announceReminder" />

    <section aria-labelledby="todo-list-heading">
      <h2 id="todo-list-heading" class="sr-only">待办列表</h2>
      <div v-if="todoStore.visibleItems.length" class="surface-card overflow-hidden">
        <TodoItem
          v-for="todo in todoStore.visibleItems"
          :key="todo.id"
          :todo="todo"
          :editing="editingId === todo.id"
          @edit="editingId = todo.id"
          @close="editingId = null"
          @duplicate="void duplicateTodo(todo.id)"
        />
      </div>
      <div v-else class="surface-card grid place-items-center px-6 py-16 text-center">
        <div
          class="mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-sky-100 text-sky-600"
        >
          <Check :size="24" />
        </div>
        <p class="m-0 font-medium text-slate-800 dark:text-slate-100">暂无待办</p>
        <p class="mb-0 mt-1 text-sm text-slate-500 dark:text-slate-400">从上方添加第一项任务吧。</p>
      </div>
    </section>

    <!--
      One line, and everything it needs is in the store: the day there is a
      navigation shell, this line moves into a page of its own and the component
      does not change.
    -->
    <ArchiveSection />
  </main>
</template>
