# 实现记录

## 范围

- 待办新增 `dueDate`（本地日历日期）与 `reminderAt`（带时区绝对时刻）字段。
- 表单新增"截止日"与"提醒时间"控件；列表项显示截止日徽标（今天 / 明天 / 逾期 / N 天后）。
- 提醒到点通过 `tauri-plugin-notification` 系统通知。
- 权限首次请求、被拒绝时 `warn` 降级、不抛错。

## 关键文件

- `crates/contracts/src/lib.rs`：`Todo` 追加 `dueDate` / `reminderAt`，均为 `#[ts(optional)] Option<String>`。
- `src/bindings/models/Todo.ts`：手工同步生成的 TS 绑定（测试二进制在本机因 `STATUS_ENTRYPOINT_NOT_FOUND` 无法执行，手工绑定与 `TodoPatch.ts` ts-rs 输出格式一致）。
- `src-tauri/Cargo.toml`：新增 `tauri-plugin-notification = "2"`。
- `src-tauri/capabilities/default.json`：追加 `notification:default`。
- `src-tauri/src/lib.rs`：`Builder` 追加 `.plugin(tauri_plugin_notification::init())`。
- `src/lib/dueDate.ts`：截止日解析 / 文案 / 色调工具。
- `src/lib/notifications.ts`：前端提醒调度层（当前基于 `setTimeout` + 即时补发，延时上限 clamp 到 2^31-1 ms）。
- `src/stores/todos.ts`：`add` / `update` 接受可选日期字段；toggle/remove 自动取消提醒；`rescheduleAll` 启动时重调度。
- `src/main.ts`：启动后调用 `rescheduleAll()`。
- `src/App.vue`：表单新增可展开的日期控件；列表项新增截止日徽标。

## 设计要点

- 调度放在前端而非自定义 Rust command：Tauri v2 JS API 没有 `schedule()` 但 Rust `Schedule` type 使用 `time::OffsetDateTime`（chrono 风格）不易暴露。前端调度足以覆盖"应用运行时"场景；后续可通过 Rust `NotificationBuilder::schedule` 叠加后台调度。
- 数据语义：`dueDate` 仅本地日期；`reminderAt` 为 ISO8601 绝对时刻；仅设截止日不主动推送，仅 UI 标注逾期。

## 已通过的验证

- `cargo check` / `cargo test -p todo-contracts` 通过。
- `pnpm run check` / `pnpm run build` 通过。
