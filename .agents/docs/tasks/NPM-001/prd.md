# NPM-001 产品需求

## 目标

取消单一桌面端项目带来的 npm workspace 编排，保留 Cargo workspace 作为 Rust 服务端与 Tauri 的唯一 Monorepo 边界。

## 非目标

- 不调整 Axum API、Rust DTO、Tauri 权限或桌面 UI。
- 不引入新的 JavaScript 包管理器。

## 验收标准

以 `task.json` 的 acceptance_criteria 为准；开发者应能在 `apps/desktop` 目录独立安装和构建前端。
