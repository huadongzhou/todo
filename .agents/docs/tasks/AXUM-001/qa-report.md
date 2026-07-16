# AXUM-001 QA 报告

## 结果：阻塞

| 验收标准 | 验证方式 | 结果 | 证据 |
| --- | --- | --- | --- |
| Axum 提供健康与同步路由 | 审阅路由和 Cargo workspace；运行 `cargo check --workspace` | 阻塞 | `crates/server/src/lib.rs` 已定义路由；Cargo 无法下载 `serde` |
| 服务端和 Tauri 共用 Rust crate | `cargo metadata --no-deps --format-version 1` | 通过（元数据） | workspace 含 `todo-contracts`，Tauri 的 path dependency 指向该 crate |
| 桌面端只消费生成 bindings | `npm run check:frontend` | 通过（静态） | `src/types/todo.ts`、`src/types/sync.ts` 仅指向 bindings；前端检查通过 |
| 幂等与非法操作处理 | 审阅 `SyncService` 和 Rust 单元测试；运行 Cargo 测试 | 阻塞 | 测试已覆盖游标与重放；Cargo registry 阻断执行 |
| 移除 Bun/Elysia | `npm install`，搜索残留依赖 | 通过 | lockfile 不含 Elysia/Bun workspace，前端构建通过 |

## 异常路径与风险

- `TodoSyncOperation::validate` 拒绝无 patch 的 upsert、带 patch 的 delete，以及空白或超长 title；Axum 映射为 HTTP 400。
- 服务内部状态锁异常或游标耗尽映射为 HTTP 500，且不泄露内部状态细节。
- CORS 默认不放行任意来源，仅在 `CORS_ORIGIN` 配置后放行指定来源。

## 阻断原因

`cargo check --workspace` 在解析既有 `serde` 依赖时连续三次无法连接 `mirrors.ustc.edu.cn`。该镜像属于本机 Cargo 配置，非代码错误。恢复镜像/网络访问后应依次运行：

```text
cargo check --workspace
cargo test --workspace
npm run types:generate
```
