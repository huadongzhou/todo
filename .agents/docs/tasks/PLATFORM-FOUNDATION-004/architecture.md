# 架构方案

## 模块边界

- `src-tauri/Cargo.toml`：新增 `tauri-plugin-global-shortcut = "2"`；Capability 追加 `global-shortcut:default`。
- `src-tauri/src/lib.rs`：`Builder` 追加 `.plugin(tauri_plugin_global_shortcut::Builder::default().build())`。
- `src/lib/shortcuts.ts`：前端平台层。`registerShortcuts` / `unregisterShortcuts`；`isTauri()` 以外为空实现。
- `src/App.vue`：启动注册、退出注销；快捷键按下时聚焦输入框；UI 轻量提示。

## 设计要点

- 注册放在**运行时**（前端 `register()` / `unregister()`），而非 Rust builder 的静态 `with_shortcut`。理由：需要动态快捷键管理、启动/退出握手、以及未来可配置化。
- 快速新增复用**主界面**的 draft 输入框，不引入独立窗口；与现有"关闭隐藏到托盘"架构一致：主界面被隐藏时，快捷键先 show 再 focus。

## 平台与安全

- 仅桌面端启用；浏览器 / 移动端整段绕过。
- `global-shortcut:default` 仅授予主窗口。

## 验证与回滚

- `cargo check`、`pnpm run check`、`pnpm run build`。
- 回滚：移除 `tauri-plugin-global-shortcut` 依赖、`global-shortcut:default` 权限、`shortcuts.ts` 以及 App.vue 中的注册调用。
