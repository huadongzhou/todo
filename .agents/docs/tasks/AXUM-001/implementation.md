# AXUM-001 实现记录

## 变更

- 新增根 `Cargo.toml`，将 Tauri、`todo-contracts`、`todo-domain` 与 `todo-server` 组织为 Cargo workspace。
- 新增 `crates/contracts`：Todo、同步 DTO、serde JSON 映射、结构规则及 ts-rs/Specta 导出能力均以此为唯一源。
- 新增 `crates/server`：Axum 的 `/health` 与 `/v1/sync` 路由、受配置约束的 CORS、RFC 3339 服务时间和内存幂等同步服务。
- Tauri 去除重复 Todo/Status 定义，改为在 `export_type_bindings` 测试中从 `todo-contracts` 导出前端 models。
- 桌面端同步类型改为只重导出 `src/bindings/models` 中的生成结果。
- 移除 Bun/Elysia `apps/api` 与 TypeScript `packages/contracts`、`packages/domain`；npm workspace 仅保留桌面端。

## 验证

- `npm install`：通过，重建 `package-lock.json` 并移除 Elysia 依赖。
- `npm run check:frontend`：通过，29 个文件格式正确，22 个文件无 lint 或类型错误。
- `npm run build`：通过，Vite 生产构建成功。
- `cargo metadata --no-deps --format-version 1`：通过，确认 4 个 Cargo workspace member 和 Tauri → `todo-contracts` 路径依赖。
- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：未能执行编译，详见 `qa-report.md`。

## 已知限制

本机 Cargo 被配置为从 USTC 镜像获取 crates；连接 `mirrors.ustc.edu.cn` 失败，因而无法解析已存在的 `serde` 依赖，也无法生成新的根 `Cargo.lock`。代码和生成 bindings 的最终 Rust 编译校验须在可访问 registry 的环境完成。
