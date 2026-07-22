# P2 · 周期任务与提醒升级

前置：P1（SQLite 底座）。打通「捕捉」环的周期任务与「提醒」环的可靠性。

## 范围（TODO 映射）

- 模块 1.2：重复规则、实例生成、周期操作、周期标识
- 模块 2：后台调度、周期提醒；（设想项「通知快捷操作」列为可选，默认不做）

## 技术方案

- **重复规则模型**（契约扩展，`crates/contracts/`）：`Todo` 增可选字段 `recurrence`——`{ freq: "daily"|"weekly"|"monthly", interval: u16, weekday?/monthday? }` 预设制，不引入完整 RRULE；serde `default` + `#[ts(optional)]` 保持前后兼容，重跑 bindings。
- **下一次到期计算**：纯函数落 `crates/domain/`（`next_occurrence(rule, from_date)`），处理月末溢出（31 日 → 短月取月末）；同文件单测覆盖边界。
- **实例生成**：客户端本地驱动——完成或逾期确认时由前端调用 domain 规则（经 command 暴露）生成下一实例（新 id、继承标题/标签/象限/提醒相对时刻），落 SQLite 并记同步操作；周期母任务信息随实例携带（`recurrence` 字段复制），不引入独立"模板任务"表。
- **周期操作**：完成本次（生成下次）、跳过本次（同生成下次但不计完成）、结束重复（清除 recurrence）。「编辑仅本次/后续全部」为设想项，默认后置。
- **后台提醒调度**：Rust 侧用 Tauri notification 的系统级调度能力替换前端 `setTimeout`——应用启动/任务变更时把未来提醒注册给系统；应用未运行时由系统触发。桌面三平台行为差异在本阶段实测并记录。
- **周期提醒**：实例生成时随新实例注册下一次提醒，天然随周期滚动。

## 任务拆解

- [ ] contracts：`recurrence` 字段 + 校验（interval ≥ 1 等）+ bindings 重生成
- [ ] SQLite schema v2：todos 增 `recurrence` 列（可空 JSON），user_version 迁移
- [ ] domain：`next_occurrence` 纯函数 + 边界单测
- [ ] 实例生成链路：完成/跳过/结束重复三操作（storage + store + 同步操作记录）
- [ ] 创建/编辑表单增加重复规则选择（预设：每日/每周/每月/自定义间隔）
- [ ] 列表与详情的周期标识 + 下次到期展示
- [ ] 后台提醒调度替换 `setTimeout` 方案；权限与降级路径回归
- [ ] TODO 1.2 / 2 回写；README 能力清单更新

## 验收标准

1. 周期任务完成后自动出现下一实例，截止日/提醒按规则顺延，跳过与结束重复行为正确。
2. 月末边界正确（如每月 31 日在 2 月落到月末）。
3. 应用完全退出后到点提醒仍触发（桌面至少 Windows 验证，macOS/Linux 记录实测结论）。
4. 旧数据（无 recurrence 字段）不受影响，同步双向兼容。

## 验证方式

domain 与 storage `cargo test`；调整系统时间/短间隔规则做 `tauri:dev` 手测；`pnpm run check`。

## 风险与依赖

- 系统级通知调度在 Linux 各桌面环境能力不一：不可用时降级回运行期提醒并在 README 能力矩阵如实标注。
- 同步端兼容：服务端对未知字段 `deny_unknown_fields`——contracts 同仓同步升级即可，无独立部署错峰问题；若服务端已先行部署需先升级服务端。
