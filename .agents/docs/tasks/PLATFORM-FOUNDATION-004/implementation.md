# 实现记录

## 范围

- 桌面端注册全局快捷键 `CommandOrControl+Shift+A`，按下唤起主界面并聚焦输入框。
- 启动注册、退出注销；浏览器 / 移动端安全跳过。

## 关键文件

- `src-tauri/Cargo.toml`：新增 `tauri-plugin-global-shortcut = "2"`。
- `src-tauri/capabilities/default.json`：追加 `global-shortcut:default`。
- `src-tauri/src/lib.rs`：`Builder` 追加 `.plugin(tauri_plugin_global_shortcut::Builder::default().build())`。
- `src/lib/shortcuts.ts`：`registerShortcuts` / `unregisterShortcuts` 平台层。
- `src/App.vue`：`onMounted` 注册、`onBeforeUnmount` 注销；`toggleQuickAdd` 聚焦输入框；UI 显示快捷键提示。

## 设计要点

- 注册走运行时 `register()` 而非 Rust builder 的静态 `with_shortcut`：便于动态管理、启动/退出握手、未来可配置化。
- 快速新增复用主界面 draft 输入框，不引入独立窗口；与"关闭隐藏到托盘"架构一致。

## 已通过的验证

- `cargo check --manifest-path src-tauri/Cargo.toml` 通过。
- `pnpm run check` / `pnpm run build` 通过。
