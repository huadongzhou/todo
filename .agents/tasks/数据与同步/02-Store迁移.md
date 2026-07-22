# Store迁移

- 级别：直达
- 状态：完成

## 需求（产品）

- 目标与场景：既有 Store 键值数据一次性迁入 SQLite，用户无感、不丢数据。
- 范围（做 / 不做）：做 启动检测、事务式导入、完成标记、失败回退；不做 SQLite 本体（01）。
- 验收标准（逐条可检查）：
  - [ ] 旧数据全量迁入且字段无损，迁移标记幂等（二次启动不重迁）。
  - [ ] 迁移失败自动回退 Store 读取路径，应用可用不丢数据。
  - [ ] 双跑验证（有旧数据 / 全新安装）记录在实现记录。

## 规格（UI/UX）

## 实现记录（开发）

### 先决调查：旧数据到底落在哪里（实地确认，未凭想象）

任务描述假定迁移源是「Tauri Store 插件持久化的旧待办」，**实测结论是：Store 从未存过待办**，本任务据此调整了迁移源的认定。证据三条：

1. `git show df1db9e:src/stores/todos.ts`（01 之前的最后一版）——store 里只有内存 `ref<Todo[]>`，`add/update/toggle/remove` 全无任何持久化调用；再往前 `4316612` 同样没有。待办持久化是 01（`cfc2298`）才首次引入的。
2. `git log --all -S "todos.json"` 无任何命中；仓库里 Store 文件只有一个 `settings.json`（`src/lib/settings-storage.ts:6`、`src-tauri/src/lib.rs:82`），写入方只有主题、`behavior.closeToTray`、`ui.showTodayCard` 与同步态四键（`src/lib/sync-engine.ts:82-144`）。
3. 直接读本机实文件 `%APPDATA%/com.todo.crossplatform/settings.json`：键为 `sync.pending` / `sync.deviceId` / `ui.showTodayCard` / `appearance.theme`，无待办条目。AGENTS.md 与 TODO.md 6.1 所说的「Store 键值持久化」指的是设置域，不是待办。

因此**唯一真实存在的「SQLite 之外的待办数据」是 `localStorage["todos.items"]`**：它既是浏览器开发模式的主存储，也是 01 给桌面端留的降级回退键（`todo-repository.ts` 原生失败时写它）。在 Tauri 桌面运行时它由 WebView2 按源持久化，重启后仍在，是真实的、会丢的用户数据。本任务的迁移就以它为源；未为「Store 里的待办键」写读取代码——那是一条不存在的分支（AGENTS.md「不为不可能的分支写兜底」）。这同时说明：01 评审建议 (a) 的「开库失败期间产生的数据不可见」与本任务的「旧数据迁入」是同一条链路上的同一件事，合并处理是正确的。

### 编排者并入的两条 01 评审建议（本任务范围的正式扩展）

- 来源：`01-端侧SQLite与repository.md` 评审第 1 轮建议 (a)「`load` 只要原生 ok 就不读回退键 → 静默丢数据，缺回收路径」，编排者裁决「转任务处理，并入 02」。**已实现**，见下文「回收路径」与「失败回退」两点。
- 来源：同轮建议 (b)「`save`/`remove` 返回 `Promise<void>` 且吞掉失败，调用方拿不到是否落库的信号」，编排者裁决「接口形状随 02 一并定稿」。**已实现**：新增 `WriteOutcome` 并作为 `save`/`remove` 的返回类型。

### 设计要点

- **迁移源与判据**：启动时读 `localStorage["todos.items"]`，非空则整批交给新命令 `import_legacy_todos`；空则直接跳过（`{status:"skipped", reason:"no-legacy-data"}`）。
- **事务式导入**：`TodoDb::import_legacy` 用一个 `unchecked_transaction` 包住「N 行插入 + 版本位写入」，要么全部落库、调用方才敢丢弃旧副本，要么什么都没变、旧副本仍是唯一一份。
- **完成标记**：沿用 01 的 `PRAGMA user_version`。`SCHEMA_VERSION = 1` 仍是新建库的打版值，新增 `LEGACY_IMPORT_VERSION = 2` 表示「一次性迁移已提交」，由导入事务写入。选它而不是新建 meta 表，是因为它不动表结构、且与 01 留下的版本位是同一套判据（03 继续往上叠即可，版本位退化为标准的「已应用迁移序号」）。
- **幂等**：三重保证——① 导入语句用 `ON CONFLICT(id) DO NOTHING`，同 id 绝不重复插入、也绝不覆盖 SQLite 里的现行行（库里的是活数据，回退键里的是旧副本，方向只能是这一个）；② 成功后清空 `todos.items`，下次启动无源可迁；③ `user_version` 留痕。
- **失败回退（真的能退）**：导入失败时事务回滚（0 行落库、版本位不动），`todos.items` 一个字节都不动，只写一条 warn，下次启动自动重试。同时 `sqliteRepository.load` 改为「原生行 + 仍滞留在回退键里的行」合并返回（同 id 以原生行为准，按 `created_at DESC, id` 重排以对齐 SQL 的排序），因此迁移失败期间这些数据照常显示、照常可用，不会像 01 那样被一条 ok 的原生读悄悄藏起来——这正是建议 (a) 要的回收路径的另一半。
- **不复活已删除项**：原生删除成功后同时清掉回退键里的同 id 副本，否则下一轮导入会把已删的待办重新插回来。
- **浏览器运行时不动**：`migrateLegacyTodos` 首行即 `if (!isTauri()) return skipped/browser-runtime`——浏览器下 localStorage 是主存储而不是遗留物。

### 改动文件

- `src-tauri/src/todo_db.rs`：新增常量 `LEGACY_IMPORT_VERSION = 2` 与语句 `INSERT_LEGACY_TODO`（`ON CONFLICT(id) DO NOTHING`）；新增 `TodoDb::import_legacy(&[Todo]) -> Result<u32, TodoDbError>`（单事务：批量插入 + 版本位写入 + commit，返回实际插入行数）；`initialise` 的告警分支由「`stored != SCHEMA_VERSION` 就告警」改为「`stored > LEGACY_IMPORT_VERSION` 才告警」，否则迁移完成后每次启动都会误报（写入逻辑本身未变，仍只在 `stored == 0` 时打版）；新增 3 个用例并给既有的「不可用库」用例补一条 `import_legacy` 断言。
- `src-tauri/src/lib.rs`：新增 `#[tauri::command] #[specta::specta] fn import_legacy_todos(db, todos: Vec<Todo>) -> Result<u32, String>`，注册进 `collect_commands!`。实体仍直接复用 `todo_contracts::Todo`，未新建重复实体、未改契约。
- `src/bindings/commands.ts`：`pnpm run types:generate` 生成产物（未手改），新增 `importLegacyTodos`。
- `src/lib/todo-repository.ts`：
  - 新增导出 `WriteOutcome = "stored" | "degraded" | "failed"`，`TodoRepository.save`/`remove` 由 `Promise<void>` 改为 `Promise<WriteOutcome>`（建议 (b) 的接口定稿）。`"stored"` ＝ 落到本运行时的主存储（桌面 SQLite / 浏览器 localStorage），`"degraded"` ＝ SQLite 拒收、副本只在回退键里，`"failed"` ＝ 两处都没留住。回退写入函数 `writeBrowserTodos` 相应改为返回 `boolean`，不再把失败吞成 void。
  - 新增导出 `migrateLegacyTodos(): Promise<MigrationOutcome>` 与 `MigrationOutcome`（`skipped(browser-runtime|no-legacy-data)` / `imported{legacy,inserted}` / `failed{error}`），三种结果均有对应诊断日志（skipped 不打日志，避免每次启动噪音）。
  - `load` 增加滞留数据合并（见上）；`remove` 原生成功后清理回退键同 id 副本；新增 `clearBrowserTodos`、`removeBrowserTodo`、`saveBrowserTodo`、`listNativeTodos`、`newestFirst` 五个内部函数（`saveBrowserTodo` 是原 `browserRepository.save` 内联逻辑的原样提取，供两条路径共用）。
- `src/main.ts`：`bootstrapApp` 内、`hydrate()` 之前 `await migrateLegacyTodos()`。位置在既有的 `if (isCardWindow) return;` 之后——卡片窗口不读待办 store，也不该跑迁移。
- `TODO.md`：6.1「数据迁移：Store → SQLite 一次性迁移」⏳ → ✅，并按实测把描述改为「启动时把 SQLite 之外的旧待办事务式导入（含开库失败期间落在回退存储的数据），user_version 记迁移标记」。
- 未改动：`src/stores/todos.ts`（`void repository.save(...)` 对新返回类型无需调整）、组件、`crates/**`、capabilities。

### 自验结果（命令与结论）

- `cargo test --workspace`：通过，16 passed / 0 failed（`cross_platform_todo_lib` 由 8 → 11，新增 3 个 `todo_db` 用例；contracts 1、domain 1、server 3、bindings 导出 1）。新增用例：
  - `importing_legacy_todos_stores_them_and_records_the_migration`：导入前版本位 = 1，导入 2 条后行数 2、版本位 = 2。
  - `a_second_import_neither_duplicates_nor_overwrites_stored_todos`：先导入，再改标题存回（模拟导入后应用继续使用），然后把同一批旧数据再导一次——插入 0 行、总行数仍 1、标题保持「导入后编辑过的」值。
  - `a_failed_import_leaves_neither_rows_nor_the_migration_marker`：用 `BEFORE INSERT … RAISE(ABORT)` 触发器让批次中第二行失败——返回 `Err`、表内 0 行（第一行已回滚）、版本位仍是 1（标记未写）。这条就是「失败不破坏旧数据」的机制证明。
- `cargo check --workspace`：通过。
- `pnpm run types:generate`：通过；`src/bindings/commands.ts` 新增 `importLegacyTodos`（+7 行），`src/bindings/models/**` 无变化（未动契约）。
- `pnpm run check`：通过（40 文件格式、35 文件 lint + 类型检查零告警）。
- `cargo fmt --all -- --check`：本次新增的 Rust 代码零 diff（仓库仍有 3 处**既有**未格式化点：`lib.rs:13` import 顺序、`todo_db.rs:33` `SELECT_TODOS`、`todo_db.rs:303` 断言换行——均为 01 遗留，按「不改无关代码」未动）。

### 双跑验证（桌面 `pnpm run tauri:dev` 实测，可复现）

环境已可正常挂载 GUI，以 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222 pnpm run tauri:dev` 启动，用 CDP 在真实运行时内取证。走查前把 `%APPDATA%/com.todo.crossplatform` 与 WebView2 `Local Storage` 整目录备份，走查后原样还原（末尾有校验）。

**跑法一 · 全新安装**（删 `todos.db*` ＋ 删 WebView2 `Local Storage` 后启动）

- 界面正常挂载（标题「待办」），启动日志只有 `Settings restored` / `Todos restored from local storage {count:0}` / `Reminders rescheduled on startup`，**没有**迁移日志 → 无旧数据时迁移不做任何事。
- `localStorage["todos.items"]` = `null`，`list_todos` = 0 条，`todoRepository().load()` = 空。
- 在真实 UI 输入框里提交「全新安装走查项」并 `submit`：页面显示「1 项待完成」，`list_todos` 立刻返回该条，`todos.items` 仍为 `null` → 新装机路径完全走 SQLite，迁移未污染。
- 直接读库：`PRAGMA user_version` = **1**（只有建库打版，迁移标记未写）→ 「全新安装不触发迁移」有硬证据。

**跑法二 · 有旧数据**（在运行中的 webview 里把 4 条旧待办写进 `todos.items` 后重启应用，模拟老版本升级）

- 种入的 4 条覆盖了字段与冲突两种情况：`legacy-1`（带 `dueDate` + `reminderAt`）、`legacy-2`（`status=completed` 且带 `completedAt`）、`legacy-3`（最新 `createdAt`）、以及一条**故意与 SQLite 现有行同 id** 的旧副本（标题「旧副本：不得覆盖 SQLite 现有行」，`createdAt` 为 2000 年）。
- 重启后启动日志出现 `info: Legacy todos imported into SQLite {"legacy":4,"inserted":3}` → 4 条交付、3 条插入、1 条（冲突那条）被跳过。
- `list_todos` 返回 4 条，字段逐条无损：`legacy-1` 的 `dueDate:"2026-07-23"` 与 `reminderAt:"2026-07-23T09:30:00.000Z"` 都在；`legacy-2` 仍是 `completed` 且 `completedAt:"2026-07-21T10:15:00.000Z"`；冲突那条**保持 SQLite 原值**（标题仍「全新安装走查项」、`status` 仍 `open`、`createdAt` 未被 2000 年那份改写）→ 「导入不覆盖现行行」成立。
- `todos.items` 已被清空为 `null`；界面显示「3 项待完成」并列出 4 条（含已完成分组），`legacy-1` 带「今天 · 已设提醒」徽标，`Reminders rescheduled on startup {scheduled:1}` → 迁入的数据被正常渲染与排程。
- 直接读库：`user_version` = **2**，`select count(*)` = 4，逐行 dump 与上述一致。

**跑法二续 · 二次启动幂等**（不改任何数据，再重启一次）

- 启动日志**没有** `Legacy todos imported into SQLite` 那一行；`todos.items` 仍为 `null`；`list_todos` 仍是同样的 4 条、无重复条目；`user_version` 仍为 2 → 「重复启动不重迁、不产生重复条目」成立。

**跑法三 · 迁移失败自动回退旧路径**（在真实运行时内构造失败）

- 把两条数据写进 `todos.items`：一条正常（`stranded-1`「开库失败期间写入的待办」），一条缺 `title`（`broken-1`，被 Rust 契约拒收，代表任意中途失败）。
- `migrateLegacyTodos()` 返回 `{status:"failed", error:"invalid args \`todos\` for command \`import_legacy_todos\`: missing field \`title\`"}`。
- 失败后：`todos.items` **两条都还在**（旧数据未被破坏）；`list_todos` 仍是原来的 4 条 → 事务回滚，没有半截数据落库。
- 关键一条：`todoRepository().load()` 返回 **6 条**——4 条 SQLite 行 ＋ 2 条仍滞留在回退键里的行，且按 `createdAt` 倒序正确插位 → 迁移失败期间应用可用、旧数据可见不丢（这正是 01 建议 (a) 指出的「永久不可见」缺陷的修复点）。
- 去掉那条坏数据后再调一次（等价于下次启动重试）：`{status:"imported", legacy:1, inserted:1}`，`todos.items` 变 `null`，`list_todos` 变 5 条且含 `stranded-1` → 回收路径闭环。

**补充 · 删除项不复活**

- 在回退键里放一条 SQLite 里没有的 `orphan-1`，调 `todoRepository().remove("orphan-1")` → 返回 `"stored"`，回退键变 `[]`；随后 `migrateLegacyTodos()` 返回 `skipped/no-legacy-data`，`list_todos` 里没有 `orphan-1` → 已删除项不会被下一轮导入复活。

**补充 · 浏览器开发模式不回归**（01 验收标准第 2 条的同类回归检查，因本次动了 `browserRepository`）

- 用 headless Edge（`--headless=new --remote-debugging-port=9223`，独立 user-data-dir）打开同一个 vite dev server `http://localhost:1420/`：`window.isTauri` = `false`，应用正常启动（`Settings restored` / `Todos restored from local storage`）。
- `migrateLegacyTodos()` 返回 `{status:"skipped", reason:"browser-runtime"}` → 浏览器下 localStorage 是主存储，迁移绝不碰它。
- `save` → `"stored"` 且 `todos.items` 出现该条；`load` 读回；`remove` → `"stored"` 且键变 `[]` → 浏览器回退路径行为与 01 一致，只是多了返回值。
- 走查后关闭 Edge 并删除临时 user-data-dir。

**环境还原核验**：应用与 Edge 均已退出，`todos.db*` 与 WebView2 `Local Storage` 已由备份还原；还原后读库 `user_version` = 1、`todos` 表 0 行，与走查前一致，未留下任何测试数据。

### 验收标准逐条自查

- [x] 旧数据全量迁入且字段无损，迁移标记幂等（二次启动不重迁）：跑法二证明 4 条全量交付、字段（`dueDate`/`reminderAt`/`completedAt`/`status`）逐项无损、`user_version` = 2；跑法二续证明二次启动不再导入、无重复条目；Rust 侧另有 `a_second_import_neither_duplicates_nor_overwrites_stored_todos` 用例。
- [x] 迁移失败自动回退 Store 读取路径，应用可用不丢数据：跑法三证明失败时事务回滚、旧数据一字未动、`load` 仍把滞留数据合并出来、下次重试可成功；Rust 侧另有 `a_failed_import_leaves_neither_rows_nor_the_migration_marker` 用例。
- [x] 双跑验证（有旧数据 / 全新安装）记录在实现记录：见上「跑法一 / 跑法二」，均为真实 `tauri:dev` 运行时的实测，附启动日志、IPC 返回与直接读库三重证据。

### 遗留问题

1. 【范围声明】本任务未做 03 的新业务字段扩展（按补充要求）。`user_version` 现已成为「已应用迁移序号」：1 = 建库、2 = 一次性旧数据导入；03 扩展字段时应叠到 3 并配套写真正的 schema 迁移，`initialise` 的告警阈值 `LEGACY_IMPORT_VERSION` 届时需一并上移。
2. 【既有缺陷，非本次引入】走查中每次启动都会出现 `Failed to register global shortcut … global-shortcut:allow-is-registered`（`global-shortcut:default` 为空权限集），编排者已另建 `.agents/tasks/修复全局快捷键权限.md`，本次未碰。
3. 【既有】仓库有 3 处既有 rustfmt 未格式化点（`lib.rs:13`、`todo_db.rs:33`、`todo_db.rs:303`），均在 01 引入的代码里，按「不顺手重构」未动；若编排者希望统一，可另起一次纯格式化改动。
4. 【移动端】与 01 相同，Android/iOS 未验证（本机无移动工具链），迁移逻辑本身不含平台分支。

### 第 2 轮（评审回流）

本轮处理第 1 轮全部 6 条（1 阻塞 + 5 条带「本轮处理」裁决的建议）。核心是按编排者要求**重做回退/迁移语义**，而不是在原形状上打补丁。

#### 回退/迁移语义重设计

原设计把 `todos.items` 当「迁移前的旧副本」，用 `ON CONFLICT DO NOTHING` + 成功后整键 `removeItem` 处理；评审指出同一个键同时承载「降级期间产生的活数据」，方向恰好相反，于是新值被跳过后又被全清。新设计换掉的是这个**定位**：`todos.items` 不再是「旧副本」，而是**待写日志（pending-write journal）——SQLite 还欠用户的每一笔写入**，配一个墓碑键 `todos.deletions`；启动时做的不是「一次性导入」而是「把欠的账还清（replay）」。编排者要求的四个问题逐条回答：

- **一条回退键里的记录，怎么区分它是「比库里新的降级写入」还是「比库里旧的历史副本」？** —— 不再需要区分，因为后一类被新不变式消灭了：**原生写一旦成功，就立刻清掉该 id 的日志条目**（`save`/`remove` 的成功分支都调 `clearPendingWrites`）。一条日志条目只可能存在于「SQLite 至今没有接受过这个 id 的更新写入」期间，所以它永远不比库里的行旧。冲突策略因此从「跳过已知 id」改为「覆盖」（复用既有 `UPSERT_TODO`，`INSERT_LEGACY_TODO` 删除），墓碑同理必须删。仅剩的边界是「更早版本写下的裸数组」——按「先决调查」的三条证据，01 之前从未有任何代码把待办写进任何持久化存储，桌面运行时的 `todos.items` 只可能出自 01 的降级分支，故按待写日志解读；这条解读写进了代码注释。
- **清空回退键的前提条件是什么？在什么情况下必须保留？** —— 前提是**逐条的**：某条目只有在命令回报它落在 `applied` 里（即 SQLite 已 commit）时才被删（`dropApplied`）。「全清」动作（原 `clearBrowserTodos`）已删除。必须保留的情况有三类：命令整体失败（库不可用）→ 一条不删；单条被 SQLite 拒收（回报在 `rejected`）→ 该条保留并继续被 `load` 展示、下次启动重试；进程在 commit 与簿记之间崩溃 → 条目留着，下次重放一遍（upsert 与 delete 都幂等，重放无副作用）。
- **删除操作在降级期间如何表达？** —— 墓碑。`remove` 的原生失败分支写 `todos.deletions`（并摘掉同 id 的待写 upsert），只有墓碑真的写下才算 `"degraded"`；`load` 先按墓碑过滤原生行，重放时逐条 `delete`。评审指出的「`remove` 的 degraded 什么都没记、条目从 SQLite 复活」因此不再成立。
- **一条脏数据不得让整批迁移永久卡死。** —— 两段自愈：前端 `isTodo` 逐条校验（规则对齐 Rust 契约的 `deny_unknown_fields` 与必填 `completedAt`），读不回 `Todo` 的条目搬进隔离键 `todos.quarantine`（保留原文 + 时间戳 + warn 诊断），不再堵在批次里；Rust 侧 `replay_pending` 逐条应用、行级失败只进 `rejected`，只有库整体不可用才整体 `Err`。整批回滚后永远重试同一批的形态已消失。

一致性副产物：`user_version` 不再承担迁移标记（见建议 4 的处理），幂等由「条目被确认后才删 + upsert/delete 幂等」结构性保证，不依赖版本位。

#### 改动文件（第 2 轮）

- `src-tauri/src/todo_db.rs`：删 `LEGACY_IMPORT_VERSION` 与 `INSERT_LEGACY_TODO`；`SCHEMA_VERSION` 注释写死「schema 版本」语义与两条给 03 的规则；`initialise` 告警阈值改 `stored > SCHEMA_VERSION`；新增 `PendingWriteReport` / `RejectedWrite`（`Serialize + specta::Type`，仅 IPC 出参，沿用 `RuntimeInfo` 的既有做法，未新建业务实体）；`import_legacy` → `replay_pending(&[Todo], &[String]) -> Result<PendingWriteReport, TodoDbError>`（逐条复用 `save`/`delete`，`Sqlite` 错入 `rejected`、`Unavailable`/`Poisoned` 整体 `Err`）；用例由 3 个导入用例换成 4 个重放用例。
- `src-tauri/src/lib.rs`：`import_legacy_todos` → `replay_pending_writes(upserts, deletions) -> Result<PendingWriteReport, String>`，`collect_commands!` 同步。
- `src/bindings/commands.ts`：`pnpm run types:generate` 生成产物（未手改），`importLegacyTodos` → `replayPendingWrites`，新增 `PendingWriteReport` / `RejectedWrite` 类型。
- `src/lib/todo-repository.ts`：待写日志的读写与语义全部重写——新增 `PENDING_DELETIONS_KEY`/`QUARANTINE_KEY`、`readJson`/`writeJson`/`removeKey`、`isTodo`、`readPendingWrites`/`writePendingTodos`/`writePendingDeletions`、`recordPendingUpsert`/`recordPendingDeletion`/`clearPendingWrites`、`materialise`、`quarantine`、`dropApplied`；`load` 改为「墓碑过滤 + 日志覆盖 + 补入库里没有的」；`migrateLegacyTodos`/`MigrationOutcome` → `flushPendingWrites`/`PendingWriteFlush`；删除 `readBrowserTodos`/`writeBrowserTodos`/`clearBrowserTodos`/`saveBrowserTodo`/`removeBrowserTodo`。
- `src/stores/todos.ts`：消费 `WriteOutcome`——`persist` 改为消费返回值，新增 `forget(id)` 收口两处删除，新增 `degradedIds`/`failedIds`/`startupBacklog`、`noteWriteOutcome`、`notePendingWrites` 与导出的 `storageAlert`（新增 `StorageAlert` 类型）。
- `src/App.vue`：新增 `STORAGE_ALERT_CLASS`（DESIGN.md 语义色公式，无新令牌）与头部下方的 `role="status" aria-live="polite"` 提示条。
- `src/main.ts`：`await migrateLegacyTodos()` → `useTodoStore(pinia).notePendingWrites(await flushPendingWrites())`。
- `TODO.md`：6.1「数据迁移」✅ → 🚧（口径统一为移动端未验证），描述按新语义改写。
- 未改动：`crates/**`（契约未动，实体仍单源复用 `todo_contracts::Todo`）、capabilities（新 command 同为应用自有 command）、组件与其余 `src/lib/`。

#### 自验结果（命令与结论）

- `cargo test --workspace`：通过，17 passed / 0 failed（`cross_platform_todo_lib` 12：8 个既有 + 4 个新增重放用例，另 contracts 1、domain 1、server 3）。新增/调整用例：
  - `replaying_pending_writes_stores_them_without_touching_the_schema_version`：两条落库、`applied` 两个 id、`user_version` 仍为 `SCHEMA_VERSION`。
  - `a_parked_write_overwrites_the_row_it_was_never_able_to_update`：库里先有旧行，日志里的新值（改标题 + 完成态）重放后覆盖之——这是阻塞条目的机制证明。
  - `a_parked_deletion_removes_the_row_and_tolerates_unknown_ids`：墓碑删掉行，未知 id 也算 applied。
  - `one_rejected_entry_does_not_hold_up_the_rest_of_the_journal`：触发器只拒 `b`，`a`/`c` 照常落库，`rejected` 带 SQLite 原文。
  - `an_unavailable_database_reports_an_error_instead_of_panicking`：断言改为 `replay_pending`。
- `cargo check --workspace`：通过。
- `pnpm run types:generate`：通过；`src/bindings/commands.ts` 更新，`src/bindings/models/**` 无变化（未动契约）。
- `pnpm run check`：通过（格式、lint 与类型检查零告警）。
- `cargo fmt --all -- --check`：本轮新增 Rust 代码零 diff；仓库仍有 3 处 01 遗留未格式化点（遗留问题 3），按「不改无关代码」未动。

#### 走查（`pnpm run tauri:dev` 实测，重点是阻塞条目的复现路径）

走查前把 `%APPDATA%/com.todo.crossplatform` 与 WebView2 `Local Storage` 整目录备份，走查后原样还原（末尾有校验）。启动带 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`，用 CDP 在真实运行时内取证。**制造真实写失败的手段**：另起一个进程用 `node:sqlite` 打开同一个 `todos.db` 并 `BEGIN IMMEDIATE` 持住 WAL 写事务——应用的 `save`/`delete` 得到真实的 `database is locked`（SQLITE_BUSY），而 `list` 照常成功，正是评审指出的那个组合，不是模拟。

**跑法一 · 全新安装**（删 `todos.db*` 与 WebView2 `Local Storage` 后启动）：启动日志只有 `Settings restored` / `Todos restored from local storage {count:0}` / `Reminders rescheduled on startup`，无重放日志；三个回退键均为 `null`；`list_todos` 空；在真实输入框提交一条 → `list_todos` 立刻返回、`todos.items` 仍为 `null`；直接读库 `user_version` = 1。

**跑法二 · 阻塞条目复现路径（`save` 失败 → 降级写新值 → 重启）**

1. 持锁后在 UI 点「标记为完成」（真实交互，触发 `persist`）。
2. 日志：`Unable to save a todo to SQLite; using browser fallback {"error":"local todo database error: database is locked"}` 与调用侧的 `A todo write did not reach storage {"id":…,"outcome":"degraded"}`。
3. 状态：`list_todos` 仍是旧值（`status:"open"`），`todos.items` 里是**新值**（`status:"completed"` + `completedAt`），界面出现琥珀色提示条「有改动暂存在本地缓存…」。这正是评审描述的起点。
4. 放锁后重启：日志 `Pending todo writes replayed into SQLite {"applied":1,"rejected":0,"quarantined":0}`；`list_todos` 返回的是**那条完成态**（`completedAt` 无损），`todos.items` 这时才变 `null`，提示条消失。
   → 旧代码在这一步是「`DO NOTHING` 跳过（`inserted:0`）→ 整键 `removeItem` → 新值永久消失」；新代码是「覆盖成功 → 才删该条目」。丢数据路径已不可达。

**跑法二续 · 降级删除不复活**：再次持锁，在 UI 点删除 → 返回 degraded、`todos.deletions` 出现该 id、`list_todos` 里行还在、界面已不显示它；**带着锁重启** → `SQLite refused a pending todo write; keeping it for a retry {"error":"database is locked"}` + `{"applied":0,"rejected":1}`，墓碑保留、`Showing todo writes SQLite has not accepted yet {"count":1}`、列表仍为空（不复活）、提示条仍在；放锁后重启 → 墓碑被应用，SQLite 行消失、两个回退键清空、提示条消失。

**跑法三 · 有旧数据 + 脏数据自愈**：种入 5 条 `todos.items`——`legacy-1`（带 `dueDate` + `reminderAt`）、`legacy-2`（`completed` + `completedAt`）、`legacy-conflict`（与 SQLite 现有行同 id，模拟降级期间改过的新值）、`broken-1`（缺 `title`）、`broken-2`（带未知字段 `priority`，会被 `deny_unknown_fields` 拒）。重启后：`Parked local todo entries that are not readable as todos {"count":2}` + `Pending todo writes replayed into SQLite {"applied":3,"rejected":0,"quarantined":2}`；`list_todos` 3 条、字段逐项无损、`legacy-conflict` 的标题/状态/`completedAt` 已被**新值覆盖**（旧设计会跳过它）；`todos.items` 清空，2 条坏行在 `todos.quarantine` 里带时间戳；界面正常渲染（`legacy-1` 显示「今天 · 已设提醒」，`Reminders rescheduled on startup {scheduled:1}`）。直接读库：`user_version` = 1、3 行、逐行与上述一致。

**跑法三续 · 二次启动幂等**（真实进程重启，非页面刷新）：无重放日志、`list_todos` 仍是同样 3 条无重复、回退键仍为空、`todos.quarantine` 原样保留且**不再被重试**、`user_version` 仍为 1。

**补充 · 浏览器开发模式不回归**：headless Edge（独立 user-data-dir，端口 9223）打开同一个 vite dev server：`window.isTauri` = `false`，`flushPendingWrites()` = `{status:"skipped",reason:"browser-runtime"}`；`save` → `"stored"` 且 `todos.items` 出现该条；`load` 读回；`remove` → `"stored"`，`todos.items` 变 `null` 且 **`todos.deletions` 始终为 `null`**（浏览器下不产生墓碑）。走查后关闭 Edge 并删除临时 user-data-dir。

**环境还原核验**：应用与 Edge 均已退出，`%APPDATA%/com.todo.crossplatform` 与 WebView2 `Local Storage` 已由备份整目录还原；还原后读库 `user_version` = 1、`todos` 表 0 行，与走查前一致，未留下任何测试数据。

#### 验收标准复核（重设计后）

- 旧数据全量迁入且字段无损、迁移标记幂等（二次启动不重迁）：跑法三证明全量迁入与字段无损；幂等的实现方式由「版本位标记」换成「条目被 SQLite 确认后才删除 + upsert/delete 幂等」的结构性保证，跑法三续（真实进程重启）证明二次启动不重放、无重复条目。`user_version` 仍在，但按建议 4 的定论只表示 schema 版本。
- 迁移失败自动回退 Store 读取路径，应用可用不丢数据：跑法二/二续证明命令失败与单条被拒两种情形下数据一字未动、`load` 仍把它们合并展示、界面有提示、下次启动自动重试并成功。
- 双跑验证：跑法一（全新安装）与跑法三（有旧数据）均为真实 `tauri:dev` 运行时实测，附启动日志、IPC 返回与直接读库三重证据。

#### 遗留问题（第 2 轮补充）

5. 【平台特性，非本次引入】WebView2 的 `localStorage` 提交是异步落盘的（走查中实测约数秒），进程被强杀时最后几笔写入可能未落盘。这对待写日志与隔离键同样成立，是 01 起就存在的回退存储属性，不随本次改动引入；正常退出与重启路径实测均可持久（隔离键跨真实进程重启保留）。若日后要求「降级写入必须抗强杀」，应换掉回退存储本身（如落文件），属另一任务。
6. 【取舍声明】被隔离到 `todos.quarantine` 的条目只写 warn 诊断，不点亮界面提示条——它们不是「暂存待补写的改动」，用现有文案会误导，而单为其加一档 UI 需要产品/设计定文案。若编排者认为需要用户可感知，可另起任务。

## 评审记录

### 第 1 轮

- 阻塞 · `src/lib/todo-repository.ts:205-217`（联动 `src-tauri/src/todo_db.rs:51-57` 与 `src/lib/todo-repository.ts:139-141`）· 「回退键里 id 与 SQLite 现有行相同、但内容更新」的副本会被静默删除：`save` 的降级分支（`:160`）在原生写失败时把**最新**的 todo 写进 `todos.items`，此时该 id 通常已在 SQLite 里（旧值）；下次启动 `migrateLegacyTodos` 把它交给 `import_legacy`，`ON CONFLICT(id) DO NOTHING` 跳过（`inserted` 少计但不报错），命令返回 ok，随即 `clearBrowserTodos()` **无条件 removeItem 整个键**，这份唯一的新副本被永久删除；而 `load` 的合并又用 `!known.has(todo.id)` 把它过滤掉，用户从头到尾看不到它，日志只有一条读起来像成功的 `info: Legacy todos imported into SQLite {legacy:1, inserted:0}`。可达性已核实：`TodoDb` 的连接是进程级的，但 `save` 仍会因 SQLITE_FULL / SQLITE_BUSY（第二个实例或外部工具持库）/ IO error 等逐语句失败，而 `list` 照常成功，正是这个组合。开发用来论证 DO NOTHING 的前提（「库里的是活数据、回退键里的是旧副本」）只对「SQLite 之前的遗留数据」成立；而开发在「先决调查」里已认定真实来源就是降级回退键，那条链路上方向恰好相反——同一个键混了语义相反的两类数据，却只有一个冲突策略与一个「全清」动作 · 验收标准 1「旧数据全量迁入且字段无损」与 2「不丢数据」；并入本任务的 01 建议 (a)「回退数据的回收路径」
  - 处理（开发）：认可，且认可「不能在现有形状上打补丁」。这段语义已整体重做为**待写日志（pending-write journal）**，不再是「旧副本导入」。四个设计问题逐条回答见「实现记录 · 第 2 轮」的「回退/迁移语义重设计」节，要点：① 不再需要「区分新旧」——新增不变式「原生写成功即清掉该 id 的日志条目」（`todo-repository.ts` 的 `save`/`remove` 成功分支调 `clearPendingWrites`），使日志条目只在「SQLite 没有更新副本」期间存在，因此永远是较新的一份，冲突策略从 `ON CONFLICT DO NOTHING` 改为 `UPSERT_TODO`（覆盖），`INSERT_LEGACY_TODO` 已删除；② 不再有「全清」动作——`clearBrowserTodos` 已删除，改为只删除命令回报 `applied` 的那些 id（`dropApplied`）；③ 删除以墓碑表达（新键 `todos.deletions`），`load` 按墓碑过滤，已删项不再从 SQLite 复活；④ 逐条容错 + 坏行隔离，一条脏数据不再卡死整批。实测复现（`tauri:dev` 真实运行时，用第二个进程持 WAL 写事务制造真实 SQLITE_BUSY，`list` 仍成功）：勾选完成 → `save` 报 `database is locked` → 新值（`status:completed` + `completedAt`）落入日志、界面出现降级提示条；重启后启动日志 `Pending todo writes replayed into SQLite {"applied":1,...}`，`list_todos` 返回的正是那条完成态、`todos.items` 才被清掉 → 评审给出的丢数据路径不再成立。
  - 复核（评审）：认可。逐项核实：`INSERT_LEGACY_TODO` 与「全清」`clearBrowserTodos` 全仓已无残留（`grep` 零命中），重放走 `UPSERT_TODO` 覆盖（`todo_db.rs:51-60,254-287`），条目只按命令回报的 `applied` 逐个删除（`todo-repository.ts:358-365`），删除以墓碑表达且 `load` 先按墓碑过滤（`:279-292`）。第 1 轮给出的丢数据路径（新值被 `DO NOTHING` 跳过后又被整键 `removeItem`）在新代码中已不可达。不变式「原生写成功即清该 id」的成立范围经独立复核为「单实例 + localStorage 写成功 + 正常退出」——三个前提各有一条可破路径，均不在正常路径上，故不判阻塞，另以本轮建议 (1)(2) 列出。
- 建议 · `src/lib/todo-repository.ts:14-20` 与 `src/stores/todos.ts:26,120,165` · `WriteOutcome` 定型了但没有任何消费方：`persist()` 是 `void repository.save(todo)`，`remove`/`applyRemoteDelete` 是 `void repository.remove(...)`，三态返回值全部被丢弃；`migrateLegacyTodos` 也不经这个接口（直接调 `nativeCommands.importLegacyTodos`）。诊断日志仍在 repository 内部写，所以不是全丢，但并入的 01 建议 (b) 想解决的「调用方拿不到是否落库的信号」在调用侧一处也没闭环，`degraded` 目前在运行时不可观测 · 并入本任务的 01 建议 (b)；AGENTS.md 通用「简约至上：不做没要求的灵活性」（要么被消费、要么不必存在）
  - 裁决（编排者）：本轮处理。`WriteOutcome` 是编排者从 01 并入本任务的建议 (b)，其存在意义就是给调用方「是否真的落库」的信号；无消费方等于这条并入没做完，必须闭环。
  - 处理（开发）：已闭环，三处。① 调用侧真正消费：`src/stores/todos.ts` 的 `persist` 改为 `repository.save(todo).then((outcome) => noteWriteOutcome(todo.id, outcome))`，新增 `forget(id)` 承担 `remove`/`applyRemoteDelete` 的删除路径（原来两处裸 `void repository.remove(...)`），`noteWriteOutcome` 按三态分流——`stored` 清记录、`degraded` 记入 `degradedIds`、`failed` 记入 `failedIds`，并在非 `stored` 时以调用方身份写一条诊断（`warn`/`error` 分级）。② 用户可感知：store 新增 `storageAlert` computed（`failed` → error 档「有改动没能保存到本机…」，`degraded` 或启动积压 → warn 档「有改动暂存在本地缓存…」），`src/App.vue` 在头部下方渲染 `role="status" aria-live="polite"` 的提示条，配色取 DESIGN.md 语义色公式（浅色 100 底 + 700 字 / 暗色 900/40 底 + 300 字），未新增设计令牌。③ 启动重放的结果也被消费：`main.ts` 由 `await migrateLegacyTodos()`（丢弃返回值）改为 `useTodoStore(pinia).notePendingWrites(await flushPendingWrites())`，命令失败或有被拒条目时同样点亮提示条。实测：制造 SQLITE_BUSY 后在 UI 勾选完成，日志出现调用侧的 `A todo write did not reach storage {"id":…,"outcome":"degraded"}`，界面出现琥珀色提示条；库恢复并重启后提示条消失。
  - 复核（评审）：认可。三处消费均已核实：`stores/todos.ts:78-85` 的 `persist`/`forget` 消费返回值并分流进 `degradedIds`/`failedIds`（`:56-69`），`:39-47` 的 `storageAlert` 与 `App.vue:185-193` 的提示条把它变成用户可感知状态，`main.ts:29` 消费 `flushPendingWrites` 的返回值。`void` 前缀符合 AGENTS.md TypeScript 规范。遗留一点复位缺口（`startupBacklog` 置真后无路径复位）另列本轮建议 (8)。
- 建议 · `src/lib/todo-repository.ts:162-180`（含 `:85-89`）· `remove` 的 `"degraded"` 名不副实：回退键里没有墓碑机制，`removeBrowserTodo` 在键为空时直接 `return true`，于是「原生删除失败 + 回退键本来就没有这条」这一最常见组合返回 `"degraded"`，实际什么都没记录——下次 `load` 该条目仍从 SQLite 读回，用户眼里是「删掉的待办自己回来了」。三态里这一格与 `save` 的 `"degraded"`（确有副本留存）语义不对等 · 正确性维度；`WriteOutcome` 的文档注释「SQLite refused the write; the browser fallback holds the only copy」（`:17-18`）与该分支实际行为不符
  - 裁决（编排者）：本轮处理。与本轮阻塞条目同源——都是回退键把「活数据」与「旧副本」两类语义相反的数据混在一个键、一个冲突策略里。必须与阻塞条目一并重新设计，分开修必然二次返工。
  - 处理（开发）：已在同一次重设计中修复。删除现在有墓碑：新增回退键 `todos.deletions` 存「SQLite 拒收其删除的 id」，`remove` 的原生失败分支改调 `recordPendingDeletion(id)`（同时把该 id 从 `todos.items` 摘掉，删除覆盖更早的降级写入），只有真正写下墓碑才返回 `"degraded"`，写不进去才是 `"failed"`——三态与 `save` 侧对等，`WriteOutcome` 的注释也随之改写。读路径同步：`load` 先按墓碑过滤原生行，再叠加日志里的新值，因此墓碑落下、重放还没成功那段时间里，已删待办不会从 SQLite 读回来。浏览器运行时没有 SQLite 在后面，`browserRepository.remove` 走 `clearPendingWrites`（真删而非墓碑），不会凭空产生 `todos.deletions`。实测：持锁使 `delete_todo` 报 `database is locked` → 删除返回 degraded、`todos.deletions` 出现该 id、界面立刻不再显示它；带着墓碑重启（锁仍在）→ 重放被拒 `{"applied":0,"rejected":1}`、墓碑保留、列表仍为空（不复活）；放锁后重启 → 墓碑被应用，SQLite 行消失、两个回退键都清空。
  - 复核（评审）：认可。`remove` 的 degraded 现在以「墓碑真的写下」为条件（`todo-repository.ts:314-332` → `:193-200`），与 `save` 侧对等；`load` 的墓碑过滤（`:279-284`）堵住了「删掉的待办自己回来」。墓碑生命周期另行复核，无问题（见核查结论 ②）。
- 建议 · `src/lib/todo-repository.ts:201-231` · 一条脏数据会把整批迁移永久卡死：`readBrowserTodos`（`:42-52`）只做 `Array.isArray` 判断、逐项无校验，整批交给命令后任一行被 Rust 契约拒收（缺 `title` 等）即整批 `Err`，事务回滚、键不清空、下次启动重试仍被同一条卡住，其余旧数据永远迁不进去。开发「跑法三」已实测复现该状态（靠人工删除坏数据才解开）。数据本身没丢（`load` 合并仍可见），故不判阻塞，但「旧数据全量迁入」在有坏行时无法自愈，可考虑逐条校验、跳过坏行并把跳过项留在回退键里 · 验收标准 1「旧数据全量迁入」
  - 裁决（编排者）：本轮处理。同上，属同一段回退键/迁移逻辑的重设计范围；「一条脏数据永久卡死迁移且无自愈」在真实用户机器上等同于旧数据永久丢失，与阻塞条目同一严重级别。
  - 处理（开发）：已修，自愈路径分两段。前端段：新增 `isTodo` 逐条校验，规则对齐 Rust 契约（`deny_unknown_fields` → 键集必须是 `Todo` 字段子集；`completed_at` 无 `serde(default)` → `completedAt` 必须存在），凡是过得了这道校验的条目就一定过得了 IPC 反序列化；过不了的不再混进批次，而是搬到隔离键 `todos.quarantine`（带时间戳保留原始条目）并写一条 warn 诊断——它们不是「暂时写不进去」而是「任何版本都读不回 Todo」，留在日志里只会被永远重试。Rust 段：`import_legacy` 整批事务改为 `TodoDb::replay_pending`，逐条 `save`/`delete` 各自成一次隐式事务，行级失败记进 `rejected`（带 SQLite 原文），只有「库整体不可用」（`Unavailable`/`Poisoned`）才整体 `Err`；前端只删除命令回报 `applied` 的条目，被拒条目留在日志里、继续被 `load` 合并展示、下次启动再试。实测：种入 5 条（3 条合法 + 1 条缺 `title` + 1 条带未知字段 `priority`）后重启 → `Parked local todo entries that are not readable as todos {"count":2}`、`Pending todo writes replayed into SQLite {"applied":3,"rejected":0,"quarantined":2}`，3 条合法数据（含 `dueDate`/`reminderAt`/`completedAt`）全部入库且冲突 id 被新值覆盖，`todos.items` 清空，2 条坏行躺在 `todos.quarantine` 里；再次重启不再重放、不再重试坏行。另有 Rust 用例 `one_rejected_entry_does_not_hold_up_the_rest_of_the_journal`（触发器只拒 `b`，`a`/`c` 照常落库）。
  - 复核（评审）：认可。整批事务已换成逐条应用（`todo_db.rs:254-287`：`Sqlite` 错入 `rejected`，仅 `Unavailable`/`Poisoned` 整体 `Err`），前端逐条校验后把坏行搬走（`todo-repository.ts:143-163,346-356,384-388`），「一条脏数据永久卡死整批」已不可达，Rust 用例复跑通过。校验规则与契约逐字段比对无误（`crates/contracts/src/lib.rs:9-23`：`deny_unknown_fields` 对应键集子集判断，`completed_at` 无 `serde(default)` 对应 `completedAt` 必存在，`due_date`/`reminder_at` 带 `default` 对应可缺省）。自愈机制本身引出两条新问题（手写校验的漂移风险、隔离/覆盖时忽略写失败），另列本轮建议 (3)(4)(5)。
- 建议 · `src-tauri/src/todo_db.rs:21-24,237-240` · `LEGACY_IMPORT_VERSION` 只写不读：全仓无任何地方拿它做迁移判据（`migrateLegacyTodos` 只看 `todos.items` 是否为空，`import_legacy` 只靠 `ON CONFLICT`），幂等三重保证里第 ③ 条实际不承担任何作用；代价是 `user_version` 被改成「1=建库、2=已导入」的混合语义，而全新安装永远停在 1，1 与 2 的表结构完全相同。03 若按「版本 N ⇒ 迁移 1..N 均已执行」的常规读法推理会出错（v1 的库从未也不需要执行「迁移 2」），需显式把 {1,2} 一起当作「pre-3」。建议 03 落地前要么让这个版本位真的被读、要么撤回改义 · 遗留问题 1 自述的 03 交接风险；AGENTS.md 通用「简约至上」
  - 裁决（编排者）：本轮处理。`user_version` 的语义必须在派发 03「契约扩展 v1 全字段」之前定论——03 要靠它判断是否执行字段迁移，带着混合语义开工会把错误固化进 schema 演进链。请明确它到底是 schema 版本还是迁移序号，二选一并在代码注释中写死。
  - 处理（开发）：**定论为「schema 版本」**，`LEGACY_IMPORT_VERSION` 已删除，`user_version` 回到只由建库时打的 `SCHEMA_VERSION = 1`。选它的理由是重设计之后另一个选项已不成立：一次性导入被待写日志重放取代，那是常态化的数据搬运、可反复发生，本来就不是「第 2 号迁移」，硬编成迁移序号会造出一个全新安装永远不会经历、且与版本 1 表结构完全相同的空号。定论写死在 `todo_db.rs` `SCHEMA_VERSION` 的注释里，含两条给 03 的硬规则：① 只有列布局变化才动这个值，抬高它的那一版必须同时带上把每个更低版本升上来的 `ALTER TABLE`；② 搬数据永不碰它。因此 03 的判据是标准读法「`stored < SCHEMA_VERSION` ⇒ 执行 `stored+1..=SCHEMA_VERSION` 的 schema 迁移」，不需要把 {1,2} 特判成 pre-3。`initialise` 的告警阈值同步改为 `stored > SCHEMA_VERSION`，与之自洽——本 build 认得的值只有 `SCHEMA_VERSION` 一个，更高的只可能出自更新的 build，既不会误报也不会把未知版本静默接受（既有用例 `an_already_recorded_schema_version_is_not_overwritten` 覆盖「不覆写更高版本」，新用例 `replaying_pending_writes_stores_them_without_touching_the_schema_version` 覆盖「搬数据不动版本位」）。实测走查中全新安装、迁入 3 条旧数据、二次启动三种状态下直接读库，`PRAGMA user_version` 恒为 1。
  - 复核（评审）：认可定论本身。`LEGACY_IMPORT_VERSION` 已删除（全仓零命中），语义写死在 `todo_db.rs:19-35` 的注释里（含给 03 的两条硬规则），告警阈值 `stored > SCHEMA_VERSION`（`:159-168`）与之自洽——本 build 认得的值只有 1，既不误报也不静默接受未知版本；`replaying_pending_writes_stores_them_without_touching_the_schema_version` 与 `an_already_recorded_schema_version_is_not_overwritten` 两个用例覆盖到位。但对「已经跑过第 1 轮 build、`user_version` 已被写成 2」的本机库存在误判，另列本轮建议 (7)。
- 建议 · `TODO.md:108`（对照 `:107`）· 「数据迁移」标 ✅，而同节「SQLite 落地」因移动端未验证仍为 🚧；迁移与它依据的是同一份未验证事实（遗留问题 4 也自述移动端未验证），两行口径不一致 · AGENTS.md「功能实现或状态变化后同步更新对应条目」
  - 裁决（编排者）：本轮处理。口径不一致的成本极低，顺手统一即可；以「移动端未验证」为准。
  - 处理（开发）：已按「移动端未验证」统一。`TODO.md:108` 由 ✅ 改为 🚧，描述同步为「启动时把 SQLite 之外的待办写入重放进库（旧数据 + 降级期间的新增/修改/删除），逐条容错、坏行隔离；桌面端已落地，移动端待随移动形态验证」，与 `:107`「SQLite 落地」🚧 的口径一致。
  - 复核（评审）：认可。`TODO.md:108` 现为 🚧 且描述与 `:107` 口径一致，与本轮实现（重放而非一次性导入）表述相符。

核查结论（无问题，不计条目）：① **开发推翻的任务前提成立**——独立复核三条证据：`git show df1db9e:src/stores/todos.ts` 确认 01 之前 store 只有内存 `ref`、无任何持久化调用；`git log --all -S "todos.items"` 只命中 `cfc2298`(01) 与 `d1c1a36`，`-S "todos.json"` 零命中；全仓 Store 文件只有 `settings.json`（`src/lib/settings-storage.ts:6`、`src-tauri/src/lib.rs:93`）；直接读本机 `%APPDATA%/com.todo.crossplatform/settings.json`，键为 `sync.pending`/`sync.deviceId`/`ui.showTodayCard`/`appearance.theme`，无待办条目 → 「Store 插件从未存过待办、没写 Store 读取分支」不属漏迁。② **事务回滚覆盖版本位**——开发用例只证到「插入阶段失败」，未证 pragma 本身；本轮实测 SQLite `PRAGMA user_version` 参与事务并随 ROLLBACK 回退（`BEGIN; PRAGMA user_version=2; ROLLBACK` 后读回 1），故 `import_legacy` 的「行 + 标记」原子性成立；`journal_mode=WAL` 下 `synchronous` 仍为默认 2(FULL)，commit 已 fsync，「清空回退键先于确认落库」不成立（`clearBrowserTodos` 只在命令返回 ok、即 `commit()` 之后调用）。③ **`load` 合并语义**——`known` 集合去重，无重复项；`newestFirst`（`:103-107`）的 `created_at DESC, id ASC` 与 `SELECT_TODOS`（`todo_db.rs:36-38`）一致，UUID/ISO 字符串在 SQLite BINARY 与 JS UTF-16 比较下同序，`Array.prototype.sort` 稳定，顺序不乱；「已删项复活」两条路径均已堵——原生删除成功走 `:168` 清同 id 副本，原生删除失败走 `:179` 同样清副本。④ **`initialise` 告警阈值**——由 `stored != SCHEMA_VERSION` 改为 `stored > LEGACY_IMPORT_VERSION` 后，唯一新增的静默值是 2（本次的合法值），3 及以上仍告警，未引入「更高版本被静默接受」。⑤ **capabilities**——`import_legacy_todos` 是应用自有 command 而非插件 command，`src-tauri/gen/schemas/acl-manifests.json` 中不存在任何应用 command（`grep -rl save_todo src-tauri/gen/` 零命中），与 01 起 `list_todos`/`save_todo` 在 `default.json` 无对应权限却可用互证，故无需新增权限声明；卡片窗口的排除由 `src/main.ts:23` 的 `if (isCardWindow) return;` 提前返回完成，`migrateLegacyTodos()` 在其后（`:28`），排除正确。⑥ **清单与越界**——`git status` 为 `TODO.md`、`src-tauri/src/lib.rs`、`src-tauri/src/todo_db.rs`、`src/bindings/commands.ts`、`src/lib/todo-repository.ts`、`src/main.ts` 六个文件加本任务文件，与「改动文件」逐项一致；01 的改动已在 `cfc2298` 提交、不在工作区，无需排除；未见 03 的字段扩展，实体仍单源复用 `todo_contracts::Todo`。⑦ **环境还原**——复制本机 `todos.db`(+wal/shm) 到临时目录读取：`user_version` = 1、`todos` 0 行，与开发自述的还原结论一致，走查未留脏数据。

验证：`cargo test --workspace` 通过（16 passed / 0 failed：`cross_platform_todo_lib` 11 含 3 个新增导入用例、contracts 1、domain 1、server 3）；`cargo check --workspace` 通过；`pnpm run check` 通过（40 文件格式、35 文件 lint+类型零告警）；`pnpm run types:generate` 重跑后 `git status` 无新增改动、`src/bindings/` 仍为 `commands.ts +7 行`，生成产物与调用方（`src/lib/native.ts` → `nativeCommands.importLegacyTodos`）同步、无漂移。本轮 1 条阻塞、5 条建议。

### 第 2 轮

本轮把待写日志/墓碑/隔离的重设计当作一次新设计评审。总判定：**无阻塞**。不变式「一次成功的原生写会把该 id 从两个键里清掉，因此键里的条目永远比库里新」在「单实例 + localStorage 写成功 + 正常退出」三前提下成立（推演见核查结论 ①），三个前提各有一条可破路径，但都不在正常运行路径上，且破坏后的损失（一条待办回退到较旧版本）不大于第 1 轮已修复的损失，故按建议列出而非阻塞。

- 建议 · `src/lib/todo-repository.ts:294-313`（联动 `:207-217`、`:176-186`、`:346-356`）· 不变式的三个执行点都把自己的失败信号丢掉了：① `save` 成功后调 `clearPendingWrites(todo.id)` 但丢弃其布尔返回值，无论是否真的清掉都返回 `"stored"`——清理失败就留下一条比库里旧的日志条目，而策略已改为 UPSERT 覆盖，下次启动即用旧值盖掉新行（`:298-301` 的注释恰好写明了这个后果，却没有据此分流）；② `recordPendingUpsert` 忽略 `writePendingDeletions` 的返回值，摘墓碑失败时同一 id 会同时留在两个键里，而 `replay_pending` 先做 upsert 再做 deletions（`todo_db.rs:264-284`），结果是「刚写回来的待办在下次启动被删掉」；③ `quarantine` 忽略 `writeJson` 返回值，随后 `flushPendingWrites:385-388` 仍无条件用过滤后的 `upserts` 重写 `todos.items`，隔离写失败＝坏条目被直接删除。三处都是「不变式靠这一步维持」的位置，建议至少让失败可观测（写 error 诊断 / 计入 `failedIds`），隔离写失败时不得删原条目 · 正确性维度；本任务设计要点自述的不变式（`todo-repository.ts:40-58` 注释）
  - 处理（编排者）：转任务处理，并入 数据与同步/03-契约扩展v1全字段。三个执行点吞掉自身失败信号，是新不变式的真实破口；03 会重写 `todo-repository` 的全部读写路径以承载新字段，届时一并把 `clearPendingWrites`/`recordPendingUpsert`/`quarantine` 的返回值纳入 `WriteOutcome` 体系，避免本轮再改一版、03 再改一版。
- 建议 · `src/lib/todo-repository.ts:40-58`（联动 `src-tauri/src/lib.rs:196-199`、实现记录遗留问题 5）· 不变式隐含两个未声明的前提，注释里没写、也没有测试守护：① **正常退出**——WebView2 的 `localStorage` 延迟落盘（开发实测约数秒）意味着「原生写成功 → 清日志条目」的清理动作可能只在内存里；此时硬杀进程，重启后旧条目会 UPSERT 覆盖更新的行。旧设计下同一场景只是一次无害重复写，新设计把它升级成了覆盖更新数据，遗留问题 5 只描述了平台属性、没有描述这个后果差异。② **单实例**——仓库未装 `tauri-plugin-single-instance`，两个实例共享同一 WebView2 profile（同一 localStorage）与同一 `todos.db`；实例 B 的启动快照可能在实例 A 写成功之后才重放（旧值盖新值），或 B 的一次成功写把 A 刚写下的降级条目清掉（降级写永久丢失）。第 1 轮论证 SQLITE_BUSY 可达时用的正是「第二个实例」这个前提，两者不能只取其一。建议把前提写进不变式注释，或给日志条目加单调序号/写入时间戳，让重放能比较而不是无条件覆盖 · 正确性维度；验收标准 2「不丢数据」
  - 处理（编排者）：转任务处理。「WebView2 延迟落盘 + 硬杀」与「无 single-instance 插件」两个未声明前提中，后者已有归属——`桌面端形态/06-常驻自启单实例` 正是做单实例的子任务，届时该前提由实现兜住；前者属回退键固有性质，随 03 的读写路径重写一并评估。本轮不改。
- 建议 · `src/lib/todo-repository.ts:143-163`（联动 `:176-217`、`:225-231`）· 「读不回 Todo 的条目」在非启动路径上会被静默删除，而不是进隔离键：`readPendingWrites` 把 malformed 过滤出去，`recordPendingUpsert`/`recordPendingDeletion`/`clearPendingWrites` 又都用过滤后的 `upserts` 整体写回 `todos.items`，坏条目就此消失，无隔离、无诊断。桌面端启动时 `flushPendingWrites` 会先隔离，窗口很窄；但**浏览器运行时 `flushPendingWrites` 首行就 `skipped/browser-runtime`，从不隔离**，而那里 `todos.items` 是主存储——坏条目先在 `load`（`materialise`）里对用户隐身，再被下一次 `save` 永久删掉。03「契约扩展 v1 全字段」若新增必填字段，浏览器开发档案里的既有条目正好落进这个形状 · 正确性维度；本轮自述的取舍「它们仍是用户的字节」（`:62-67` 注释）
  - 处理（编排者）：转任务处理，并入 数据与同步/03-契约扩展v1全字段。「非启动路径静默删除 malformed 条目」「浏览器运行时从不隔离而 `todos.items` 正是主存储」是本条最实的风险，且 03 扩字段后旧结构条目必然大量出现（旧条目缺新字段即读不回 Todo），必须在 03 里连同校验一并定稿。
- 建议 · `src/lib/todo-repository.ts:62-67,346-356` · 隔离键 `todos.quarantine` 的数据就此永久沉默：不在 UI 呈现（开发遗留问题 6 自陈）、不参与 `notePendingWrites`（`stores/todos.ts:72-75` 只看 failed/rejected）、无去重、无上限、无任何重新取回的入口，只留一条 warn 诊断。判定为**建议不为阻塞**——原始字节仍完整保存在 localStorage 且有诊断留痕，不满足「静默丢数据」的判据；但「用户永远不知道自己有数据没进来」是真实缺口，且隔离只增不减会持续占用 localStorage 配额。建议要么在提示条里加一档「有 N 条数据无法读取」的文案（需产品/设计定文案），要么给出一条可导出/可清理的路径 · 验收标准 1「旧数据全量迁入且字段无损」
  - 处理（编排者）：本次不处理，已知悉。评审已判定字节完整保留且有 warn 诊断，不构成丢数据；把隔离条目呈现给用户需要产品与文案决策（用户能拿它做什么？），不适合塞进直达级任务。待 发布工程/06-首启引导与空状态 或 外观与设置 有 UI/UX 阶段时一并设计。
- 建议 · `src/lib/todo-repository.ts:99-132`（对照 `crates/contracts/src/lib.rs:9-23`）· `isTodo`/`TODO_FIELDS` 是手写的契约镜像，规则当前与 Rust 侧逐字段一致（已核对，见上文复核），**不判违反 AGENTS.md「实体单源」**——实体类型仍来自 `src/bindings/models/Todo`，这里只是运行时守卫，而 TS 类型在运行时被擦除，守卫无法从生成产物自动获得。问题在于没有任何机制让它随契约一起变：03 新增字段后若漏改 `TODO_FIELDS`，本 build 自己写下的降级条目会在下次启动被判为 malformed 搬进隔离键（用户不可见、不再重试），而 `pnpm run check` 一声不吭。建议把字段表锁到生成类型上（例如 `const TODO_FIELDS = { id: true, … } satisfies Record<keyof Todo, true>`，再取 `Object.keys`），让漏改在类型检查阶段就失败 · AGENTS.md「端侧」实体单源与契约同步；简约至上
  - 处理（编排者）：转任务处理，并入 数据与同步/03-契约扩展v1全字段。评审给出的编译期锁定写法（`satisfies Record<keyof Todo, true>`）正是为了防 03 扩字段时漂移，在 03 落地最省事且立刻能被新字段验证。
- 建议 · `src/App.vue:114-120,185-193` · 提示条本身合规：配色逐字取自 DESIGN.md「语义色」表的既有值（`bg-amber-100 text-amber-700` / `dark:bg-amber-900/40 dark:text-amber-300`，error 档同理用 red 档），**未引入 DESIGN.md 未定义的新令牌**，浅暗成对，`role="status" aria-live="polite"`，信息以文字承载不孤用颜色，间距 `mb-6`(24) 合 4/8 节奏。但本任务是**直达级且「规格（UI/UX）」为空**，而这是一个 DESIGN.md「组件视觉基准」尚未收录的新组件形态（提示条/alert；文档「待补充」节明确要求 toast 等新组件落地时在该节增补条目）：圆角取了按钮档 `rounded-lg` 而非卡片档、无图标、无关闭途径、未给移动端断点考量，这四项属视觉决策而非实现细节。请编排者裁决是否补一次 UI/UX 阶段并在 DESIGN.md 增补「提示条」条目 · DESIGN.md「组件视觉基准」「待补充」；workflow.md 任务分级
  - 处理（编排者）：本次不处理，维持现状。评审已确认配色逐字取自 DESIGN.md 既有语义色、无新令牌、浅暗成对、a11y 合格——直达级不补 UI/UX 阶段。圆角/图标/关闭途径/移动端四项视觉决策与 DESIGN.md 增补「提示条」组件基线，转由 `桌面端形态/01-主窗导航壳` 的 UI/UX 阶段统一定稿（该子任务本就要定主窗壳层的 chrome，提示条是其一部分），届时按全局规则 3 由 UI/UX 同步进 DESIGN.md。
- 建议 · `src-tauri/src/todo_db.rs:143-170` · `user_version` 定论自洽，但对**已经跑过第 1 轮 build 的本机库**不成立：那些库的版本位已被写成 2，本 build 既不改写（`stored != 0` 分支）也不认得它，于是每次启动都会打一条 `Todo database schema revision 2 is newer than the 1 this build knows` 的 warn；更要紧的是按定论 03 抬高 `SCHEMA_VERSION` 到 2 时，这些库会被标准读法判成「已是 2、无需迁移」，而它们的列布局其实还是 v1 → 静默跳过真正的 schema 迁移，把错误固化。第 1 轮 build 从未发布，只影响开发/测试机，故为建议；建议在派发 03 前明确处置（开发机删库重建，或 03 的判据对 2 做一次性甄别）并记进遗留问题 · 遗留问题 1 自述的 03 交接风险
  - 处理（编排者）：转任务处理，并入 数据与同步/03-契约扩展v1全字段，且列为 03 的**前置必办项**。本机库 `user_version` 已被第 1 轮 build 写成 2，03 若把 SCHEMA_VERSION 抬到 2 会被判成「无需迁移」而跳过真正的字段迁移——这正是本条指出的误判，必须在 03 动 schema 之前处置（要么跳号到 3，要么加一次性纠偏）。
- 建议 · `src/stores/todos.ts:36-47,71-75` · `startupBacklog` 置真后没有任何复位路径：启动重放有被拒条目或命令失败后，即使本会话内这些待办随后写成功（日志条目已被 `clearPendingWrites` 清掉、库已是最新），警示横幅仍会挂到进程结束，且提示条无关闭途径。文案说「重启后会自动补写」，而此时其实已经补写完了，属状态与事实脱节。建议在 `noteWriteOutcome` 收到 `stored` 且无其他降级/失败 id 时复位，或让 `storageAlert` 依据日志当前剩余量计算 · 正确性维度（状态处理遗漏）
  - 处理（编排者）：转任务处理，并入 数据与同步/03-契约扩展v1全字段。`startupBacklog` 无复位路径导致横幅整会话不消失、文案与事实脱节，属 `WriteOutcome` 消费侧的收尾，与本轮第 1 条建议同一段代码，一并在 03 处理。

核查结论（无问题，不计条目）：① **不变式的可达性推演**——逐条排除了编排者点名的序列：(a) 同一 id 的交错写不会记下旧值，因为 `persist` 传的始终是 `items` 里的同一个活对象（`add` 的 raw 对象与 `find` 拿到的 proxy 同一 target），失败分支 `recordPendingUpsert(todo)` 序列化的是**当下**状态而非发起时的快照；(b) 「replay 部分成功后崩溃」无害——`dropApplied` 只删 `applied`，重放的 upsert/delete 都幂等，重跑一次不改变结果；(c) 「同一 id 先 degraded 写、再成功写、再 degraded 删」按 `clearPendingWrites` → `recordPendingDeletion` 顺序推演，两个键最终只留墓碑，正确；(d) 唯一真正的破口是三个前提（见建议 (1)(2)）。另核实主窗口在 `flushPendingWrites` 期间不可能有并发写入方：`main.ts:54-62` 先 `await bootstrapApp()` 再 `mount`，重放在挂载与 `initSyncEngine` 之前完成；今日卡片窗口（`main.ts:23` 提前返回）只经事件把 toggle/remove 意图回传主窗口（`today-card-sync.ts:14-23`），自己不碰 repository，故不存在双窗口同时改日志。② **墓碑生命周期**——不会无限增长：`replay_pending` 对未知 id 的删除也算成功（`todo_db.rs:275-284` + 用例 `a_parked_deletion_removes_the_row_and_tolerates_unknown_ids`），因此任何一次成功重放都会清空墓碑；同 id 重建时 `recordPendingUpsert:182-184` 摘掉墓碑（后写者赢）；库整体不可用时的累积上限＝停机期间的删除次数，可接受。③ **墓碑不会屏蔽同步**——`applyRemoteUpsert` 建条目后走 `persist`，成功则 `clearPendingWrites` 抹墓碑、失败则 `recordPendingUpsert` 抹墓碑，两条路都解锁；`applyRemoteDelete` → `forget` 与本地删除同路径；`sync-engine.ts:151-156` 确认远端变更一律经 store 而非直接调 repository，故不绕过 `WriteOutcome` 与日志体系。④ **重放与远端的先后**——`main.ts:29,38,48-49` 顺序为 重放 → hydrate → 同步，远端变更只会落在已经结清的库上，不存在「远端值被待写日志覆盖」。⑤ **清单与越界**——`git status` 为 `TODO.md`、`src-tauri/src/{lib.rs,todo_db.rs}`、`src/{App.vue,main.ts}`、`src/bindings/commands.ts`、`src/lib/todo-repository.ts`、`src/stores/todos.ts` 八个文件加本任务文件，与「改动文件（第 2 轮）」逐项一致，无清单遗漏、无 03 的字段扩展越界；`crates/**` 未动，实体仍单源复用 `todo_contracts::Todo`。⑥ **capabilities**——`replay_pending_writes` 与既有 `save_todo`/`delete_todo` 同为应用自有 command，`src-tauri/gen/schemas/` 中不存在应用 command 条目，无需新增权限声明（同第 1 轮结论）。⑦ **`PendingWriteReport`/`RejectedWrite` 的派生**——只派生 `Serialize + Type`（不派生 `TS`），因此只出现在 tauri-specta 生成的 `commands.ts` 而不落 `src/bindings/models/`；与 `RuntimeInfo` 的做法略有出入，但两者都是生成产物、前端不手写、也无调用方需要从 `models/` 导入，不构成规范问题。

验证：`cargo test --workspace` 通过（17 passed / 0 failed：`cross_platform_todo_lib` 12 含 4 个重放用例、contracts 1、domain 1、server 3）；`cargo check --workspace` 通过；`pnpm run check` 通过（40 文件格式、35 文件 lint+类型零告警）；`pnpm run types:generate` 重跑后 `git status` 文件集不变、`src/bindings/` 无新增改动，生成产物与调用方（`src/lib/native.ts` → `nativeCommands.replayPendingWrites`）同步、无漂移。本轮 0 条阻塞、8 条建议。
