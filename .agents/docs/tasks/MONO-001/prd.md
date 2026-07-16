# PRD：标准 Monorepo 目录迁移

## 目标

将现有桌面客户端迁移到 `apps/desktop`，与 `apps/api` 和 `packages/*` 构成职责清晰的标准 Monorepo。

## 非目标

- 不改变待办界面、领域行为、HTTP 契约或 Tauri 原生能力。
- 不迁移至 Turborepo、Nx 或更换 npm 包管理器。

## 验收

1. 客户端、服务端和共享包目录边界符合 Apps/Packages 模式。
2. 根目录命令可检查、构建客户端并运行 API 测试。
3. 生成的 Tauri bindings 和现有客户端构建不回归。
