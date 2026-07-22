# P4 · 账号与服务端持久化

前置：P1。打通「多端同步」环的数据归属与可靠性。已锁定：邮箱 + 密码自建认证；服务端 SQLite。

## 范围（TODO 映射）

- 模块 5：注册/登录/登出、数据归属、服务端鉴权
- 模块 6.2：服务端持久化、登录绑定同步

## 技术方案

### 服务端

- **存储**：rusqlite（workspace 复用）；`crates/server/` 内 storage 模块，表：
  - `users(id TEXT PK, email TEXT UNIQUE NOT NULL, password_hash TEXT NOT NULL, created_at TEXT NOT NULL)`
  - `tokens(token TEXT PK, user_id TEXT NOT NULL, created_at TEXT NOT NULL, expires_at TEXT NOT NULL)`
  - `operations(revision INTEGER PK AUTOINCREMENT, user_id TEXT NOT NULL, operation_id TEXT NOT NULL, todo_id TEXT, kind TEXT, occurred_at TEXT, patch TEXT, UNIQUE(user_id, operation_id))`
  - 现内存态 `SyncService` 改为读写 SQLite，游标即 revision，按 user_id 隔离。
- **认证**：`argon2` 哈希密码（新依赖，理由：密码存储行业基线，不可自实现）；登录签发不透明随机 token（服务端存表可吊销，不用 JWT，避免签名密钥管理）；`Authorization: Bearer` 中间件校验，`/v1/sync` 纳入保护。
- **接口**（contracts 先行）：`POST /v1/auth/register`、`POST /v1/auth/login`（返 token + 用户信息）、`POST /v1/auth/logout`；错误码区分邮箱占用/凭证错误/Token 失效。
- **兼容迁移**：现按 deviceId 的匿名同步在登录后绑定——首次登录时客户端把本地全量任务作为操作补推到该账号（服务端幂等去重）。

### 客户端

- 登录 UI：桌面自设置面板进入（README 视图结构）；表单校验、错误就近提示（DESIGN.md 表单基准）。
- Token 存放：桌面经 Store 插件（settings.json 同级），不入 SQLite（与业务数据分离）；登出即删。
- sync-engine：请求携带 token；401 时置「未登录」态并停止自动同步，UI 提示重新登录；未登录完全离线（现状不变）。

## 任务拆解

- [ ] contracts：auth 请求/响应 DTO + 校验（邮箱格式、密码长度 ≥ 8）+ bindings
- [ ] 服务端 storage 模块 + 三表 + SyncService 改造（`cargo test` 覆盖注册/登录/隔离/幂等）
- [ ] argon2 接入与 token 中间件；`/v1/sync` 鉴权
- [ ] 客户端登录/注册表单 + token 管理 + sync-engine 改造
- [ ] 首次登录本地数据补推绑定
- [ ] TODO 5 / 6.2 回写；README 能力清单与矩阵更新

## 验收标准

1. 注册→登录→同步→登出全链路可用；密码以 argon2 哈希存储，接口无鉴权则 401。
2. 服务端重启后操作日志与游标完整，两设备同账号数据收敛一致。
3. 未登录时应用完整离线可用，无同步请求发出。
4. 不同账号数据严格隔离（测试覆盖）。

## 验证方式

`cargo test --workspace`（服务端集成测试为主）；双实例 `tauri:dev` + 本地 server 手测多端收敛；`pnpm run check`。

## 风险与依赖

- 明文传输：自托管场景先要求反代 TLS，README 部署说明注明；不在本阶段内置 HTTPS。
- 忘记密码/邮箱验证暂不做（无邮件服务），TODO 标设想；token 过期时长暂定 30 天。
