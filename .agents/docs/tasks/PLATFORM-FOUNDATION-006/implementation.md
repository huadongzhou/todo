# 实现记录：同步传输与设备通信

## 实现概要

服务端（Axum + 内存 `SyncService`）与共享契约（`todo-contracts`）已就绪。本任务补齐客户端传输层与同步引擎，使 Tauri 应用通过 HTTP 与服务端同步待办变更。前端 WebView 的 `fetch` 足够，无需 HTTP 插件。

## 变更清单

### 新增文件

- `src/lib/sync-transport.ts` — 传输层。`syncWithServer`（POST /v1/sync）、`checkHealth`（GET /health）。`isTauri()` 以外为空实现；失败抛 `SyncTransportError`。
- `src/lib/sync-engine.ts` — 同步引擎。持有 `device_id` / `cursor` / 待确认操作队列，通过 `settings-storage` 持久化。`recordOperation`、`applyRemoteChange`、`syncNow`、`setSyncServerUrl`、`initSyncEngine`。暴露 `syncStatus` / `lastError` / `lastSyncedAt` / `deviceId` / `serverUrlRef` 供 UI 绑定。

### 修改文件

- `src/lib/settings-storage.ts` — 追加 `loadStringPreference` / `saveStringPreference`。
- `src/stores/todos.ts` — `add` / `toggle` / `remove` / `update` 成功后调用 `recordOperation`；新增 `applyRemoteUpsert` / `applyRemoteDelete`（直接写入 store，不触发新操作记录，避免循环）。
- `src/main.ts` — bootstrap 后 `initSyncEngine()` 并触发首次同步。
- `src/App.vue` — 设置页增加「同步」配置组：服务端地址输入、手动同步按钮、同步状态、设备 ID。

## 关键实现细节

- 操作式同步（op-based）：本地变更 → 记录操作 → 推送 → 拉取远端变更 → 按 revision 排序字段级合并（服务端优先）。
- 设备 ID 首次启动生成（`crypto.randomUUID()`）并持久化。
- 本地变更后防抖 1s 自动同步；启动时同步一次以重建状态。
- 服务端不可达时写 `warn` 诊断日志，保留待确认队列，下次再试。

## 验证

- `pnpm run check` — 通过。
- `pnpm run build` — 通过。
- `cargo check --workspace` — 通过。
- `cargo test -p todo-contracts -p todo-domain -p todo-server` — 3 通过（幂等确认 / 游标推进 / 重放保护 / health）。
- Tauri 测试二进制受本机 `STATUS_ENTRYPOINT_NOT_FOUND` 阻断（与新增代码无关）。
