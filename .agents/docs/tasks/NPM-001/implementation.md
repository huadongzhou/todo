# NPM-001 实现记录

## 变更

- 删除根 `package.json`、根 `package-lock.json` 和无引用的根 `tsconfig.base.json`，取消 npm workspace 编排。
- 在 `apps/desktop/` 执行 npm 安装，生成独立的 `apps/desktop/package-lock.json`。
- 保留根 `Cargo.toml` 与全部 Rust workspace member，不调整服务端、领域逻辑或 Tauri 路径依赖。
- 将 `docs/tasks/API-001`、`ARCH-001`、`MONO-001` 迁移到 `.agents/docs/tasks/`。
- 更新 `AGENTS.md`，明确 npm 命令在 `apps/desktop` 执行、Cargo 命令在仓库根目录执行。

## 验证

- `npm install`（工作目录 `apps/desktop`）：通过。
- `npm run check`（工作目录 `apps/desktop`）：通过，29 个文件格式正确，22 个文件无警告、lint 或类型错误。
- `npm run build`（工作目录 `apps/desktop`）：通过。
- `cargo metadata --no-deps --format-version 1`（根目录）：通过，确认 4 个 Rust workspace member。
- `cargo fmt --all -- --check`：通过。
- `git diff --check`：通过。

## 已知限制

Axum 任务的 Cargo registry 网络阻塞仍存在；本任务未新增 Rust 依赖，故以不解析依赖的 workspace 元数据验证为边界。
