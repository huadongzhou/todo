import "virtual:uno.css";
import "@/styles/main.css";
import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import { useSettingsStore } from "@/stores/settings";
import { useTodoStore } from "@/stores/todos";
import { initSyncEngine, syncNow } from "@/lib/sync-engine";

const pinia = createPinia();
await useSettingsStore(pinia).bootstrap();
void useTodoStore(pinia).rescheduleAll();

// Restore sync state (device id / cursor / pending ops) and pull any remote
// changes so the in-memory store is rebuilt from the server on startup.
await initSyncEngine();
void syncNow();

createApp(App).use(pinia).mount("#app");
