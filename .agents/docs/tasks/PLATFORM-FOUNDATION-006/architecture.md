# 架构方案：同步传输与设备通信

## 关键前提

- 服务端已就绪：`crates/server`（Axum + 内存 `SyncService`）提供 `GET /health` 与 `POST /v1/sync`，协议复用 `crates/contracts`。
- 契约已导出 TS：`SyncRequest` / `SyncResponse` / `TodoSyncOperation` / `TodoSyncChange` / `TodoPatch` / `SyncOperationKind` 均已通过 ts-rs 生成到 `src/bindings/models/`。
- 客户端缺口：没有任何代码调用服务端。本任务补齐客户端传输层与同步引擎。
- 前端 WebView 具备标准 `fetch`，无需 `tauri-apps/plugin-http`。

## 模块边界

### `src/lib/sync-transport.ts` — 传输层（平台层）

- `syncWithServer(baseUrl, request): Promise<SyncResponse>` — `POST ${baseUrl}/v1/sync`。
- `checkHealth(baseUrl): Promise<boolean>` — `GET ${baseUrl}/health`。
- `isTauri()` 以外为空实现；失败抛 `SyncTransportError`。

### `src/lib/sync-engine.ts` — 同步引擎（平台层）

- 持有同步状态：`device_id`、`cursor`、`pending: TodoSyncOperation[]`。
- `init()`：从 `settings-storage` 恢复 `device_id` / `cursor` / `pending`；首次生成 `device_id`。
- `recordOperation(op)`：追加到 `pending` 并持久化。
- `applyRemoteChange(change)`：调用 todos store 的 `applyRemoteUpsert` / `applyRemoteDelete`。
- `sync()`：组装 `SyncRequest { device_id, cursor, operations: pending }`，调用传输层，按 revision 排序应用变更，更新 `cursor`，清除已确认操作，持久化。
- 暴露 `status`（idle / syncing / error）与 `lastError` 供 UI 绑定。

### `src/stores/todos.ts` — 本地变更记录 + 远端合并

- 在 `add` / `toggle` / `remove` / `update` 成功后调用 `recordOperation`。
- 新增 `applyRemoteUpsert(todoId, patch, revision)` / `applyRemoteDelete(todoId)`：直接写入 `items`，不触发 `recordOperation`（避免循环）。
- 新增 `activeAndOverdue` 等现有计算属性保持不变。

### `src/stores/settings.ts` — 同步设置

- 追加 `syncServerUrl`（默认 `http://127.0.0.1:3000`）+ `deviceId` + `setSyncServerUrl`。
- 持久化键 `sync.serverUrl` / `sync.deviceId`。

### `src/App.vue` — 同步设置 UI

- 设置页增加「同步」配置组（桌面端显示）：服务端地址输入、手动同步按钮、同步状态、设备 ID。

### `src/main.ts` — 启动同步

- bootstrap 后调用 `initSyncEngine()` 并触发首次同步。

## 数据与兼容

- 不迁移任务数据；不影响现有 `appearance.theme` / `behavior.closeToTray` / `ui.showTodayCard` 键。
- 新增设置键 `sync.serverUrl` / `sync.deviceId` / `sync.cursor` / `sync.pending`。
- todos store 仍为纯内存；服务端作为状态来源，启动时通过同步重建。

## 平台与安全

- 前端不直接调用 `fetch` 访问服务端；全部集中在 `sync-transport.ts` 平台层。
- 同步设置组仅在 `isDesktop` 时渲染。
- 服务端地址由用户配置，默认本地 `127.0.0.1:3000`。

## 验证与回滚

- 执行 `pnpm run check`、`pnpm run build`、`cargo check --workspace`。
- 回滚时移除 `sync-transport.ts` / `sync-engine.ts`、todos store 的 `applyRemote*` 与 `recordOperation` 调用、settings 中的同步字段、App.vue 中的同步设置组；旧设置键可保留无业务影响。
