# TSC-001 实现记录

- 根 `tsconfig.json` 继承 Vue DOM 配置，并合并前端源码与 Vite/UnoCSS 配置的编译选项和 include 范围。
- 删除 `tsconfig.app.json` 与 `tsconfig.node.json`。
- 保留 `@/*` 到 `src/*` 的路径别名、严格检查、Node 配置文件的 ES2023/module 选项。
