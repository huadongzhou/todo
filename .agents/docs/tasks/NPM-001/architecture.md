# NPM-001 架构方案

## 目标结构

```text
Cargo.toml
crates/
  contracts/
  domain/
  server/
apps/desktop/
  package.json
  package-lock.json
  src/
  src-tauri/
.agents/docs/tasks/
```

根目录不再提供 Node package 或 npm workspace。前端依赖锁定、安装、Vite 和 Tauri CLI 均在 `apps/desktop` 执行；`cargo` 命令仍在仓库根目录执行。历史任务记录迁入 `.agents/docs/tasks`，以符合工程规范。

## 风险与验证

锁文件移动后必须在 `apps/desktop` 重新执行 npm 安装和前端检查/构建。Cargo workspace 使用 `cargo metadata --no-deps` 验证，以避免受已知 registry 网络故障影响。
