# 实现记录：今日桌面卡片

## 实现概要

按 `architecture.md` 落地今日桌面卡片。核心决策：卡片不依赖任何插件，由前端 `WebviewWindow` 构造器创建；跨窗口数据同步走 Tauri event。

## 变更清单

### 新增文件

- `src/lib/today-card.ts` — 卡片平台层。`openTodayCard` / `closeTodayCard` / `isTodayCardOpen`。`isTauri()` 以外为空实现。DPI 感知定位：`primaryMonitor()` → `currentMonitor()` → `center` 回退，物理像素 ÷ `scaleFactor` 换算逻辑像素，右下角 24px 边距，`preventOverflow: true`。
- `src/lib/today-card-sync.ts` — 跨窗口 event 同步。`setupTodayCardSync()`（主窗口 push + 响应 request/toggle/remove）、`setupTodayCardReceiver()`（卡片接收 + 冷启动 request）、`emitToggle` / `emitRemove`。`selectTodayOpenTodos` 过滤今日截止 + 逾期的未完成待办。
- `src/components/TodayCard.vue` — 卡片视图。接收 `today-card:todos` 事件渲染列表，勾选 / 删除回写主窗口。复用 `formatDueDate` / `dueDateTone` / `TONE_LABEL_CLASS`。

### 修改文件

- `src/stores/settings.ts` — 追加 `showTodayCard` 字段（桌面默认 false）+ `setShowTodayCard`，持久化键 `ui.showTodayCard`。
- `src/App.vue` — 根据 `?card=today` 切换主界面 / 卡片视图；主窗口挂载时启动 `setupTodayCardSync`；`watch(settingsStore.showTodayCard)` 开关卡片；设置页“桌面行为”组增加“显示今日卡片”开关。
- `src-tauri/capabilities/default.json` — 追加 `core:webview:allow-create-webview-window` 及窗口创建 / 定位 / 置顶 / 任务栏等能力。

## 关键实现细节

- 卡片窗口 URL 为 `index.html?card=today`，复用同一前端 bundle，无需新增 HTML 入口。
- 卡片是独立 webview，拥有独立空 Pinia store，因此通过 event 同步而非共享状态。
- 主窗口 store 变更时 deep watch 推送；卡片冷启动时 emit `today-card:request` 索取最新列表。
- 类型修复：`WebviewWindow` 构造器选项类型为 `Omit<WebviewOptions,'x'|'y'|'width'|'height'> & WindowOptions`；`Todo.dueDate` 可选，映射时用 `?? null` 收窄。

## 验证

- `pnpm run check` — 通过（格式 + 类型 + lint）。
- `pnpm run build` — 通过。
- `cargo check --manifest-path src-tauri/Cargo.toml` — 通过。
