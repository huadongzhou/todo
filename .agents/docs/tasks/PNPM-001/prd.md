# PNPM-001 产品需求

## 目标

将唯一前端项目的依赖安装、脚本执行和 Tauri 前置命令统一为 pnpm，并保证 lockfile 单一。

## 非目标

- 不调整 Cargo workspace、Axum 服务或前端功能。
- 不引入 pnpm workspace。

## 验收标准

以 `task.json` 的 acceptance_criteria 为准。
