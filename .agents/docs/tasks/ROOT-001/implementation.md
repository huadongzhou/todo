# ROOT-001 实现记录

## 变更

- 将 `apps/desktop` 中的前端源码、Tauri 工程、npm manifest、npm lockfile 和构建配置移动到仓库根目录。
- 不迁移 `node_modules` 与 `dist`；它们为可再生的本地生成物。
- 根 `Cargo.toml` 的 Tauri member 改为 `src-tauri`。
- `src-tauri/Cargo.toml` 的 `todo-contracts` path dependency 改为 `../crates/contracts`。
- 将根 npm package 名称调整为 `cross-platform-todo`，并重新执行根 npm 安装以同步 lockfile。
- 更新工程规范中的目录和命令位置。

## 验证

- `npm install`：通过。
- `npm run check`：通过，38 个文件格式正确，22 个文件无警告、lint 或类型错误。
- `npm run build`：通过。
- `cargo metadata --no-deps --format-version 1`：通过；workspace member 为 `src-tauri`、contracts、domain、server。
- `cargo fmt --all -- --check`：通过。
- `git diff --check`：通过。

## 已知限制

完整 Cargo 编译、Rust 测试和 bindings 导出仍受既有 USTC Cargo registry 网络故障影响；本次迁移未引入新的 Rust registry 依赖。
