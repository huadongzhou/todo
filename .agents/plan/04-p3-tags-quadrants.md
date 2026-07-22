# P3 · 标签与四象限

前置：P1（数据底座）、P0（视图页承载）。打通「组织」环。

## 范围（TODO 映射）

- 模块 3：标签管理、贴标签、筛选、标签统计
- 模块 4：象限属性、四象限总览、快速设置、默认推断；（拖拽移动为设想项，默认后置）

## 技术方案

- **契约扩展**：`Todo` 增 `tags: Vec<String>`（默认空）与 `quadrant`（可选枚举 `q1..q4`，重要×紧急）；`TodoPatch` 对应扩展；标签本体走独立轻量表而非仅字符串散落——`tags(name TEXT PK, color TEXT, created_at)`，任务关联存 `todo_tags(todo_id, tag_name)`（SQLite schema v3）。同步侧：标签操作并入现有 op-based 通道（upsert patch 携带 tags/quadrant 字段）；标签表本身暂不跨端同步（随任务字段自然重建），在文件中显式记录此取舍。
- **标签管理 UI**：设置面板或视图页内标签管理区——创建/重命名/删除；颜色沿 DESIGN.md 语义色公式（浅 100 底 700 字 / 暗 900/40 底 300 字），提供固定色板（sky/emerald/amber/red/violet/slate 六色）而非自由取色。
- **筛选**：任务页顶部标签筛选条（多选交集）；筛选态持久于会话，不入库。
- **四象限视图**：视图页新增 tab，2×2 网格（`CalendarMonth` 同级组件 `QuadrantBoard.vue`）；未设象限任务归入「未分类」列；列表行与编辑表单可快速设置象限。
- **默认推断**（产品定义）：仅在用户未显式设置时，按截止日临近度预填紧急维度（≤1 天为紧急）；推断值以视图计算呈现，不写库——显式设置才落库，避免同步抖动。

## 任务拆解

- [ ] contracts：tags/quadrant 字段与校验 + bindings 重生成
- [ ] SQLite schema v3：todos 列扩展 + tags/todo_tags 表 + storage command 面扩展
- [ ] 标签管理 UI（CRUD + 六色板）
- [ ] 创建/编辑表单接入标签与象限；列表行标签展示与快速设象限
- [ ] 任务页标签筛选条
- [ ] `QuadrantBoard.vue` 四象限视图 tab + 未分类列 + 紧急推断展示
- [ ] 标签维度统计并入 P0 统计区
- [ ] TODO 3 / 4 回写；README 能力清单更新

## 验收标准

1. 标签可增删改、上色，任务可贴多标签，按标签筛选正确。
2. 四象限视图正确归类，快速设置即时生效并同步；未设象限任务按截止日预填紧急展示但不落库。
3. 桌面同步链路携带 tags/quadrant 双向无损。
4. 浅暗两模式下标签色与象限视图对比达标。

## 验证方式

contracts/storage `cargo test`；`tauri:dev` 双端窗口（主窗 + 今日卡片）走查同步；`pnpm run check`。

## 风险与依赖

- 标签重命名/删除的关联更新（todo_tags 级联）需事务处理；删除标签给确认弹窗（破坏性操作规范）。
- 与 P2 可并行，但同时改 contracts 时先合并字段设计，一次生成 bindings，避免两次迁移互踩。
