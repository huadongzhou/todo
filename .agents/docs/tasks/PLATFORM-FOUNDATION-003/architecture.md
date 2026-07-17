# 架构方案

## 模块边界

- `crates/contracts/src/lib.rs`：`Todo` 结构追加 `dueDate: Option<String>` 与 `reminderAt: Option<String>`；重建 TS 绑定。
- `src-tauri/Cargo.toml`：新增 `tauri-plugin-notification = "2"`；`capabilities/default.json` 追加 `notification:default`。
- `src-tauri/src/lib.rs`：`Builder` 追加 `.plugin(tauri_plugin_notification::init())`。
- `src/lib/notifications.ts`：前端平台层。封装权限检查、发送、调度；`isTauri()` 以外为空实现。
- `src/lib/dueDate.ts`：截止日解析与文案计算（今天 / 明天 / 逾期 N 天），前端纯函数工具。
- `src/stores/todos.ts`：
  - `add` / `update` 接受 `dueDate` 与 `reminderAt`；
  - 变更后调用 `scheduleNotification(todo)` / `cancelNotification(id)`；
  - 启动时 `rescheduleAll()` 扫描未完成待办。
- `src/App.vue`：表单与列表项展示调整。

## 平台与安全

- 通知仅由 `tauri-plugin-notification` 提供；Capability 只授予 `main` 窗口 `notification:default`。
- 前端全部经过 `lib/notifications.ts` 平台层，以便浏览器降级。
- 调度时机选择在用户空间完成（前端在变更时调度），后端只负责加载插件；避免引入自定义 Rust command。

## 数据与兼容

- 新增字段均为 `Option<String>`，老数据读取为 `null` 不破坏。
- `dueDate` 格式 `YYYY-MM-DD`；`reminderAt` 格式 ISO8601（带时区）。
- 不修改既有 `id` / `title` / `status` / `createdAt` 字段语义。

## 验证与回滚

- `pnpm run check`、`pnpm run build`、`cargo check`、`cargo test --no-run`。
- 回滚：移除 `tauri-plugin-notification` 依赖、`notification:default` 权限、`notifications.ts` / `dueDate.ts` 平台层、回退 Todo 字段。旧字段可保留。
