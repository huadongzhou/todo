# P1 · SQLite 本地底座

客户端与移动端统一本地库。Rust 侧 rusqlite + command 封装（已锁定决策），repository 在 Rust 层，前端经生成 bindings 调用，SQL 不进前端。

## 范围（TODO 映射）

- 模块 6.1：SQLite 落地、Store → SQLite 数据迁移

## 技术方案

- **依赖**：`rusqlite`（`bundled` 特性，SQLite 编译内置，桌面与 iOS/Android 同构）。新依赖理由：已锁定决策的唯一载体。
- **位置**：`src-tauri/src/storage/` 模块（需用 Tauri path API 定位应用数据目录）；连接经 `Mutex<Connection>` 挂 Tauri state。
- **Schema v1**（迁移用 `PRAGMA user_version` 递增管理）：
  - `todos(id TEXT PK, title TEXT NOT NULL, status TEXT NOT NULL, created_at TEXT NOT NULL, completed_at TEXT, due_date TEXT, reminder_at TEXT, updated_at TEXT NOT NULL)`
  - `sync_ops(operation_id TEXT PK, todo_id TEXT NOT NULL, kind TEXT NOT NULL, occurred_at TEXT NOT NULL, patch TEXT, pushed INTEGER NOT NULL DEFAULT 0)`
  - 预留：P2/P3 字段以 `ALTER TABLE ADD COLUMN`（可空）+ user_version 递增落地，不在 v1 提前建。
- **Command 面**（全部 `#[tauri::command]` + `#[specta::specta]`，注册 `collect_commands!`，重跑 `types:generate`）：
  `storage_load_todos` / `storage_upsert_todo` / `storage_delete_todo` / `storage_enqueue_op` / `storage_take_unpushed_ops` / `storage_mark_ops_pushed`。入参/出参用 contracts 实体（`Todo`、`TodoSyncOperation`），维持实体单源。
- **前端 repository**：`src/lib/repository.ts` 定义接口，两个实现——SQLite 版（经 `nativeCommands`）与浏览器开发回退版（localStorage）；按 `isTauri()` 选择注入 Pinia store，组件零感知（AGENTS 视图端规范）。
- **迁移**：首次启动检测旧 Store 数据 → 单事务导入 todos 与未推送操作 → 写迁移完成标记（存 SQLite meta）→ 此后不再读旧 Store；迁移失败保留旧数据并 `log::warn!` 降级继续用 Store（容错降级规范）。
- **错误处理**：`StorageError` 枚举手写 `Display`/`Error`（AGENTS Rust 规范），command 边界转为可序列化错误信息。

## 任务拆解

- [ ] 引入 rusqlite（bundled）；`storage` 模块骨架 + 连接管理 + user_version 迁移器
- [ ] Schema v1 与 CRUD/操作队列函数（同文件 `#[cfg(test)]` 内存库测试）
- [ ] Command 封装 + specta 注册 + `types:generate`
- [ ] `repository.ts` 接口与双实现；Pinia store 改为经 repository 读写
- [ ] sync-engine 改造：操作队列走 SQLite（`take_unpushed` / `mark_pushed`）
- [ ] Store → SQLite 一次性迁移 + 失败降级
- [ ] README 能力清单勾选 SQLite 项、TODO 6.1 回写

## 验收标准

1. 桌面端任务与操作日志全部读写 SQLite，重启数据完整；旧 Store 用户首启无损迁移。
2. 浏览器开发模式仍可用（localStorage 回退），行为与之前一致。
3. 同步链路（推送/合并）在 SQLite 底座上回归通过。
4. `cargo test --workspace` 含 storage 单测全绿。

## 验证方式

storage 层 `cargo test`（内存 SQLite）；`pnpm run tauri:dev` 走查迁移与重启持久性；`pnpm run check`。

## 风险与依赖

- 移动端路径与权限差异在 P6 实测；本阶段以桌面为验证面，接口保持平台无关。
- 迁移是一次性不可逆动作：先备份旧 Store 文件（复制 `.bak`）再导入。
