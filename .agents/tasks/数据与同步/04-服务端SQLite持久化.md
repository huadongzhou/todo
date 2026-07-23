# 服务端SQLite持久化

- 级别：直达
- 状态：完成

## 需求（产品）

- 目标与场景：服务端操作日志与数据由内存态（Arc+Mutex）迁至 SQLite，重启不丢。
- 范围（做 / 不做）：做 服务端库表、SyncService 存取改造；不做 备份（05）与监控（06）。
- 验收标准（逐条可检查）：
  - [ ] 服务端重启后操作日志与数据完整，同步回归（离线补推、冲突服务端优先字段级合并）通过。
  - [ ] cargo test --workspace 通过，含持久化读写测试。

## 规格（UI/UX）

## 实现记录（开发）

### 落点：把「服务端的全部状态」还原成一张只追加的表

改动前 `SyncService` 有三份内存态：`operation_ids`（去重集）、`changes`（变更日志）、`cursor`（游标）。三份里只有第一份是独立信息——后两份完全可以从日志推出来。所以本任务没有做「把三份状态各存一张表」，而是只落一张 `sync_changes`：

| 原内存态 | 现在从哪里来 |
|---------|------------|
| `operation_ids` | `operation_id` 上的 UNIQUE 索引（`SELECT ... WHERE operation_id = ?`） |
| `changes` | 表本身 |
| `cursor` | `MAX(revision)` |

`SyncService` 因此**不再持有任何状态**，只持有 `SyncStore`；每个请求现读现算。这条是「重启不丢」的根：不存在「内存里对、库里错」的中间态可言，因为内存里没有东西。

### 分层与依赖方向

- `crates/contracts/`：**未改一行**。DTO 与校验已由 03 扩到 v1 全字段，本任务只是换了它们的存放处。
- `crates/domain/`：**未改一行**，也未被 SQLite 沾染。游标推进仍走 `next_sync_cursor`，由 server 侧调用。
- `crates/server/`：新增 `sync_store.rs`（数据库层，AGENTS.md 架构节写明「未来的数据访问入口」在服务端），`lib.rs` 只改 service。handler（`async fn sync`）与路由 `create_router`（`/v1/` 前缀）一字未动；共享状态仍是 `Arc<SyncService>` 经 `with_state` 注入，`Mutex` 从 service 下沉到 store（连接是被互斥的那个资源）。
- 错误类型 `SyncStoreError` 手写枚举 + `Display` + `std::error::Error`，未引入 thiserror/anyhow。`rusqlite` 用 `workspace = true` 复用 `bundled`；另加 `serde_json`（patch 列的编解码，仓库已在用的同一版本）。

### 事务边界：为什么一次同步只能有一个事务

`sync` 要做三件事——读当前 revision、追加、读回该设备欠的变更。拆成三次连接调用，两个并发请求就会从同一个 revision 起编号并撞号。所以 store 暴露的是 `with_log(|log| ...)`：闭包跑在一个事务里，`Ok` 提交、`Err` 回滚。签名是 `impl FnOnce(&SyncLog) -> Result<T, E> where E: From<SyncStoreError>`——**判断请求本身合不合法的权归调用方**，`ServerError::CursorExhausted` 这种业务判定不必为了穿过闭包而伪装成存储错误漏进数据库层。

### 持久化选择与理由

- **`synchronous = FULL`（而非沿用端侧 WAL 默认的 NORMAL）**：响应告诉设备「你的操作已受理」，设备据此把它从待推队列里删掉。若该次提交此刻只在操作系统页缓存里，掉电就是**这条操作全世界只剩零份**。端侧没有这个非对称性（用户还在，数据还在库里），服务端有，所以这里加钱买 fsync。用例 `a_new_log_is_stamped_and_configured_for_durable_commits` 直接断言 `PRAGMA synchronous = 2`。
- **打不开就不启动**：`SyncStore::open` 返回 `Result`，`main` 直接 `?`。端侧 `TodoDb::open` 是 best-effort（打不开还有用户和回退存储），服务端打不开只会把操作确认进虚空。这条不对称在 `SyncStore` 的文档注释里写明了。
- **patch 存 JSON 文本**：`TodoPatch` 的语义是「缺字段 = 不变」，而 `skip_serializing_if = "Option::is_none"` 恰好让 serde_json 的往返**逐字保留字段集**。拆成 20 个列反而要人为区分「NULL 是不变还是清空」，凭空造一个契约里没有的三态。
- **`DATABASE_PATH` 环境变量**（默认 `todo-server.db`），与既有 `HOST`/`PORT`/`CORS_ORIGIN` 同一风格；默认文件与 WAL 边车已进 `.gitignore`。

### 借鉴 01/02/03 踩过的坑

1. **版本位是「关于布局的声明」而非布局本身**：`SCHEMA_VERSION = 1` 的文档注释把这条连同「revision 2 到来时必须列驱动迁移（读 `PRAGMA table_info` 补列），不得只 branch 版本号」写进代码。**没有**顺手造 `reconcile_columns` 与空的 `ADDED_COLUMNS`：今天只有一个 revision，没有任何列可对账，那是一份没有读者的机器。`stored > SCHEMA_VERSION` 只 warn 不动文件（用例 `a_log_from_a_newer_revision_is_left_untouched`）。
2. **坏数据不就地清零**：读不回的行（kind 不认识、patch 不是 JSON、revision 超出 u32）**跳过并 `tracing::warn!`**，不降级成空值。这里与端侧的差别值得写明——端侧要建 `todo_quarantine` 表是因为「每次 save 都重写全部 21 列」，不抢救原字节就会被下一次写抹掉；**服务端的日志只追加、永不重写任何一行，原字节由表本身保管**，所以隔离表在这里是多余的一份拷贝。用例 `an_unreadable_row_is_skipped_instead_of_failing_the_whole_read` 同时断言两件事：好行照常返回，坏行**仍在表里**（`COUNT(*)` 仍是 3）。
3. **kind 不降级**：端侧 `status_from_text` 把未知状态降级为 `Open`，因为两种读法展示的是同一个任务；upsert 与 delete 是相反动作，猜错就是替用户做了没人要求的事，所以它只报「读不回」。理由写在 `kind_from_text` 上方。

### 改动文件

- `crates/server/src/sync_store.rs`（**新增**）：`SyncStore` / `SyncLog` / `SyncStoreError`、schema 与四条语句常量、`initialise`（WAL + synchronous=FULL + 版本位）、`read_change` 的逐行降级、8 个用例。
- `crates/server/src/lib.rs`：`SyncService` 由三份内存态改为持有 `SyncStore`（`Default` → `new(store)`），`sync` 改在一个事务内编排；`ServerError::StateUnavailable` → `Storage(SyncStoreError)` + `From` + 500 前记 `tracing::error!`；重导出 `SyncStore` / `SyncStoreError`；测试新增 `TempDatabase`（含 `-wal`/`-shm` 清理）与 5 个跨重启用例，既有 3 个用例改用 `SyncStore::in_memory()`。
- `crates/server/src/main.rs`：读 `DATABASE_PATH`（默认常量 `todo-server.db`）、开库失败即退出、`SyncService::new(store)`、启动日志加一行路径。
- `crates/server/Cargo.toml`：`rusqlite = { workspace = true }`、`serde_json = "1"`。
- `.gitignore`：`todo-server.db` 及 `-wal` / `-shm` 边车。
- `TODO.md`：6.2「服务端持久化」⏳ → ✅。
- `Cargo.lock`：随两个依赖更新（仅 `todo-server` 的 dependencies 段 +2 行，无版本变动）。
- 未改动：`crates/contracts/**`、`crates/domain/**`、`src/**`（含 `src/bindings/**`，契约未变故无需 `types:generate`）、`src-tauri/**`。

### 自验结果（命令与结论）

- `cargo test --workspace`：通过，**45 passed / 0 failed**（`cross_platform_todo_lib` 20、contracts 8、domain 1、**server 16 ＝ 既有 3 + 新增 13**）。基线 32 passed 全部保留、无回归。
- `cargo check --workspace`：通过。
- `pnpm run check`：通过（40 文件格式、42 文件 lint + 类型零告警）——本任务未动前端，此项为基线不变红的证据。
- `cargo fmt -p todo-server`：本次新增/改写代码零 diff。仓库仍有 2 处**既有**未格式化点（`src-tauri/src/lib.rs:14`、`src-tauri/src/todo_db.rs:678`，01 遗留、02/03 已记为遗留问题），按「不改无关代码」未动。
- `pnpm run types:generate`：**未运行且不需要**——`crates/contracts/` 一行未改，bindings 无输入变化。

新增的 13 个用例（重启不丢由 Rust 测试直接覆盖，不靠手工启停）：

| 位置 | 用例 | 覆盖点 |
|------|------|-------|
| sync_store | `appended_changes_are_still_there_after_closing_and_reopening_the_file` | **同一临时文件关闭再打开，日志与 revision 仍在** |
| sync_store | `an_operation_id_is_still_recognised_after_a_reopen` | 去重集跨重启 |
| sync_store | `a_new_log_is_stamped_and_configured_for_durable_commits` | 版本位 + `synchronous = FULL` |
| sync_store | `a_log_from_a_newer_revision_is_left_untouched` | 更高版本位不改写 |
| sync_store | `a_patch_keeps_exactly_the_fields_it_carried` | 字段级合并的输入：**没带的字段读回仍是 absent** |
| sync_store | `a_delete_is_stored_without_a_patch` | delete 无 patch 往返 |
| sync_store | `changes_up_to_the_cursor_are_not_sent_again` | `revision > cursor` 过滤 |
| sync_store | `a_failed_action_leaves_the_log_as_it_was` | 事务回滚 |
| sync_store | `an_unreadable_row_is_skipped_instead_of_failing_the_whole_read` | 坏行不拖垮整读、原字节仍在表里 |
| lib | `the_log_a_restarted_server_serves_is_the_one_it_stopped_with` | **重启后新设备拉到完整日志与游标** |
| lib | `an_offline_backlog_pushed_after_a_restart_is_recorded_once` | **离线补推 + 重启后重放不重复记账** |
| lib | `field_level_patches_reach_a_restarted_server_intact` | **重启后取回的 patch 字段集与 revision 顺序不变** |
| lib | `a_rejected_request_leaves_nothing_behind` | 批内一条不合法则整批不落库 |

### 端到端走查（真实进程，含硬杀重启）

**跑法一 · curl 直连 + 硬杀（`DATABASE_PATH` 指临时文件，PORT=3111）**

1. 推一批 3 条离线操作（含全字段 patch 与一条 delete）→ `nextCursor: 3`。
2. `Stop-Process -Force` **硬杀**进程（等价掉电，非优雅退出）。
3. 同一文件重新 `cargo run` 产物启动 → 新设备 `cursor: 0` 拉取：3 条变更逐字段原样返回，`nextCursor: 3`。
4. 同一批**再推一次**（模拟 ack 丢失）：3 条全部 acked、`nextCursor` 仍为 3；直接读库确认表里仍是 **3 行**，没有重复。
5. `cursor: 3` 增量拉取返回空；不合法请求仍返回 **400**。
6. 直接读库：`user_version = 1`、`journal_mode = wal`，patch 列内容 `{"title":"写服务端持久化","important":true,"sortOrder":1.5,"tagIds":["tag-a"]}` —— 只有该操作携带的字段。

**跑法二 · `pnpm run tauri:dev` 真实应用走查（服务端 PORT=3000，CORS_ORIGIN=http://localhost:1420，CDP 取证）**

走查前把 `%APPDATA%/com.todo.crossplatform` 整目录备份，走查后原样还原（末尾有核验）。本机 `sync.pending` 里恰好有**上一次会话真实积压的 5 条操作**（从未推送成功过），直接用作离线补推样本。

1. **离线补推**：应用首次同步成功 → 5 条积压全部落库（revision 1–5），`sync.pending` 清零、`sync.cursor = 5`；客户端按 revision 顺序重放，被删除的那条任务正确消失，界面只剩 `CARD-PROBE 今日任务`。
2. **重启不丢 + 二次补推**：硬杀服务端 → 在真实 UI 里新增并勾选一条待办 → `syncNow()` 报 `Failed to fetch`，2 条操作滞留 `sync.pending`（离线队列成立）。同一库重新拉起服务端 → 同步成功，`pending: 0`、`cursor: 7`；读库确认新操作接在 **revision 6、7**——**没有从 1 重新编号**（内存态版本重启后会从 1 起编，会把旧 revision 重新发一遍）。
3. **冲突服务端优先 · 字段级合并**：本地把该待办改名为「本地改名」并推送（revision 8）；随后以另一台设备身份 curl 推送**只带 `title` 与 `notes`** 的操作（revision 9）；应用再同步 → `title` 变为「远端改名」、`notes` 变为「来自另一台设备」，而 patch 里没有的 `status: completed` 与 `completedAt` **保持本地值不变**。服务端优先与字段级合并在新底座下行为不变。
4. **双端最终一致（跨重启）**：再次硬杀并重启服务端，以全新设备（`cursor: 0`）拉取全量 9 条并按 server-wins 折叠 → `{title: 远端改名, notes: 来自另一台设备, status: completed, completedAt: 2026-07-23T00:29:14.173Z}` 与 `{title: CARD-PROBE 今日任务, dueDate: 2026-07-23, status: open}`，与运行中应用的实际状态**逐字段一致**。
5. **界面复核**：`document.body.innerText` 为「我的待办 / 1 项待完成 / 待办列表 / 远端改名 / CARD-PROBE 今日任务 / 今天」，`syncStatus = idle`、`lastError = null`，无错误横幅。

**环境还原核验**：应用与服务端进程均已退出；`%APPDATA%/com.todo.crossplatform` 由备份还原并删除走查产生的 `todos.db-wal` / `-shm`；还原后 `settings.json` 与备份逐字一致（5 条 pending 原样回到队列、无 `sync.cursor` 键），`todos.db` 为 `user_version = 1`、7 列、0 行，与走查前一致。临时库只在 scratchpad，仓库工作区 `git status` 无多余文件。

### 验收标准逐条自查

- [x] **服务端重启后操作日志与数据完整**：Rust 用例 `appended_changes_are_still_there_after_closing_and_reopening_the_file`、`an_operation_id_is_still_recognised_after_a_reopen`、`the_log_a_restarted_server_serves_is_the_one_it_stopped_with` 直接覆盖；跑法一（硬杀后同一文件重启，3 条变更与游标原样）与跑法二第 2 步（新操作接在 revision 6、7）为真实进程证据。
- [x] **同步回归（离线补推）**：用例 `an_offline_backlog_pushed_after_a_restart_is_recorded_once`；跑法二第 1、2 步用真实应用的真实积压队列跑通，重放不重复记账。
- [x] **同步回归（冲突服务端优先字段级合并）**：用例 `a_patch_keeps_exactly_the_fields_it_carried`、`field_level_patches_reach_a_restarted_server_intact` 锁住「服务端交还的字段集与顺序不变」这个前提；跑法二第 3 步在真实应用里验证合并结果，第 4 步验证跨重启双端最终一致。
- [x] **`cargo test --workspace` 通过，含持久化读写测试**：45 passed / 0 failed，其中 13 个为本任务新增，9 个直接读写 SQLite（4 个用真实临时文件）。

### 遗留问题（供编排者裁决，本轮未做）

1. **日志无限增长**：`sync_changes` 只追加不裁剪，长期运行会一直变大。压实（同一 todo 的历史操作折叠）需要先定「设备最久允许离线多长时间」这个产品口径，超出本子任务范围；与 05 备份容灾同属运维面，建议在 05 或另立子任务处理。
2. **日志未按账号/设备隔离**：当前全库共享一条日志，任何设备 `cursor: 0` 就能拉到全部变更。这是改动前的既有形状（内存态同样如此），本任务未改变它；归属 07「登录绑定同步」。
3. **`TodoPatch` 无法表达「显式置空」**：`completedAt: null` 经 `Option<String>` 解码为 `None`，再序列化时被 `skip_serializing_if` 略去，因此「清空某字段」与「不改该字段」在线上不可区分（走查中真实观察到：应用推出的 `"completedAt": null` 在库里与回包里都不出现）。这是 03 定下的契约形状，与本任务的存储改造无关、前后行为一致，但它会让「远端清空截止日」这类操作失效；建议由契约侧另立任务评估（`Option<Option<T>>` + `deserialize_with`）。

## 评审记录

### 第 1 轮

结论：无阻塞。分层与依赖方向、事务边界、重启不丢、同步语义、坏行口径、错误映射、启动策略逐项核对通过（核对要点见本轮末尾「核对结论」）。以下 7 条均为建议级。

- 建议 · crates/server/src/lib.rs:106-111 · handler `async fn sync` 在 async 上下文里直接调用同步阻塞路径（`Mutex<Connection>` + `synchronous = FULL` 的 fsync 提交），改动前临界区是纯内存操作（微秒级），改动后每次写提交都要落盘；并发请求数超过 Tokio worker 线程数时，全部 worker 会阻塞在同一把连接锁上，`/health` 与新连接接受一并受影响。当前无吞吐类验收标准且实测 24 并发正确，故不阻塞；建议记录该取舍或将 `sync_service.sync` 放进 `spawn_blocking` · AGENTS.md 通用「先想后写：假设与困惑都要摆明」
  - 处理（编排者）：转任务处理，并入 数据与同步/06-监控告警。评审已确认行为正确、无吞吐类验收标准，现在改成 `spawn_blocking` 属没有度量支撑的优化；06 要建健康端点与告警，正好把同步请求时延纳入观测——有数据说话时再决定改不改。
- 建议 · crates/server/src/sync_store.rs:228-240 · `with_log` 的文档把「两个请求不会从同一 revision 起编号」归因于「一个事务」，但真正提供互斥的是 `Mutex<Connection>`：`Connection::transaction()` 是 `BEGIN DEFERRED`，两个连接各自先读 `MAX(revision)` 再 INSERT 时，后升级为写事务的一方在 WAL 下拿到的是 `SQLITE_BUSY_SNAPSHOT`（该错误不走 busy handler，rusqlite 默认的 5s busy_timeout 对它无效），并不是「排队后重算」。今天服务端只开一条连接，结论正确、行为正确，但归因写错会在后续（05 备份恢复期间的第二个连接、或多进程部署）被当成已有保证。建议把注释改成陈述「单连接互斥 + 事务原子性」这个真实前提，或改用 `transaction_with_behavior(Immediate)` 让不变式不依赖进程拓扑 · AGENTS.md 通用「不删除既有代码注释」的另一面：注释须与实现事实一致
  - 处理（编排者）：转任务处理，并入 数据与同步/05-备份容灾。评审点明这条归因错误的危险在于「05 可能引入第二个连接」——那时 `BEGIN DEFERRED` 的竞态就从不可达变为可达。把它交给会真正制造第二个连接的那个子任务，比现在改一句注释更能防住问题。
- 建议 · crates/server/src/lib.rs:269-302 · 缺一条自动化断言：「重启后推入一条**新**操作，其 revision 接在旧最大值之后」。现有用例里 `the_log_a_restarted_server_serves_is_the_one_it_stopped_with` 只读回、`an_offline_backlog_pushed_after_a_restart_is_recorded_once` 走的是去重路径（不分配 revision）、`a_rejected_request_leaves_nothing_behind` 的新操作被校验拦下。该性质目前仅由人工走查（跑法二第 2 步 revision 6、7）与本轮评审的端到端复核（重启后 op-4 得到 revision 4）覆盖，回归时测试套抓不住 · 验收标准 1「服务端重启后操作日志与数据完整」
  - 处理（编排者）：转任务处理，并入 数据与同步/05-备份容灾。「重启后编号接续」正是恢复演练要断言的核心性质，05 做从快照拉起服务的演练，这条断言在那里既有归属又有真实场景。
- 建议 · crates/server/src/sync_store.rs:139-161 · 坏行每被读一次就 `warn!` 一次：一条永久读不回的行会在此后每个 `cursor` 低于它的同步请求上重复刷日志。端侧 02 的口径是 `INSERT OR IGNORE` 进 `todo_quarantine`，同一坏值只记一次并留下可查询的诊断面；服务端这里既没有一次性记录，也没有计数器，长期只剩重复噪声。不建隔离表的理由（日志只追加、生产代码里对 `sync_changes` 只有 INSERT/SELECT，原字节由表自身保管）成立，此条只针对诊断的可读性 · 02 建立的「保留字节 + 隔离 + 诊断」口径中的「诊断」一环
  - 处理（编排者）：转任务处理，并入 数据与同步/06-监控告警。坏行的诊断面属可观测性建设，06 是它的正确归属。
- 建议 · crates/server/src/sync_store.rs:151 · `patch` 列的 JSON 载荷没有版本兜底：`TodoPatch` 带 `deny_unknown_fields`，一旦契约加了新字段、随后服务端二进制回滚到旧版本，新字段写下的那些行会整行 `serde_json::from_str` 失败 → 被静默跳过 → 所有设备永远拉不到这些操作（只剩一条 warn）。`SCHEMA_VERSION` 的文档注释只声明了「列布局」的前后兼容，未覆盖列**内**的 JSON 形状；改动前 patch 只活在单个进程的内存里，不存在这个降级窗口，是本次落盘新引入的面。今日无任何字段能触发，且 AGENTS.md 端关系节明确「当前未引入显式版本号机制」、兼容手段是重生成 bindings 并同步全部调用方（不承诺降级），故不阻塞 · AGENTS.md 端侧「契约字段变更以重新生成 bindings 并同步全部调用方为兼容手段」
  - 处理（编排者）：转任务处理，并入 数据与同步/05-备份容灾，且列为该子任务的**必办项**。「服务端二进制回滚后新字段行整行读不回并被静默跳过」是落盘新引入的面，回滚安全属容灾范畴；05 必须给出 patch JSON 的前后兼容策略（`deny_unknown_fields` 在存储读取路径上是否该放宽），而不只是做快照。
- 建议 · crates/server/src/sync_store.rs:327-357 与 crates/server/src/lib.rs:193-225 · `TempDatabase` 在两个测试模块里各写了一份，差别只在 `open` / `restart` 一个方法；重复的是 WAL 边车清理逻辑（`""`/`-wal`/`-shm` 三个后缀），改一处漏一处时表现为测试间歇残留文件 · AGENTS.md 通用「简约至上 / 手术式改动」
  - 处理（编排者）：本次不处理，已知悉。测试夹具在两个模块各有一份，属可接受的重复；AGENTS.md「简约至上」反对的是为不存在的需求做抽象，不是要求测试代码 DRY。
- 建议 · crates/contracts/src/lib.rs:281-346（遗留问题 3：`TodoPatch` 无法表达「显式置空」）· 判定为**非本任务阻塞，应转契约侧任务**。依据三条：① 行为前后一致——改动前 `SyncService` 同样持有反序列化后的 `TodoPatch` 并用同一套 `skip_serializing_if = "Option::is_none"` 序列化进响应（见 `git show HEAD:crates/server/src/lib.rs`），落盘只是在中间多加了一次同构往返，没有引入也没有放大该缺陷；② 缺陷根因在 `crates/contracts/` 的 `Option<T>` 形状（03 定下）与视图端 `applyRemoteUpsert` 的 `!== undefined` 合并约定，不在本任务范围（「做 服务端库表、SyncService 存取改造」）；③ 两条验收标准分别是「重启不丢 + 同步回归」与「cargo test 通过」，回归的判据是与改动前行为一致，该缺陷不构成回归。本轮已独立复现：向 `/v1/sync` 推 `patch: {"dueDate": null}`，回包与库中该行的 patch 均为 `{}`，远端清空截止日确实失效。建议按开发的提议（`Option<Option<T>>` + `deserialize_with`）另立契约侧任务，同时需一并改视图端的合并与 bindings · workflow.md 全局规则 5「禁止擅自扩大或缩小范围」＋ 母任务拆解清单 04 行的验收「服务端重启后操作日志与数据不丢，同步回归通过」
  - 处理（编排者）：转任务处理，并入 任务管理/03-基础字段扩展，且列为该子任务的**前置必办项**。评审已核实这不是本任务引入的回归（改动前同一形状、同一行为），根因在契约的 `Option<T>` 形状与视图端 `applyRemoteUpsert` 的 `!== undefined` 合并约定；而「清空截止日/清空备注」正是 任务管理/03 要交付的编辑交互，缺了它那些字段只能设不能清。

核对结论（供编排者参考，非问题条目）：

- **分层与依赖方向**：`git status` 显示改动仅落 `crates/server/`（`lib.rs` / `main.rs` / `Cargo.toml` / 新增 `sync_store.rs`）与 `.gitignore` / `Cargo.lock` / `TODO.md` / 本任务文件，与「实现记录」清单逐项对齐，无清单遗漏、无 01/02/03 的未提交残留。`crates/domain/` 与 `crates/contracts/` 零改动，SQLite 未渗入 domain；游标推进仍走 `todo_domain::next_sync_cursor`。`crates/server/` 只加了 service（`SyncService`）与数据访问层（`sync_store`，对应 AGENTS.md 架构节「服务端……与未来的数据访问入口」）；handler `async fn sync` 与 `create_router` 的 `/v1/sync` 注册一字未动。
- **并发反例检索**：未找到成立的反例。`SyncStore` 全进程只有一条 `Connection`，被 `Mutex` 包住，`with_log` 先拿锁再开事务，因此「读 `MAX(revision)` → 判重 → INSERT → 读回」四步天然串行，`BEGIN DEFERRED` 的竞态在本拓扑下不可达（归因表述问题已单列建议）。实测 24 个并发 `/v1/sync` 各推一条新操作：24 个 `nextCursor` 互不相同（5–28），全量拉取的 revision 序列为 1–28 连续，无重号、无跳号。批内失败（`CursorExhausted` 或校验不通过）由 `Transaction` 的 Drop 回滚，用例 `a_failed_action_leaves_the_log_as_it_was` 与 `a_rejected_request_leaves_nothing_behind` 覆盖。
- **重启不丢**：`appended_changes_are_still_there_after_closing_and_reopening_the_file` 用作用域块让 `SyncStore`（连带 `Connection`）先 Drop 再对同一临时文件重开，是真关闭再打开而非同连接读回；lib 侧 `TempDatabase::restart()` 同理。`synchronous = FULL` 的非对称性论证成立：服务端一旦回包「已受理」，设备即删待推队列，此时若提交只在页缓存中，掉电后该操作全世界零份；端侧丢的是本地最近一次提交，服务端仍是可回取的对端副本，损失面确实不同。（论证中「用户还在，数据还在库里」一句偏乐观——端侧 NORMAL 同样会丢掉未 fsync 的最后一次提交；但这不改变结论，代码注释本身只陈述服务端一侧的事实，准确。）
- **同步语义不回归**：端到端复核（`cargo run -p todo-server` 产物 + `taskkill /F` 硬杀）确认离线补推 3 条一次落库 → 同批重推全部 acked 且 revision 仍为 1/2/3 不重复记账 → 硬杀重启后新设备 `cursor: 0` 拉回 3 条原样、`nextCursor: 3` → 重启后推新操作得 revision **4**（未从 1 重编）→ `cursor: 4` 增量拉取为空。字段级合并的前提（服务端交还的字段集与 revision 顺序原样）由 `a_patch_keeps_exactly_the_fields_it_carried`、`field_level_patches_reach_a_restarted_server_intact` 锁住，且合并发生在设备侧、本任务未触及。关于开发用本机 `sync.pending` 5 条真实积压做样本、称新操作接在 6、7 的证据：样本本身不可复核（走查后环境已还原），但它要证的性质是确定性的且与样本无关——本轮用独立的临时库复现了同一性质（重启后接 4 而非 1），故该结论成立；证据链的可信部分是 revision 连续性而非那 5 条的来历。
- **坏行处理**：读不回即跳过 + `warn`、不降级，与 02/03 的「保留字节 + 隔离 + 诊断」在「保留字节」和「不就地清零」两点上一致；不建隔离表的理由站得住（端侧每次 save 重写全部列，不抢救就会被下一次写抹掉；服务端 `sync_changes` 生产代码只有 INSERT 与 SELECT，坏行原字节由表自身保管，隔离表只是多一份拷贝）。用例 `an_unreadable_row_is_skipped_instead_of_failing_the_whole_read` 确实同时断言了好行照常返回与 `COUNT(*)` 仍为 3。
- **`kind` 不降级 / 错误映射**：`kind_from_text` 返回 `Option` 并报「读不回」而非猜 upsert，理由（upsert 与 delete 是相反动作）成立且写在函数上方。`ServerError::Storage(SyncStoreError)` 与 `CursorExhausted` 合并为 500 且响应体固定为 `{"error":"sync state is unavailable"}`，SQLite 原文只进 `tracing::error!`，无内部细节外泄；`InvalidRequest` → 400 + 契约校验文案是改动前既有行为（实测返回 `{"error":"a title must contain 1 to 500 non-whitespace characters"}`）。
- **`DATABASE_PATH` 与启动策略**：默认值 `todo-server.db` 与既有 `HOST`/`PORT`/`CORS_ORIGIN` 同风格；`.gitignore` 三行覆盖默认文件与 `-wal`/`-shm` 边车，端到端跑完后仓库 `git status` 无多余文件，恰当。开库失败即退出符合 AGENTS.md——「原生侧失败必须容错降级」写在「#### 客户端」节下，约束的是有用户在场、有回退存储的 Tauri 侧；「#### 服务端」节无对应要求，且服务端无库时只能把操作确认进虚空，fail-fast 是正确取舍。`serde_json = "1"` 直接声明（不走 workspace）与 `crates/contracts`、`src-tauri` 的既有写法一致。

验证：`cargo test --workspace` 通过（45 passed / 0 failed：cross_platform_todo_lib 20、contracts 8、domain 1、server 16，与自验数一致）；`cargo check --workspace` 通过，无警告；`pnpm run check` 通过（40 文件格式、42 文件 lint 与类型零告警）；`pnpm run types:generate` 未运行——`crates/contracts/` 零改动，无输入变化，符合 AGENTS.md 契约变更条件；`cargo run -p todo-server` 产物端到端复核通过——离线补推 / 重放去重 / `taskkill /F` 硬杀重启后日志与游标完整 / 重启后新操作 revision 接续为 4 / 24 并发推送 revision 1–28 连续无重号 / 非法请求 400、存储错误不泄漏内部细节。
