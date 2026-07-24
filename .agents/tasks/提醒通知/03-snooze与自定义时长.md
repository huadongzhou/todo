# snooze与自定义时长

- 级别：简化
- 状态：完成

## 需求（产品）

- 目标与场景：通知可稍后提醒，快捷时长可配（5 分钟 / 1 小时 / 今晚 / 明天）。
- 范围（做 / 不做）：做 snooze 状态机与时长配置；Windows 通知按钮能力不足时按设计降级（点开应用后的快捷条）。
- 验收标准（逐条可检查）：
  - [ ] 稍后提醒到时再触发；快捷时长可在设置内配置并生效。

## 规格（UI/UX）

先通读了 01（`提醒通知/01-Rust调度底座`）与 02（`提醒通知/02-周期提醒接入`）的实现记录，本规格接其架构：桌面提醒由 Rust 后台线程每 30s 轮询 SQLite、按纯规则 `due_reminders` 现算到点/逾期、`reminder_deliveries` 按 `(todo_id, reminder_at)` 防重发。关键结论：**在这套模型里 snooze 就是「把该任务的 `reminderAt` 改到一个新的未来时刻」**——新的 `(todo_id, reminder_at)` 即新投递键，下一轮（≤30s）轮询自然发它，旧键已投递不重发。因此 snooze 的核心机制**无需任何新运行时/契约代码**，前端调既有 `todoStore.update(id, { reminderAt })` 即可（`src/stores/todos.ts:813` 的 `update` 已处理 `reminderAt` 补丁并挂出站同步 op）。本规格的工作量集中在两个界面面：**设置里的时长配置** 与 **应用内 snooze 快捷条（降级路径）**。

### 0. 三个关键判断（正面表态）

| 议题 | 判断 |
|------|------|
| **a) snooze 从哪发起** | 两套并存、以降级为主交付。**理想（有按钮）**：通知气泡带一个「稍后」动作按钮——但 `tauri-plugin-notification` 的 Windows 动作按钮能力**未经本仓核实**，且 01 的 Rust 直发通知当前既未挂动作、也没有 fire 事件回前端。故把它定为**能力确认后再接的渐进增强**，交编排者转开发核实（见第 6 节）。**降级（无按钮，本任务主交付）**：点通知打开应用后，主窗顶部列表上方出现一条 **snooze 快捷条**，对「刚到点、仍待处理」的提醒逐条给出稍后/完成/跳过。降级路径**不依赖任何未确认能力**，是本任务的可交付底线。 |
| **b) 快捷时长怎么配** | 设置面板新增「提醒」fieldset，四个时长各一个**开关复选框**（默认全开），**不做自定义数值输入**（简约至上，四档已覆盖常见场景）。「今晚」「明天」是相对时刻，**本任务固定默认值：今晚 = 当天 20:00、明天 = 次日 09:00**，**本任务内不单独可配**——它们应与「默认提醒时间」（`外观与设置/02`，未落地）、「晨间摘要时间」（`提醒通知/06`，未落地）保持一致，待那两项落地后由「明天」复用晨间时刻、避免双份配置漂移（见第 3、6 节）。 |
| **c) snooze 后列表怎么呈现** | snooze = 覆写 `reminderAt` 为新未来时刻，于是：① **不新增持久化「已稍后到 X」徽标**（那需要一个「是 snooze 还是普通设提醒」的存储标志＝契约前置，超简约级范围），沿用现有 `· 已设提醒` 徽标即可；② snooze 的反馈走**既有唯一 live region**（transient/success：「已稍后：<任务> 将在 <时刻> 再次提醒」）＋快捷条内联，而非列表持久标识；③ 与**逾期标注**的关系：逾期标注是 01 对「已过点 reminder」打的通知文案前缀，snooze 把 `reminderAt` 推到未来后该提醒**不再逾期**，自然消解；与**到期徽标**（红/琥珀/中性）**正交**——到期徽标由 `dueDate` 驱动（`src/lib/dueDate.ts`），snooze 只动 `reminderAt`，一个截止日已逾期的任务把提醒 snooze 到今晚，列表到期徽标仍是红色「逾期 N 天」，两者互不影响。若产品坚持要持久「已稍后」徽标，作为契约前置交编排者裁决（第 6 节），本规格默认不做。 |

### 1. snooze 发起：两套路径

#### 1.1 理想路径 —— 通知动作按钮（能力确认后接，本任务不实现）

- 若开发核实 `tauri-plugin-notification` 在目标平台支持动作按钮、且 01 的 Rust 调度端能挂动作并回收点击：通知**只挂一个** snooze 动作「稍后 · <默认时长>」（不在气泡里塞四个按钮——通知气泡跨平台按钮数受限，逐档选择留给应用内快捷条）。**默认时长** = 用户已启用的四档中、按 `[5 分钟, 1 小时, 今晚, 明天]` 顺序**第一个能解析为未来时刻**的一档（例如 20:00 后「今晚」不可用则顺延取下一档）。
- 点该动作 = 对该任务执行一次 snooze（写 `reminderAt`），无需打开主窗。
- 这属**能力/契约前置**，非本任务交付；若届时能力不足或不接，降级路径（1.2）已独立覆盖需求。

#### 1.2 降级路径 —— 应用内 snooze 快捷条（本任务主交付）

用户点通知气泡 → 主窗被唤到前台（唤前台若需 Rust 侧激活处理，见第 6 节；即便不接，用户以任何方式打开应用都能看到此条）→ 主窗顶部出现一条 **snooze 快捷条**，对「刚到点、仍待处理」的提醒给出操作。详见第 2 节。

### 2. snooze 快捷条（组件级规格，五项齐全）

**新增组件** `src/components/ReminderSnoozeBar.vue`，在 `src/App.vue` 主窗分支内、**创建表单与快捷键提示之后、待办列表 `<section>` 之前**挂载（独立 `<section>`，**不嵌进任何 `<TodoFields>` 外壳**，故与 `任务管理/02` B2 的容器禁止清单无涉——快捷条自身不是那种容器，也不给创建/编辑容器套高度或滚动壳）。放在创建表单之后是遵 DESIGN.md 设计原则 1「单屏只有一个主操作」：创建表单是页面主 CTA，居顶不被打断，快捷条紧贴它所关切的任务列表。

**挂载门**：本任务用 `v-if="settingsStore.isDesktop"` 门控快捷条渲染（桌面为主；移动端本地通知与其快捷条接入归 `2.3`，届时去掉此门复用本组件，故样式按窄屏就绪设计）。

**待处理提醒的判定**（与 01 `due_reminders` 的 gating 对齐，保持同一口径，避免快捷条与实际发出的通知不一致）：`status === "open"` ∧ `archivedAt == null` ∧ 未被依赖锁定（`todoStore.dependencyLock(id).locked === false`）∧ `reminderAt` 存在且 ≤ 现在。再减去**本会话已跳过/已关闭**的项。命中集合按 `reminderAt` **由近及远**排序。

**逐条呈现（一次一条）**：快捷条一次只展示命中集合中最近到点的**一条**，处理（稍后/完成/跳过）后自动前进到下一条；集合空则整条卸载。逐条而非列表，是为了避免历史积压时铺一屏、并让窄屏与焦点管理简单（01 已记录「首次升级历史过期提醒会补发」的迁移现象，逐条 + 可关闭天然吸收）。

**结构与布局**：
- 外层：`surface-card`（DESIGN.md 卡片唯一档：`rounded-xl` + 1px 边框 + `shadow-sm`，含暗色对）+ `mb-6 p-4`（内边距 16，对齐创建表单 `p-4` 与列表行 `px-4`）。
- 首行：左侧 `BellRing`（`lucide-vue-next`，`:size="20"` 按钮档）置于 `h-11 w-11 rounded-xl bg-sky-100 text-sky-600`（复用 App.vue 页头/空态既有的 sky-100/sky-600 图标底片，`aria-hidden="true"`）＋ 标题文案「提醒待处理」；右侧 `X`（`:size="18"`）关闭整条的 ghost 按钮（`aria-label="关闭提醒快捷条"`）。
- 主体：当前任务标题（`text-slate-800 dark:text-slate-100`，`truncate` 防溢出）＋一行次要说明「已到点 · <相对时刻>」（`text-xs text-slate-500 dark:text-slate-400`，`tabular-nums`）。
- 操作区（`flex flex-wrap items-center gap-2`，与列表行补卡按钮同一 flex-wrap 节奏）：
  - **已启用的时长按钮**（复用 `Button variant="ghost"`，样式沿用 `TodoItem.vue` 补卡按钮串 `min-h-11 !px-3 !py-1 text-xs`；文案即「5 分钟 / 1 小时 / 今晚 / 明天」，纯文字＝可访问名）。只渲染**启用且解析为未来时刻**的档（如 20:00 后「今晚」不出现）。
  - **「完成」**（`Button variant="ghost" class="min-h-11"`，调 `todoStore.toggle(id)`）。
  - **「跳过」**（ghost，本会话把该项移出队列并前进，不改数据）。
  - 尾随计数「还有 N 项」（`text-xs text-slate-500 dark:text-slate-400`，`N>0` 才显示）。
- 若启用档为空：不渲染时长按钮，代之一行内联提示「未启用快捷时长，可在设置中开启」（`text-xs text-slate-500 dark:text-slate-400`），完成/跳过/关闭仍在。

**状态**：
- 默认：如上。
- 悬停：按钮走 `Button.vue` 既有 ghost hover（`hover:bg-slate-100 hover:text-slate-950`）；卡片本身无 hover 态。
- 聚焦：所有可交互元素为 `<Button>`，聚焦样式继承 `Button.vue` 的 `focus-visible:focus-ring`（注：`focus-ring` shortcut 现有失效缺陷已转 `外观与设置/04`，修复后本条按钮一并生效；本任务不在组件里旁路它，以免与全仓修复冲突）。
- 加载：无异步加载态——命中集合是 store 内存数据的纯派生，同步可得。
- 空数据：命中集合为空即整条 `v-if` 不渲染（不是显示「暂无提醒」空卡片——没有待处理提醒时页面就该像没有这条）。
- 错误：snooze 写入即 `todoStore.update`，与编辑保存同路径；写入失败的降级由 store/持久层既有机制处理，快捷条不新增错误 UI。

**snooze 动作后的反馈（走既有唯一 live region，不新增第二个）**：点某时长按钮 → 计算目标时刻 → `todoStore.update(id, { reminderAt: 目标ISO })` → 该项因 `reminderAt` 变为未来而离开命中集合、前进到下一条 → 经**页面既有单一 `role="status"` live region**播报一条 transient/success：「已稍后：<任务标题> 将在 <目标时刻> 再次提醒。」快捷条自身**不设 `aria-live`**（避免出现第二个 live region，遵「三类页面提示单一 live region」）。为此需给 `src/lib/page-alert.ts` 的 `AlertSource` 增补一个 `reminder` 源（transient/success，排在 `export` 之后＝最低优先级，不会盖住 storage/表单的 standing 提示）——这是既有 `pickPageAlert` 文档里预留的「第四个源出现时」扩展点，属前端小改、非契约。

**时长解析规则**：
- 5 分钟 = `now + 5min`；1 小时 = `now + 60min`（恒为未来）。
- 今晚 = 当天 20:00 本地；仅当 `20:00 > now` 时提供，否则该档在快捷条与通知里都隐藏（已过 20:00 时「今晚」无 snooze 语义，不滚动到明晚以免文案与实际不符）。
- 明天 = 次日 09:00 本地（恒为未来）。
- 通用不变式：**只呈现解析结果严格晚于 now 的档**，产出 ISO 8601 字符串（与既有 `reminderAt` 存储口径一致）。

**暗色适配**：`surface-card` 自带暗色（`dark:border-slate-800 dark:bg-slate-950`）；文字对已在上文成对给出（`text-slate-800 dark:text-slate-100`、`text-slate-500 dark:text-slate-400`）；`bg-sky-100 text-sky-600` 图标片沿用既有用法（App.vue 页头同款，暗色下 sky-100 底仍为浅片、图标 sky-600，与既有一致，不单独反色）；ghost 按钮暗色 hover 继承 `Button.vue`。无新增色值。

**移动端适配**：操作区 `flex-wrap`，窄屏时时长按钮与完成/跳过自然折行、绝不横向滚动（DESIGN.md 硬性）；所有按钮 `min-h-11`（≥44px 触控），相邻 `gap-2`（8px，达相邻可点目标 ≥8px）；标题 `truncate` + `min-w-0` 防长标题撑破；320px 下逐元素无越界。桌面为主但形态已按窄屏就绪，供 `2.3` 直接复用。

**可访问性**：
- 容器 `<section aria-labelledby="reminder-bar-heading">`，标题「提醒待处理」为可见 `<h2 id="reminder-bar-heading">`（或视觉弱化但对读屏可见）。
- 时长按钮可访问名＝可见文字（「5 分钟」等），非图标按钮，无需额外 `aria-label`；关闭为图标按钮，带 `aria-label="关闭提醒快捷条"`。
- 键盘全通路：条内 Tab 顺序＝视觉顺序（时长档 → 完成 → 跳过 → 关闭 → 右上关闭 ×，二者取其一实现即可，避免两个关闭混淆——建议只保留右上 `X` 关闭整条 + 主体内「跳过」跳过单条）。快捷条出现不抢焦点（非模态、非弹层，故不套焦点圈闭）。
- 动作后焦点管理：处理一条后前进到下一条，焦点落到新当前条的第一个时长按钮；处理最后一条后整条卸载，焦点回落到列表首个可交互元素或 `document.body` 的兜底由前进逻辑负责（不留悬空焦点）。
- 不靠颜色单传：待处理、稍后目标、剩余计数全由文字直接说明。

### 3. 快捷时长配置（设置面板，五项齐全）

**位置**：`src/App.vue` 设置面板内，新增一个「提醒」`fieldset`，置于既有「任务」fieldset 之后、「桌面行为」之前。**不加 `isDesktop` 门**——与「任务」fieldset 同理（其注释明言「任务如何表现不是桌面能力」）：稍后时长是提醒行为偏好，`2.3` 移动端也会复用，故不按平台收窄。fieldset 结构、`legend`、行内 `label`+`checkbox` 样式**逐字复用**既有设置项形状（`任务`/`桌面行为` 组的 `mb-2 flex min-h-10 ... rounded-lg px-3 py-2 hover:bg-slate-50 dark:hover:bg-slate-900` + 双行 `title`/`description`）。

**控件**：四个复选框（开关每一项），存储走既有 `saveBooleanPreference`/`loadBooleanPreference`（`src/lib/settings-storage.ts` 已提供，含浏览器回退），键名建议 `reminder.snooze.5m` / `.1h` / `.tonight` / `.tomorrow`，默认全 `true`；在 `useSettingsStore`（`src/stores/settings.ts`）按 `rolloverOverdue` 同一模式加 4 个 ref + setter + bootstrap 读取。fieldset 顶部一句说明「以下时长会作为『稍后提醒』的快捷选项」。各项 `description`：
- 5 分钟 —— 「延后 5 分钟后再次提醒。」
- 1 小时 —— 「延后 1 小时后再次提醒。」
- 今晚 —— 「延后到今天 20:00 再次提醒（已过 20:00 时当天不提供此项）。」
- 明天 —— 「延后到明天 09:00 再次提醒。」
- **不提供自定义数值输入**（超简约级；四档足够）。全部取消勾选是允许的（快捷条/通知届时不显示时长档并给内联提示），不硬性拦截最后一项——拦截「最后一个开关」会引入额外交互噪音，简约至上。

**状态**：复选框即时落库（同 `setRolloverOverdue`）；持久化失败沿用既有 `persistenceError` 通道文案（设置面板底部「设置暂时无法保存」），不新增。悬停/聚焦沿用既有设置行（原生 checkbox 浏览器默认焦点框，与「任务」组一致）。无加载/空/错误新态。

**暗色适配**：整组复用既有设置行的暗色类（`dark:hover:bg-slate-900`、`dark:text-slate-100`、`dark:text-slate-400`），与「任务」组成对，无新增。

**移动端适配**：行 `min-h-10` 容器 + checkbox，触控命中沿用既有设置项（既有设置项已是当前基线）；文案自然换行不截断；无固定宽高。

**可访问性**：`fieldset`+`legend`（「提醒」）分组，读屏进组报出组名；每个 checkbox 关联可见 `label`（双行文字即可访问名 + 说明）；键盘可达、空格切换，与既有设置项一致。

### 4. snooze 后的呈现（列表标识与逾期/到期关系）

- **列表徽标**：沿用 `TodoItem.vue` 现有 `· 已设提醒`（挂在到期徽标内，`todo.dueDate` 存在时显示），**不新增持久「已稍后到 X」徽标**（理由见第 0 节 c 与第 6 节：需存储标志区分 snooze 与普通设提醒，属契约前置，超本级范围）。已知边界：无截止日的「纯提醒」任务当前列表本就无提醒徽标（既有限制，属 `任务管理/02` 徽标行范围），snooze 它同样不产生列表标识——其反馈由 live region + 快捷条承担，本任务不改徽标行。
- **与逾期标注关系**：01 的「【逾期】」是对已过点未投递提醒打的**通知文案前缀**；snooze 把 `reminderAt` 移到未来 → 新键不再是逾期状态 → 到点按普通（非逾期）文案发。即 snooze **消解**该次提醒的逾期标注。
- **与到期徽标关系**：正交。到期徽标（`dueDateTone` 红/琥珀/中性）只看 `dueDate`；snooze 只动 `reminderAt`。截止日已逾期的任务 snooze 提醒到今晚，列表仍显示红色「逾期 N 天」到期徽标不变——符合语义（deadline 逾期与「这条提醒推迟」是两件事）。

### 5. 令牌与出处核对（步骤 6 自查，均有出处，无新增设计令牌）

| 取值 | 出处 |
|------|------|
| `surface-card`（rounded-xl + 边框 + shadow-sm + 暗色对） | `uno.config.ts` shortcut；DESIGN.md「圆角与阴影」唯一阴影档 |
| `bg-sky-100 text-sky-600` 图标底片、`h-11 w-11 rounded-xl` | `src/App.vue` 页头与空态既有同款用法 |
| `BellRing`/`X` 图标，`:size="20"`/`18` | DESIGN.md 图标：唯一源 `lucide-vue-next`，20 按钮档/独立操作；纯图标按钮带 `aria-label` |
| ghost 按钮 `min-h-11 !px-3 !py-1 text-xs`、`Button variant="ghost"/"default"` | `src/components/ui/button/Button.vue` CVA；`TodoItem.vue` 补卡按钮同串；DESIGN.md 触控 ≥44px |
| 文本 `text-slate-800/500/400/100`、`tabular-nums`、`truncate`/`min-w-0` | DESIGN.md 中性色（slate 阶）、「数字用 tabular-nums」、禁横向滚动 |
| success 提示（emerald，transient） | DESIGN.md 语义色公式 + `src/App.vue` `ALERT_TONE_CLASS.success` 既有 |
| 间距 `p-4`/`mb-6`/`gap-2`/`mt-6`/`min-h-10` | DESIGN.md 间距 4/8 节奏；既有创建表单/设置组同值 |
| 设置行 `label`+checkbox 整串、`dark:hover:bg-slate-900` 等 | `src/App.vue` 既有「任务」「桌面行为」设置行逐字复用 |

**tone 类串落点注意（避 `dueDate.ts` 陷阱）**：本规格所有带 `dark:` 变体的颜色类串一律**内联写在 `.vue` 模板/`<script>`**（`ReminderSnoozeBar.vue`、`App.vue`），不新建 `.ts` 常量表存类串——UnoCSS 默认不扫 `.ts`，放 `.ts` 会令 `dark:` 类不生成（`TodoItem.vue` 注释反复标注的既有陷阱）。若确需一处 tone 映射，放在组件 `<script setup>` 内（`.vue` 被扫描），如 App.vue 的 `ALERT_TONE_CLASS`。

### 6. 前置与交编排者知悉（能力/契约）

以下不阻塞本任务**降级路径 + 时长配置**的交付（那两项零契约、零新令牌、可直接进开发），但需编排者知悉、按需转开发核实：

1. **通知动作按钮能力（能力前置）**：`tauri-plugin-notification` 在 Windows（及各目标平台）是否支持自定义动作按钮、以及 01 的 Rust 直发通知能否挂动作并回收点击事件——需开发核实。确认可用则接第 1.1 节的「稍后 · 默认时长」单动作；不可用则维持降级路径。
2. **点通知唤前台（能力/接线前置）**：降级路径希望「点通知气泡 → 唤起主窗」。01 的调度未挂通知激活处理。若要点气泡即唤前台，需 Rust 侧接激活/`onClick`（属 01/调度端小改）。**即便不接**，快捷条对「以任何方式打开应用」都可见，需求仍满足，故列为增强而非阻塞。
3. **snooze 落库的入站重排（已被 01/02 结构性满足，仅记录）**：snooze 写 `reminderAt` 经 `todoStore.update` 落 SQLite，下一轮轮询即发新键；跨设备经既有出站 op 同步。无需新代码。
4. **持久「已稍后到 X」列表徽标（契约前置，本规格默认不做）**：如产品要在列表持久区分「snooze 的提醒」与「普通设提醒」，需一个存储标志（如 `snoozedUntil` 字段）＝契约变更，超简约级范围。交编排者裁决；不采纳则维持第 4 节（沿用 `· 已设提醒` + live region 反馈）。
5. **今晚/明天时刻的一致性（跨任务前置，记录待接线）**：本任务固定今晚 20:00 / 明天 09:00。待 `外观与设置/02`（默认提醒时间）、`提醒通知/06`（晨间摘要时间）落地后，「明天」应复用晨间时刻以免双份配置漂移。本任务不预埋其配置，仅以常量实现并注明接线点。

### 7. 对需求的说明

需求验收「稍后提醒到时再触发；快捷时长可在设置内配置并生效」两条均由本规格覆盖：前者由降级快捷条（+ 能力允许时的通知按钮）写 `reminderAt`、经 01 轮询到时再发；后者由第 3 节设置项配置四档、快捷条/通知消费之。范围内的「Windows 通知按钮能力不足时按设计降级」已正面处理为第 1–2 节的两套路径，以降级为可交付底线。

## 实现记录（开发）

按规格实现，交互与视觉决策未改动。零契约、零 bindings、零 Rust 运行时改动——snooze 全走既有 `todoStore.update(id,{reminderAt})`，由 01 轮询架构承接。

### 能力核实（规格第 6 节点名，本轮实地核实：结论＝不可用，只交付应用内快捷条）

核实对象 `tauri-plugin-notification`（Cargo.lock 锁定 **2.3.3**），读插件源码（`~/.cargo/registry/.../tauri-plugin-notification-2.3.3/src/{lib,desktop}.rs`）：

- **桌面动作按钮：不支持。** 01 直发通知走的桌面路径是 `app.notification().builder().…show()`，其桌面 builder（`desktop.rs` 内层 `Notification`）只有 `title/body/icon/sound/show` 字段，`show()` 落到 `notify_rust::Notification`（仅 summary/body/icon/sound）。顶层 `NotificationBuilder` 虽有 `action_type_id()` 方法，但那是 mobile/JS 通道用；桌面 `show()` 完全不读它，动作在桌面路径上被丢弃。
- **点击回收：无。** 桌面 `show()` 做 `tauri::async_runtime::spawn(async move { let _ = notification.show(); })`——句柄即弃，既不等待也不回传任何用户交互事件。Windows 上要挂 toast 动作按钮＋激活回收，需绕开本插件直接用 `tauri-winrt-notification`＋COM activator＋AppUserModelID/快捷方式注册＝**新增依赖 + 大量原生工作**，违反 AGENTS「勿新增依赖」且与本简化级不成比例。
- **结论**：理想路径（1.1 通知动作按钮）**不接**，如规格所留「能力确认后再接的渐进增强」；交付主交付＝应用内 snooze 快捷条（1.2/第 2 节），不依赖该能力。
- **「点通知唤前台」（第 6 节点 2）**：同因——桌面 `show()` 弃句柄、无激活事件，本插件无回调可挂；接它同样需绕开插件的原生工作。故**不接**，列为边界。快捷条对「以任何方式打开应用」都可见（挂主窗顶部），需求不依赖此增强，非阻塞。

### 「刚到点、仍待处理」的数据来源（规格点名，本轮判断）

按规格定的**前端纯派生**，不加新 Rust command、不读 `reminder_deliveries`：命中集合＝`todoStore.visibleItems` 过滤 `status==="open"` ∧ `reminderAt` 存在且 `Date.parse(reminderAt) ≤ now` ∧ `!dependencyLock(id).locked`，再减本会话已跳过/已关闭项，按 reminderAt 由近及远（最近到点在前）取第一条。`visibleItems` 已承载「archived」的单一读法故不重复判 archivedAt；坏 reminderAt（`Date.parse` 为 NaN）自然排除，与 domain `due_reminders` 的 gating 口径一致（锁定/完成/归档不显）。选纯派生而非新 command：契约/bindings 零改动、同步可得、无第二数据源。为让「时间流逝致新提醒到点」能自动浮现，加一个 30s tick 的 `now` ref（对齐 01 轮询节奏），`onBeforeUnmount` 清理。

### 改动文件

- `src/components/ReminderSnoozeBar.vue`（新增）——应用内 snooze 快捷条。`v-if="isDesktop && current"` 门控；逐条呈现最近到点的待处理提醒；已启用且解析为未来时刻的时长档（5 分钟/1 小时/今晚/明天）+ 完成（`toggle`）/跳过（本会话移出，不改数据）/关闭整条（本会话移出当前整批）；snooze＝`todoStore.update(id,{reminderAt:目标ISO})`；今晚固定 20:00、明天固定 09:00（`TONIGHT_HOUR/TOMORROW_HOUR` 常量，注明将来复用「默认提醒时间/晨间摘要时间」的接线点）；「今晚」仅 `20:00>now` 时出现。反馈经 `announce` 事件上交 App 的单一 live region（transient/success），组件自身不设 `aria-live`。动作后焦点前进到新当前条首个按钮，末条卸载后落 body。tone/时间格式化等 `dark:` 类串与纯逻辑均内联在 `.vue`（避 `dueDate.ts` 的 UnoCSS 不扫 `.ts` 陷阱）。
- `src/App.vue`——① 设置面板新增「提醒」`fieldset`（置「任务」后、「桌面行为」前，**无 `isDesktop` 门**，同「任务」组理由），四档各一开关复选框（默认全开），行结构逐字复用既有设置行；顶部说明「以下时长会作为『稍后提醒』的快捷选项」。② 引入并在「快捷键提示之后、列表 `<section>` 之前」挂载 `<ReminderSnoozeBar @announce="announceReminder" />`。③ 新增 `reminderAlert` ref（形同 `exportAlert`，页面本地、transient/success）并接入 `pickPageAlert` 第四参；`announceReminder(message|null)` 包成/清空该源。
- `src/stores/settings.ts`——按 `rolloverOverdue` 同一模式加 4 个 ref（`snooze5m/1h/Tonight/Tomorrow`，默认 `true`）+ 4 个具名 setter（共用私有 `persistSnoozePreference` 免四份 try/catch 漂移）+ bootstrap 读取（键 `reminder.snooze.{5m,1h,tonight,tomorrow}`）+ catch 兜底默认 + 导出。
- `src/stores/settings.ts` 相邻的 `src/lib/settings-storage.ts` 未改——直接复用既有 `saveBooleanPreference`/`loadBooleanPreference`（含浏览器 localStorage 回退）。
- `src/lib/page-alert.ts`——`AlertSource` 增补 `reminder` 源（`SOURCE_ORDER` 排 `export` 之后＝最低优先级），复用文档预留的「第四个源」扩展点；更新顶部与不变式注释（reminder 为 transient/success、有确定终点＝快捷条在下一动作/卸载时清，故可短暂盖 standing 存储行）。仍单一 live region。
- `TODO.md`——2.4「通知上稍后提醒（snooze）」⏳→🚧（降级为应用内快捷条，注记通知按钮能力核实结论）、「自定义稍后时长」⏳→✅。

### 自验结果（命令与结论）

- `pnpm run check`：通过。58 文件格式全过；61 文件无 lint/类型错误（含新增组件与 App/store 改动）。
- `cargo test --workspace`：全绿，**251 passed / 0 failed**（与 01+02 基线一致；本任务零 Rust 改动，未增未减）。
- `cargo check --workspace`：无 warning、无 error。
- `pnpm run types:generate`：未运行——本任务无 command/契约/导出类型变更，`src/bindings/**` 不涉及（符合预期）。
- `pnpm run dev` 走查（浏览器 dev，port 1420 经确认serving本 worktree）：设置面板「提醒」fieldset 正确渲染于「任务」之后，四档复选框默认全开（DOM 实测 `[false(rollover),true,true,true,true]`）；切换「5 分钟」即时落 localStorage（`reminder.snooze.5m` false↔true），复选后复位默认，走查后 `removeItem` 清理、四键归 `null`（＝默认），未创建任何 todo；无 console 报错。**快捷条按 `isDesktop` 门在浏览器 dev 不渲染（isTauri 为 false），符合规格挂载门**。
- **验证边界**（同 01/02）：快捷条的实时 snooze 流程（到点浮现→点时长写 reminderAt→下一轮 Rust 轮询按新键重发）、动作后焦点管理、`announce` 播报，需 Tauri 桌面运行时观察；本环境不跑 `tauri:dev`（避免污染用户 appdata、非交互无法观测 toast）。其判定链为 store 内存纯派生 + 既有 `update`/`toggle`/`pickPageAlert` 路径，已由 `pnpm run check` 类型层与浏览器 dev 设置项走查覆盖到可验部分；OS 通知管道部分留待评审/集成在真实桌面确认。

### 遗留 / 提示

- **持久「已稍后到 X」列表徽标**（规格第 6 节点 4）：本规格默认不做（需存储标志区分 snooze 与普通设提醒＝契约前置，超简约级），沿用 `· 已设提醒` + live region 反馈。如产品坚持，交编排者裁决。
- **今晚/明天时刻一致性**（第 6 节点 5）：本任务以常量 20:00/09:00 实现并在组件注明接线点，待「默认提醒时间」（外观与设置/02）、「晨间摘要时间」（提醒通知/06）落地后由「明天」复用晨间时刻，避免双份配置漂移。
- transient reminder 成功行覆盖 standing 存储行的短暂性：与既有 form-alert transient「到下一次击键才清」同一被接受的取舍——快捷条在下一动作（跳过/完成/关闭）或卸载时 `announce(null)` 清行，存储 standing 行随后回归。

### 第 2 轮（回流修复：收尾裁决阻塞）

只处理第 2 轮那一条阻塞（transient reminder 播报在「snooze 掉最后一条」边界永久覆盖 standing 存储/同步告警），未顺手改别的。

**改动文件**

- `src/components/ReminderSnoozeBar.vue`——① `vue` 导入补 `watch`；② 新增 `watch(current, (value, previous) => { if (!value && previous) emit("announce", null); })`（置于生命周期钩子前，带成块注释说明为何这是缺失的第三个确定终点）。这给快捷条补上「命中集合空了」这一清除时机：snooze 掉最近到点的最后一条、或当前条被别处变更移出（另一设备同步完成等）导致 `current` 由非空变 null 时，收回本源那条「已稍后…」播报。守卫 `!value && previous` 使之只在「有→空」边触发，「非空→非空」（切下一条）不误收回。既有 `complete/skip/close` 的 `emit("announce", null)` 保留（「每次动作即清」语义正交，不删）。
- `src/lib/page-alert.ts`——不变式注释补第三个清除时机「when its last reminder leaves (the due set empties)」，使「transient 覆盖 standing 仅因有确定终点」的自述在此边界重新成立。纯注释，`AlertSource`/排序/逻辑未动。

**为何 emit null 只清 reminder 源（按要求核对多源合流）**：`emit("announce", null)` → `App.vue` `announceReminder(null)` 仅 `reminderAlert.value = null`；`pageAlert = pickPageAlert(formAlert, storageAlert, exportAlert, reminderAlert)` 重新合流，storage/sync 的 standing 告警原样回归。语义是「reminder 源没有内容了」，非「清空整个 live region」——storage/form/export 三源不受影响。仍单一 `role="status"` live region。

**自验结果（命令与结论）**

- `pnpm run check`：通过（58 文件格式、61 文件 lint+类型，含本轮改动）。
- `cargo test --workspace`：全绿 **251 passed / 0 failed**（29+14+150+58，与基线一致；本轮零 Rust 改动）。
- `cargo check --workspace`：无 warning、无 error。
- `pnpm run types:generate`：未运行——本轮无 command/契约/导出类型变更（符合预期）。
- `pnpm run dev` 浏览器实测（port 1420，`fetch` 确认 dev 服务本 worktree：`page-alert.ts` 注释与 `ReminderSnoozeBar.vue` 新 watcher 均已随服务生效）：因 `isDesktop=isTauri()` 为无依赖缓存 computed、浏览器下恒 false，快捷条 `<section>` 在浏览器 dev 仍不渲染（与 01/02 及第 1 轮同）、按钮不可点，故以**导入本仓真实 `page-alert.ts` 的 `pickPageAlert` + 镜像 App.vue 四源合流与本组件 watcher 判定式**跑回流场景：① 一条 standing 存储/同步告警（`{tone:warn,lifetime:standing,source:storage}`）独占提示条 → 命中 storage；② 一条到点提醒 `current: null→A`，watcher 因 previous 为 null 不触发、storage 仍在；③ snooze 最后一条：先 `announceReminder(message)` → transient/success 盖住 storage（命中 reminder），随后 `current: A→null` 触发 watcher → `announceReminder(null)` → 命中回到 **storage（存储告警重新可见）**；④ 守卫：`current: B→C`（非空→非空）watcher 不收回、确认行保留。四步结果与预期完全一致，证明「收回本源播报后 standing 告警重新可见」且「切下一条不误收回」。走查未创建任何 todo、未写 localStorage（`reminder.snooze.*` 与 todo 键均空），无需清理。
- **验证边界**（同 01/02 与第 1 轮）：真实桌面快捷条渲染态下**点击** snooze 按钮触发上述链路，需 Tauri 桌面运行时（`isTauri()` 为 true）观察；本环境不跑 `tauri:dev`。本轮改的是组件 `watch` 判定式 + App 既有 `announceReminder`/`pickPageAlert` 合流，其正确性已由上面导入真实模块的合流实测 + 类型层 `pnpm run check` 覆盖到可验部分。

## 评审记录

### 第 1 轮

范围＝「实现记录」改动文件清单：`ReminderSnoozeBar.vue`（新增）、`App.vue`、`stores/settings.ts`、`lib/page-alert.ts`、`TODO.md`。`git status` 核对无清单外改动（依赖 01/02 已提交，不计遗漏）。

独立核实要点（均通过，无阻塞）：
- **能力核实结论可信（真实非偷懒）**：实地读 `tauri-plugin-notification` 2.3.3 源。`NotificationBuilder::action_type_id()`（`lib.rs:116`）挂在跨端共享 builder 上，但桌面 `show()`（`desktop.rs:26-53`）只读 `title/body/icon/sound`、从不消费 `action_type_id`，动作在桌面路径被静默丢弃；内层 `imp::Notification` 结构（`desktop.rs:100-112`）也只有 body/title/icon/sound；`imp::show()`（`desktop.rs:216-218`）`spawn(async move { let _ = notification.show(); })` 弃句柄、无点击回收。桌面经本插件确实拿不到动作按钮/激活回调，Windows toast 动作需绕开插件（winrt+COM+AppUserModelID）＝新增依赖+大量原生工作，与简化级不成比例。结论「不接理想路径、只交付应用内快捷条」基于事实。
- **snooze 机制正确**：snooze＝`todoStore.update(id,{reminderAt:未来ISO})`（`ReminderSnoozeBar.vue:196`）。旧 `(id,旧reminderAt)` 已在 01 `delivered` 集、新时刻为新键（domain `reminder.rs:83-86` 测试 `a_reminder_edited_to_a_new_time_is_raised_again` 已覆盖），下一轮轮询发新键、旧键不重发；「今晚」仅 `20:00>now` 呈现、已过则隐藏不顺延（`:121-126`），「明天」按日历日+1（DST 安全，`:146-151`）；snooze 已逾期提醒把 reminderAt 推未来即消解逾期、按普通文案重发——均正确。
- **「刚到点、仍待处理」判定与 01 gating 同口径**：`matches`（`:68-80`）过滤 `status==="open"` ∧ reminderAt 存在且可解析且 ≤now ∧ 未依赖锁定，源自 `visibleItems`（`stores/todos.ts:332`＝`items` 去归档，非按列表/搜索/象限过滤的子集），population 与 domain `due_reminders` 一致；差异仅在（有意）不排除 delivered（快捷条本就是「已发通知后」的跟进 UI）与（有意）本会话 dismissed。不会漏显 01 会发的项。「本会话 skip 不持久」＝规格既定取舍（持久化需存储标志＝契约前置，超简化级）；重启后仍开且过点的项会在条内重现但不再发新 OS 通知（01 delivered 持久），可关闭，可接受。
- **快捷时长配置**：四档默认全开、键 `reminder.snooze.{5m,1h,tonight,tomorrow}` 走既有 `saveBooleanPreference`/`loadBooleanPreference`，按 `rolloverOverdue` 同模式落 store（4 ref+具名 setter 共用 `persistSnoozePreference`+bootstrap+catch 兜底），`dev` 走查实测：置「任务」后、默认 `[true,true,true,true]`、切「5 分钟」即写 localStorage `reminder.snooze.5m=false`、复位复原。今晚 20:00/明天 09:00 为 `TONIGHT_HOUR/TOMORROW_HOUR` 常量并注明将来复用接线点。
- **单一 live region**：`page-alert.ts` `AlertSource` 增 `reminder`、`SOURCE_ORDER` 排 export 之后＝最低源优先级（`:24,:35`），一致；页面仍单一 `role="status"` 区（`App.vue:380-390`），组件自身无 `aria-live`。reminder 为 transient/success，`transient`(0)<`standing`(1) 故稳定压过 storage 的 standing——此为规格明列的有意排序（详见下条建议）。
- **规格还原与容器边界**：快捷条为独立根 `<section>`、直接挂 `App.vue` 主窗（列表 section 之前），不嵌 TodoFields，与 02 B2 容器禁令无涉；`v-if="isDesktop && current"` 门控合理（浏览器 isTauri=false 无 Rust 调度/通知，`dev` 实测不渲染快捷条）；所有 `dark:` 类串内联在 `.vue`，`pnpm run build` 后 `.dark` 选择器与 `text-slate-100` 均入 CSS（`.ts` 陷阱已避）。

- 建议 · src/components/ReminderSnoozeBar.vue:247 + src/App.vue:195-198 · snooze 掉「最近到点的最后一条」待处理提醒后，内层 `<section v-if="isDesktop && current">` 因 current 变 null 而隐藏，但 `<ReminderSnoozeBar>` 标签在 `App.vue:738` 无 v-if 故组件不卸载、`onBeforeUnmount` 不触发；此路径下刚 `announce` 出的「已稍后…」transient/success 行既无「下一动作」（按钮已随 section 隐藏、点不到）也无「卸载」来清，会一直覆盖 standing 的 `storageAlert`（同步/存储告警），直到有新提醒到点且用户再交互或应用退出。这与 `page-alert.ts:53-57` 自述不变式「reminder 可覆盖 storage standing 仅因其有 definite end：next action / unmount」在此边界不完全成立，可能短暂或长时间遮住真实同步告警。· 判定依据：page-alert.ts 自述不变式 + 检查维度 1（正确性/边界）。属边界组合（并发 standing 存储告警 + snooze 至空集），两条验收标准均不受影响，故列建议不列阻塞；可选修法：current 变 null 时一并 `emit("announce", null)`（与多条场景「下一动作即清」同质，代价是最后一条确认更短暂），交编排者/开发裁量。
  - 裁决（编排者）：升级为阻塞，见第 2 轮（收尾裁决）。这与 任务管理/03 的 applyCharCap、任务管理/11 的 URL blur 是同一类缺陷——一条 transient 提示覆盖 standing 的存储/同步告警（数据丢失级），且 `page-alert.ts` 自述不变式在此边界不成立。修法只有一行（评审已给：current 变 null 时 `emit("announce", null)`）。同类问题前面一律钉死，不留到日后。

验证：cargo test --workspace 通过（全绿 0 failed，含 domain reminder.rs 全部用例）；cargo check --workspace 通过（无 warning/error）；pnpm run check 通过（58 文件格式、61 文件 lint+类型）；pnpm run build 通过（dark: 变体入 CSS，.ts 陷阱已避）；pnpm run dev 走查：提醒 fieldset 置「任务」后、四档默认全开、切换即落 localStorage、快捷条浏览器不渲染，走查后已清 localStorage、未建 todo、无 console 报错。

### 第 2 轮（收尾裁决）

- 阻塞 · `src/components/ReminderSnoozeBar.vue:247` + `src/App.vue:195-198` · 沿自第 1 轮 · snooze 掉最后一条待处理提醒后，内层 `<section v-if>` 隐藏但组件不卸载（`App.vue` 挂载处无 v-if），`onBeforeUnmount` 不触发，刚播报的「已稍后…」transient/success 行没有「下一动作/卸载」可清，会覆盖 standing 的存储/同步告警直至新提醒到点或应用退出——`page-alert.ts` 自述「transient 覆盖仅因有确定终点」的不变式在此边界不成立。与 任务管理/03 applyCharCap、任务管理/11 URL blur 同类（transient 覆盖 standing 数据丢失级告警）。修法：`current` 变 null 时一并 `emit("announce", null)` 收回本源播报 · 正确性维度（提示遮蔽）
  - 处理（开发）：已按评审修法在 `ReminderSnoozeBar.vue` 增补 `watch(current, (value, previous) => { if (!value && previous) emit("announce", null); })`（导入补 `watch`）。为快捷条给出缺失的第三个「确定终点」：命中集合从有到空（snooze 掉最后一条，或当前条被别处变更移出——如另一设备同步完成）时，`current` 由非空变 null，本源那条 transient/success 播报被收回。**只收回 reminder 源**：`emit("announce", null)` 经 `App.vue` `announceReminder(null)` 仅置 `reminderAlert.value=null`，`pickPageAlert(form,storage,export,reminder)` 随即重挑，storage/sync 的 standing 告警原封回归——核对 `page-alert.ts` 多源合流确认 emit null 语义是「reminder 源没有内容了」而非「清空整个 live region」，不碰 storage/sync 源。**守卫**：条件 `!value && previous` 只在「有→空」边触发；切到下一条待处理提醒是「非空→非空」，不误触发收回（快捷条内 snooze 非最后一条时确认行照常保留到下一动作）。complete/skip/close 各自既有的 `emit("announce", null)` 保留不动（它们要「每次动作即清」，语义不同于「集合空了」）。同步把 `page-alert.ts` 不变式注释补上「集合空了」这第三个清除时机，使自述与实现重新一致。零契约/命令/bindings 变更。
  - 复核（评审）：认可。逐条核实——**(a) watch 谓词只在「有→空」边收回**：`watch(current, (value, previous) => { if (!value && previous) emit("announce", null); })`——非空→非空（切下一条待处理）`!value` 为 false 不触发；null→null `previous` 为假不触发；初始挂载无 `immediate` 不触发；仅 A→null（snooze 掉最后一条／当前条被别处移出）触发。导入本仓 `page-alert.ts` 真实 `pickPageAlert` 运行时逻辑、镜像 `App.vue` 四源合流实测：`emit("announce", null)→announceReminder(null)→reminderAlert=null`，`pickPageAlert(form,storage,export,reminder)` 重挑后 reminder 源退出、rank 111 的 storage 重新胜出，standing 存储/同步告警原样回归——**只清 reminder 源，不碰 form/storage/export**。**(b) 既有 emit(null) 全部保留、语义未变**：`complete`/`skip`/`close`（含 `onBeforeUnmount`）各自的 `emit("announce", null)` 均在；末条 skip/complete/close 会与 watch 各发一次 null，属幂等（同置 `reminderAlert=null`），无冲突、无重复收回。**(c) page-alert.ts 仅注释**：不变式新增「when its last reminder leaves (the due set empties)」一句，`AlertSource`/`SOURCE_ORDER`(reminder:3)/`rank`/`pickPageAlert` 与第 1 轮认可态一致、无逻辑改动。**(d) 无溢出**：工作区仅含第 1 轮清单五文件（`ReminderSnoozeBar.vue`/`App.vue`/`settings.ts`/`page-alert.ts`/`TODO.md`）+ 任务文件，第 2 轮未引入任何新文件/新路径，代码 delta 限于 `ReminderSnoozeBar.vue`（补 `watch` 导入 + watch 块）与 `page-alert.ts`（注释）。第 1 轮 blocker（transient 永久遮蔽 standing 存储/同步告警）已消解。

- 建议 · src/components/ReminderSnoozeBar.vue:191-202 + :255-257 · snooze 最近到点的**最后一条**（命中集合 snooze 后即空）时，`snooze()` 同步 `emit("announce", message)` 与 watch 的 `emit("announce", null)` 落在 Vue 同一次 flush（默认 pre-watcher 先于渲染 effect），`reminderAlert` 在同一 flush 内被置值又清空、`role="status"` 从不渲染该确认——此边界下「已稍后…」对读屏**完全不播报**（非「更短暂」而是「不播报」），略偏离规格第 2 节「反馈走 live region」。**非阻塞**：不影响两条验收标准（snooze 仍写 `reminderAt`、到时重发；快捷时长配置生效），且属第 1 轮裁决已接受的取舍（standing 存储/同步告警可见性 > transient 确认，优先级正确）；非最后一条（非空→非空）确认照常保留、不受影响。此为按 Vue 默认 flush 批处理的推断，真实渲染时序与第 1 轮同属需 Tauri 桌面运行时（`isTauri()` 为 true）确认的边界，交编排者裁量。· 判定依据：规格第 2 节反馈 live region + 检查维度 1（正确性/可访问性）
  - 处理（编排者）：转任务处理，并入 外观与设置/04-字号缩放与高对比（读屏播报批量实测）。评审已判定非阻塞、属第 1 轮已接受的取舍（standing 存储/同步告警可见性 > transient 确认，优先级正确），且是按 Vue 默认 flush 批处理的推断、需真实桌面读屏确认。它正是「读屏最终播报了什么」这类需真实读屏环境验证的点，与 外观与设置/04 已并入的一批播报点一并在装 NVDA 的机器上实测——届时若确认「snooze 最后一条无播报」不可接受，再连同其余播报点一起校准。

验证：pnpm run check 通过（58 文件格式、61 文件 lint+类型）；cargo check --workspace 无 warning/error；cargo test --workspace 全绿 251 passed（29+14+150+58）/0 failed；pnpm run dev 快捷条受 isDesktop/isTauri 门在浏览器 dev 不渲染（同第 1 轮），改以导入本仓 `page-alert.ts` 真实 `pickPageAlert` 运行时逻辑 + 镜像 App.vue 四源合流 + 本组件 watch 谓词跑回流四场景（storage standing 独占 → reminder due 不误收回 → snooze 末条 transient 盖住 → A→null watch 收回、storage 重现 → B→C 守卫不误触发），结果与预期逐项一致；脚本为独立 node、未写 localStorage/未建 todo，无需清理。
