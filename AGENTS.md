# 跨端待办应用：工程与协作规范

本仓库交付的是跨平台待办应用；`.agents/` 保存研发协作配置，不是独立流程项目。

## 描述

这是一个面向 Windows、macOS、Linux、iOS 和 Android 的跨端待办应用。用户可创建、查看、组织和完成待办任务，并在不同设备上获得一致的任务管理体验。

当前已实现基础待办、主题、Store 持久化、提醒通知、桌面卡片与桌面端跨设备同步；下一阶段目标包括账号登录、周期任务/标签/四象限，以及客户端与移动端的 SQLite 持久化。产品目标形态见 README.md「产品定义」。

## 场景上下文

以下文档不会自动进入上下文，命中引入时机再读。工程规范以本文档为准；产品与设计事实以对应文档为准。

- **DESIGN.md** —— 设计令牌与 UI 视觉规范的唯一事实源。引入时机：写或改任何视觉样式（颜色、字体、间距、圆角、阴影、动效、暗色适配）前；禁止凭记忆写色值，先查此文档。
- **TODO.md** —— 功能点目录与实现状态（模块 → 功能 → 功能点 → 状态）。引入时机：确认任务范围、进度或挑选下一项工作时；功能实现或状态变化后同步更新对应条目。
- **PROMPT.md** —— 常用提示词（多 Agent 流程触发与控制词等）。引入时机：需要复用团队工作流提示词时。
- **.agents/workflow.md** —— 多 Agent 协作流程：角色、任务文件与状态机。引入时机：任务进入多 Agent 流程时（判定见「多 Agent 流程」节）。
- **PRODUCT.md** —— 产品说明。待补充：尚未创建，创建后在此接线。

## 多 Agent 流程

产生代码或文档改动的任务默认进入多 Agent 流程：当前会话担任编排者，按 `.agents/workflow.md` 定级、创建任务文件并按状态机推进至「完成」。

以下情况不进入流程，直接处理：

- 纯咨询：答疑、代码解释、方案讨论等不产生改动的请求。
- 操作类：运行命令、查看日志或状态。
- 微改：单文件、无逻辑变化的行级修改（错别字、文案、注释）。
- 用户明确要求跳过（如「直接改」）。

用户可随时用控制词覆盖默认判定（「直接改」「走全流程/简化/直达」「继续 <任务名>」等），见 PROMPT.md「多 Agent 流程」节。

## 架构

### 视图端

视图端是基于 Vue 3 的共享 Web 界面，负责页面渲染、交互和视图状态。组件使用 shadcn-vue 约定组织，Pinia 管理状态；组件不直接依赖平台专有 API，并通过生成的 bindings 与客户端能力协作。

### 客户端

客户端是基于 Tauri 2 的跨平台原生应用，负责应用生命周期、窗口与系统能力，并承载视图端。Rust command 与 capability 以最小权限方式向视图端暴露原生能力；目标平台包括 Windows、macOS、Linux、iOS 和 Android。本地任务数据当前以官方 Store 插件持久化，目标迁移为 SQLite（桌面与移动统一），并作为同步与周期任务的本地数据底座，经 repository 接口接入视图端。

### 服务端

服务端是基于 Axum 的 Rust HTTP 应用，负责路由、CORS、服务逻辑与未来的数据访问入口。`crates/domain/` 承载不依赖 Tauri、HTTP 或数据库的领域规则；`crates/contracts/` 是 HTTP DTO 与生成 TypeScript bindings 的唯一来源。

## 目录

```text
src/                 # Vue、Pinia、生成的 Tauri bindings
src-tauri/           # Tauri Rust 入口、capabilities 与打包配置
crates/
  contracts/         # Rust DTO；HTTP 与 TypeScript bindings 的唯一源
  domain/            # Rust 纯领域规则
  server/            # Axum 服务端
.agents/             # 工作流及角色职责（非产品代码）
```

## 端关系图

```mermaid
flowchart LR
  subgraph View[视图端：Vue 3 Web 应用]
    UI[页面与 shadcn-vue 组件] --> Store[Pinia Store]
    Store --> ViewDomain[TypeScript 类型与视图规则]
  end

  subgraph Client[客户端：Tauri 2 原生应用]
    TauriAPI[@tauri-apps/api] --> Core[Tauri 2 Rust Core]
    Core --> Bindings[生成的 IPC bindings]
    Core --> Native[Windows / macOS / Linux / iOS / Android]
  end

  subgraph Server[服务端：Axum HTTP 服务]
    Routes[路由与服务逻辑] --> Domain[Rust 领域规则]
    Contracts[contracts DTO] --> Routes
  end

  UI --> TauriAPI
  ViewDomain -. HTTP DTO .-> Contracts
  Client -. HTTP 请求 .-> Routes
```

## 开发规范

### 通用

倾向谨慎而非速度。

- **先想后写**：有歧义先问、别私下选；有更简方案先讲；假设与困惑都要摆明。
- **简约至上**：解决问题的最小代码；不做没要求的「灵活性/可配置性」；一次性代码不抽象；不为不可能的分支写兜底。
- **手术式改动**：只碰必要处、与现有风格一致；不顺手重构或改无关代码/格式；改动产生的孤立 import/变量要清掉，既有死代码只标记不删除。
- **可验证收尾**：把任务转成可检查的成功标准并循环至通过；验证手段见「应用命令」节的当前阶段验证标准。
- 以需求和仓库现状为准，信息不完整时可做合理假设并注明；改动前先确认影响范围，尽量减少对无关模块的影响。
- 不删除既有代码注释；代码注释沿用英文，面向用户的界面文案使用中文，与现有代码保持一致。
- 禁止引导式噪音内容。

### 端侧

跨端与 HTTP 契约一律以 Rust 生成的 bindings 为唯一来源，跨层输入输出必须是可序列化结构；当前未引入显式版本号机制，契约字段变更以重新生成 bindings 并同步全部调用方为兼容手段。

业务实体同样单源：`Todo`、`TodoStatus` 等实体唯一定义在 `crates/contracts/`，经 ts-rs 生成 `src/bindings/models/`；视图端禁止手写实体结构，`src/types/` 仅做生成类型的再导出（实体的视图侧引用入口）或存放纯视图态类型。

#### 视图端

- 技术栈为 TypeScript、Vue 3 Composition API、Pinia、UnoCSS 与 shadcn-vue。界面与交互需求只改 `src/`：页面组合在 `App.vue`，业务组件放 `src/components/`（shadcn-vue 基础组件按 `src/components/ui/<组件>/` 组织），可复用逻辑与平台探测放 `src/lib/`，视图侧类型放 `src/types/`，状态放 `src/stores/`。
- 组件保持平台无关，不得直接依赖平台专有 API；原生调用仅经 `@tauri-apps/api` 与 `src/bindings/`，统一走 `src/lib/native.ts` 的 `nativeCommands`，禁止以字符串名直接 `invoke`；`src/bindings/` 为生成产物，禁止手改。
- 所需原生能力 bindings 中没有时，先按「客户端」流程新增 command 并重新生成，再在视图端消费。
- 视图状态集中在 Pinia；持久化、同步等后续能力通过 repository 接口注入，避免组件耦合实现细节；前端诊断日志经 `src/lib/diagnostics.ts` 的 `writeDiagnostic` 记录。
- 与服务端交互统一经 `src/lib/sync-transport.ts` 传输层，禁止在前端手写与 Rust DTO 同名的契约，组件内不得直接 `fetch` 服务端；非 Tauri 运行时（浏览器开发、移动端）传输层降级为 no-op，应用离线可用。
- 完成前按「应用命令」节的当前阶段验证标准执行验证。

#### 客户端

- 新增原生能力：在 `src-tauri/src/lib.rs` 编写 `#[tauri::command]` + `#[specta::specta]` 函数并注册进 `collect_commands![...]`，运行 `pnpm run types:generate` 刷新 `src/bindings/commands.ts` 后前端方可调用。
- 系统权限在 `src-tauri/capabilities/default.json` 按最小权限声明，只加当前需求用到的能力。
- 原生侧失败必须容错降级而非中断应用（如托盘创建失败继续运行、设置读取失败取默认值），以 `log::warn!` 记录原因。

#### 服务端

- 新增接口按依赖方向落层：DTO 与校验先进 `crates/contracts/`（serde 解码，派生 `ts_rs::TS` 导出 TypeScript bindings），纯规则进 `crates/domain/`（不得依赖 Tauri、HTTP 或数据库），`crates/server/` 只加 service 与 handler，以 `/v1/` 前缀注册进 `create_router`。
- contracts DTO 统一派生 `Clone, Debug, Deserialize, Serialize, TS, Type`，标注 `#[serde(rename_all = "camelCase", deny_unknown_fields)]` 与 `#[ts(rename_all = "camelCase")]`；可选字段加 `#[serde(default, skip_serializing_if = "Option::is_none")]` 与 `#[ts(optional)]`（允许写入 null 的用 `#[ts(optional = nullable)]`）；校验作为契约方法就近实现（如 `validate() -> Result<(), ContractValidationError>`）。
- handler 只做提取与编排，业务逻辑放独立 service（如 `SyncService`）；共享状态以 `Arc` + `Mutex` 经 `with_state` 注入；Axum 路由、CORS 与服务逻辑按功能模块组织。
- 依赖方向单向：server → domain → contracts，不得反向引用；contracts 变更后必须重跑 `pnpm run types:generate` 并同步前端调用方。
- 新行为补测试，`cargo test --workspace` 通过后才算完成。

### 编程语言

#### Rust

- 原生层与服务层使用 Rust 2021，最低支持 Rust 1.88。
- Tauri IPC command 必须同时使用 `#[tauri::command]` 与 `#[specta::specta]`。
- 错误类型用普通枚举手写 `Display` 与 `std::error::Error`；仓库未引入 `thiserror`/`anyhow`，勿新增此类依赖。
- 测试与实现同文件，置于 `#[cfg(test)] mod tests`。

#### TypeScript

- 新增代码禁止 `any`；`verbatimModuleSyntax` 已开启，纯类型导入必须使用 `import type`；模块引用统一走 `@/` 别名（映射 `src/`）。
- 时间戳一律 ISO 8601 字符串，本地 id 用 `crypto.randomUUID()`。
- 不等待的 Promise 显式加 `void` 前缀。

#### JavaScript

- 应用源码使用 TypeScript；JavaScript 仅用于无法使用 TypeScript 的构建配置、自动化脚本或第三方工具入口。
- JavaScript 脚本必须明确输入、输出与错误处理，不将业务逻辑分散到临时脚本中。

#### HTML

- 页面结构使用语义化 HTML；表单控件必须关联可见标签或可访问名称。
- 图标按钮、弹窗与状态提示必须提供可访问文本、键盘操作和恰当的 ARIA 属性。
- 不使用内联事件处理器或内联样式承载业务逻辑。
 
#### CSS

- 使用 UnoCSS 与项目设计令牌实现样式；优先复用 shadcn-vue 组件和已有样式约定。
- 新增样式必须同时考虑桌面端与移动端，避免固定宽高导致文字溢出或触控区域不足。
- 不使用 `!important` 覆盖基础组件，除非现有样式机制无法表达且在变更中说明原因。
- 主题色与通用样式 shortcut 集中在 `uno.config.ts`（`brand` 色、`surface-card`、`focus-ring`）；重复出现的原子类组合沉淀为 shortcut，而非在组件间复制类串。
- 暗色样式以 `dark:` 变体与浅色成对提供，参照既有 shortcuts 写法。

#### Vue

- 使用 Vue 3 Composition API 与 `<script setup lang="ts">`；组件按单一职责拆分，组件文件用 PascalCase。
- 页面负责组合布局与路由上下文，可复用交互沉淀为组件；状态与副作用集中在 Pinia store 或组合式函数。
- 模板中的列表必须提供稳定 `key`；异步视图必须显式处理加载、空数据和错误状态。
- Pinia store 使用 setup 语法 `defineStore("<名>", () => { ... })` 并导出 `useXxxStore`。



## 应用命令

```bash
pnpm install
pnpm run dev                # Desktop Vite+ 开发服务器
pnpm run tauri:dev          # Tauri Desktop 开发
pnpm run check              # Desktop 格式、lint 与类型检查
pnpm run build              # Desktop 前端生产构建
pnpm run types:generate     # 从 Rust 导出 ts-rs 与 tauri-specta bindings
pnpm run tauri:build

# 从仓库根目录执行 Rust 服务端命令
cargo run -p todo-server
cargo test --workspace      # server、domain、contracts 与绑定导出测试
cargo check --workspace
```

移动端首次配置在对应工具链机器的仓库根目录执行 `pnpm run tauri android init` 或 `pnpm run tauri ios init`。Android 需要 Android SDK；iOS 初始化和构建仅可在 macOS/Xcode 环境执行。

当前阶段验证标准：前端尚未接入测试框架，行为变更以 `pnpm run check` 通过并经 `pnpm run dev` 或 `pnpm run tauri:dev` 手动走查为验证证据；Rust 变更以 `cargo test --workspace` 与 `cargo check --workspace` 通过为准；契约或类型变更必须运行 `pnpm run types:generate` 并确认生成结果与调用方同步。

