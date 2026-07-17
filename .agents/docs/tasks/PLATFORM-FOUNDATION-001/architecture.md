# 架构方案

## 模块边界

- `src/stores/settings.ts`：Pinia 运行态、启动恢复和用户动作。
- `src/lib/settings-storage.ts`：官方 Store 持久化；浏览器开发模式回退到 `localStorage`。
- `src/lib/appearance.ts`：DOM 主题属性、系统主题解析和当前原生窗口主题同步。
- `src/lib/diagnostics.ts`：结构化前端日志缓冲及官方 Log 插件转发。

## 平台与安全

- Tauri 端仅加入官方 `tauri-plugin-store`；Capability 只给 `main` 窗口授予 `store:default`，保留现有 `log:default`。
- UI 不直接使用插件 API；调用集中在 `lib` 层，浏览器不具备 Tauri 运行时时安全降级。
- 使用 Tauri Core 窗口 API 设置原生主题，不新增未类型化 IPC command。

## 数据与兼容

- 持久化键为 `appearance.theme`，值仅允许 `light`、`dark`、`system`。
- 读取到未知值或失败时默认 `system`。
- 不涉及 HTTP、领域 DTO 或任务数据迁移。

## 验证与回滚

- 执行前端检查及 `cargo test --manifest-path src-tauri/Cargo.toml export_type_bindings`。
- 回滚时移除 Store 插件和 `settings` 平台层即可；旧偏好文件可以保留且无业务影响。
