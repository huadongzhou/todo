# AXUM-001 架构方案

## 模块边界

```text
crates/contracts  Rust DTO + serde + Specta/ts-rs 导出能力
       ├── crates/server       Axum HTTP 路由与内存同步服务
       └── apps/desktop/src-tauri  Tauri 原生层与 bindings 导出测试
apps/desktop/src  仅消费生成的 TypeScript bindings
```

`todo-contracts` 是 Todo、同步请求和响应的唯一业务 DTO 源。Rust 服务端通过 serde 解码 JSON，Tauri 通过同一 crate 导出 TypeScript bindings；`RuntimeInfo` 等仅原生 IPC 专属对象仍定义在 Tauri 层。

## API

- `GET /health`：返回服务状态和 RFC 3339 时间。
- `POST /v1/sync`：接收 `SyncRequest`，最多 100 个操作；upsert 必有 patch，delete 不得带 patch。违反规则返回 400。

## 安全

默认不返回宽松的跨域头；仅在设置 `CORS_ORIGIN` 后允许指定来源的 GET/POST/OPTIONS 与 `Content-Type`。

## 迁移

移除 `apps/api` 与 `packages/contracts`、`packages/domain` 的 TypeScript 实现。根目录改为 npm workspace + Cargo workspace；不生成 Bun 锁文件。因项目已使用 ts-rs 12，Cargo workspace 的最低 Rust 版本统一为 1.88。

## 验证与回滚

执行 Cargo crate 测试、Tauri bindings 导出、桌面类型检查与构建。若迁移未被采纳，可恢复原 `apps/api` 和 TypeScript packages；本任务不涉及数据迁移。
