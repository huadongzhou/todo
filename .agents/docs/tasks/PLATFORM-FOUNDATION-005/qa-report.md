# QA 报告

## 结果摘要

已实现并静态验证通过；完整 Tauri 桌面运行时测试受本机动态链接环境（`STATUS_ENTRYPOINT_NOT_FOUND`）阻断，待修复后复验运行时行为。

## 已实现并可静态验证

- [x] `src/lib/today-card.ts` 平台层含创建 / 复用 / 关闭 / DPI 感知定位 / 降级。
- [x] `src/lib/today-card-sync.ts` 含主窗口 push、卡片冷启动 request、toggle/remove 双向同步。
- [x] `src/components/TodayCard.vue` 渲染今日列表 + 勾选 / 删除 + 空态。
- [x] `App.vue` 根据 `?card=today` 切换视图；主窗口启动同步；设置开关联动卡片。
- [x] `settings.ts` 追加 `showTodayCard`（默认 false）+ 持久化键 `ui.showTodayCard`。
- [x] `capabilities/default.json` 追加窗口创建 / 定位 / 置顶等能力。
- [x] `pnpm run check` / `pnpm run build` / `cargo check` 均通过。

## 待运行时复验（受环境限制）

- [ ] 桌面端打开独立卡片窗口，按主显示器右下角定位。
- [ ] 主窗口新增 / 完成 / 删除待办后卡片实时刷新。
- [ ] 卡片内完成 / 删除后主窗口实时刷新。
- [ ] 卡片窗口始终置顶、无边框、跳过任务栏、带阴影（运行时目视确认）。
- [ ] 高分屏下定位无偏移（需实际显示器复验）。
- [ ] 浏览器 / 移动端不创建卡片，不阻塞启动（`isTauri()` 守卫已静态确认，运行时待复验）。

## 未决问题

- 完整 Tauri 测试进程在本机启动前以 `STATUS_ENTRYPOINT_NOT_FOUND` 退出，与新增代码无关。
