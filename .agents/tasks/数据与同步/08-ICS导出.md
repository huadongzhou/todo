# ICS导出

- 级别：简化
- 状态：完成

## 需求（产品）

- 目标与场景：任务可导出为 .ics 日历文件，进主流日历应用。
- 范围（做 / 不做）：做 导出生成（含周期任务规则映射）与导出入口；不做 订阅 feed（09）。
- 验收标准（逐条可检查）：
  - [ ] 导出文件被主流日历（Outlook/Google/Apple 任一）正确识别，周期任务展开正确。
  - [ ] 导出入口可用，空数据导出有合理提示。

## 规格（UI/UX）

本节只覆盖**导出入口与其状态**；.ics 文件格式、周期规则映射、导出范围的取数逻辑属开发范围，不在此定义。

### 0. 入口位置的判断与理由

**结论：放在既有设置面板内，新增一个 `<fieldset>`「日历」，排在「同步」之后（即面板最末）。**

理由四条：

1. **动作性质对得上**。导出是设备级、低频、一次性动作，与设置面板里已有的「立即同步」同类（App.vue:319-326：一个 ghost 按钮 + 一行状态文字）。同一类动作放同一处，用户不必学第二套心智模型。
2. **不占用主界面的唯一主操作位**。DESIGN.md 设计原则 1「单屏只有一个主操作」——主界面的 Primary CTA 是「添加」（App.vue:360）。在页头再加一个导出按钮会与之竞争，也会把当前只有 2 个控件（今日图标 + 设置）的页头挤成工具栏。
3. **它是全量动作，不是单条动作**。导出的对象是任务集合而非某一条任务，所以不放待办行内（待办行现有的操作是完成/删除，均为单条语义）。
4. **给 09（ICS 订阅）留位**。fieldset 取名「日历」而非「导出」，09 的订阅地址控件可直接落进同一个 fieldset，不必再造新分区。

**不按 `isDesktop` 隐藏**：既有的「桌面行为」「同步」两个 fieldset 带 `v-if="settingsStore.isDesktop"`（App.vue:244、291），本 fieldset **不带**——导出在浏览器与移动端同样有意义（下载 / 系统分享）。若某运行时确实无法落地导出，按「不展示无解释的死控件」原则**整块隐藏 fieldset**，不要渲染成永久禁用态。

### 1. 结构与布局

新增：`src/App.vue` 设置面板内一个 fieldset；不新建组件（结构量不足以拆组件，且与相邻 fieldset 同构，拆开反而割裂）。复用组件：`src/components/ui/button/Button.vue`（`variant="ghost"`）。

```
<fieldset>                      ← m-0 mt-6 border-0 p-0（同 App.vue:290-293）
  <legend>日历</legend>          ← mb-3 font-medium text-slate-800 dark:text-slate-200（同 App.vue:295）
  <p>说明文案</p>                ← mb-3 text-xs text-slate-500 dark:text-slate-400（同 App.vue:328）
  <div>                         ← flex flex-wrap items-center justify-between gap-3（同 App.vue:310，加 flex-wrap）
    <span>行内状态文字</span>     ← text-xs text-slate-500 dark:text-slate-400（同 App.vue:311）
    <Button variant="ghost">导出 .ics</Button>  ← class="min-h-11 !px-3 text-sm"
  </div>
</fieldset>
```

- 说明文案：「把带日期的任务导出为 .ics 文件，可导入 Outlook、Google 日历或苹果日历。」
- 按钮文案：默认「导出 .ics」；进行中「导出中…」。不带图标（与「立即同步」一致，避免同层级引入新图标尺寸）。
- 行内状态文字（左侧）只承载**非结果性**信息：未导出过时为空（该 `<span>` 不渲染，靠 `justify-between` 自然右对齐按钮）；导出过一次后显示「上次导出：{本地时间}」，格式复用 App.vue:132 `formatSyncTime` 的 `toLocaleString()` 口径。
- **结果反馈复用既有提示条，不新造机制**：App.vue:186-194 的 `role="status"` 提示条是本仓库已确立的页面级提示模式。做法——把 `STORAGE_ALERT_CLASS`（App.vue:119-122）改名为通用的 `ALERT_TONE_CLASS` 并补一个 `success` 键，模板里把 `todoStore.storageAlert` 换成一个 `pageAlert` 计算属性：

  ```
  pageAlert = todoStore.storageAlert ?? exportAlert
  ```

  即**同一个 DOM 节点、同一套配色、同一个 live region**，只是数据源多了一个。优先级固定为「存储告警 > 导出结果」——存储告警关乎数据丢失，不能被一句导出成功挤掉。`exportAlert` 是 `App.vue` 内的局部 `ref<{ tone, message } | null>`（形状与 `StorageAlert` 一致），不进 store（它不是待办域的状态）。
- 提示条的通用视觉基线由「桌面端形态/01-主窗导航壳」统一定稿（编排者裁决），本任务不定义，只按现状复用。

### 2. 状态

| 状态 | 触发 | 按钮 | 提示条（`pageAlert`） | 行内状态文字 |
|------|------|------|----------------------|-------------|
| 默认 | — | 「导出 .ics」，`variant="ghost"` | 不显示 | 空 / 「上次导出：…」 |
| 悬停 | 指针悬停 | ghost 变体既有 `hover:bg-slate-100 hover:text-slate-950`（Button.vue:12） | 不变 | 不变 |
| 聚焦 | Tab / 点击 | ghost 变体既有 `focus-visible:focus-ring`（Button.vue:7） | 不变 | 不变 |
| 加载（进行中） | 点击后 | `disabled` + 文案「导出中…」+ `aria-busy="true"`；禁用态沿用 CVA 的 `disabled:opacity-50 disabled:pointer-events-none` | 清空上一次结果（避免旧结果与新动作并存） | 不变 |
| 成功 | 文件已产出 | 恢复默认，并把焦点显式还给按钮 | `success` 调：「已导出 {n} 项任务。」桌面端补出文件去向：「已导出 {n} 项任务：{文件名或路径}。」 | 更新为「上次导出：{本地时间}」 |
| 空数据 | 可导出任务数为 0 | 恢复默认 | `warn` 调：「没有可导出的任务：先给任务设置截止日或提醒时间，再试一次。」 | 不变（不写「上次导出」，因为没产出文件） |
| 错误 | 写文件失败 / 生成失败 | 恢复默认 | `error` 调：「导出失败：{原因}。请重试或换一个保存位置。」拿不到原因时退化为「导出失败，请重试。」 | 不变 |
| 用户取消 | 系统保存/分享面板被取消 | 恢复默认 | **不显示任何提示**（取消不是错误） | 不变 |

补充规则：

- 空数据文案必须给下一步动作（DESIGN.md「空状态」基准：说明文案 + 下一步引导），只说「没有数据」不合格。
- 成功提示**必须点名文件去向**（文件名或完整路径），否则用户不知道文件落在哪；具体是系统保存对话框还是默认目录由开发选择，UI 只要求结果里能读到去向。
- 加载态门槛：DESIGN.md「空状态」基准要求超过 300ms 才显示进度提示；导出耗时不可预知，因此**点击后立即进入「导出中…」**（比 300ms 后再变更稳，且避免用户重复点击），不加骨架屏、不加旋转图标（避免为此引入新的动效与图标资产）。
- 结果提示的消失时机：**下一次点击导出时清空**，或**关闭设置面板时清空**（`settingsOpen` 由 true → false 时把 `exportAlert` 置 null）。不做定时自动消失——那会引入一套本仓库尚不存在的计时消息机制。
- 重入保护：进行中按钮已 `disabled`，处理函数另需守卫，避免键盘重复触发产生两个文件。

### 3. 暗色适配（浅/暗成对）

| 元素 | 浅色 | 暗色 | 出处 |
|------|------|------|------|
| legend 文字 | `text-slate-800` | `dark:text-slate-200` | App.vue:295 现状 |
| 说明与行内状态文字 | `text-slate-500` | `dark:text-slate-400` | App.vue:311/328 现状；DESIGN.md 中性色「次要文字」 |
| ghost 按钮 | `text-slate-600` + `hover:bg-slate-100` | 沿用 Button.vue 现有 ghost 定义 | Button.vue:12（现状即无 `dark:` 分支，本任务不改按钮变体） |
| 提示条 · success | `bg-emerald-100 text-emerald-700` | `dark:bg-emerald-900/40 dark:text-emerald-300` | DESIGN.md「语义色」基准行：新语义场景沿用「浅 100 底 + 700 字 / 暗 900/40 底 + 300 字」，成功色用 emerald |
| 提示条 · warn | `bg-amber-100 text-amber-700` | `dark:bg-amber-900/40 dark:text-amber-300` | App.vue:120 现状 |
| 提示条 · error | `bg-red-100 text-red-700` | `dark:bg-red-900/40 dark:text-red-300` | App.vue:121 现状 |
| 焦点环 | `focus-ring`（ring-offset-white） | `focus-ring`（`dark:ring-offset-slate-950`） | `uno.config.ts` shortcut |

### 4. 移动端适配

- **位置不变**：入口仍在设置面板内。移动端整体导航属「移动端形态」母任务，本任务不引入平台分支的入口。
- **触控区**：按钮 `min-h-11`（44px，DESIGN.md「间距与布局」触控基准；`h-11` 在 App.vue:169 已有同值用法）。它与相邻的「立即同步」按钮之间由 fieldset 的 `mt-6`（24px）拉开，满足相邻可点目标 ≥ 8px。
- **窄屏行为**：状态行容器加 `flex-wrap`，宽度不足时状态文字与按钮**换行堆叠**（状态文字在上、按钮在下、左对齐），不设固定宽度、不加新断点、不出现横向滚动（DESIGN.md「禁止移动端横向滚动」）。说明文案与提示条文字自然换行，不截断、不省略号。
- **提示条**：与桌面端同位置（页头下方），窄屏下整行显示，`px-3 py-2` 内边距不变。
- **系统面板接管**：移动端点击后由系统分享/保存面板接管，此期间按钮保持「导出中…」；面板被取消 → 按「用户取消」处理（无提示）。
- 最小视口 320px 下上述布局不溢出（说明文案与按钮均为流式，无固定宽度）。

### 5. 可访问性

- **可访问名称**：按钮为文字按钮，可访问名即「导出 .ics」/「导出中…」，不需要 `aria-label`（DESIGN.md 只要求纯图标按钮补 `aria-label`）。
- **分组语义**：`<fieldset>` + `<legend>日历</legend>`，与相邻两个 fieldset 同构；沿用 App.vue:291-296 的 `aria-labelledby` 写法给 legend 一个 id（如 `calendar-heading`）。
- **键盘**：按钮在设置面板 DOM 顺序中排在同步控件之后，Tab 顺序与视觉顺序一致；Enter/Space 触发；焦点样式来自 CVA 的 `focus-visible:focus-ring`，禁止移除。
- **忙态与焦点**：进行中 `aria-busy="true"`；因为按钮此时是 `disabled`，浏览器可能丢焦点——结束后若焦点已落到 `<body>`，用 ref 把焦点显式还回按钮，屏幕阅读器用户不会掉出上下文。
- **播报**：结果只经既有的 `role="status" aria-live="polite"` 提示条播报，**不新增第二个 live region**（同页两个 live region 会互相打断）。错误同样用 polite——它跟随用户主动点击发生，不属于需要打断的紧急信息，且与既有 `error` 调存储告警的处理保持一致。
- **不靠颜色单传**：三种调性都由文字直接说明结果（成功/没有可导出的任务/导出失败），符合 DESIGN.md「状态信息必须有文字伴随」。
- **对比度**：提示条三调沿用 DESIGN.md 语义色公式，与既有到期标签、存储告警同源；ghost 按钮的 `text-slate-600` 压白底同现状。DESIGN.md 已记录的 `default` 按钮对比度偏差与本任务无关（本任务用 ghost）。

### 6. 令牌出处自查

| 取值 | 出处 |
|------|------|
| `mt-6` / `mb-3` / `gap-3` / `px-3 py-2` | DESIGN.md「间距与布局」4/8 节奏 + App.vue 既有设置面板同款 |
| `min-h-11`（44px） | DESIGN.md「间距与布局」触控目标 ≥ 44×44px；`h-11` 现状见 App.vue:169 |
| `text-sm`(14) / `text-xs`(12) | DESIGN.md「字体」字号阶梯：14 按钮与次要正文、12 辅注 |
| `font-medium` | DESIGN.md「字体」字重：标签 500 |
| `rounded-lg` | DESIGN.md「圆角与阴影」：按钮与输入 `rounded-lg`；提示条沿用现状 |
| slate-500/600/800、slate-200/400 暗色 | DESIGN.md「中性色」slate 阶 |
| amber / red 提示条配色 | DESIGN.md「语义色」表 + App.vue:119-122 现状 |
| emerald 成功配色 | DESIGN.md「语义色」基准行明文规定（公式 + 「成功色用 emerald」），非自造 |
| `focus-ring` / `surface-card` | `uno.config.ts` shortcuts |
| ghost 按钮配色与禁用态 | `src/components/ui/button/Button.vue` CVA |

**结论：无新增设计令牌。** 所有取值均可在 DESIGN.md、`uno.config.ts` 或既有代码中找到出处；emerald 成功调是 DESIGN.md 语义色基准的直接套用，不是新令牌。

### 7. 交给编排者/开发的两点说明（不阻塞）

1. **导出范围的措辞**：说明文案与空数据文案按「导出带日期的任务」写。若开发确定的实际范围不同（例如含无日期任务、或仅导出未完成任务），请同步调整这两句文案，使界面说的与实际做的一致——文案随实现走，不需回退设计阶段。
2. **文件去向的实现方式**：桌面端用系统保存对话框还是写入默认目录，由开发按现有 capability 决定（当前 `src-tauri/capabilities/` 未含 dialog/fs 权限）。UI 侧的硬要求只有一条：成功提示里必须能读到文件名或路径。

## 实现记录（开发）

### 改动文件

| 文件 | 改动 |
|------|------|
| `crates/domain/src/ics.rs` | 新增。纯规则：`build_calendar(todos, generated_at_unix) -> Calendar{content, event_count}` 与 `export_file_name`。含 RFC 5545 折行（75 octet，按字符边界）、TEXT 转义、CRLF、日期/瞬时解析与格式化、RRULE 映射。22 个单测同文件。不依赖 Tauri / HTTP / 数据库。 |
| `crates/domain/src/lib.rs` | 仅加 `pub mod ics;` 一行。 |
| `src-tauri/Cargo.toml` | 新增 `todo-domain` 路径依赖（客户端 → domain → contracts，方向未反向）。 |
| `Cargo.lock` | 由上一行的路径依赖引起：`cross-platform-todo` 的依赖列表多出 `todo-domain` 一行。非手改，`cargo` 自动写入。 |
| `src-tauri/src/lib.rs` | 新增 `CalendarExport` 出参（`path: Option<String>` + `event_count`）、`export_calendar` command（`#[tauri::command]` + `#[specta::specta]`，已进 `collect_commands!`）、`export_directory`、手写错误枚举 `CalendarExportError` + `Display`/`Error`；bindings 导出测试补 `CalendarExport::export`。 |
| `src/bindings/commands.ts`、`src/bindings/models/CalendarExport.ts` | 生成产物，由 `pnpm run types:generate` 刷新，未手改。 |
| `src/lib/calendar-export.ts` | 新增。平台探测 `canExportCalendar()` + `exportCalendar()`，把 command 结果收敛为 `exported / empty / failed` 三态。 |
| `src/App.vue` | `STORAGE_ALERT_CLASS` → `ALERT_TONE_CLASS` 并补 `success` 键；新增 `pageAlert` 计算属性（`todoStore.storageAlert ?? exportAlert`）、`exportAlert`/`exporting`/`lastExportedAt`/`exportButtonRef`、`runExport()`、`restoreExportFocus()`、`watch(settingsOpen)` 清空结果；模板新增设置面板末尾的「日历」fieldset，提示条数据源改为 `pageAlert`。 |
| `TODO.md` | 6.3「ICS 导出」置 ✅ 并写明范围。 |

### 规格留给开发的两点决定

1. **导出范围与文案**：实际导出「有 `startsAt` 时间段 / 有 `dueDate` / 有 `reminderAt`」三类任务（按此优先级取一，`startDate` 不算——它是「可以开始」不是「这天有事」；不按完成状态过滤）。据此把说明文案改为「把**设置了截止日或提醒时间的**任务导出为 .ics 文件，可导入 Outlook、Google 日历或苹果日历。」，与空数据文案「先给任务设置截止日或提醒时间」对齐；空数据文案照规格原文未改。
2. **文件去向**：写入系统下载目录（`app.path().download_dir()`，取不到时退到 app data 目录），文件名 `todo-<UTC 时间戳>.ics`，成功提示带完整路径。**不接系统保存对话框**——那需要新增 `tauri-plugin-dialog` + `tauri-plugin-fs` 两个插件依赖与对应 capability，而由 command 自己 `std::fs::write` 一步到位，符合最小权限与「简约至上」。

### capability 逐条说明

**本次未新增任何 capability 权限，`default.json` 与 `today-card.json` 均未改动。** 理由：Tauri 2 的 ACL 只约束插件（含 `core:`）命令，应用自己用 `#[tauri::command]` 注册的命令不需要在 capability 里声明——仓库现状即证据（`list_todos` / `save_todo` / `delete_todo` / `replay_pending_writes` 都不在 `default.json` 里）。`export_calendar` 走同一条路径：目录解析用 Rust 侧 `app.path()`，文件写出用 `std::fs::write`，前端只调用这一个 command，不碰 `@tauri-apps/plugin-fs`/`plugin-dialog`。走查中导出成功、无 ACL 拒绝日志，实测确认。卡片窗口不涉及导出，`today-card.json` 无需改。

### RRULE 映射（以 `crates/contracts/` 实际形状为准）

| 契约 | RRULE |
|------|-------|
| `Daily` | `FREQ=DAILY` |
| `Weekly` + `weekdays` 非空 | `FREQ=WEEKLY;BYDAY=MO,WE,FR` |
| `Weekly` + `weekdays` 空 | `FREQ=WEEKLY`（RFC 默认取 DTSTART 的星期，正是契约注释所说的语义） |
| `Monthly` + `month_day` | `FREQ=MONTHLY;BYMONTHDAY=n` |
| `Monthly` + `on_last_day` | `FREQ=MONTHLY;BYMONTHDAY=-1` |
| `Yearly` | `FREQ=YEARLY` |
| `Workday`，`interval == 1` | `FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR`（最接近的标准写法） |
| `interval > 1` | 追加 `INTERVAL=n`（等于 1 时不写，RFC 默认即 1） |
| `until` | `UNTIL=YYYYMMDD`（全天事件）/ `UNTIL=YYYYMMDDT235959Z`（定时事件），值类型与 DTSTART 一致 |
| `count`（且无 `until`） | `COUNT=n`；UNTIL 与 COUNT 不同时出现 |

两类规则**故意不出 RRULE**（事件本身照常导出，只是不重复）：`calendar == Lunar`（RFC 5545 无农历，套公历会把日期算错，比不重复更糟）；`Workday` 且 `interval > 1`（「每 N 个工作日」与 `FREQ=WEEKLY;INTERVAL=N`（每 N 周）不是一回事，且工作日是节假日感知的）。

### 自验结果（命令与结论）

- `cargo test --workspace`：**通过，119 passed**（基线 97 + 本次新增 22 条 ICS 单测），0 failed。
- `cargo check --workspace`：**通过**，无 warning。
- `pnpm run types:generate`：**通过**，重生成 `src/bindings/commands.ts` 与 `src/bindings/models/CalendarExport.ts`；`src/bindings/**` 未手改。
- `pnpm run check`：**通过**（41 文件格式全绿；44 文件 0 warning / 0 lint / 0 type error）。
- `pnpm run tauri:dev` 真实 GUI 走查（WebView2 远程调试 9222 + CDP 驱动真实 DOM 点击）：

  | 走查项 | 结果 |
  |--------|------|
  | 入口位置 | 「日历」fieldset 为设置面板最后一个 fieldset（`legends` = 外观/桌面行为/同步/日历），`aria-labelledby="calendar-heading"` 与 legend id 对上 |
  | 说明与按钮 | 文案、`variant="ghost"`、`min-h-11 !px-3 text-sm` 与规格一致；未导出过时行内状态 span 不渲染 |
  | 成功态 | 点击后立即 `导出中…` + `aria-busy="true"` + `disabled=true`；结束后恢复 `导出 .ics`、`aria-busy="false"`、焦点回到按钮（`document.activeElement === button`）；提示条 `已导出 7 项任务：C:\Users\ADMIN\Downloads\todo-20260723T034845Z.ics。`；行内状态变为「上次导出：2026/7/23 10:48:45」 |
  | 成功态配色 | 浅色 `rgb(209,250,229)` 底 / `rgb(4,120,87)` 字（emerald-100 / emerald-700）；暗色 `rgba(6,78,59,0.4)` 底 / `rgb(110,231,183)` 字（emerald-900/40 / emerald-300），与 DESIGN.md 语义色公式一致 |
  | 空数据 | 只剩无日期任务时导出：amber 调「没有可导出的任务：先给任务设置截止日或提醒时间，再试一次。」，**未生成新文件**，行内状态不变 |
  | 重入保护 | 同一 tick 连点 3 次，只多出 1 个文件 |
  | 结果消失时机 | 关闭设置面板后提示条消失，重新打开不复现 |
  | 提示条优先级 | 外部进程持有 SQLite 写锁制造存储告警后再导出：提示条仍显示存储告警（amber），导出照常产出文件，页面上 `p[role="status"]` 始终只有 1 个（未新增第二个 live region） |
  | 浏览器运行时 | `http://127.0.0.1:1420` 下 `isTauri()=false`，「日历」fieldset 整块不渲染（无永久禁用的死控件） |

  走查后已清理：SQLite 中的测试待办全部删除（`listTodos` 返回 0）、`todos.items`/`todos.deletions`/`todos.quarantine` 已清空、下载目录中 4 个导出文件已删除；`git status` 仅剩本次改动文件，无残留。

### 验收标准逐条自查

- ⚠️ **验收 1「导出文件被主流日历正确识别，周期任务展开正确」——未达验收原文强度，缺一次真实日历导入。** 这条验收要的是「主流日历（Outlook/Google/Apple 任一）正确识别」，而**没有任何一款主流日历应用打开过本任务导出的文件**：本机三者都没装，也不会把用户任务数据上传到在线日历账号去验。下面两条是已经取到的证据，强度只到「符合 RFC 5545，并被一个独立的成熟解析器完整接受、按标准展开」，**不等于**验收原文说的「被主流日历正确识别」；请勿据此认为已在真实日历中验证过。缺口是环境性的，需由编排者安排一次人工导入才能闭合。
  - ✅ **RFC 5545 层面可判定的断言全部通过**（这是「格式合规」，不是「主流日历识别」）。用 **ical.js（Mozilla/Thunderbird 的 iCalendar 解析器，与被测代码无共用实现）** 解析真实导出的文件，另加手写结构断言，全部 PASS：文件以 CRLF 结束、无裸 LF；最长行 74 octet（≤75）；无 U+FFFD，12 次重复的中文标题去折行后与原文逐字相等（多字节字符未被折断）；`BEGIN/END:VCALENDAR` 闭合、`VERSION:2.0`、`PRODID` 为 `-//owner//product//language` 形；7 个 VEVENT 的 `BEGIN/END` 配平；每个 VEVENT 都有 `UID`（互不重复）、`DTSTAMP`、`DTSTART`；有 `DTEND` 的都晚于 `DTSTART` 且值类型与 `DTSTART` 一致；定时事件的 `DTSTART` 时区为 UTC（因此文件不需要 `VTIMEZONE`）；每条 `RRULE` 都带 `FREQ`、不同时带 `UNTIL` 与 `COUNT`、`UNTIL` 值类型与 `DTSTART` 一致。转义与折行实测：`SUMMARY:健身\; 拉伸\, 有氧\\核心`、`DESCRIPTION:第一行\n第二行`。
  - ✅ **周期展开由 ical.js 独立计算**（不是我的代码自证；ical.js 是一个库，不是日历应用，因此这条证的是「规则写法在标准解析器下展开正确」）：`FREQ=MONTHLY;BYMONTHDAY=-1;UNTIL=20261231T235959Z` → 2026-07-31、08-31、09-30、10-31、11-30、12-31（每月最后一天，含 30/31 天差异，且在 UNTIL 处停住）；`FREQ=WEEKLY;BYDAY=MO,WE,FR;COUNT=12` → 07-24、07-27、07-29、07-31、08-03、08-05；`FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR` → 07-23、24、27、28、29、30（跳过周末）。
  - ❌ **真实日历应用导入：未做，无记录。** 待编排者安排（文件由设置面板导出即可）。在此之前，验收 1 应视为**未满足**，而不是「已验证」。
  - ⚠️ **另有一类降级不属于「展开正确」而属于「明确告知」**：农历与「每 N 个工作日」在 .ics 中无对应写法，导出为单次事件；第 2 轮起由提示条明说条数（见下方第 2 轮小节），但导入后这些任务在日历里确实不重复。
- ✅ **验收 2「导出入口可用，空数据导出有合理提示」**：入口在设置面板「日历」fieldset，真实点击可用（见上表）；空数据给 amber 提示并附下一步动作「先给任务设置截止日或提醒时间，再试一次。」，不写「上次导出」也不产出文件。

### 遗留与说明（不阻塞）

- 错误提示里的 `{原因}` 是 Rust 侧 `CalendarExportError` 的英文 Display 文本（如 `the calendar file was not written: ...`），外层中文句子由前端拼。这跟既有「同步失败：{lastError}」同一处理方式（App.vue 现状），故未另造中文错误码表。
- 规格「用户取消」一行在当前实现下不可达：没有系统保存/分享面板，所以没有可取消的环节；三态（成功/空/失败）已覆盖全部出口。09（ICS 订阅）的控件可直接落进同一个「日历」fieldset。
- 移动端未验证：本机无 Android/iOS 工具链。`download_dir()` 在移动端可能取不到，代码已退到 app data 目录，但没有实机证据。

### 第 2 轮（评审回流）

处理范围：第 1 轮 2 条阻塞 + 编排者裁决「本轮处理」的 5 条建议（`AGENTS.md` 端关系图那条由编排者自行改文档，未动）。

#### 改动文件（第 2 轮）

| 文件 | 改动 |
|------|------|
| `crates/domain/src/ics.rs` | ① 四处按字节下标切片全部改走 `str::get` / `strip_prefix`，任意 `String` 输入不再 panic（阻塞 1）；② `Calendar` 新增 `unrepeatable_recurrences`，统计被降级为单次事件的周期任务；③ `parse_instant` 改为返回 `ZonedInstant`（瞬时 + 原值时区），`EventWindow` 带上 `zone_offset_seconds`，`until_value` 据此把 `UNTIL` 取成「本地当日最后一秒换算到 UTC」；④ `export_file_name` 增加 `attempt` 形参，重名时追加计数（`todo-…-2.ics`）。新增 6 条单测（共 40 条）。 |
| `src-tauri/src/lib.rs` | `CalendarExport` 新增 `unrepeatable_recurrences` 出参；写文件改为 `write_new_file()`——用 `OpenOptions::create_new(true)` 独占创建并按 `attempt` 递增取名，名字被占就换下一个（检测与占用是同一个原子操作，`exists()` 后再写仍会覆盖）。 |
| `src/bindings/commands.ts`、`src/bindings/models/CalendarExport.ts` | 生成产物，`pnpm run types:generate` 刷新，未手改。 |
| `src/lib/calendar-export.ts` | `exported` 出口带上 `unrepeatableRecurrences`，并写进诊断日志。 |
| `src/App.vue` | ① `runExport()` 先把结果算进局部变量，仅在 `settingsOpen` 仍为真时写入 `exportAlert`——面板已关闭时结果直接丢弃（与「关闭设置面板时清空」同一语义），不再出现无从清除的滞留提示条；② 成功提示在原句后追加降级说明「其中 {m} 项周期任务只导出为单次事件：农历与“每 N 个工作日”无法用日历标准表示。」，`m == 0` 时不追加。**同一个提示条、同一 live region、同一 `success` 调、优先级未变**，未新增机制。 |
| `.agents/tasks/数据与同步/08-ICS导出.md` | 「改动文件」表补 `Cargo.lock`（阻塞 2）；「验收标准逐条自查」验收 1 改写措辞，明说未在真实日历中验证过（建议 ⑤）；本小节与各条 `处理（开发）` 行。 |

`Cargo.lock` 第 2 轮未再变化（`todo-domain` 依赖第 1 轮已写入），仅补进第 1 轮清单。

#### 关于 ③ 的边界（说明，不是遗留缺陷）

`UNTIL` 用的时区来自**存储值自己写的偏移**（`+08:00` / `-0800` / `Z`）。带偏移的值（同步下行、其他端写入）由此得到精确的本地日末；只写 `Z` 的值在本仓库内是唯一可能的读法——`App.vue` 的 `toInstant()` 走 `toISOString()`，落库即 UTC，此时「本地日」只能按 UTC 日理解，输出与改动前逐字节相同。也就是说这次修的是「有时区信息时算错」，而不是凭空猜一个时区。

#### 自验结果（第 2 轮命令与结论）

- `cargo test --workspace`：**通过，125 passed / 0 failed**（todo_domain 40 = 34 + 新增 6、todo_server 57、cross_platform_todo_lib 20、todo_contracts 8）。
- `cargo check --workspace`：**通过**，无 warning。
- `pnpm run types:generate`：**通过**，`CalendarExport.ts` 与 `commands.ts` 重生成，含 `unrepeatableRecurrences`；`src/bindings/**` 未手改。
- `pnpm run check`：**通过**（41 文件格式全绿；44 文件 0 warning / 0 lint / 0 type error）。
- `pnpm run tauri:dev` 真实 GUI 走查（WebView2 远程调试 9222 + CDP 驱动真实 DOM 点击）：

  | 走查项 | 结果 |
  |--------|------|
  | ② 降级提示（成功态） | 库中 5 条任务（农历年度、每 2 个工作日、每周一、普通截止日、无日期）→ 提示条：`已导出 4 项任务：C:\Users\ADMIN\Downloads\todo-20260723T042108Z.ics。其中 2 项周期任务只导出为单次事件：农历与“每 N 个工作日”无法用日历标准表示。`，仍为 emerald 成功调，`p[role="status"]` 全页仍只有 1 个 |
  | ② 计数口径 | 能映射的每周规则与无周期任务都不计入；无日期因而没进文件的任务也不计入（单测 `a_task_with_no_date_cannot_be_counted_as_a_lost_recurrence`） |
  | ① 进行中关门 | 点「导出 .ics」后同一 tick 点「关闭设置」→ 3 秒后主界面 `p[role="status"]` 数量为 **0**（改前会滞留一条无从清除的成功提示）；重开面板也无陈旧提示 |
  | ④ 同秒重复导出 | 同一 tick 并发调用 `export_calendar` 三次 → `todo-20260723T042119Z.ics`、`…-2.ics`、`…-3.ics` 三个文件，无一被覆盖 |
  | ③ UNTIL | `startsAt="2026-07-23T20:00:00-08:00"` + `until="2026-12-31"` → 文件中 `RRULE:FREQ=DAILY;UNTIL=20270101T075959Z`；ical.js 独立展开得 162 次，末次 `2027-01-01T04:00:00Z`（即本地 12-31 20:00，含到 `until` 当天）。把该值改回旧的 `20261231T235959Z` 再让 ical.js 展开：161 次、末次落在本地 12-30，**少一次**——回归方向由独立解析器确认 |
  | 阻塞 1 端到端 | 存入 `dueDate="2026-07-日期"`、`reminderAt="2026-07-23T10:15:00日"`、`startsAt="2026-07-23T10:15:00+0日"`、`endsAt="….🎉"`、`until="2026-12-日"` 的脏任务后导出：命令正常返回（该任务被跳过，`eventCount` 不变），无线程崩溃、无未捕获异常 |
  | 空数据 | 清空全部任务后点击导出：amber 调「没有可导出的任务：先给任务设置截止日或提醒时间，再试一次。」，未产出文件 |

  走查后已清理：SQLite 中 6 条测试待办全部删除（`list_todos` 返回 0）、下载目录中 7 个导出文件全部删除、`settings.json` 的 `sync.pending` 与 localStorage 的同步队列残留清空、dev 进程与 1420/9222 端口已释放；`git status` 与走查前逐行一致，仓库无残留。

### 第 3 轮（评审回流）

处理范围：第 3 轮（收尾裁决）唯一一条阻塞（`src-tauri/src/lib.rs` 写入失败留残件）。其余条目已由编排者裁决完毕，本轮未动；`UNTIL` 时区一条已转 任务管理/06，本轮未碰。

#### 改动文件（第 3 轮）

| 文件 | 改动 |
|------|------|
| `src-tauri/src/lib.rs` | ① `write_new_file` 的循环体抽成 `write_new_file_with(directory, generated_at, write)`，写入动作作为形参传入（原函数只是把 `file.write_all(content)` 传进去，行为不变）；② 写入失败时先 `drop(file)` 再 `let _ = std::fs::remove_file(&path)`，然后返回**原始**写入错误——残件被清掉、名字重新空出，用户看到的仍是导致失败的真实原因；③ 新增单测 `a_failed_write_leaves_no_file_and_no_taken_name`。 |

未改 command 签名与任何契约类型（`CalendarExport` 出参未动，`export_calendar` 形参未动，`write_new_file*` 是模块内私有函数），故本轮未跑 `types:generate`；`src/bindings/**` 与前端未改动。

#### 清扫为什么这么写

- **先关句柄再删**：Windows 上删除一个仍被打开的文件会受句柄影响，先 `drop(file)` 让删除是确定的。
- **删除失败不冒泡**：`let _ = remove_file(...)` 丢弃清扫自身的错误。清扫失败时用户没有可执行的下一步，而把它换上去会**掩盖**真正的失败原因；因此返回值恒为写入错误 `CalendarExportError::Write(<原始 io 错误>)`，经既有链路（`map_err(to_string)` → `calendar-export.ts` 收敛为 `failed` → `App.vue` 的 error 调「导出失败：{原因}。」）呈现，出口与规格 §2 错误行一致。清扫也不会 panic（不 `unwrap`）。
- **只删自己刚建的那个**：`create_new(true)` 成功那一支才走到这里，路径必然是本次导出原子创建的新文件，不会误删同名旧文件。

#### 「写入失败」是怎么构造的

`write_all` 的真实失败原因（磁盘写满、卷中途被拔出、配额耗尽）在跑测试的机器上无法稳定制造，因此**用注入**：把写入动作作为形参传给 `write_new_file_with`，单测传入一个返回 `io::ErrorKind::StorageFull` 的闭包。除这一次写入外，其余全是发货代码在**真实目录**（`std::env::temp_dir()` 下的独占子目录）上跑：文件由 `create_new` 真实创建、删除是真实 `remove_file`、目录用 `read_dir` 真实读回。

单测断言三件事：① 错误信息里带得到写入失败的原因（`no space left on device`）；② 目录读回为空——没有残件；③ 紧接着的一次真实导出仍拿到 attempt 0 的名字（`todo-<戳>.ics`，不是 `-2.ics`）——名字没被占住。

**该用例确实能抓住这个缺陷**（做了一次反向验证）：临时注释掉 `remove_file` 那一行后重跑，用例失败并打印 `a failed export left files behind: ["todo-20260320T094640Z.ics"]`；恢复后转绿。反向验证的注释已复原，临时残留目录已删除。

#### 自验结果（第 3 轮命令与结论）

- `cargo test --workspace`：**通过，126 passed / 0 failed**（基线 125 + 本轮新增 1：cross_platform_todo_lib 21、todo_domain 40、todo_server 57、todo_contracts 8，doc-test 0）。
- `cargo check --workspace`：**通过**，无 warning。
- `pnpm run check`：**通过**（41 文件格式全绿；44 文件 0 warning / 0 lint / 0 type error）。
- `pnpm run types:generate`：**未跑，本轮无契约或 command 签名变更**（改动全在私有函数与测试内）；`cargo test` 里的 bindings 导出测试照常通过，`git status` 相对本轮改动前无新增/改写文件。
- 残留核验：`git status` 为 13 项（10 改 + 3 未跟踪），与第 2 轮结束时逐行一致，未多出任何文件；`%TEMP%` 下 `todo-ics-write-failure-*` 已清空；下载目录无 `todo-*.ics`。

## 评审记录

### 第 1 轮

- 阻塞 · `crates/domain/src/ics.rs:319`（另见 :349、:356、:380）· 日期/时间字符串按**字节下标**切片，非 ASCII 内容会跨 UTF-8 字符边界导致 panic，而不是按函数自身约定返回 `None`。`parse_date` 的 `&value[8..10]`、`parse_instant` 的 `&value[17..19]` 与 `&value[19..]`、`parse_offset` 的 `value.split_at(1)` 这四处的下标都不受前置守卫保护（守卫只检查了 `bytes[4]/[7]/[10]/[13]/[16]` 这些位置是 ASCII，第 8/17/19 字节与 offset 首字节没有任何约束）。已实测复现：`due_date = "2026-07-日期"` 时 `bytes.len() >= 10 && bytes[4] == b'-' && bytes[7] == b'-'` 全部为真，随后 `&value[8..10]` panic（`end byte index 10 is not a char boundary; it is inside '日'`）；`reminder_at = "2026-07-23T10:15:00日"` 时守卫同样全过，`&value[19..]` 得到 `"日"`，`split_at(1)` panic。此类值是可表示且可达的——`crates/contracts/src/lib.rs:103` 的 `Todo::validate()` 只校验 id/title/notes/时间段配对/估时/周期/清单/标签，**完全不校验 `due_date`/`reminder_at`/`starts_at`/`ends_at`/`RecurrenceRule::until` 的格式**，这些字段是自由 `String`，可经同步下行或历史数据进入本地库；`recurrence_property` 还会拿 `rule.until` 再走一次 `parse_date`。panic 发生在 `export_calendar` 这个 `#[tauri::command]` 内，会让整次导出以线程崩溃收场（前端既拿不到 `empty` 也拿不到 `failed`，`exporting` 靠 `finally` 复位但用户只看到无提示）· 判定依据：AGENTS.md 客户端「原生侧失败必须容错降级而非中断应用」；AGENTS.md 通用「边界与异常情形」；`ics.rs:305-309`/`:329-336` 自身文档承诺「不可用的值交给调用方当作无法使用」（即返回 `None`）
  - 处理（开发）：已修。四处不受守卫保护的切片全部改掉——`parse_date` 的 `value[0..4]/[5..7]/[8..10]`、`parse_instant` 的 `value[0..10]/[11..13]/[14..16]/[17..19]/[19..]` 与小数位后的 `fraction[digits..]` 一律改走 `str::get(..)?`（不在字符边界上就返回 `None`，正是这两个函数文档承诺的行为）；`parse_offset` 的 `value.split_at(1)` 改为 `strip_prefix('+')` / `strip_prefix('-')`（按字符切，不按字节），其后 `digits[0..2]/[2..4]/[3..5]` 也改为 `get`——len == 4 分支同样能被 `"+0日"` 这类值切进多字节字符，评审只点了 `split_at`，这条一并修掉。新增两条单测：`a_value_that_is_not_ascii_is_refused_rather_than_panicking` 逐个覆盖评审复现的 `"2026-07-日期"`、`"2026-07-23T10:15:00日"` 以及月份位多字节、秒位截断、`.5日`、`+0日`、`+00:日` 等形态；`no_string_at_all_can_stop_an_export` 拿 16 个种子串（空串、纯汉字、emoji、组合字符、BOM、全连字符……）× 每一个字节级前缀（含 `from_utf8_lossy` 后的截断残值）共数百个值，同时灌进 `due_date`/`reminder_at`/`starts_at`/`ends_at`/`recurrence.until` 五个字段跑 `build_calendar`，断言导出照常完成。GUI 走查另做了端到端确认：把同样的脏值存进本地库再点导出，命令正常返回、脏任务被跳过、无线程崩溃（见「第 2 轮 → 走查项 · 阻塞 1 端到端」）。
  - 复核（评审）：认可。① 静态复核：`crates/domain/src/ics.rs` 全文已无 `&value[a..b]` 形式的字符串切片，`parse_date`（:360-362）、`parse_instant`（:406-425）、`parse_offset`（:445-456）全部走 `str::get` / `strip_prefix`；剩余的 `bytes[4]/[7]/[10]/[13]/[16]` 与 `digits.as_bytes()[2]` 都是对 `&[u8]` 取值且长度已由前置守卫保证，不是字符串切片，不会 panic。② 独立复现（不复用被测仓库的单测）：在仓库外另建一个只以路径依赖引用 `todo-domain` 的可执行程序，构造 46,858 个字符串——33 个种子（空串、纯汉字、emoji、组合字符 U+0301、BOM、阿拉伯-印度数字、全角数字、U+10FFFF、全连字符、合法值……）× 每个字节位置的 `from_utf8_lossy` 截断残值 × 每个字节位置插入多字节填充字符，另加 40,000 条按混合字母表随机拼装后再按随机字节位切断的噪声——把每个值分别单独灌进 `due_date`/`reminder_at`/`starts_at`/`ends_at`/`recurrence.until` 五个字段（5 轮），再做一轮五字段同时灌入 + 一轮与合法值随机混搭，共 28 万余次 `build_calendar` 调用，无一 panic，`content` 均以 CRLF 收尾。③ 反向确认未误伤合法值：16 个合法形态（`YYYY-MM-DD` 含闰日/纪元边界，`Z`/`z`/`T`/`t`/无时区/`±HH:MM`/`±HHMM`/小数秒 3 位与 9 位）逐条断言 `DTSTART` 输出正确且 `event_count == 1`，全部通过。
- 阻塞 · `Cargo.lock` · 「实现记录 → 改动文件」表缺该文件。`git status` 显示 `Cargo.lock` 已修改（新增 `todo-domain` 进 `cross-platform-todo` 的依赖列表），属本任务改动（由 `src-tauri/Cargo.toml` 新增路径依赖引起），不属兄弟子任务 03，也不是无关工作区改动 · 判定依据：roles/reviewer.md 工作步骤 2「清单外的改动……属于则记阻塞『清单遗漏』」
  - 处理（开发）：已补。第 1 轮「改动文件」表 `src-tauri/Cargo.toml` 一行之后新增 `Cargo.lock` 行，并注明它由该路径依赖引起、内容是 `cross-platform-todo` 依赖列表多出 `todo-domain` 一行、由 cargo 自动写入而非手改。第 2 轮未再改动该文件。
  - 复核（评审）：认可。清单已含 `Cargo.lock`，`git diff Cargo.lock` 实测只有 `cross-platform-todo` 依赖块新增 `todo-domain` 一行，与描述一致。本轮重新核对 `git status`：工作区 11 个变更文件（8 改 + 3 新增）与两轮「改动文件」表并集逐一对上，另加 `AGENTS.md`（编排者本轮自行修改，见末条），无清单外改动。
- 建议 · `src/App.vue:156` · 导出进行中关闭设置面板时，`watch(settingsOpen)` 的清空发生在结果产生**之前**，导出完成后 `runExport` 仍会写入 `exportAlert`，于是提示条在面板已关闭的主界面上出现，且此后再无清除路径（提示条无关闭按钮、不自动消失、`watch` 只在 true→false 的那一刻触发），整会话滞留 · 判定依据：规格 §2 补充规则「结果提示的消失时机：下一次点击导出时清空，或关闭设置面板时清空」
  - 裁决（编排者）：本轮处理。「提示条出现后无从清除」会让一条陈旧结果一直挂在页面上，而规格明写结果提示应在下次点击或关闭设置面板时清空——这是规格未还原，不是可选优化。
  - 处理（开发）：已修。`runExport()` 不再直接往 `exportAlert` 里写，而是先把三种结果算进局部变量 `result`，最后一步 `if (settingsOpen.value) exportAlert.value = result`。面板在导出期间被关掉时结果直接丢弃——这与规格「关闭设置面板时清空」是同一件事（结果晚到一步而已），不是新规则；`watch(settingsOpen)` 的原有清空保持不变，两者一起把「提示条只在面板开着时出现、面板一关就没有」这条规格补全。`lastExportedAt`（行内状态文字）照常更新，它本来就只在面板内可见。走查确认：点导出后同一 tick 关面板，3 秒后主界面 `p[role="status"]` 数量为 0，重开面板也无陈旧提示。
  - 复核（评审）：认可。`src/App.vue:213` 的 `if (settingsOpen.value) exportAlert.value = result;` 是唯一写入点，`watch(settingsOpen)`（:156）保留，两处合起来使「提示条只在面板开着时存在」成立。就编排者点名的「失败也被丢弃是否可接受」独立判断：可接受，理由三条——(1) 规格 §2 明写「关闭设置面板时清空」，晚到一步的结果与已到的结果同属该规则，特事特办反而是新规则；(2) 失败并未无声消失，`src/lib/calendar-export.ts:68/72` 两条 `writeDiagnostic("warn"/"error")` 仍会落进诊断日志；(3) 失败时 `lastExportedAt` 不更新（`src/App.vue:193` 只在 `exported` 分支赋值），用户重开面板看到「上次导出」未变，仍有一个弱信号提示这次没成。另核 `restoreExportFocus()`（:169）在面板已关时不会误抢焦点：`v-if="settingsOpen"` 卸载 section 后 Vue 会把 `exportButtonRef` 置空，`?.$el` 短路，函数只在 `document.activeElement === document.body` 时才动焦点。
- 建议 · `crates/domain/src/ics.rs:181-234` · `RecurrenceCalendar::Lunar` 与 `Workday && interval > 1` 两类规则静默丢弃 `RRULE`，事件降级为单次，界面无任何说明。用户导出后不会知道自己的农历生日在日历里不再重复。降级本身的理由成立（RFC 5545 无农历、`INTERVAL=N` 周 ≠ N 个工作日），问题只在「静默」——`Calendar` 只回传 `event_count`，没有可供 UI 说明降级条数的出参 · 判定依据：需求验收 1「周期任务展开正确」；DESIGN.md「状态信息（逾期、错误、成功）必须有文字或图标伴随」
  - 裁决（编排者）：本轮处理。农历与「每 N 个工作日」被降级为单次事件而界面只字不提，用户会以为周期任务导出成功——验收标准第 1 条写的是「周期任务展开正确」，静默降级正是它要防的情形。降级本身可以保留（RFC 5545 确实表达不了农历），但必须让用户看得见：复用规格已建立的提示条，说明有多少条周期任务被降级。
  - 处理（开发）：已修，按裁决只加说明、不改降级行为。`Calendar` 新增 `unrepeatable_recurrences`：`build_calendar` 里 `recurrence_property` 返回 `None` 的那一支计数（只数真正进了文件的事件，没有日期因而没进文件的任务不计，否则会把警告说大）。该计数经 `CalendarExport.unrepeatable_recurrences` → `unrepeatableRecurrences` 传到前端，成功提示在原句后追加一句：「其中 {m} 项周期任务只导出为单次事件：农历与“每 N 个工作日”无法用日历标准表示。」，`m == 0` 时不追加。**沿用规格已建立的机制，未新造交互**——同一个提示条 DOM 节点、同一个 `role="status" aria-live="polite"` live region、同一个 `success` 调、「存储告警 > 导出结果」优先级不变，只是成功句变长；因此不需要回退设计阶段。走查实测：4 事件中 2 条被降级 → 提示条按上文原样显示，全页 `p[role="status"]` 仍只有 1 个。
  - 复核（评审）：认可。计数口径准确：`crates/domain/src/ics.rs:82-87` 的自增位于 `event_window(todo)` 返回 `Some` 之后（:63-66 已 `continue` 掉无日期任务），且只在 `todo.recurrence` 存在、`recurrence_property` 返回 `None` 时计数——「没进文件的任务不计入」由结构保证，不是靠单测约定；`recurrence_property` 返回 `None` 的出口只有 `Lunar`（:209）与 `Workday && interval != 1`（:238），与提示文案「农历与『每 N 个工作日』」一一对应，没有把其他情形混进来。机制复用属实：`src/App.vue:281-290` 仍是同一个 `<p role="status" aria-live="polite">` 节点，`pageAlert`（:152）的 `todoStore.storageAlert ?? exportAlert.value` 保持「存储告警 > 导出结果」优先级不变，`ALERT_TONE_CLASS.success` 单键新增，未新增第二个 live region、未新增计时/关闭机制。降级说明是拼在成功句尾的同一条 message（:198-205），`dropped === 0` 时为空串不追加。
- 建议 · `crates/domain/src/ics.rs:239-244` · 定时事件的 `UNTIL` 固定取 `YYYYMMDDT235959Z`，但契约把 `until` 注释为「Local date the repetition stops on, inclusive」。对 UTC 以西时区的晚间事件（例如 UTC−8 的每日 20:00，实际瞬时为次日 04:00Z），本地日 2026-12-31 那一次的 UTC 值已越过 `20261231T235959Z`，会被少展开一次 · 判定依据：`crates/contracts/src/lib.rs:209` 契约注释；RFC 5545 3.3.10（`UNTIL` 与 `DTSTART` 同值类型，此处按 UTC 比较）
  - 裁决（编排者）：升级为转任务处理，并入 任务管理/06-周期规则引擎，列为该子任务的**必办项**。评审的不认可成立——本轮改动是严格改进，但我裁决时点名的「UTC 以西用户即会命中」那一类没有被覆盖，因为前端 `toInstant()` 一律写 `Z`，domain 拿不到设备本地偏移。`recurrence` 目前无任何界面入口，整条 `UNTIL` 路径在产品内暂不可达；而 任务管理/06 正是让它变为可达的子任务，届时必须把导出设备的本地偏移喂进 `export_calendar`，否则周期任务导出会在非 UTC 时区少展开一次。
  - 裁决（编排者）：本轮处理。UTC 以西时区的晚间事件少展开一次是确定的正确性缺陷，且中国以外的用户即会命中；属验收标准第 1 条「周期任务展开正确」的直接反例。
  - 处理（开发）：已修。`UNTIL` 不再固定取 UTC 日末，而是取「`until` 那一天在**事件自身时区**里的最后一秒，换算回 UTC」：`parse_instant` 改为返回 `ZonedInstant { unix_seconds, offset_seconds }`（原来只回瞬时，时区被丢掉），`EventWindow` 带上 `zone_offset_seconds`，`until_value` 算 `days_from_civil(until) * 86400 + 86399 - offset`。评审给的例子实测：`starts_at = "2026-07-23T20:00:00-08:00"` + `until = "2026-12-31"` → `UNTIL=20270101T075959Z`，ical.js 独立展开 162 次、末次 `2027-01-01T04:00:00Z`（本地 12-31 20:00，含到 `until` 当天）；把该值换回旧的 `20261231T235959Z` 再展开只有 161 次、末次落在本地 12-30——少一次由独立解析器双向确认。东侧同理收紧（`+08:00` → `20261231T155959Z`）。**边界说明**：时区取自存储值自己写的偏移；只写 `Z` 的值（本仓库 `toInstant()` 走 `toISOString()`，落库即 UTC）唯一可能的读法就是 UTC 日，输出与改动前逐字节相同。也就是说这次修的是「值带了时区却没用上」，没有凭空猜时区。新增单测 `an_end_date_on_a_timed_event_covers_the_whole_local_day` 覆盖东西两侧。
  - 复核（评审）：**不认可，提请编排者裁决**（按 roles/reviewer.md 步骤 2 第三种处置，不计入本轮阻塞）。算式本身正确，但裁决所指的用户群没有被覆盖到。
    - **算式与边界组合已独立复核通过**（仓库外的独立程序直接调 `build_calendar` 断言，不经 ical.js、不经被测单测）：以东 `+08:00` → `UNTIL=20261231T155959Z`；以西 `-08:00` → `20270101T075959Z`；半时区 `+05:30` → `20261231T182959Z`；跨年（上述以西例即 2026→2027）正确；`until` 恰为 DTSTART 当天，以西 `-08:00` → `20260724T075959Z`（≥ DTSTART 的 `20260724T040000Z`，保住当天这一次）、以东 `+08:00` → `20260723T155959Z`；闰日 `2028-02-29`（`-08:00`）→ `20280301T075959Z`；`until` 是纯日期而 DTSTART 是定时事件——契约里 `until` 本就只有 `YYYY-MM-DD` 一种形态，`until_value`（`ics.rs:274-282`）按 `window.start` 的值类型分派，全天 → `VALUE=DATE` 的 `UNTIL=20261231`，定时 → UTC 瞬时，与 RFC 5545 3.3.10「UNTIL 与 DTSTART 同值类型」一致。展开次数我用日历天数独算复核（不依赖 ical.js）：DTSTART `20260724T040000Z`、`FREQ=DAILY`、新 `UNTIL=20270101T075959Z` → 7/24–7/31 共 8 次 + 8 月 31 + 9 月 30 + 10 月 31 + 11 月 30 + 12 月 31 + 1/1 共 1 = **162**，本地日恰好 7/23–12/31；旧值 `20261231T235959Z` → **161**，本地末次落在 12/30。开发所报 162/161 属实。
    - **但对本仓库自己写出的数据，缺陷仍在**：`App.vue:222-226` 的 `toInstant()` 走 `toISOString()`，落库的瞬时一律带 `Z`，`parse_instant` 因此得到 `offset_seconds = 0`，`until_value` 退回 UTC 日末。实测（同一独立程序）：`starts_at = "2026-07-23T20:00:00Z"` + `until = "2026-12-31"` → `UNTIL=20261231T235959Z`，与改动前逐字节相同。而 UTC−8 的用户在界面上设的「晚 20:00」正是存成 `2026-07-24T04:00:00Z` 这种形状——末次仍停在本地 12/30，**少一次照旧**。也就是说本条修好的是「值自带偏移」的那一类（当前只可能来自其他端下行），编排者裁决时点名的「中国以外的用户即会命中」那一类没有被覆盖。
    - **开发的边界说明只对了一半**：「只写 `Z` 的值唯一可能的读法就是 UTC 日」在 `crates/domain/` 内部成立（纯规则拿不到时区），但导出这条链路上是拿得到的——`export_calendar` 已经由调用方喂进任务清单与时钟，前端 `new Date().getTimezoneOffset()` 同样可以一并喂进导出设备的本地偏移；契约把 `until` 注释为「Local date」，这个 local 指的是用户所在时区，不是「存储字符串碰巧写的那个偏移」。所以这不是「无解」，是「要不要为此再动一次 command 签名」。
    - **不判阻塞的理由**：`recurrence` 目前没有任何界面入口（`src/` 内除 `bindings/`、`todo-repository.ts` 校验与 `stores/todos.ts` 透传外无写入点），`until` 只能经同步下行产生，整条 `UNTIL` 路径在产品内暂不可达；且本轮改动是严格改进，不存在回退。是否补「导出设备本地偏移」入参属设计取舍（用创建时区还是导出时区、夏令时怎么算），已超出开发可自行决定的范围，故按第三种处置提请编排者终局裁决：要么接受当前口径并把「只写 `Z` 的值按 UTC 日理解」写进遗留说明，要么转子任务补入参。
- 建议 · `crates/domain/src/ics.rs:97-102` · 文件名 `todo-<秒级 UTC 时间戳>.ics`，同一秒内的第二次导出会**静默覆盖**前一个文件，与该函数文档「A distinct name per export means a second export never silently overwrites the first」的承诺不完全成立（`exporting` 重入守卫只挡同一次进行中的重复触发，不挡完成后同一秒内的再次点击）。取值本身可预测且带 `todo-` 前缀，与用户下载目录内其他文件冲突的概率极低，此点无需处理 · 判定依据：简约至上（注释与行为需一致）；规格 §2「重入保护」的意图
  - 裁决（编排者）：本轮处理。静默覆盖用户文件在任何情况下都不可接受，何况函数注释还承诺了不覆盖；修法自便（提高时间戳精度或碰撞时改名），成本很低。
  - 处理（开发）：已修，选「碰撞时改名」而不是「提高时间戳精度」——毫秒只是把概率压小，仍然不是「never」，而注释承诺的是 never。`export_file_name(generated_at, attempt)` 新增 `attempt` 形参：第 0 次仍是 `todo-<戳>.ics`，之后追加计数（`todo-<戳>-2.ics`、`-3.ics`…），命名规则留在 domain（纯规则），碰撞检测留在 `src-tauri`。写文件从 `std::fs::write` 换成 `OpenOptions::new().write(true).create_new(true)`——「这个名字是否被占」与「占下这个名字」是同一个原子操作，先 `exists()` 再写仍会覆盖两次导出交错产生的文件；遇到 `AlreadyExists` 就换下一个 attempt，上限 64 次后按写失败报错。函数文档同步改写，注释与行为一致。走查实测：同一 tick 并发导出三次得到 `…Z.ics`、`…Z-2.ics`、`…Z-3.ics` 三个文件，无一被覆盖；单测 `a_taken_file_name_gets_a_counter_rather_than_overwriting` 固定命名序列。
  - 复核（评审）：认可。就编排者点名的三问逐条独立取证（把 `src-tauri/src/lib.rs:158-181` 的 `write_new_file` 原样搬进仓库外的独立程序，仍调真实 `ics::export_file_name`，对真实目录跑）：**并发下确实无覆盖**——32 线程同秒同目录并发写，得到 32 个互不相同的路径、磁盘上 32 份互不相同的内容，一份没丢；这一点由 `create_new(true)`（Windows 下即 `CREATE_NEW`）把「查名字」与「占名字」合成一个原子操作保证，代码里也没有 `exists()` 预检这类竞态写法。**达到上限时是报错而不是覆盖**——先把 64 个名字全部占满再写，返回 `Err("64 file names in this folder are already taken")`，且事后逐个读回 64 个占位文件，内容与写入前完全一致，没有任何一个被改写。**错误会传达给用户**——该 `Err` 经 `CalendarExportError::Write` 的 `Display` 包成 `the calendar file was not written: …`，由 `export_calendar` 的 `map_err(|error| error.to_string())` 交给前端，`calendar-export.ts:69` 收敛为 `failed`，`App.vue:206-211` 渲染成 error 调「导出失败：{原因}。请重试或换一个保存位置。」，正是规格 §2 错误行要求的出口。函数文档已与行为一致（`ics.rs:103-112`）。
- 建议 · `.agents/tasks/数据与同步/08-ICS导出.md`「验收标准逐条自查」· 验收 1 的「被主流日历正确识别」缺真实应用导入记录。评审侧复核认为现有证据强度足以**支撑到「符合 RFC 5545 且被独立成熟解析器接受」**：ical.js 与被测实现无共用代码，折行/转义/CRLF/UID 唯一性/DTSTAMP/DTSTART/`VALUE=DATE`/`UNTIL` 值类型这些可判定断言全部由它复核通过，周期展开也由它独立计算。剩余缺口是环境性的——本次评审同样无 Outlook/Google/Apple 环境，且不应把用户任务数据上传到在线日历账号做验证，故**不判为阻塞**（判阻塞会退回一个开发端无法自行闭合的动作），转由编排者安排一次人工导入 · 判定依据：需求验收 1
  - 裁决（编排者）：本轮处理。验收标准自查的措辞必须与实际取证强度一致，否则这份记录会误导后续读者以为已在真实日历中验证过。
  - 处理（开发）：已改写。验收 1 的行首标记由「部分实证」改为 ⚠️ **未达验收原文强度，缺一次真实日历导入**，并在同一行把界限说死：没有任何一款主流日历应用打开过本任务导出的文件，下面的证据强度只到「符合 RFC 5545 且被一个独立的成熟解析器完整接受、按标准展开」，**不等于**验收原文的「被主流日历正确识别」，请勿据此认为已在真实日历中验证过。两条 ✅ 子项各自加了限定语（「这是格式合规，不是主流日历识别」「ical.js 是一个库、不是日历应用」），原来的 ❌ 子项改为「真实日历应用导入：未做，无记录」并写明「在此之前验收 1 应视为**未满足**」。另补一条 ⚠️ 提醒农历与「每 N 个工作日」在日历里确实不重复（第 2 轮起由提示条明说）。`需求` 节的验收勾选框未动（不属开发可改范围）。
  - 复核（评审）：认可，措辞已与取证强度对齐，未见仍然过强的表述。逐句核对：行首「未达验收原文强度，缺一次真实日历导入」把结论前置；「没有任何一款主流日历应用打开过本任务导出的文件」是事实陈述；两条 ✅ 都自带降级限定（「这是格式合规，不是主流日历识别」「ical.js 是一个库，不是日历应用」）；❌ 一条写明「未做，无记录」并给出「在此之前验收 1 应视为未满足」的处置结论。没有出现「已验证/已通过/符合验收」这类会被误读为完成的词。唯一还需编排者留意的不是措辞而是事实：`需求` 节验收 1 的复选框仍为未勾选状态，与本节结论一致，勾选权在编排者。
- 建议 · `AGENTS.md`「架构 / 端关系图」· 本次新增 `src-tauri → todo-domain` 依赖，**不违反**依赖方向约束：AGENTS.md 服务端节的原文是「server → domain → contracts，不得反向引用」，约束的是 crates 链条不得回指（domain 不得依赖 server/HTTP/DB/Tauri），而 `crates/domain/Cargo.toml` 依旧只依赖 `todo-contracts`、`ics.rs` 只 `use todo_contracts::…`，纯度保持；客户端引用 domain 属新增的下行依赖，与既有 `src-tauri → todo-contracts` 同形，有先例。但 AGENTS.md 把 `crates/domain/` 描述在「服务端」小节下、端关系图也未画客户端到 domain 的边，现状已与文档不符 · 判定依据：AGENTS.md「服务端」依赖方向条 + 「架构」节描述；根目录文档非开发角色可自行改动，提请编排者裁决是否补一句
  - 处理（编排者）：本次处理（编排者直接改文档）。已在 AGENTS.md「架构 → 客户端」与端关系图中补上 `src-tauri → crates/domain/` 这条边，并写明依赖方向约束的实际含义是「domain 不得反向引用上层」，而非「只有 server 可以依赖 domain」。属规范文档同步，不改代码，无需回评审回路。
  - 复核（评审）：认可（按编排者要求只核对、不据此判条目）。`git diff AGENTS.md` 实测三处且仅此三处：「架构 → 客户端」补「客户端可直接依赖 `crates/contracts/` 与 `crates/domain/`（纯规则在两端复用，如 ICS 生成）」；端关系图补 `Core --> Domain` 与 `Core --> Contracts` 两条边；「服务端」依赖方向条改写为「下层不得反向引用上层……`src-tauri/` 与 `crates/server/` 都可以依赖 `domain` 与 `contracts`」。与本次代码现状对得上：`crates/domain/Cargo.toml` 仍只依赖 `todo-contracts`，`ics.rs` 只 `use todo_contracts::…`，domain 纯度未破。

验证：`cargo test --workspace` 通过（119 passed / 0 failed：todo_domain 34、todo_server 57、cross_platform_todo_lib 20、todo_contracts 8）；`cargo check --workspace` 通过，无 warning；`pnpm run check` 通过（41 文件格式全绿，44 文件 0 warning / 0 lint / 0 type error）；`pnpm run types:generate` 重跑后 `git status` 无新增或改写，生成产物与调用方无漂移。capability 结论独立复核成立：`src-tauri/gen/schemas/acl-manifests.json` 中检索不到任何应用自注册 command（`list_todos` 命中数 0），也不含 `fs:` 类权限，`capabilities/default.json` 与 `today-card.json` 未改动且无需改动。上述阻塞条目 1 的 panic 由 `rustc` 独立最小复现验证，非静态推断。

### 第 2 轮

本轮复核范围：第 1 轮 8 个条目中带「处理（开发）」/「处理（编排者）」而尚无复核行的全部 8 条（复核结论已逐条追加在第 1 轮对应条目下），以及第 2 轮新增改动（`crates/domain/src/ics.rs`、`src-tauri/src/lib.rs`、`src/lib/calendar-export.ts`、`src/App.vue`、生成产物、任务文件）。复核结论：6 条认可，1 条（建议 ③ `UNTIL` 时区）不认可并提请编排者裁决（按 roles/reviewer.md 步骤 2，不计入本轮阻塞），1 条（AGENTS.md，编排者自行处理）核对通过。

本轮新发现：

- 建议 · `src-tauri/src/lib.rs:166-177` · 写文件分两步：`OpenOptions::create_new(true)` 先落地一个 0 字节文件，再 `write_all` 写内容。`write_all` 失败（磁盘满、卷被拔出）时，错误会照常报给用户，但那个已经建出来的空/半截 `.ics` 留在下载目录里不删——界面说「导出失败」，目录里却多了一个看起来像成品、双击就会被日历应用尝试导入的文件；这个残件还会占住该名字，让下一次同秒导出跳到 `-2`。修法很轻：`write_all` 出错时 `let _ = std::fs::remove_file(&path);` 再返回 `Err`。同一函数里 `create_new` 那条原子性设计是对的，本条只补它的失败清扫 · 判定依据：AGENTS.md 通用「边界与异常情形」「可验证收尾」；规格 §2 错误态（界面所述结果与磁盘实际状态需一致，参照空数据态明确的「未产出文件」）
  - 裁决（编排者）：升级为阻塞，见第 3 轮（收尾裁决）。理由：失败路径会在用户下载目录留下一个半截的 `.ics`，而日历应用会照样接受并导入它——这不是「导出失败」，是「悄悄产出了错误数据」；修法只有一行，代价远低于风险。

验证：`cargo test --workspace` 通过（125 passed / 0 failed：todo_domain 40、todo_server 57、cross_platform_todo_lib 20、todo_contracts 8，doc-test 0）；`cargo check --workspace` 通过——先 `cargo clean -p todo-domain -p cross-platform-todo` 强制重编，确认 0 warning，不是缓存结论；`pnpm run check` 通过（41 文件格式全绿，44 文件 0 warning / 0 lint / 0 type error）；`pnpm run types:generate` 重跑后 `git status` 与重跑前逐行一致（仍是 8 改 + 3 未跟踪），`commands.ts` 与 `CalendarExport.ts` 无漂移。另做两项仓库外的独立取证（均为临时程序，跑完即弃，未在仓库内留下任何文件）：① UTF-8 扫描——只以路径依赖引用 `todo-domain`，46,858 个构造值 × 5 个字段 × 多种组合共 28 万余次 `build_calendar`，无 panic，且 16 个合法形态输出仍正确（对应阻塞 1）；② 文件名碰撞——原样搬 `write_new_file` 对真实目录跑，32 线程并发得 32 个不同文件、内容零丢失，占满 64 个名字后返回 `Err` 且 64 个既有文件字节未变（对应建议 ④）。`UNTIL` 的东/西/半时区/跨年/当日/闰日/纯日期 7 组组合与 162 : 161 的展开次数差由独立程序 + 日历天数手算复核，未依赖 ical.js（对应建议 ③）。

### 第 3 轮（收尾裁决）

- 阻塞 · `src-tauri/src/lib.rs:166-177` · 沿自第 2 轮 · 写文件分两步：`OpenOptions::create_new(true)` 先建 0 字节文件，再 `write_all`。`write_all` 失败时残件不删——界面报「导出失败」，而下载目录里多出一个空的或写了一半的 `.ics`，日历应用会照常接受并导入它，同时该文件名还被永久占住（`create_new` 下次会跳过它）。用户看到的是失败，拿到的是一份可导入的错误数据 · 正确性维度 + AGENTS.md 客户端「原生侧失败必须容错降级」（降级不应留下有害产物）
  - 处理（开发）：已修。`write_all` 失败时先 `drop(file)`（Windows 上删除会受未关闭的句柄影响），再 `let _ = std::fs::remove_file(&path)`，然后返回**原始**写入错误。三点按要求落实：① 残件被删除，导出失败时目录里不再多出一个日历应用会照常导入的空/半截 `.ics`；② 名字随之空出，下一次导出仍拿 attempt 0 的名字，不被永久占住；③ **清扫失败不抛出也不掩盖**——`remove_file` 的返回值被 `let _ =` 丢弃、不 `unwrap`、不参与返回值，用户看到的错误恒为导致导出失败的真实原因（`CalendarExportError::Write(<原始 io 错误>)` → 前端 error 调「导出失败：{原因}。」），与规格 §2 错误行一致。`create_new` 的原子创建未动，只补它的失败清扫。为让「写入失败」可被单测构造，循环体抽成 `write_new_file_with(directory, generated_at, write)`，写入动作作为形参传入（`write_new_file` 只是把 `file.write_all(content)` 传进去，对外行为与签名不变，均为模块内私有函数，未动 command 签名）。新增单测 `a_failed_write_leaves_no_file_and_no_taken_name`：**写入失败用注入构造**（磁盘写满、卷被拔出无法在测试机上稳定制造），闭包返回 `io::ErrorKind::StorageFull`，其余全是发货代码在真实临时目录上跑——`create_new` 真建文件、真 `remove_file`、`read_dir` 真读回，断言「错误里带得到原因」+「目录读回为空」+「下一次导出仍是 attempt 0 的名字」。反向验证：临时注释掉 `remove_file` 后该用例失败并打印 `a failed export left files behind: ["todo-20260320T094640Z.ics"]`，恢复后转绿——用例确实能抓住本缺陷。详见「实现记录 → 第 3 轮」。
  - 复核（评审）：认可。五个点逐条独立取证（把 `src-tauri/src/lib.rs:162-215` 的 `write_new_file` / `write_new_file_with` 原样搬进仓库外的独立程序，仍以路径依赖调真实 `ics::export_file_name`，对 `%TEMP%` 下的真实目录跑；程序跑完即弃，仓库内未留任何文件）：
    - **① 残件被删、名字空出**：注入 `StorageFull` 的写入失败后 `read_dir` 读回为空，紧接着的一次真实导出仍拿到 attempt 0 的名字（`todo-<戳>.ics`，不是 `-2.ics`）。与开发所述一致。
    - **② 清扫失败不抛出、不掩盖**：不能只看 `let _ =` 这一行的字面。开发用的只读属性并不足以让删除失败（Rust 现行 `remove_file` 在 Windows 上会先清只读位，实测文件仍被删掉）；我改用**共享模式冲突**真正制造删除失败——在注入的写入闭包里以 `share_mode(FILE_SHARE_READ|FILE_SHARE_WRITE)`（不含 `FILE_SHARE_DELETE`）另开一个句柄，`remove_file` 因而真失败（文件确实留在目录里）。此时函数**没有 panic**，返回值仍是 `the calendar file was not written: no space left on device`，不含任何删除失败的字样——用户看到的仍是导致失败的真实原因。另一组（删除成功）返回同一条消息，两条路径出口一致。
    - **③ 重构未改发货路径行为，原子性与并发无覆盖未退化**：`write_new_file` 只是把 `file.write_all(content.as_bytes())` 作为 `FnOnce` 形参传给 `write_new_file_with`，循环体、`create_new(true)`、`AlreadyExists → continue`、64 次上限全部原封未动，`export_calendar` 与 `CalendarExport` 签名未动（`git status` 与 `pnpm run types:generate` 复跑均无漂移）。第 2 轮的并发结论在重构后重测：**32 线程同秒同目录并发写 → 32 个互不相同的路径、磁盘上 32 份互不相同的内容，一份没丢**；先占满 64 个名字再写 → 返回 `Err("…64 file names in this folder are already taken")`，事后逐个读回 64 个占位文件字节未变，无一被覆盖。未退化。
    - **④ 新增用例确实能抓住回归**（未采用开发的手法、未改动仓库任何文件）：把该用例的三条断言原样跑在**两个**函数变体上——现行版本 PASS；把 `remove_file` 那一行删掉的变体在第二条断言处失败，打印 `a failed export left files behind: ["todo-20260320T094640Z.ics"]`，与开发报告的失败信息逐字相同。用例有效。
    - **⑤ 只删本次创建的文件**：目录里预置一个同名旧文件（内容 `OLD FILE - MUST SURVIVE`）后再触发失败导出——`create_new` 让本次落到 attempt 1（`-2.ics`），清扫只删掉这个 `-2.ics`，旧文件内容读回逐字未变、仍在目录里。删除路径来自 `create_new` 成功那一支的 `path`，结构上不可能指向别的文件。
    - 另核 `cargo test --workspace` 中 `tests::a_failed_write_leaves_no_file_and_no_taken_name` 实跑通过，测试目录取 `%TEMP%/todo-ics-write-failure-<pid>-<tid>` 且首尾各清一次，不污染仓库；`io::ErrorKind::StorageFull` 自 Rust 1.83 稳定，低于工作区 `rust-version = "1.88.0"`，不引入工具链要求。

### 第 4 轮

本轮复核范围（编排者划定，范围极窄）：第 3 轮（收尾裁决）唯一一条阻塞的「处理（开发）」，以及第 3 轮新增改动本身（仅 `src-tauri/src/lib.rs`）。第 1、2 轮其余条目已由编排者裁决完毕，未重开；`UNTIL` 时区一条已转 任务管理/06（`git diff` 复核该文件只多出「编排者并入项」一段与一条补充验收标准，属编排者改动，非本任务代码）。

结论：**通过，无阻塞、无建议**。第 3 轮那条阻塞已复核认可（复核行已追加在第 3 轮条目下），五个复核要点全部独立取证成立：残件被删且名字空出；清扫失败（用共享模式冲突真实制造）既不 panic 也不掩盖原始错误；重构只把写入动作抽成 `FnOnce` 形参，发货路径行为与 `create_new` 原子性未变，32 线程并发无覆盖的第 2 轮结论重测后仍成立；新增用例经「删掉 `remove_file` 的变体」独立验证确能捕获回归；清扫只删本次 `create_new` 创建的那个文件，预置的同名旧文件字节未变。

改动清单核对：`git status` 为 13 项（10 改 + 3 未跟踪），与第 3 轮「实现记录」所述逐行一致，第 3 轮相对第 2 轮只多出 `.agents/tasks/任务管理/06-周期规则引擎.md`（编排者转任务留痕），无清单遗漏、无清单外代码改动。

验证：`cargo test --workspace` 通过（126 passed / 0 failed：cross_platform_todo_lib 21、todo_domain 40、todo_server 57、todo_contracts 8，doc-test 0；其中 `a_failed_write_leaves_no_file_and_no_taken_name` 实跑 ok）；`cargo check --workspace` 通过——先 `cargo clean -p cross-platform-todo -p todo-domain` 强制重编，0 warning，非缓存结论；`pnpm run check` 通过（41 文件格式全绿，44 文件 0 warning / 0 lint / 0 type error）；`pnpm run types:generate` 复跑后 `git status` 与复跑前逐行一致，`commands.ts` 与 `CalendarExport.ts` 无漂移（本轮无契约变更，确认无隐性漂移）。另有一项仓库外独立取证程序（以路径依赖引用 `todo-domain`，原样搬 `write_new_file*` 对真实目录跑）：正常/回归两个变体的用例对照、删除失败的共享模式冲突、同名旧文件存活、32 线程并发 32 个不同文件、占满 64 名后返回 Err 且 64 个既有文件字节未变，全部如上述结论；该程序跑完即弃，仓库内无残留。
