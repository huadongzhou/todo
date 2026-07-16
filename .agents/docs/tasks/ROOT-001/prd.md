# ROOT-001 产品需求

## 目标

将唯一的 Vue/Tauri 桌面端项目提升至仓库根目录，去除已不需要的 `apps/desktop` 层级，同时保持 Cargo workspace 的共享 Rust crates 结构。

## 非目标

- 不修改 Axum API、领域规则、共享 DTO 或 UI 功能。
- 不移动 node_modules、dist 等生成物。

## 验收标准

以 `task.json` 的 acceptance_criteria 为准。
