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
import Button from "@/components/ui/button/Button.vue";
import TodayCard from "@/components/TodayCard.vue";
import TodoFields, { createEmptyDraft, type TodoDraft } from "@/components/TodoFields.vue";
import TodoItem from "@/components/TodoItem.vue";
import type { ThemePreference } from "@/lib/appearance";
import { canExportCalendar, exportCalendar } from "@/lib/calendar-export";
import { formAlert } from "@/lib/form-alert";
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

  const registered = await registerShortcuts({ toggleQuickAdd });
  if (registered.length > 0) activeShortcut.value = registered[0];

  // Start pushing today's todos to the card (no-op when no card is open).
  stopTodayCardSync = setupTodayCardSync() ?? undefined;
});

onBeforeUnmount(() => {
  if (isCard) return;
  void unregisterShortcuts();
  stopTodayCardSync?.();
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
const exportAlert = ref<{ tone: "success" | "warn" | "error"; message: string } | null>(null);

/**
 * What the single page-level alert says.
 *
 * The form's line comes first, ahead of the storage alert it used to sit behind.
 * It is the direct answer to something the user just did and it lasts one
 * keystroke, whereas a storage alert is a standing condition that comes back by
 * itself; letting the standing one win would mean a device that once failed to
 * write could never again tell anyone why a submit was refused.
 */
const pageAlert = computed(() => formAlert.value ?? todoStore.storageAlert ?? exportAlert.value);

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
    const outcome = await exportCalendar(todoStore.items);
    let result: { tone: "success" | "warn" | "error"; message: string };
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
      };
    } else if (outcome.kind === "empty") {
      result = {
        tone: "warn",
        message: "没有可导出的任务：先给任务设置截止日或提醒时间，再试一次。",
      };
    } else {
      result = {
        tone: "error",
        message: outcome.reason
          ? `导出失败：${outcome.reason}。请重试或换一个保存位置。`
          : "导出失败，请重试。",
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

    <section aria-labelledby="todo-list-heading">
      <h2 id="todo-list-heading" class="sr-only">待办列表</h2>
      <div v-if="todoStore.items.length" class="surface-card overflow-hidden">
        <TodoItem
          v-for="todo in todoStore.items"
          :key="todo.id"
          :todo="todo"
          :editing="editingId === todo.id"
          @edit="editingId = todo.id"
          @close="editingId = null"
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
  </main>
</template>
