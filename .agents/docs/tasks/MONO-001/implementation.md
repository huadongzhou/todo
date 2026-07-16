# 实现记录：标准 Monorepo 目录迁移

## 变更范围

- 将既有 Vue、Vite、UnoCSS、TypeScript、shadcn-vue 配置及 `src-tauri` 整体移动到 `apps/desktop`。
- 创建 `@todo/desktop` workspace 并承接所有客户端依赖和 Tauri 脚本。
- 根目录只保留 npm workspace、统一命令和 `tsconfig.base.json`。
- 更新 README、AGENTS.md、忽略规则和 API 任务路径说明。

## 验证结果

- `npm install`：通过，workspace lockfile 已更新。
- `npm run check`：通过，desktop、contracts、domain 与 API 均通过检查。
- `npm run api:test`：通过，3 个 Bun 测试通过。
- `npm run build`：通过，`apps/desktop` Vite 生产构建完成。
- `cargo metadata --locked --offline --no-deps --manifest-path apps/desktop/src-tauri/Cargo.toml`：通过，Rust manifest、入口和 target 路径均解析为新目录。

## 已知限制

`npm run types:generate` 需要下载 Cargo 依赖，但当前用户级 Cargo 配置把 crates.io 指向不可连接的 USTC 镜像；离线缓存也缺少 serde。因此完整 Rust 编译与 bindings 生成尚未在本机完成，不是目录迁移的 TypeScript/Vite 问题。
