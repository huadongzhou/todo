# QA 报告：标准 Monorepo 目录迁移

| 验收项 | 验证方式 | 结果 | 证据 |
| --- | --- | --- | --- |
| Apps/Packages 结构 | 检查目录和 workspace package | 通过 | `apps/desktop`、`apps/api`、`packages/*` |
| 全 workspace 检查 | `npm run check` | 通过 | desktop、contracts、domain、API 均成功 |
| API 回归 | `npm run api:test` | 通过 | 3 个 Bun 测试通过 |
| Desktop 构建 | `npm run build` | 通过 | Vite 生产构建成功 |
| Rust bindings 生成 | `npm run types:generate` | 环境阻断 | Cargo USTC 镜像不可连接；离线缓存缺少 serde |
| Rust 项目新路径解析 | `cargo metadata --locked --offline --no-deps --manifest-path apps/desktop/src-tauri/Cargo.toml` | 通过 | workspace root、入口和 target 指向 `apps/desktop/src-tauri` |

结论：JavaScript/TypeScript 迁移验证通过；Rust 完整验证需在可访问 Cargo registry 或具备完整缓存的环境复验。
