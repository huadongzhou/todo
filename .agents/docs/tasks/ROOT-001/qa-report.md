# ROOT-001 QA 报告

## 结果：通过

| 验收标准 | 验证方式 | 结果 | 证据 |
| --- | --- | --- | --- |
| 桌面端位于根目录 | 路径清单核对 | 通过 | 根目录含 `src/`、`src-tauri/`、`package.json`、`package-lock.json`；`apps/` 无产品源文件 |
| Cargo workspace member 正确 | `cargo metadata --no-deps --format-version 1` | 通过 | 返回 `src-tauri`、contracts、domain、server 四个 member |
| Tauri contracts 路径与 bindings 位置正确 | Cargo 元数据与前端类型检查 | 通过 | `todo-contracts` 解析至 `crates/contracts`；Vite 类型检查通过 |
| 根 npm 命令可用 | `npm install`、`npm run check`、`npm run build` | 通过 | 三项命令均成功 |

## 回归范围

前端格式、lint、TypeScript 类型检查和生产构建均通过。Rust 源码格式检查通过；完整 Cargo 编译仍由已知 registry 网络问题阻塞，未归因于本迁移。
