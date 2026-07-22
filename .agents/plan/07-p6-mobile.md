# P6 · 移动端

前置：P1（SQLite 同构底座）；账号绑定同步依赖 P4。产品闭环的最后一环。

## 范围（TODO 映射）

- 模块 9：构建初始化、移动适配、三段导航框架、「我的」页、主屏小组件、deep link、卡片刷新
- 模块 2：移动端本地通知
- 模块 6.2：移动端同步接入

## 技术方案

### 应用本体

- **构建**：`pnpm run tauri android init`（Android SDK 机器）先行，iOS 需 macOS/Xcode 环境后补；CI 暂不建，本地构建为准。
- **主界面框架**：README 视图结构——底部导航「任务 / 视图 / 我的」三段；桌面/移动同一 Vue 代码库，以视口宽度切换导航形态（桌面两段顶部切换，移动三段底部导航），不拆分代码库。
- **「我的」页**：账号登录态（入口与状态）、设置（主题/提醒偏好等，复用设置面板内容重排为页）、关于（版本信息经现有 `get_runtime_info`）。
- **适配**：触控目标 ≥44px、safe-area（`env(safe-area-inset-*)`）、`min-h-dvh`、手势回退——按 DESIGN.md 间距与布局基准逐项过；今日卡片/月历表窗口逻辑在移动端禁用（`isCard` 分流保持桌面独占）。
- **通知**：Tauri notification 插件移动端能力实测；周期/后台调度行为与桌面差异记录进本文件。
- **同步**：sync-transport 在移动 Tauri 运行时天然启用（`isTauri()` 为真），服务器地址设置移入「我的」；P4 完成则登录绑定，未完成期间按 deviceId 匿名同步。

### 主屏小组件（最大不确定项，先调研后实施）

- Tauri 2 无 widget 框架：iOS WidgetKit（Swift）与 Android AppWidget/Glance（Kotlin）需在 `tauri ios/android init` 生成的原生工程内编写。
- 数据通道：小组件直读共享 SQLite——iOS 经 App Group 容器放库文件，Android 同应用沙箱直读；只读展示（当月统计 + 当日/周/月任务），写操作一律跳转应用。
- 点击 deep link：`todo://` scheme（tauri deep-link 插件）路由到任务/视图页。
- 调研任务先行：两平台各出一页结论（可行性、刷新机制 Timeline/WorkManager、库文件路径共享方案），写回本文件后再排实施。

## 任务拆解

- [ ] Android 构建初始化与真机跑通（存量功能回归：任务 CRUD、主题、SQLite、通知）
- [ ] 响应式导航框架：移动三段底部导航 + 「我的」页
- [ ] 移动适配过检（触控/safe-area/dvh/横屏）
- [ ] 移动通知实测与差异记录
- [ ] 移动同步接入与服务器地址设置迁移
- [ ] 小组件调研（Android 先行，iOS 待 macOS 环境）→ 结论回写
- [ ] Android AppWidget 实施（统计 + 速览 + deep link + 刷新）
- [ ] iOS 构建、适配回归与 WidgetKit 实施（依赖 macOS/Xcode）
- [ ] TODO 9 回写；README 能力清单与矩阵更新

## 验收标准

1. Android 真机全功能可用：任务管理、周期、标签象限、统计月历、通知、同步。
2. 三段底部导航符合视图结构定义，触控与 safe-area 达 DESIGN.md 基准。
3. Android 主屏小组件展示当月统计与任务速览，点击直达对应视图，数据随任务变更刷新（允许系统级刷新延迟）。
4. iOS 侧同标准（环境就绪后）。

## 验证方式

真机走查为主；Rust 层沿用 `cargo test`；`pnpm run check`。每平台差异结论写回本文件。

## 风险与依赖

- 小组件原生开发是全计划技术风险最高点：调研若判定成本过高，降级方案为「通知栏常驻摘要」（Android）先行，产品定义相应标注分期。
- iOS 全链路依赖 macOS/Xcode 机器，可能整体后置；计划内 iOS 任务不阻塞 Android 交付。
- rusqlite bundled 在移动目标的交叉编译需在构建初始化时首先验证（P1 已选定方案的前提假设）。
