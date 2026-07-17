# 设计：今日桌面卡片

## 窗口形态

- 无边框（`decorations: false`）、始终置顶（`alwaysOnTop: true`）、跳过任务栏（`skipTaskbar: true`）、带阴影（`shadow: true`）。
- 固定尺寸：360 × 480（逻辑像素），不可拉伸（`resizable: false`）。
- 不抢焦点创建（`focus: false`），避免打断用户当前工作。

## 定位规则

- 挂在主显示器（`primaryMonitor()`）工作区右下角，右边距 / 底边距 24 逻辑像素。
- 主显示器不可用时回退 `currentMonitor()`，仍不可用则居中（`center: true`）。
- 使用 `Monitor.size`（物理像素）÷ `scaleFactor` 换算为逻辑像素定位，避免高分屏偏移。
- `preventOverflow: true` 保证窗口不超出工作区。

## 卡片内容

- 顶部标题“今日待办”+ 未完成数量。
- 列表每条：勾选圆圈 + 标题 + 截止日徽标（复用 `formatDueDate` / `dueDateTone` / `TONE_LABEL_CLASS`）。
- 悬停行显示删除按钮。
- 空态：今日无待办时显示“今日暂无待办”。

## 同步协议（Tauri event）

- `today-card:todos`：主窗口 → 卡片。主窗口 store 变更后 push 今日未完成列表（含 `id/title/dueDate/status`）。
- `today-card:toggle`：卡片 → 主窗口。带 `todoId`，主窗口执行 `store.toggle(id)`。
- `today-card:remove`：卡片 → 主窗口。带 `todoId`，主窗口执行 `store.remove(id)`。
- 主窗口启动 / 获得焦点时主动 push 一次，确保冷启动卡片拿到最新数据。

## 设置开关

- `settings.showTodayCard`（桌面默认 `false`，持久化到 `ui.showTodayCard`）。
- 开启时打开卡片；关闭时关闭已有卡片。
- `isDesktop` 为 false 时不渲染该开关。

## 降级

- `isTauri()` 为 false（浏览器 / 移动端）时不创建卡片，不阻塞启动。
- `WebviewWindow` 创建失败时写 `warn` 诊断日志，不阻塞启动。
