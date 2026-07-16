# ROOT-001 架构方案

```text
src/                 Vue 前端
src-tauri/           Tauri 原生层
package.json         唯一 npm manifest
package-lock.json    唯一 npm lockfile
Cargo.toml           Rust workspace
crates/              共享 DTO、领域规则与 Axum 服务
```

`src-tauri` 成为 Cargo workspace member，并以 `../crates/contracts` 引用共享 DTO。Tauri 的 bindings 输出仍采用从 `src-tauri` 到 `src/bindings` 的相对路径 `../src/bindings`，不需要变更。`node_modules` 和 `dist` 是可再生文件，不进入迁移范围。

## 验证

根目录执行 npm check/build；`cargo metadata --no-deps` 验证 Cargo member 与 path dependency，不触发已知的 registry 网络问题。
