# AXUM-001 产品需求

## 目标

将服务层从 Bun/Elysia 迁移为 Axum，并让服务端、Tauri 原生层和桌面前端围绕 Rust DTO 工作，消除业务 DTO 的人工双写。

## 非目标

- 不引入账户、数据库、持久化、生产部署或同步 UI。
- 不改变现有待办页面的交互和视觉设计。

## 用户故事

作为跨端待办应用的维护者，我希望 HTTP API 和 Tauri 共享同一 Rust 契约源，从而在修改同步 DTO 时不必手动维护 Rust 与 TypeScript 两份业务定义。

## 验收标准

以 `task.json` 的 acceptance_criteria 为准；错误请求应得到可区分的 HTTP 400 响应，服务内部失败不得暴露内部细节。
