# 架构方案

## 模块边界

- `src/lib/tray.ts`：前端平台层。创建 `TrayIcon`、构建菜单、更新今日数量、响应点击；`isTauri()` 以外为空实现。
- `src/stores/settings.ts`：追加 `closeToTray` 字段（桌面默认 true）及持久化。
- `src-tauri/src/lib.rs`：
  - 注册 `tauri::tray::TrayIconBuilder`，挂载系统托盘；启用 `tauri` 的 `image-png` feature 以支持内置 PNG 图标。
  - `on_close_requested` 拦截主界面关闭事件：`closeToTray` 为真时 `prevent_default()` 并隐藏窗口。
- 托盘不新增独立 capability，仍复用 `default` 主板能力。

## 单实例（本期不作，原因与后续见下）

- 参考项目 Voicetypr / Blinko 都依赖社区 crate `tauri-plugin-single-instance = "2"`，但本仓库的离线 Cargo 镜像缓存中没有该 crate（仅有 autostart / log / store / opener 等官方插件），因此无法在不联网的前提下加入该依赖。
- 本期只实现托盘与关闭隐藏；单实例留待可联网拉取 `tauri-plugin-single-instance` 或改用纯 Rust 自实现（文件锁 / 域套接字）后再推进。后续接入时仅在 `lib.rs` 追加一个 `plugin(...)` 调用的门槛。

## 平台与安全

- 前端不直接调用 `@tauri-apps/api/tray` 之外的底层 API；全部集中在 `lib/tray.ts` 平台层。
- `closeToTray` 持久化键为 `behavior.closeToTray`；移动端不读取该键。
- 单实例插件仅影响进程启动路径，不影响现有 Store / Log 权限。

## 数据与兼容

- 新增设置键 `behavior.closeToTray`，合法值为 `true` / `false`；读取异常时默认 `true`。
- 不迁移任务数据；不影响现有 `appearance.theme` 键。
- 托盘图标资源：优先 `icons/tray-icon.png`；缺失时使用 `icons/icon.png`。

## 验证与回滚

- 执行 `pnpm run check`、`pnpm run build`、`cargo check --manifest-path src-tauri/Cargo.toml`。
- 回滚时移除 `single-instance` 插件、`image-png` feature、`on_close_requested` 拦截以及 `lib/tray.ts`；旧设置键可保留无业务影响。
