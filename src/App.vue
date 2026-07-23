<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import {
  Check,
  ClipboardList,
  Plus,
  Settings2,
  SlidersHorizontal,
  Trash2,
  X,
} from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import TodayCard from "@/components/TodayCard.vue";
import type { ThemePreference } from "@/lib/appearance";
import { dueDateTone, formatDueDate, TONE_LABEL_CLASS } from "@/lib/dueDate";
import {
  closeTodayCard,
  openTodayCard,
  isTodayCardOpen,
  isTodayCardWindow,
} from "@/lib/today-card";
import { MAX_TITLE_CHARS } from "@/lib/native";
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
const draft = ref("");
const draftDueDate = ref("");
const draftReminderAt = ref("");
const settingsOpen = ref(false);
const showDetails = ref(false);
const draftInputRef = ref<HTMLInputElement | null>(null);
const activeShortcut = ref<string>(DEFAULT_QUICK_ADD_SHORTCUT);

/** Brings the main window to the user's attention and focuses the draft box. */
async function toggleQuickAdd(): Promise<void> {
  showDetails.value = false;
  settingsOpen.value = false;
  await new Promise((resolve) => requestAnimationFrame(resolve));
  draftInputRef.value?.focus();
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
 * Tones for the storage alert, following the semantic-colour formula in
 * DESIGN.md ("100 底 + 700 字" light, "900/40 底 + 300 字" dark).
 */
const STORAGE_ALERT_CLASS: Record<"warn" | "error", string> = {
  warn: "bg-amber-100 text-amber-700 dark:bg-amber-900/40 dark:text-amber-300",
  error: "bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-300",
};

/** Converts a `datetime-local` input value to an ISO8601 instant (local tz). */
function toInstant(value: string): string | null {
  if (!value) return null;
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? null : parsed.toISOString();
}

/** Formats an ISO timestamp into a short local time label for the UI. */
function formatSyncTime(iso: string): string {
  const parsed = new Date(iso);
  if (Number.isNaN(parsed.getTime())) return iso;
  return parsed.toLocaleString();
}

function submit(): void {
  const dueDate = draftDueDate.value || null;
  const reminderAt = toInstant(draftReminderAt.value);
  if (todoStore.add({ title: draft.value, dueDate, reminderAt })) {
    draft.value = "";
    draftDueDate.value = "";
    draftReminderAt.value = "";
    showDetails.value = false;
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

    <p
      v-if="todoStore.storageAlert"
      class="mb-6 rounded-lg px-3 py-2 text-sm"
      :class="STORAGE_ALERT_CLASS[todoStore.storageAlert.tone]"
      role="status"
      aria-live="polite"
    >
      {{ todoStore.storageAlert.message }}
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
    </section>

    <form class="surface-card mb-6 p-2" @submit.prevent="submit">
      <div class="flex gap-2">
        <!--
          The title stops where the contract stops (`MAX_TITLE_CHARS`, generated
          from Rust). Without the cap a longer paste is accepted here, refused by
          the database, and lost on the next restart — the user would never see
          again what they had just typed.
        -->
        <input
          ref="draftInputRef"
          v-model="draft"
          :maxlength="MAX_TITLE_CHARS"
          class="min-w-0 flex-1 rounded-lg border-0 bg-transparent px-3 py-2 outline-none placeholder:text-slate-400 focus:ring-2 focus:ring-sky-500"
          placeholder="添加一项待办…"
          aria-label="待办标题"
        />
        <Button
          type="button"
          variant="ghost"
          class="!p-2"
          aria-label="展开更多选项"
          :aria-expanded="showDetails"
          @click="showDetails = !showDetails"
        >
          <SlidersHorizontal :size="18" />
        </Button>
        <Button type="submit" :disabled="!draft.trim()">
          <Plus :size="18" /><span class="ml-1">添加</span>
        </Button>
      </div>

      <div
        v-if="showDetails"
        class="mt-2 grid gap-2 border-t border-slate-100 pt-3 sm:grid-cols-2 dark:border-slate-800"
      >
        <label class="block">
          <span class="mb-1 block text-xs font-medium text-slate-500 dark:text-slate-400"
            >截止日</span
          >
          <input
            v-model="draftDueDate"
            type="date"
            class="w-full rounded-lg border-0 bg-slate-100 px-3 py-2 text-sm focus:ring-2 focus:ring-sky-500 dark:bg-slate-900"
          />
        </label>
        <label class="block">
          <span class="mb-1 block text-xs font-medium text-slate-500 dark:text-slate-400"
            >提醒时间</span
          >
          <input
            v-model="draftReminderAt"
            type="datetime-local"
            class="w-full rounded-lg border-0 bg-slate-100 px-3 py-2 text-sm focus:ring-2 focus:ring-sky-500 dark:bg-slate-900"
          />
        </label>
      </div>
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
        <article
          v-for="todo in todoStore.items"
          :key="todo.id"
          class="flex items-center gap-3 border-b border-slate-100 px-4 py-3 last:border-0 dark:border-slate-800"
        >
          <button
            class="flex h-6 w-6 shrink-0 items-center justify-center rounded-full border border-slate-300 text-white focus-ring dark:border-slate-600"
            :class="
              todo.status === 'completed'
                ? 'border-sky-500 bg-sky-500'
                : 'bg-white dark:bg-slate-950'
            "
            :aria-label="todo.status === 'completed' ? '标记为未完成' : '标记为完成'"
            @click="todoStore.toggle(todo.id)"
          >
            <Check v-if="todo.status === 'completed'" :size="15" />
          </button>
          <div class="min-w-0 flex-1">
            <p
              class="m-0 text-slate-700 dark:text-slate-200"
              :class="{
                'text-slate-400 line-through dark:text-slate-500': todo.status === 'completed',
              }"
            >
              {{ todo.title }}
            </p>
            <p
              v-if="todo.dueDate"
              class="mb-0 mt-1 inline-block rounded-md px-1.5 py-0.5 text-xs font-medium"
              :class="TONE_LABEL_CLASS[dueDateTone(todo.dueDate, todo.status === 'completed')]"
            >
              {{ formatDueDate(todo.dueDate) }}
              <span v-if="todo.reminderAt"> · 已设提醒</span>
            </p>
          </div>
          <Button
            variant="ghost"
            class="!p-2 text-slate-400 hover:text-red-600"
            :aria-label="`删除 ${todo.title}`"
            @click="todoStore.remove(todo.id)"
          >
            <Trash2 :size="17" />
          </Button>
        </article>
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
