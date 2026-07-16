# 跨端待办应用：工程与协作规范

本仓库交付的是跨平台待办应用；`.agents/` 保存研发协作配置，不是独立流程项目。

## 描述

这是一个面向 Windows、macOS、Linux、iOS 和 Android 的跨端待办应用。用户可创建、查看、组织和完成待办任务，并在不同设备上获得一致的任务管理体验。

当前阶段聚焦内存态待办示例及基础交互；持久化、同步、账号和提醒不在本阶段范围内。

## 架构

### 视图端

视图端是基于 Vue 3 的共享 Web 界面，负责页面渲染、交互和视图状态。组件使用 shadcn-vue 约定组织，Pinia 管理状态；组件不直接依赖平台专有 API，并通过生成的 bindings 与客户端能力协作。

### 客户端

客户端是基于 Tauri 2 的跨平台原生应用，负责应用生命周期、窗口与系统能力，并承载视图端。Rust command 与 capability 以最小权限方式向视图端暴露原生能力；目标平台包括 Windows、macOS、Linux、iOS 和 Android。

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

## 关系图

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

### Rust

- 原生层与服务层使用 Rust 2021，最低支持 Rust 1.88；跨层输入和输出必须是可序列化、版本明确的结构。
- Tauri IPC command 必须同时使用 `#[tauri::command]` 与 `#[specta::specta]`，由 `tauri-specta` 生成类型安全 bindings。
- HTTP DTO 以 `crates/contracts/` 为 Rust 单一来源，使用 serde 解码，并派生 `ts_rs::TS` 导出 TypeScript bindings。
- Rust 领域规则放在 `crates/domain/`，不得依赖 Tauri、HTTP 或数据库；Axum 路由、CORS 与服务逻辑按功能模块组织。

### TypeScript

- 前端使用 TypeScript、Vue 3 Composition API、Pinia、UnoCSS 与 shadcn-vue；新增代码禁止 `any`。
- 视图组件保持平台无关，不得直接依赖平台专有 API；原生调用仅经 `@tauri-apps/api` 和 `src/bindings/`。
- 视图状态集中在 Pinia；持久化、同步等后续能力通过 repository 接口注入，避免组件耦合实现细节。
- 禁止在前端手写与 Rust DTO 同名的契约；新增或修改行为应补充相应验证。

### HTML

- 页面结构使用语义化 HTML；表单控件必须关联可见标签或可访问名称。
- 图标按钮、弹窗与状态提示必须提供可访问文本、键盘操作和恰当的 ARIA 属性。
- 不使用内联事件处理器或内联样式承载业务逻辑。

### Vue

- 使用 Vue 3 Composition API 与 `<script setup lang="ts">`；组件按单一职责拆分。
- 页面负责组合布局与路由上下文，可复用交互沉淀为组件；状态与副作用集中在 Pinia store 或组合式函数。
- 模板中的列表必须提供稳定 `key`；异步视图必须显式处理加载、空数据和错误状态。

### CSS

- 使用 UnoCSS 与项目设计令牌实现样式；优先复用 shadcn-vue 组件和已有样式约定。
- 新增样式必须同时考虑桌面端与移动端，避免固定宽高导致文字溢出或触控区域不足。
- 不使用 `!important` 覆盖基础组件，除非现有样式机制无法表达且在变更中说明原因。

### JavaScript

- 应用源码使用 TypeScript；JavaScript 仅用于无法使用 TypeScript 的构建配置、自动化脚本或第三方工具入口。
- JavaScript 脚本必须明确输入、输出与错误处理，不将业务逻辑分散到临时脚本中。

### 类型

- 新增代码禁止 `any`；外部输入优先使用 `unknown`，并在边界处完成类型收窄与校验。
- 组件 props、emits、store 状态、函数参数与返回值应显式定义类型。
- 跨端和 HTTP 契约必须使用 Rust 生成的 bindings；类型变更必须同步更新生成结果与调用方。

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
cargo test -p todo-server
cargo check --workspace
```

移动端首次配置在对应工具链机器的仓库根目录执行 `pnpm run tauri android init` 或 `pnpm run tauri ios init`。Android 需要 Android SDK；iOS 初始化和构建仅可在 macOS/Xcode 环境执行。

## 项目管理

### 开始工作前

1. 阅读 `.agents/workflows/feature-delivery.md`，按任务类型选择标准或轻量流程。
2. 阅读 `.agents/agents/<角色>.md`，遵守该角色职责与写入范围。
3. 对具体功能，在 `.agents/docs/tasks/<TASK-ID>/` 创建记录；从 `.agents/docs/tasks/_template/` 复制模板。

### 工作流使用

#### 1. 选择流程

- 新功能、跨端交互、数据/API/权限变更：使用标准流程。
- 仅文本、颜色、间距、图标或已有组件的展示调整：使用轻量流程。

#### 2. 创建任务上下文

在 `.agents/docs/tasks/<TASK-ID>/` 复制 `task.json` 与 `handoff.md`，填写任务名称、验收标准、当前负责人和所选流程。每完成一个阶段，更新状态并追加一条交接记录。

#### 3. 执行角色

- 标准流程：产品 → UX/UI → 架构 → 开发 → QA → Code Review → 发布准备。
- 轻量流程：需求确认 → 开发 → 轻量验证 → Code Review → 完成。

角色说明位于 `.agents/agents/`。只有在 `task.json` 的 `skipped_roles` 中写明原因、影响评估和替代证据后，才可跳过非必要角色。

#### 4. 交接与完成

下一角色先读取 `handoff.md` 的最新记录及其列出的文件。需求、权限、用户数据、付费或生产发布存在重大不确定性时停止并请求人工确认；符合完成定义后才关闭任务。

### 通用规则

1. 只修改当前待办应用任务相关文件；不得把流程文档当作产品功能实现。
2. 每项需求必须有可验证验收标准、负责人和状态。
3. 开发变更使用独立 Git 分支或 worktree；QA 与审查角色默认只读。
4. 不得使用生产数据、生产密钥或不可逆操作验证需求。
5. 每阶段更新 `.agents/docs/tasks/<TASK-ID>/task.json`；跨角色时在 `handoff.md` 追加简短交接记录。
6. 影响产品范围、账号/权限、用户数据、付费或生产发布的未知事项必须人工确认。

### 完成定义

功能只有在验收标准有验证证据、相关测试通过、审查通过，且发布条件（如适用）准备完成后才能标记完成。
