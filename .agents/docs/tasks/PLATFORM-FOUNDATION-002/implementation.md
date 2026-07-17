# 实现记录

## 范围

- 系统托盘（桌面端）。
- 关闭主界面隐藏到托盘（可在设置内关闭）。
- 设置项 `behavior.closeToTray`，持久化到官方 Store；浏览器模式回退 `localStorage`。

**本期不作单实例**：`tauri-plugin-single-instance = "2"` 不在本仓库的离线 Cargo 镜像缓存中，需要联网拉取或纯 Rust 自实现。详见 `architecture.md`。

## 关键文件

- `src-tauri/Cargo.toml`：为 `tauri` 启用 `image-png` 与 `tray-icon` feature；新增 `log = "0.4"`。
- `src-tauri/src/lib.rs`：
  - `setup_tray` + `build_tray`：创建托盘、注册菜单与双击/`CloseRequested` 拦截。
  - `close_to_tray_enabled`：读取 Store；缺失或畸形默认 `true`。
  - 托盘丢失时写 `warn` 日志并继续，不阻塞启动。
- `src/stores/settings.ts`：新增 `closeToTray`、`isDesktop`、`setCloseToTray`。
- `src/lib/settings-storage.ts`：新增 `loadBooleanPreference` / `saveBooleanPreference`，沿用"原生 Store → localStorage 回退"策略。
- `src/App.vue`：设置面板追加桌面行为分组（`v-if="isDesktop"`）。

## 设计要点

- 关闭隐藏由 Rust 侧 `WindowEvent::CloseRequested` 拦截并 `prevent_close()` + `hide()`，前端只控制偏好开关，保持平台行为集中在 Rust 端。
- 托盘由 Rust 拥有：原生关闭事件集中在 Rust，前端不应重复创建托盘。
- 单实例预留：后续接入仅在 `lib.rs` 追加 `tauri_plugin_single_instance::init(...)` 调用。

## 已通过的验证

- `cargo check --manifest-path src-tauri/Cargo.toml --offline` 通过。
- `cargo test --manifest-path src-tauri/Cargo.toml --no-run --offline` 通过。
- `pnpm run check`（格式、lint、类型检查）通过。
- `pnpm run build` 通过。
