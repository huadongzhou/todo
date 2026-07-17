# 设计：同步传输与设备通信

## 同步模型

采用**操作式同步（op-based）**，服务端为权威：

- 客户端把每个本地待办变更记录为一个 `TodoSyncOperation`（upsert 或 delete）。
- 同步时把尚未确认的操作连同当前 `cursor` 发给 `POST /v1/sync`。
- 服务端为每个新操作分配单调递增的 `revision`，返回 `revision > cursor` 的所有 `TodoSyncChange`。
- 客户端按 revision 顺序把变更合并到本地 store，更新 `cursor`，清除已确认的操作。

服务端协议（既有）保证：重复操作不会推进游标（幂等确认），非法 upsert/delete 组合返回 400。

## 模块边界

### `src/lib/sync-transport.ts` — 传输层

- `syncWithServer(baseUrl, request)` → `POST /v1/sync`，返回 `SyncResponse`。
- `checkHealth(baseUrl)` → `GET /health`，返回是否可达。
- `isTauri()` 以外为空实现（浏览器 / 移动端不发起真实请求）。
- 失败时抛 `SyncTransportError`，由引擎按错误处理。

### `src/lib/sync-engine.ts` — 同步引擎（平台层）

- 持有 `device_id`、`cursor`、待确认操作队列。
- `recordOperation(op)`：本地变更时由 todos store 调用，写入待确认队列。
- `applyRemoteChange(change)`：把单个远端变更合并到本地 store（字段级，服务端优先），不产生新操作记录。
- `sync()`：组装 `SyncRequest`，调用传输层，处理响应。
- 持久化：`device_id` / `cursor` / 待确认队列通过 `settings-storage` 持久化。

### `src/stores/todos.ts` — 本地变更记录

- 在 `add` / `toggle` / `remove` / `update` 成功后调用 `recordOperation`，把变更转为 `TodoSyncOperation`。
- 提供 `applyRemoteUpsert` / `applyRemoteDelete` 方法供引擎调用，直接写入 store 而不触发新的操作记录。

### `src/stores/settings.ts` — 同步设置

- 追加 `syncServerUrl`（默认 `http://127.0.0.1:3000`）+ `deviceId`（只读显示）+ `setSyncServerUrl`。

### `src/App.vue` — 同步设置 UI

- 设置页增加「同步」配置组：服务端地址输入、手动同步按钮、同步状态（空闲 / 同步中 / 错误）、设备 ID 显示。

## 合并规则（服务端优先）

- `upsert`：若本地存在该 todo，按 patch 字段覆盖（title / status / dueDate / completedAt）；若不存在，用 patch 新建（需含 title）。
- `delete`：移除本地对应 todo。
- 按 revision 顺序应用，后续变更覆盖前者，保证收敛。

## 同步触发时机

- 应用启动时（bootstrap 后）同步一次，重建本地状态。
- 本地变更后防抖（~1s）自动同步。
- 手动点击「同步」按钮立即同步。

## 降级

- `isTauri()` 为 false 时传输层为空实现，引擎不阻塞启动。
- 服务端不可达时写 `warn` 诊断日志，保留待确认队列，下次再试。
