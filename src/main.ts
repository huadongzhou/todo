import "virtual:uno.css";
import "@/styles/main.css";
import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import { useSettingsStore } from "@/stores/settings";
import { useTodoStore } from "@/stores/todos";
import { initSyncEngine, syncNow } from "@/lib/sync-engine";
import { isTodayCardWindow } from "@/lib/today-card";

const pinia = createPinia();
await useSettingsStore(pinia).bootstrap();

// Restore the local todo database before anything reads the store, so a restart
// shows the persisted list rather than an empty one.
//
// Only the main window does this. The card window (`index.html?card=today`)
// runs this same entry file, but it renders <TodayCard>, which never reads the
// todo store — it gets today's list from the main window over the
// `today-card:todos` event. Hydrating there would fill a second store whose
// only effect is that `rescheduleAll()` arms a duplicate set of reminders (and
// immediately fires the overdue ones) in that window.
if (!isTodayCardWindow()) {
  await useTodoStore(pinia).hydrate();
  void useTodoStore(pinia).rescheduleAll();
}

// Restore sync state (device id / cursor / pending ops) and pull any remote
// changes so the in-memory store is rebuilt from the server on startup.
await initSyncEngine();
void syncNow();

createApp(App).use(pinia).mount("#app");
