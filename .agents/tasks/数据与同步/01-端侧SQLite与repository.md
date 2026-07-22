# 端侧SQLite与repository

- 级别：直达
- 状态：完成

## 需求（产品）

- 目标与场景：SQLite 成为端侧唯一任务存储（rusqlite + command 封装），视图端持久化调用经 repository 接口收口，组件不感知实现。
- 范围（做 / 不做）：做 库表设计、连接管理、CRUD command、repository 接入 Pinia；不做 Store 迁移（02）与新业务字段（03）。
- 验收标准（逐条可检查）：
  - [ ] 任务读写全部走 SQLite，应用重启数据不丢。
  - [ ] 组件零改动，持久化经 repository 收口；浏览器开发模式回退 localStorage 不回归。
  - [ ] cargo test --workspace、pnpm run check 通过。

## 规格（UI/UX）

## 实现记录（开发）

- 改动文件：
  - `Cargo.toml`：新增 `[workspace.dependencies]`，加入 `rusqlite = { version = "0.40.1", features = ["bundled"] }`。选 `bundled` 的理由：随 crate 从源码编译 SQLite，Windows 无系统 sqlite3，且移动端交叉编译同样不依赖系统库；未引入 `thiserror`/`anyhow`（AGENTS.md 禁项），错误类型手写枚举。
  - `src-tauri/Cargo.toml`：`rusqlite = { workspace = true }`。
  - `src-tauri/src/todo_db.rs`（新增）：端侧 SQLite 库。`todos` 表（id / title / status / created_at / completed_at / due_date / reminder_at），`PRAGMA journal_mode = WAL`、`PRAGMA user_version = 1`（供 03 扩展字段时判版本）；`TodoDb` 持 `Option<Mutex<Connection>>`，开库失败降级为「不可用」并 `log::warn!`，不中断应用；`TodoDbError` 为手写枚举 + `Display`/`Error`；`list`（created_at DESC）/`save`（`ON CONFLICT(id) DO UPDATE` upsert）/`delete`（未知 id 也成功）；同文件 `#[cfg(test)] mod tests` 覆盖增删改查、排序、不可用降级与「重开同一文件数据仍在」。
  - `src-tauri/src/lib.rs`：`mod todo_db`；新增三个 `#[tauri::command] #[specta::specta]` 命令 `list_todos` / `save_todo` / `delete_todo`（实体直接复用 `todo_contracts::Todo`，不新建重复实体），注册进 `collect_commands!`；新增 `setup_todo_db`，在 `setup` 中把库开在 app data 目录 `todos.db` 并 `app.manage`，路径解析或建目录失败时降级为不可用库。
  - `src/bindings/commands.ts`：`pnpm run types:generate` 生成产物（未手改）。
  - `src/lib/todo-repository.ts`（新增）：`TodoRepository` 接口（`load` / `save` / `remove`）与两个实现——Tauri 运行时走生成的 command，其余运行时走 `localStorage`（key `todos.items`）；原生调用失败（含命令返回 error）时 `writeDiagnostic("warn", …)` 后回退 localStorage，与 `settings-storage.ts` 的降级写法一致。`todoRepository()` 按 `isTauri()` 选实现。
  - `src/stores/todos.ts`：store 内注入 `todoRepository()`，新增 `hydrate()`；`add` / `update` / `toggle` / `applyRemoteUpsert` 写 `repository.save`，`remove` / `applyRemoteDelete` 写 `repository.remove`。组件与 `App.vue` / `TodayCard.vue` 零改动。
  - `src/main.ts`：启动时在 `rescheduleAll` 与同步之前 `await useTodoStore(pinia).hydrate()`。
  - `TODO.md`：6.1「SQLite 落地」⏳ → 🚧（桌面端已落地，移动端待验证）。
  - `Cargo.lock`：随依赖新增自动更新。
- 为 02/03 预留：02 的迁移逻辑与 03 的新业务字段本次均未做。留给 02 的接口＝`TodoRepository` 这一层收口点与 `save`/`load` 命令；留给 03 的接口＝实体单源仍是 `crates/contracts::Todo`（新增字段只需改契约 + `todo_db.rs` 的建表与行映射 + 重跑 `types:generate`），并已用 `PRAGMA user_version` 记录 schema 版本。
- 自验结果（命令与结论）：
  - `cargo test --workspace`：通过（新增 5 个 `todo_db` 测试；全工作区 11 passed / 0 failed）。
  - `cargo check --workspace`：通过。
  - `pnpm run types:generate`：通过，`src/bindings/commands.ts` 已同步三个新命令与 `Todo` 类型；`src/bindings/models/**` 无变化。
  - `pnpm run check`：通过（40 文件格式、35 文件 lint + 类型检查零告警）。
  - 手动走查（浏览器 `pnpm run dev`，验收标准第 2 条）：**已执行**。新增待办后 `localStorage["todos.items"]` 出现该条；刷新页面列表仍显示（“1 项待完成”）；勾选完成后 localStorage 中 `status` 变 `completed` 且写入 `completedAt`；删除后 localStorage 变 `[]`。控制台无错误，仅有既有的 `Sync failed: Sync transport is only available in the Tauri desktop runtime`（浏览器下同步本就 no-op，非回归）。
  - 手动走查（桌面 `pnpm run tauri:dev`，验收标准第 1 条）：**部分执行**。应用可启动、原生侧正常，但 Vue 界面无法挂载，原因是**既有缺陷**（与本任务无关）：`src-tauri/capabilities/default.json` 缺 `core:window:allow-set-theme`，`src/lib/appearance.ts` 的 `applyTheme` 调 `getCurrentWindow().setTheme()` 抛 `window.set_theme not allowed`，该异常发生在 `main.ts` 首个 `await useSettingsStore(pinia).bootstrap()`（早于本次新增的 `hydrate()`）中，顶层 await 被 reject 导致 `createApp().mount()` 不执行。故改用替代证据：以 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` 启动应用，通过 WebView2 调试协议在真实 Tauri 运行时内执行脚本，验证结果如下：
    - 直接 `invoke('save_todo'/'list_todos')` 写入两条待办并读回，字段（含 `dueDate`/`completedAt`/`status`）完整；`%APPDATA%/com.todo.crossplatform/todos.db` 已生成。
    - 完全退出应用并重启后再 `list_todos`，两条数据原样返回 → **重启不丢数据**成立。
    - 在 Tauri 运行时内 `import('/src/lib/todo-repository.ts')` 后 `save`/`load`/`remove` 均落 SQLite，`localStorage["todos.items"]` 始终为 `null` → repository 在桌面端确实走原生路径、未误落回退分支。
    - 在 Tauri 运行时内建 Pinia 后 `useTodoStore().hydrate()` 读回既有两条；`add` / `toggle` / `remove` 后 `list_todos` 依次可见新增、状态翻转与删除 → store→repository→SQLite 全链路成立。
    - 验证后已删除测试用 `todos.db`（含 `-wal`/`-shm`），未在环境中留下脏数据。
- 第 2 轮（评审回流）改动文件：
  - `src/lib/today-card.ts`：新增导出 `isTodayCardWindow()`，把「当前 webview 是不是卡片窗口」的判定与卡片 URL 常量放在同一模块，供各处复用（原判定散在 `App.vue` 内联）。
  - `src/App.vue`：`const isCard` 由内联的 `URLSearchParams` 读取改为调用 `isTodayCardWindow()`；渲染与其余逻辑未动。
  - `src/main.ts`：`hydrate()` 与 `rescheduleAll()` 收进 `if (!isTodayCardWindow())`，卡片窗口不再填 store、不再排程提醒（修复第 1 轮阻塞条目）。
  - `src-tauri/src/todo_db.rs`：`initialise` 改为先读 `PRAGMA user_version` 再决定是否写入，只有「从未写过版本」的新库才被打上 `SCHEMA_VERSION`，已有版本保留原值并在与期望不一致时 `log::warn!`（修复第 1 轮 `user_version` 建议条目）；新增两个版本位用例。
- 第 2 轮自验结果（命令与结论）：
  - `cargo test --workspace`：通过（`todo_db` 由 5 个增至 7 个用例；全工作区 13 passed / 0 failed）。
  - `cargo check --workspace`：通过。
  - `pnpm run check`：通过（40 文件格式、35 文件 lint + 类型检查零告警；首次因新函数换行格式失败，已 `vp check --fix` 后复跑通过）。
  - `pnpm run types:generate`：重跑通过，`src/bindings/**` 无新增差异（本轮未改 command 与契约，属确认无漂移）。
  - 手动走查（浏览器 `pnpm run dev`，今日卡片场景 — 第 1 轮评审指出上一轮未覆盖）：**已执行**。先在 localStorage 预置两条待办（其中一条带已过期的 `reminderAt`）。
    - 打开 `http://127.0.0.1:1420/`（主窗口路径）：控制台依次打印 `Settings restored` → `Todos restored from local storage {"count":2}` → `Reminders rescheduled on startup`。
    - 打开 `http://127.0.0.1:1420/?card=today`（卡片窗口路径）：只打印 `Settings restored`，**没有** `Todos restored from local storage`，也**没有** `Reminders rescheduled on startup` → 卡片窗口不再 hydrate、不再重复排程，重复通知的来源被切断。
    - 同一页面 `location.search === "?card=today"`、`<h1>` 为「今日待办」→ 卡片视图正常渲染（浏览器下无 Tauri 事件，列表为空属预期）。
  - 手动走查（桌面 `pnpm run tauri:dev`，今日卡片场景）：**GUI 走查仍不可行**，原因是评审第 1 轮已记录、本轮裁决为「不动」的既有缺陷 `core:window:allow-set-theme`（本轮实测复现：dev 日志报 `window.set_theme not allowed. Permissions associated with this command: core:window:allow-set-theme`，`main.ts` 顶层 await 被 reject，Vue 不挂载）。故沿用上一轮的替代证据手法，以 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` 启动，通过 WebView2 调试协议在真实运行时内分层验证：
    - 从主窗口 `openTodayCard()` 成功建出卡片窗口，调试目标列表出现 `http://localhost:1420/index.html?card=today`。
    - 在主窗口求值 `isTodayCardWindow()` → `false`；在卡片 webview 求值 → `true`，且 `location.search === "?card=today"` → `main.ts` 的新分支在真实运行时判定正确。
    - 卡片 webview 的 `localStorage["todos.items"]` 始终为 `null` → 卡片窗口没有走任何持久化路径。
    - 在主窗口按 `main.ts` 的顺序手动 `hydrate()` 后再 `setupTodayCardSync()`，捕获其向 `today-card:todos` 推送的载荷：内容正是从 SQLite 恢复出来的今日任务（`{id, title, dueDate: "2026-07-23", status: "open"}`）→ 卡片的数据源仍是主窗口 store→事件推送这条链，与卡片窗口是否 hydrate 无关。
    - 直接读实库版本位：`node:sqlite` 打开 `%APPDATA%/com.todo.crossplatform/todos.db`，`PRAGMA user_version` = 1、`journal_mode` = wal → `user_version` 修复后新库仍被正确打版（注意 WAL 未 checkpoint 时直接读文件头第 60 字节会看到 0，须经 SQLite 读取）。
    - 验证用待办已全部删除（`load()` 返回 0 条），随后连 `todos.db`（含 `-wal`/`-shm`）一并删除，环境无脏数据。
- 遗留问题：
  1. 【既有缺陷，非本次引入】`capabilities/default.json` 缺 `core:window:allow-set-theme`，导致 `pnpm run tauri:dev` 下整个前端无法挂载。按「不改与任务无关的代码」未在本任务修复，建议编排者另建直达级任务处理（同时可考虑让 `settings` store 的 bootstrap 兜底不再向外抛）。
  2. 移动端（Android/iOS）未验证：rusqlite `bundled` 在移动端理论可编译，但本机无移动工具链，故 TODO.md 6.1 标为 🚧 而非 ✅。
  3. 【既有缺陷，非本次引入，第 2 轮实测新发现】`src-tauri/capabilities/default.json:5` 的 `"windows": ["main"]` 只覆盖标签为 `main` 的窗口，卡片窗口标签是 `today`，于是它拿不到任何权限：在卡片 webview 内调用 `listen`/`emit` 均被拒（`event.listen not allowed on window "today" … allowed on: [windows: "main"]`），卡片在真实桌面运行时收不到 `today-card:todos`。这与本次改动无关（该文件与 `today-card-sync.ts` 本次均未改动，属里程碑 5 遗留），也正是本轮「卡片仍能显示数据」只能验证到「主窗口推送载荷正确」这一层的原因。建议编排者与遗留问题 1 合并成一个 capabilities 直达任务处理（把卡片窗口纳入权限范围）。

## 评审记录

### 第 1 轮

- 阻塞 · `src/main.ts:15` · `hydrate()` 无条件在每个 webview 执行，今日卡片窗口（`index.html?card=today`，见 `src/lib/today-card.ts:40`）同样加载 `main.ts`，于是卡片窗口的 Pinia store 首次被填满，紧随其后的 `rescheduleAll()`（`src/main.ts:16`）在卡片窗口再排一套 `setTimeout` 提醒，并对已过期提醒立即 `fire`（`src/lib/notifications.ts:121-125`）——用户会收到重复通知，打开今日卡片开关时还会被过期提醒刷屏；改动前卡片窗口此刻 store 为空，`rescheduleAll` 是 no-op，故属本次引入的回归 · AGENTS.md 通用「改动前先确认影响范围，尽量减少对无关模块的影响」＋正确性维度（`src/App.vue:40-42` 已有 `isCard` 判定可复用） · 附：手动走查未覆盖今日卡片场景
  - 处理（开发）：已修。`src/main.ts` 把 `hydrate()` 与 `rescheduleAll()` 一并收进 `if (!isTodayCardWindow())`；判定从 `src/App.vue:40-42` 提取为 `src/lib/today-card.ts` 的 `isTodayCardWindow()`（与卡片 URL 常量同模块，单一来源），`App.vue` 改为调用它。
    - 取舍（卡片数据从哪来）：卡片窗口的数据本就不来自它自己的 store。`TodayCard.vue` 只订阅 `today-card:todos` 事件（`src/lib/today-card-sync.ts:113` 的 receiver），列表由主窗口 `setupTodayCardSync()` 用 `selectTodayOpenTodos(主窗口 store)` 推送；卡片窗口的 Pinia store 自始至终没有任何读取方。因此「卡片窗口不 hydrate」不会让卡片显示不出数据，只是把改动前的语义（卡片窗口 store 为空 → `rescheduleAll` 是 no-op）恢复回来。
    - `rescheduleAll()` 一并收进条件而不是只挡 `hydrate()`：它才是重复通知的直接来源，只挡 hydrate 会让「卡片窗口不排程」依赖「store 恰好没人填」这个隐含前提（同步引擎的 `applyRemoteUpsert` 也会填 store），条件写在源头更稳。
    - 未改动事件通道、`TodayCard.vue` 与 `today-card-sync.ts` 的任何逻辑。
    - 实测见「实现记录」的自验结果：浏览器下卡片页不再打印 `Todos restored from local storage` / `Reminders rescheduled on startup`（主窗口页照常打印），Tauri 运行时下卡片 webview 的 `isTodayCardWindow()` 为 `true`、主窗口为 `false`，且主窗口推给卡片的 `today-card:todos` 载荷仍是从 SQLite 恢复出来的今日任务。
  - 复核（评审）：认可。四点均独立核实：① 排程链路确已切断——`src/main.ts:23-26` 把 `hydrate()`/`rescheduleAll()` 收进守卫，卡片窗口内再无 `scheduleReminder` 的调用方（`applyRemoteUpsert`/`applyRemoteDelete` 都不排程，见 `src/stores/todos.ts:130-166`；`add`/`toggle` 只由主窗口经 `today-card-sync.ts:85-95` 执行）；② 数据显示链路未破——独立核实开发主张成立：`src/components/TodayCard.vue` 全文不 import `useTodoStore`，列表只来自 `setupTodayCardReceiver` 订阅的 `today-card:todos`（`TodayCard.vue:18-25`），写回也只是 `emitToggle`/`emitRemove` 事件，卡片自身 store 确无读取方；③ `rescheduleAll()` 一并挡掉不引入新问题——`src-tauri/src/lib.rs:186-194` 的关闭事件是 `prevent_close` + `hide()`，主窗口隐藏到托盘后 webview 与 `setTimeout` 计时器均存活，提醒仍由主窗口发出，卡片窗口从来不需要排程（改动前它 store 为空，`rescheduleAll` 本就是 no-op，语义一致）；④ `App.vue` 等价——`git diff` 显示仅把 `typeof window !== "undefined" && new URLSearchParams(...).get("card") === "today"` 换成同逻辑的 `isTodayCardWindow()`（`today-card.ts:53-58` 保留了 `typeof window === "undefined"` 短路），返回值与渲染分支不变；`today-card.ts` 无模块级副作用，且此前已在 `App.vue` 的依赖图内，`main.ts` 新增 import 不改变加载行为。
- 建议 · `src/lib/todo-repository.ts:75-118` · 原生调用失败时写入 `localStorage["todos.items"]`，但 `load` 只要原生 `listTodos` 返回 ok 就直接返回并且从不读回退键——「开库失败期间产生的数据」在下次开库成功后永久不可见，只留一条 warn 诊断，属静默丢数据 · AGENTS.md 客户端「原生侧失败必须容错降级」（降级本身合规，但缺回收路径）；与 02「Store 迁移／失败自动回退旧路径」同类，建议编排者裁决是否并入 02 处理
  - 处理（编排者）：转任务处理，并入 数据与同步/02-Store迁移。02 的验收标准正是「迁移失败自动回退旧路径，双跑验证」，「回退期间产生的数据如何回收」与它是同一个问题域，分开做会两次改动同一段回退逻辑。
- 建议 · `src-tauri/src/todo_db.rs:106-112` · `initialise` 无条件 `pragma_update(user_version = SCHEMA_VERSION)`，从不先读现存版本；03 扩展字段把常量升到 2 时，旧库会在未执行任何迁移的情况下被盖成 2，版本位失去判据 · 本任务「为 02/03 预留」目标（版本位需可判版本）
  - 裁决（编排者）：本轮处理。版本位是 03「契约扩展 v1 全字段」的迁移判据，本任务「为 02/03 留口子」是其显式目标；此处不修，03 会带着一个假的版本位开工，代价远高于现在补一行读取判断。
  - 处理（开发）：已修。`initialise` 改为先 `PRAGMA user_version` 读现存版本再决定写不写：读到 0（SQLite 对「从未写过版本」的库的回答，也就是刚建出来的文件）才写入 `SCHEMA_VERSION`；已有非 0 版本一律原样保留，与 `SCHEMA_VERSION` 不一致时只 `log::warn!` 记一条并把处置权交给迁移步骤。这样 03 把常量升到 2 时，旧库仍是 1，版本位可作判据。按任务边界，本轮不实现任何真实迁移逻辑（属 02/03）。新增两个用例：`a_never_stamped_database_records_the_current_schema_version`、`an_already_recorded_schema_version_is_not_overwritten`。
  - 复核（评审）：认可。`src-tauri/src/todo_db.rs:119-127` 确为先 `query_row("PRAGMA user_version")` 再判断：`stored == 0` 才 `pragma_update`，非 0 一律保留、与 `SCHEMA_VERSION` 不符时只 `log::warn!`，03 把常量升到 2 时旧库仍读回 1，版本位对迁移可判据成立。两个用例覆盖了两条分支：`a_never_stamped_database_...` 先断言新建库为 0 再断言被打成 `SCHEMA_VERSION`（新库正常打版），`an_already_recorded_schema_version_is_not_overwritten` 用 `SCHEMA_VERSION + 1` 模拟「非当前版本的既存库」再 `initialise`，断言版本位未被覆盖——这正是「旧库不被误盖」的那条 `stored != 0` 分支（因 `SCHEMA_VERSION` 当前为 1，无法在不改常量的前提下构造 stored=1/期望=2，用 +1 走同一分支等价可接受）。两个用例本轮实跑通过。
- 建议 · `src/lib/todo-repository.ts:15-22` · `save`/`remove` 返回 `Promise<void>` 且在内部吞掉失败，调用方无法得知写入是否真的落到 SQLite；02 的「迁移失败自动回退旧路径」需要这个信息，接口形状届时需扩展（本任务不必改，记录以免 02 返工） · 本任务「为 02/03 预留」目标
  - 处理（编排者）：转任务处理，并入 数据与同步/02-Store迁移。02 需要「写入是否真的落库」的信号才能判定迁移成败，接口形状的扩展随 02 一并做，避免本任务先改一版、02 再改一版。
- 建议 · `src-tauri/capabilities/default.json:6-27` · 缺 `core:window:allow-set-theme`，`src/lib/appearance.ts:23` 的 `setTheme()` 抛权限错误；`src/stores/settings.ts:47-53` 的 catch 分支又一次 `await applyTheme(...)` 二次抛出，兜底失效，最终 `main.ts` 顶层 await 被 reject、Vue 不挂载。经 `git log`/`git status` 佐证：该两文件本次未改动，最后一次改动为 df1db9e，确属既有缺陷而非本次引入 · 超出本任务范围，建议编排者另建直达任务（同时修 bootstrap 的兜底）
  - 处理（编排者）：转任务处理，另建直达任务 .agents/tasks/修复卡片窗口权限与同步.md。该缺陷使 `pnpm run tauri:dev` 的常规 GUI 走查在后续所有子任务中都无法进行，属全局阻碍，优先于 02 执行。

越界与收口核查结论（无问题，不计条目）：未见 02 的迁移逻辑与 03 的新业务字段；`todos` 表列与 `crates/contracts::Todo` 字段一一对应，实体单源成立，`src/types/todo.ts` 仅再导出生成类型；`src/` 内除 `todo-repository.ts` 外无组件触碰持久化（`settings-storage.ts` 属设置域，非本任务范围）；`applyRemoteUpsert`/`applyRemoteDelete` 均已走 repository；新增依赖仅 rusqlite（`bundled`），未引入 `thiserror`/`anyhow`（Cargo.lock 中二者均为既有传递依赖）。

验证：`cargo test --workspace` 通过（11 passed / 0 failed，含 5 个 `todo_db` 用例）；`cargo check --workspace` 通过；`pnpm run check` 通过（40 文件格式、35 文件 lint+类型零告警）；`pnpm run types:generate` 重跑后 `src/bindings/**` 无新增差异（仍为 commands.ts +40 行），生成产物与调用方同步；`git status` 改动清单与「实现记录」一致，无清单遗漏；桌面手动走查的替代证据（WebView2 调试协议在真实 Tauri 运行时验证 invoke／重启不丢／repository 未误落回退／store 全链路）判定为足以支撑验收标准第 1 条，但未覆盖今日卡片窗口场景，见本轮阻塞条目。

### 第 2 轮

- 建议 · `src/main.ts:28-31`（联动 `src/stores/todos.ts:130-160`、`src/lib/todo-repository.ts:90-103`）· 本轮把 `hydrate()`/`rescheduleAll()` 收进了 `if (!isTodayCardWindow())`，但紧随其后的 `initSyncEngine()` / `syncNow()` 仍在卡片窗口无条件执行，而卡片窗口的 store 现在恒为空：它一旦拉到远端变更，`applyRemoteUpsert` 全部命中「未知 todo → 物化」分支（`src/stores/todos.ts:147-159`），以 `createdAt = occurredAt`、`reminderAt: null` 造行并经 `persist()` 写回 repository，等于用一份丢了 `reminderAt`、`createdAt` 被改写的副本覆盖真实数据。链路可达性已核实：`src/lib/sync-transport.ts:54` 用的是 WebView 原生 `fetch`，不受 capabilities 管辖，卡片窗口确实会真的发起同步（且它读不到主窗口的 store 插件，`settings-storage.ts:98-105` 回退 localStorage → 自己生成 deviceId、cursor 从 0 起，等于拉全量）。**本轮不判阻塞**的原因：`save_todo` 这条 IPC 目前被 `src-tauri/capabilities/default.json:5` 的 `"windows": ["main"]` 挡下，写入落到 `todo-repository.ts:102` 的 localStorage 回退（与主窗口同源，共享 `todos.items` 键），SQLite 暂时写不坏；且该缺陷在第 1 轮代码里同样存在（彼时卡片窗口的 `hydrate()` 在真实桌面运行时也因同一权限限制回退到空数组），不属本轮改动新引入 · 正确性维度 ＋ AGENTS.md 通用「改动前先确认影响范围」 · 提请编排者注意时序：遗留问题 1/3 的 capabilities 修复一旦把卡片窗口纳入权限范围，这条会立刻从「潜在」变成真实的 SQLite 覆写（`reminder_at` 丢失、`created_at` 被改写、列表排序错乱），建议把「卡片窗口不跑同步引擎」（即把 `initSyncEngine()`/`syncNow()` 一并纳入同一守卫）与那个 capabilities 任务放在一起做，或并入 02
  - 处理（编排者）：转任务处理，并入 .agents/tasks/修复卡片窗口权限与同步.md。评审已指出时序风险——capabilities 一旦放开卡片窗口权限，本条会立刻从潜在升级为真实的 SQLite 覆写，故必须与权限修复同一个任务落地，不得拆开。

验证：`cargo test --workspace` 通过（13 passed / 0 failed：`cross_platform_todo_lib` 8 个含 7 个 `todo_db` 用例、contracts 1、domain 1、server 3）；`cargo check --workspace` 通过；`pnpm run check` 通过（40 文件格式、35 文件 lint+类型零告警）；`git status` 与「第 2 轮（评审回流）改动文件」一致（`src/App.vue`、`src/main.ts`、`src/lib/today-card.ts`、`src-tauri/src/todo_db.rs`，其余为第 1 轮既有改动），无清单遗漏、无越界改动；`src/bindings/**` 未随本轮变化（跑测试后 `git diff --stat` 仍为 commands.ts +40 行，与第 1 轮一致），生成产物与调用方保持同步。本轮无阻塞问题。
