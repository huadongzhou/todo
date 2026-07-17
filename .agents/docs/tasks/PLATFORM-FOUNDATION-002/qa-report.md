# QA 报告

## 结果摘要

已实现并静态验证通过；完整 Tauri 桌面运行时测试受本机动态链接环境阻断（与 Milestone 1 相同，`STATUS_ENTRYPOINT_NOT_FOUND`），待修复后复验运行时行为。

## 已实现并可静态验证

- [x] `tauri` 启用 `image-png` + `tray-icon`，`log` 0.4 新依赖，离线可解析。
- [x] Rust 侧托盘构建、菜单与双击/`CloseRequested` 拦截编译通过。
- [x] `close_to_tray_enabled` 缺省 `true`、畸形值回退、Store 打开失败回退。
- [x] `behavior.closeToTray` 在前端持久化（原生 Store → localStorage 回退）。
- [x] 设置面板桌面行为分组，仅在 `isTauri()` 时显示。

## 待运行时复验（受环境限制）

- [ ] 桌面端实际创建系统托盘；右键菜单包含显示/隐藏、今日数量、退出。
- [ ] 双击托盘切换主界面显隐。
- [ ] 默认关闭按钮隐藏到托盘；关闭设置项后恢复正常退出。
- [ ] 托盘丢失环境下降级为原生退出并写 `warn` 日志，不崩溃。
- [ ] `behavior.closeToTray` 在多次重启后仍保持。

## 未决问题

- 完整 Tauri 测试进程在本机启动前以 `STATUS_ENTRYPOINT_NOT_FOUND` 退出，与新增代码无关。修复 Windows 运行库/动态链接环境后复验。
- 未来动态"今日数量"更新、单实例需要额外插件或自实现，留待后续里程碑。

## 提示

本次仅添加了托盘与关闭隐藏。单实例因 `tauri-plugin-single-instance` 不在离线 Cargo 缓存中而推迟，已记录到 `architecture.md`。
