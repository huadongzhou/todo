# Rust调度底座

- 级别：直达
- 状态：完成

## 需求（产品）

- 目标与场景：提醒调度从前端 setTimeout 迁至 Rust 侧，应用未运行（驻托盘）时提醒不丢。
- 范围（做 / 不做）：做 调度器、启动重建调度表、逾期补发；不做 周期/交互/策略（02–07）。
- 验收标准（逐条可检查）：
  - [ ] 主窗关闭驻托盘时到点提醒仍触发；重启后调度表重建无遗漏。
  - [ ] 完全退出后再启动，逾期提醒补发并有「逾期」标注。
  - [ ] cargo test --workspace 通过（调度表重建逻辑）。

## 规格（UI/UX）

## 实现记录（开发）

### 架构决策（提醒时序：前端 setTimeout → Rust 轮询）

- **现状确认**：改前提醒完全在前端。`src/lib/notifications.ts` 用 `setTimeout` 排程，`src/stores/todos.ts` 的 `add`/`refreshReminder`/`rescheduleAll`/`deleteTodo` 驱动之；`main.ts` 启动调 `rescheduleAll`。`src-tauri/src/lib.rs` 只有托盘/窗口生命周期（关闭默认隐藏到托盘），无任何提醒调度。问题：`setTimeout` 只在 webview 存活时有效——完全退出后不触发，重启也只是「立即补发但无逾期标注」。
- **调度器持有方式**：**轮询 SQLite**，不用内存定时器。后台 `std::thread`（无新依赖）每 30s 一次，经既有 `TodoDb`（同一 `Mutex<Connection>`，与 IPC command 共用，无第二连接、无 WAL 写冲突）读全部 todo + 已投递集，交纯规则判定。选轮询而非内存定时器：调度表 = 数据本身，进程重启后「重建」即「下一次轮询」，无需单独重建步骤；gating（完成/归档/依赖锁定）从数据现算，前端无需喂给 Rust。
- **重启后重建 + 防重发**：新表 `reminder_deliveries(todo_id, reminder_at, delivered_at, PK(todo_id,reminder_at))`（`CREATE TABLE IF NOT EXISTS`，同 quarantine 表无独立 schema 版本）。发一条记一条；判定时跳过已在集合中的 (todo_id, reminder_at)。重启后读回该表即「重建调度表不遗漏、不重发」。键含 reminder_at：改提醒时间＝新键→会再发，未改＝原键→静默；周期实例（08）在 store 侧位移 reminder_at 或新 id，天然落新键，无需 Rust 感知周期规则。
- **逾期补发与标注**：`now - reminder_at > 60s 宽限` 判为逾期（宽于一个轮询间隔，故运行期到点稍晚不误标；关机期间错过则数分钟/小时级延迟＝逾期）。**标注形态 = 通知文案标注**（非数据标注，避免动契约）：`reminder_scheduler.rs::notification_text` 逾期时标题加「【逾期】」前缀，正文沿用前端原文案（有截止日「截止日：X」/ 否则「提醒时间到了」）。
- **前端如何处理（不两套并行）**：`src/lib/notifications.ts` 的 `scheduleReminder`/`cancelReminder`/`cancelAllReminders` 降级为**惰性**（`scheduleReminder` 返回 `Promise.resolve(false)`），删除 `setTimeout` 引擎（timers/fire/dueBody/clamp）。边界：**Tauri 桌面走 Rust 调度**；浏览器 dev 本就无后台（原 `!isTauri()` 即 no-op）；移动端后台调度归 2.3。store 侧钩子保留但成惰性调用——它仍把正确状态写进 SQLite，Rust 每轮从同一数据重导 gating，故 gating 语义不靠前端钩子维持。`notify`（即时 ad-hoc 通知，既有且未被调用的公开 API）与其权限路径按「既有死代码只标记不删除」保留。
- **gating / 依赖 12 / 周期 08 关系**：`due_reminders` 用 `todo_domain::dependency::dependency_lock` 现算锁定态，`status`/`archived_at` 现判完成归档——与前端 `isLocked`/`refreshReminder` 同一口径。前置完成→store 持久化状态→下一轮 Rust 见解锁且过期即补发（逾期）。因此「锁定不排提醒」「解锁补发」「周期实例重排」均无需迁移前端钩子，由 Rust 读数据保持。
- **command / capability**：**未新增 command**——调度器是 setup 里起的后台线程，前端无需调用。**未改 capability**——Rust 端经 `NotificationExt` 直发通知，不经 webview→core 的 capability 门（capability 只管 JS 侧 invoke）；`default.json` 的 `notification:default` 保留给（惰性保留的）`notify`/权限 JS 路径，卡片窗 `today-card.json` 未动。启动时 Rust 侧 best-effort `request_permission()`（macOS 用，Windows 无碍），失败仅 `log::warn!`。
- **容错**：线程 spawn 失败、单轮读库失败、单条通知 show 失败，均 `log::warn!` 后继续，不中断应用（AGENTS 客户端规范）。record 失败保留下轮重试（宁可极端下重一次，不丢——母任务「提醒零丢失」）。
- **未做（属 02–07/2.3）**：周期提醒接入、snooze、重催、勿扰、晨间摘要、多级/时区、移动端本地通知。本任务仅底座 + 单点提醒；`#[cfg(desktop)]` 门为移动端预留接入位。

### 改动文件

- `crates/domain/src/reminder.rs`（新增）——纯规则 `due_reminders(todos, delivered, now_unix, grace) -> Vec<DueReminder>`（到点/逾期/gating 判定）+ 7 项单测。
- `crates/domain/src/lib.rs`——注册 `pub mod reminder;`。
- `crates/domain/src/ics.rs`——新增 `pub(crate) fn instant_unix_seconds`（复用既有 `parse_instant`，供 reminder 判定「是否到点」，与日历导出同一 instant 读法）。
- `src-tauri/src/reminder_scheduler.rs`（新增，`#[cfg(desktop)]`）——后台线程、30s 轮询、`notification_text`（逾期标注）、通知直发与投递记录 + 2 项单测。
- `src-tauri/src/todo_db.rs`——新增 `reminder_deliveries` 表（`initialise` 建表）+ `delivered_reminders()`/`record_reminder_delivery()` 方法 + 2 项单测；扩充 unavailable 不变式测试。
- `src-tauri/src/lib.rs`——`#[cfg(desktop)] mod reminder_scheduler;`，setup 中 `#[cfg(desktop)] reminder_scheduler::spawn(app.handle())`。
- `src/lib/notifications.ts`——排程入口降级为惰性、删除 setTimeout 引擎、保留 `notify`/权限路径与 `Reminder` 类型（module 注释说明新架构）。
- `TODO.md`——2.2「后台调度」⏳→✅ 并加实现注记（2.2「周期提醒」仍 ⏳＝任务 02）。

### 自验结果（命令与结论）

- `cargo test --workspace`：全绿。249 passed / 0 failed（基线 238 + domain reminder 7 + src-tauri 投递/调度 4）。含调度表重建逻辑：`reminder::tests`（到点、已投递不重发、改时间再发、宽限内不标逾期·过宽限标逾期、完成/归档不排、无/坏提醒不排、锁定→解锁补发标逾期）、`todo_db::tests`（投递记录读回·重复忽略、重开文件后投递持久化）、`reminder_scheduler::tests`（逾期标题标注、到点纯文案）。
- `cargo check --workspace`：无 warning、无 error。
- `pnpm run check`：57 文件格式通过；60 文件无 lint/类型错误。
- `pnpm run types:generate`：运行成功，`src/bindings/**` 零 diff（本任务无 command/契约/导出类型变更，符合预期）。
- **验证边界**（同前面子任务）：「主窗关闭驻托盘时到点触发」「完全退出后重启逾期补发」需真实桌面运行时观察系统通知；本环境不跑 `tauri:dev` 原生走查（避免污染用户 appdata、且非交互环境无法观测 toast）。上述行为的**判定/持久化/标注逻辑已由 `cargo test` 覆盖**，实时触发链（线程调度→通知插件 show）留待评审/整合验收在真实桌面确认。浏览器 dev 无 Rust 调度，不适用于本任务走查。

### 遗留 / 提示

- 首次升级到本版本时，历史「已过期但从未记录投递」的提醒会在首轮轮询按逾期补发一次（新表初始为空）。当前 app 尚无正式用户（v1 开发期），属可接受的一次性迁移现象，未做特殊抑制。
- `reminder_deliveries` 只增不删；个人规模下行量可忽略，未做清理（按 delivered_at 清理不安全——会让仍存在的过期提醒重新补发）。如后续需回收，应在对应 todo 删除或 reminder_at 变更时联动清理，非本任务范围。
- 通知 `show()` 从后台线程调用在 Windows 直发可用；macOS 首次授权依赖 `request_permission()`（已 best-effort 接入）。跨平台通知授权细节属真实运行时验证项。

## 评审记录

### 第 1 轮

通过。范围＝「实现记录」改动文件清单，经 `git status`/`git diff` 核对完整（7 改 + 2 新增，与清单一致；无清单外改动，工作区无 01–13 任务管理与数据同步兄弟子任务的残留）。架构迁移逐项独立核查，均无阻塞：

- 调度正确性（`crates/domain/src/reminder.rs::due_reminders`）：独立复核判定链——完成/归档（`!Open || archived_at.is_some()`）、无/坏提醒（`instant_unix_seconds` 返 `None` 即跳，不落 epoch）、未到点（`reminder_unix > now` 跳）、已投递、依赖锁定（`dependency_lock` 现算）逐一 gating；`reminder_at == now` 判到点且不逾期，`now-reminder_at` 恰 60s 仍为准点、>60s 标逾期（严格 `>`，边界正确）。7 项单测覆盖到点/已发不重/改时再发/宽限内外/完成归档/无坏提醒/锁定→解锁补发。
- gating 口径一致（攻 12/08）：Rust `due_reminders` 的完成·归档·`dependency_lock` 与前端 `stores/todos.ts` 的 `isLocked`/`lockState`/`refreshReminder` 同源——未解析前置计已满足（`!known || completed` ⇄ `!dep || completed`）两端一致；归档必蕴含完成，Rust 额外 `archived_at` 门为超集更保守，不漏排、不误排。迁移未丢任何 gating。
- 防重发/重建（`reminder_deliveries`，PK `(todo_id,reminder_at)`）：`INSERT OR IGNORE` + 读回集判定；改时间＝新键再发、未改＝原键静默、重启读回不重发（`reminder_deliveries_survive_reopening_the_file` 覆盖）。周期实例（08）store 侧换 id/位移 `reminder_at` 天然落新键，不漏不重。并发重发不成立：仅单 `reminder-scheduler` 线程，`sleep→poll_once→sleep` 串行，`poll_once` 内每 `(id,at)` 至多一次，无轮次交叠。表只增不删属已记录取舍（按 `delivered_at` 清理会令仍存的过期提醒复发），无正确性风险。
- 单连接不变式：调度线程经 `app.state::<TodoDb>()` 取同一被管理实例，`list`/`delivered_reminders`/`record_reminder_delivery` 全走 `with_connection` 锁同一 `Mutex<Connection>`，无第二连接——与数据同步/04-05 确立的 revision 不撞号前提一致。
- 不两套并行：`src/lib/notifications.ts` 的 `scheduleReminder` 恒返 `false`、`cancel*` 为空、setTimeout 引擎已删；桌面仅 Rust 一条 `app.notification().show()` 发送路径，`notify`（未被调用的 ad-hoc API）按「死代码只标记不删」保留、不参与提醒。浏览器 dev/移动端无 Rust 调度即无后台提醒，不崩。无残留双发路径。
- 容错与门控：线程 spawn 失败、单轮读库失败、单条 show 失败、record 失败均 `log::warn!` 后继续，不中断应用（AGENTS 客户端规范）；`#[cfg(desktop)]` 正确门控模块声明与 `spawn` 调用，移动端不编译进本套；`tauri_plugin_notification::init()` 已注册，`app.notification()` 可用。
- 采信边界：开发未跑原生 GUI 走查（避免污染用户 appdata）可接受，同前面子任务先例——判定/持久化/标注/投递逻辑已由 `cargo test`（249 全绿）覆盖；「驻托盘到点触发」「完全退出后重启逾期补发」的实时触发链（线程→通知插件 show）、跨平台通知授权，属真实桌面运行时验证项，留待集成验收在真实桌面确认。评审侧亦未向 production appdata 写入数据。

验证：cargo test --workspace 通过（249 passed / 0 failed）；cargo check --workspace 通过（无 warning/error）；pnpm run check 通过（57 格式化 / 60 无 lint·类型错误）；pnpm run types:generate 通过，`src/bindings/**` 零漂移（本任务无 command/契约变更，符合预期）。
