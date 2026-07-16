# QA 报告：Bun/Elysia 服务与共享契约基线

| 验收项 | 验证方式 | 结果 | 证据 |
| --- | --- | --- | --- |
| npm workspaces 与单一锁文件 | `npm install` | 通过 | `package-lock.json` 已更新，未生成 `bun.lock` |
| Bun 服务健康检查 | Bun 测试调用 `GET /health` | 通过 | `apps/api/src/app.test.ts` |
| 同步 schema 与 cursor | Bun 测试调用 `POST /v1/sync` | 通过 | 接收操作后返回 cursor 1 |
| 幂等重放 | 同一 `operationId` 提交两次 | 通过 | cursor 保持为 1 |
| 跨 workspace 类型检查 | `npm run check` | 通过 | Vite+ 与 3 个 `tsc --noEmit` 均成功 |
| 客户端生产构建回归 | `npm run build` | 通过 | Vite 生产构建完成 |

未验证项：数据库、认证、生产部署、跨设备同步和移动端网络调用不在本任务范围。
