# 端侧SQLite与repository

- 级别：直达
- 状态：开发

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
- 自验结果（命令与结论）：

## 评审记录
