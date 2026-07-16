# 实现记录：Bun/Elysia 服务与共享契约基线

## 变更范围

- 根 `package.json` 启用 npm workspaces，保留根目录作为现有 Tauri/Vue 客户端。
- 新增 `apps/api`：Bun 运行 Elysia，暴露健康检查、OpenAPI 与内存态同步端点。
- 新增 `packages/contracts`：Zod schema 和推导 HTTP DTO。
- 新增 `packages/domain`：纯 TypeScript 同步 cursor 与日期工具。
- 新增客户端 `apps/desktop/src/types/sync.ts` 的类型再导出；既有 Tauri generated bindings 未改动。

## 验证

- `npm install`：通过；只更新 `package-lock.json`。
- `npm run check`：通过，覆盖前端、contracts、domain 与 API 类型检查。
- `npm run api:test`：通过，3 个 Bun 测试。
- `npm run build`：通过，前端生产构建完成。

## 已知限制

- 同步状态只存在于服务进程内存；重启即清空。
- 尚未实现身份验证、数据库、冲突合并、客户端网络调用和生产 CORS 来源。
- 客户端已在后续 MONO-001 任务迁移至 `apps/desktop`。

变更分支：`codex/initial-tauri-architecture`。
