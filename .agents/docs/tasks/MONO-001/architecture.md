# 架构：标准 Apps/Packages Monorepo

```text
apps/desktop  Vue + Tauri 可运行客户端
apps/api      Bun + Elysia 可运行服务
packages/*    可被多个 app 引用的库
```

根 `package.json` 仅保留 npm workspaces 与编排脚本。客户端专属依赖、Vite 配置、Tauri 配置和 Cargo 项目一起迁移到 `apps/desktop`，使 Tauri CLI 在 workspace 内解析 `src-tauri`。`src-tauri/src/lib.rs` 到 `src/bindings` 的相对路径在移动后仍为 `../src/bindings`，无需改写原生契约。
