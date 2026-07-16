# PNPM-001 实现记录

## 变更

- 根 `package.json` 固定 `packageManager: pnpm@11.5.1`，并将构建脚本中的 npm 调用改为 pnpm。
- `src-tauri/tauri.conf.json` 的开发与构建前置命令改为 pnpm。
- 删除 `package-lock.json`，通过 pnpm 生成 `pnpm-lock.yaml`。
- 新增 `pnpm-workspace.yaml`，仅存放 pnpm 11 项目级安全设置，不声明 workspace packages：
  - 显式允许已审阅的 `vue-demi` 构建脚本；
  - 为已有的 Vue 3.5.40 锁定依赖设置发布冷却期例外，其他依赖维持默认策略。
- 更新工程规范的包管理器和命令。

## 验证

- `pnpm --version`：11.5.1。
- `pnpm install`：通过。
- `pnpm run check`：通过，39 个文件格式正确，22 个文件无警告、lint 或类型错误。
- `pnpm run build`：通过。
- 路径扫描：无 `package-lock.json`，Tauri 与构建脚本不再调用 npm。

## 已知限制

完整 Tauri/Rust 构建与类型导出仍受既有 Cargo registry 网络故障影响；pnpm 前端依赖迁移本身已完成。
