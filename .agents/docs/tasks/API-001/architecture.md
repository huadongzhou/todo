# 架构：Bun/Elysia 与共享契约

## 模块边界

- 根目录仍是 Tauri/Vue 客户端，避免在本任务中进行高风险路径迁移。
- `apps/api` 是 Bun 运行的 Elysia HTTP 服务；路由负责 HTTP/schema，`modules/sync/service.ts` 负责无 HTTP 依赖的业务状态。
- `packages/contracts` 使用 Zod 定义同步 HTTP schema，并导出推导类型；这是 HTTP DTO 的唯一来源。
- `packages/domain` 只包含运行时无关的领域工具和类型。
- `apps/desktop/src/bindings` 继续由 Rust `specta`/`ts-rs` 生成，不能改由 contracts 包替代。

## 数据与失败处理

`/v1/sync` 接受 `deviceId`、客户端 cursor 与最多 100 条操作。服务内存保存已处理操作 ID 和单调 revision；相同 operation ID 会被幂等确认但不产生新 revision。该内存实现仅用于开发验证，服务重启后数据清空。

输入经 Zod/Elysia 运行时校验；无效日期、未知字段、缺少 upsert patch 或 delete 夹带 patch 均拒绝。服务不记录待办标题等内容日志。

## 兼容性、权限与验证

依赖由 npm workspaces 安装，Bun 仅作为 API 运行时和测试运行时，因此不创建 `bun.lock`。CORS 只由 `CORS_ORIGIN` 环境变量显式启用；生产环境必须配置受控来源。验证运行 `npm run check`、`npm run build` 和 `npm run api:test`。
