import "virtual:uno.css";
import "@/styles/main.css";
import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import { useSettingsStore } from "@/stores/settings";
import { useTodoStore } from "@/stores/todos";
import { writeDiagnostic } from "@/lib/diagnostics";
import { initSyncEngine, syncNow } from "@/lib/sync-engine";
import { isTodayCardWindow } from "@/lib/today-card";

const pinia = createPinia();

// The card window (`index.html?card=today`) runs this same entry file but only
// renders <TodayCard>, so every startup step that belongs to the main window
// checks this first.
const isCardWindow = isTodayCardWindow();

async function bootstrapApp(): Promise<void> {
  await useSettingsStore(pinia).bootstrap();

  if (isCardWindow) return;

  // Restore the local todo database before anything reads the store, so a
  // restart shows the persisted list rather than an empty one.
  //
  // The card never reads the todo store — it gets today's list from the main
  // window over the `today-card:todos` event. Hydrating there would fill a
  // second store whose only effect is that `rescheduleAll()` arms a duplicate
  // set of reminders (and immediately fires the overdue ones) in that window.
  await useTodoStore(pinia).hydrate();
  void useTodoStore(pinia).rescheduleAll();

  // Restore sync state (device id / cursor / pending ops) and pull any remote
  // changes so the in-memory store is rebuilt from the server on startup.
  //
  // Main window only, for the same reason: the card's todo store is empty by
  // design, and `sync-transport` uses the webview's native fetch, so a card
  // that synced would materialise every pulled change as a brand new todo
  // (losing `createdAt` / `reminderAt`) and persist it over the real rows.
  await initSyncEngine();
  void syncNow();
}

// Startup must never block rendering: a failing step degrades to defaults and
// the app still mounts, so the window is never a blank page.
try {
  await bootstrapApp();
} catch (error) {
  writeDiagnostic("error", "Startup failed; mounting with defaults", {
    error: error instanceof Error ? error.message : String(error),
  });
}

createApp(App).use(pinia).mount("#app");
