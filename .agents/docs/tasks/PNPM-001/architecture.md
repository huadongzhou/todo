# PNPM-001 架构方案

根 `package.json` 是唯一 Node manifest，并声明 `packageManager: pnpm@11.5.1`。pnpm 使用 `pnpm-lock.yaml` 锁定依赖；`package-lock.json` 被删除。Tauri 的 `beforeDevCommand` 和 `beforeBuildCommand` 改为 pnpm，确保 Tauri CLI 不会重新调用 npm。pnpm 11 将依赖构建审批配置放在 `pnpm-workspace.yaml`；该文件只配置 `vue-demi` 的已审阅构建权限，不声明 workspace packages。pnpm 的发布冷却期对已有、刚发布的 Vue 3.5.40 锁定依赖设置 `vue` 与 `@vue/*` 例外，其他依赖仍遵循默认供应链策略。

本任务保持单一前端项目，不引入 pnpm workspace；Rust 继续由 Cargo workspace 管理。
