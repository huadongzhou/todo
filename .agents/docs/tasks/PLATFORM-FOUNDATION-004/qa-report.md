# QA 报告

## 结果摘要

已实现并静态验证通过；完整 Tauri 桌面运行时测试受本机动态链接环境阻断，待修复后复验运行时行为。

## 已实现并可静态验证

- [x] `tauri-plugin-global-shortcut 2.3.2` 已加入依赖（npm + Cargo 双装），`global-shortcut:default` 权限已授予。
- [x] Rust 侧 `Builder::default().build()` 编译通过。
- [x] 前端 `shortcuts.ts` 平台层含注册 / 注销 / 失败降级。
- [x] App.vue 启动注册、退出注销；按下聚焦输入框；UI 显示当前快捷键。

## 待运行时复验（受环境限制）

- [ ] 桌面端按下 `CommandOrControl+Shift+A` 唤起主界面并聚焦输入框。
- [ ] 应用被隐藏到托盘时仍可唤醒。
- [ ] 其他应用占用该快捷键时，失败被记录但不阻塞启动。
- [ ] 浏览器无插件路径安全跳过。

## 未决问题

- 完整 Tauri 测试进程在本机启动前以 `STATUS_ENTRYPOINT_NOT_FOUND` 退出，与新增代码无关。
