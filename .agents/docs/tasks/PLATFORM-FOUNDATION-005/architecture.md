# 架构方案：今日桌面卡片

## 关键前提（修正前序误判）

- develop.md L797 明确：桌面卡片 = Tauri 多 `WebviewWindow`，**不依赖插件**。
- `tauri-plugin-window-state`（develop.md L795）仅用于主窗口位置记忆，与卡片无关。
- 卡片窗口由前端通过 `@tauri-apps/api/webviewWindow` 的 `WebviewWindow` 构造器创建，无需新增 Cargo 依赖。

## 核心难点：跨窗口数据同步

- 当前 `todos` store 为纯内存（`ref<Todo[]>`），无持久化。
- 卡片窗口是独立 webview，拥有独立的 JS 上下文与独立的空 Pinia store，**无法直接读取主窗口数据**。
- 解决方案遵循 develop.md L688/L798 的通信分层：轻量 UI 同步走 **Tauri event**（`emit` / `listen`），不走 command / Channel。

## 模块边界

### 前端平台层 `src/lib/today-card.ts`

- `openTodayCard()`：通过 `WebviewWindow.getByLabel('today')` 复用已有窗口（`show`+`set_focus`），否则 `new WebviewWindow('today', options)` 创建。
- `closeTodayCard()`：关闭并销毁卡片窗口。
- `isTodayCardOpen()`：查询卡片是否存在。
- 定位逻辑：`primaryMonitor()` → `currentMonitor()` → `center` 回退；物理像素 ÷ `scaleFactor` 换算逻辑像素。
- `isTauri()` 以外为空实现，不阻塞启动。

### 前端同步层 `src/lib/today-card-sync.ts`

- `setupTodayCardSync()`：主窗口调用。监听 todos store 变化，计算今日未完成列表，`emit('today-card:todos', list)`。
- 监听 `today-card:toggle` / `today-card:remove`，转发到 `store.toggle` / `store.remove`。
- `setupTodayCardReceiver()`：卡片窗口调用。监听 `today-card:todos` 写入卡片本地状态；卡片操作 `emit` 回主窗口。

### 前端视图 `src/components/TodayCard.vue`

- 独立卡片组件，接收 `todayCardSync` 提供的今日列表，渲染勾选 / 删除 / 空态。
- 复用 `formatDueDate` / `dueDateTone` / `TONE_LABEL_CLASS`。

### `src/App.vue`

- 根据 URL 参数 `?card=today` 判断当前是否为卡片窗口：是则渲染 `TodayCard`，否则渲染主界面。
- 主窗口挂载时调用 `setupTodayCardSync()`；卡片窗口挂载时调用 `setupTodayCardReceiver()`。
- 设置页“桌面行为”组增加 `showTodayCard` 开关。

### `src/stores/settings.ts`

- 追加 `showTodayCard` 字段（桌面默认 false）+ `setShowTodayCard`。
- 持久化键 `ui.showTodayCard`；移动端不读取。

### `src-tauri/capabilities/default.json`

- 追加 `core:webview:allow-create-webview-window`、`core:window:allow-create`、`core:window:allow-primary-monitor`、`core:window:allow-current-monitor`、`core:window:allow-available-monitors`、`core:window:allow-scale-factor`、`core:window:allow-set-position`、`core:window:allow-set-size`、`core:window:allow-set-always-on-top`、`core:window:allow-set-skip-taskbar` 等窗口能力。

## 数据与兼容

- 不迁移任务数据；不影响现有 `appearance.theme` / `behavior.closeToTray` 键。
- 卡片数据源为内存 store，主窗口退出即丢失（符合当前无持久化现状）。
- 新增设置键 `ui.showTodayCard`，合法值 `true` / `false`；读取异常时默认 `false`。

## 平台与安全

- 前端不直接调用 `@tauri-apps/api/webviewWindow` 之外的底层 API；全部集中在 `src/lib/today-card.ts` 平台层。
- 卡片窗口 `url` 设为 `index.html?card=today`，复用同一前端 bundle，无需新增 HTML 入口。
- 卡片窗口 `contentProtected: true` 防止被其他应用截屏捕获。

## 验证与回滚

- 执行 `pnpm run check`、`pnpm run build`、`cargo check --manifest-path src-tauri/Cargo.toml`。
- 回滚时移除 `today-card.ts` / `today-card-sync.ts` / `TodayCard.vue`、App.vue 中的卡片分支、settings 中的 `showTodayCard`、capabilities 中新增的窗口能力；旧设置键可保留无业务影响。
