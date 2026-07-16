# 架构：Tauri 2 跨端基础架构

## 模块边界

- `src/`：浏览器与 Tauri WebView 共用的 Vue 应用。
- `src/stores/todos.ts`：运行时状态和最小业务动作，不承担存储。
- `src/types/todo.ts`：待办领域契约。
- `src/components/ui/`：shadcn-vue 风格组件。
- `src-tauri/`：打包、能力、插件和 Rust 入口，不含业务数据。

## 数据、权限与兼容性

`Todo` 含 `id`、`title`、`status`、`createdAt` 和可选 `completedAt`。后续先定义 `TodoRepository` 再接入 SQLite、文件或同步实现。Rust DTO 是跨端契约单一来源：`ts-rs` 导出领域类型，`tauri-specta` 导出带类型的 command client；前端只能从 `src/bindings/` 引用这两类产物。初始 capability 仅提供 Tauri 核心与日志插件，不授予文件、Shell、网络或进程权限。Rust 使用 `#[cfg_attr(mobile, tauri::mobile_entry_point)]` 共享桌面与移动启动逻辑。

## 验证与风险

运行 `npm run check`、`npm run build` 和 `cargo check --manifest-path src-tauri/Cargo.toml`。Android 需 Android SDK；iOS 初始化和构建需 macOS/Xcode，不能在非目标工具链机器伪造验证结果。
