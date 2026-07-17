# QA 报告（待运行时复验）

## 验收结果

| 验收项 | 结果 | 证据 |
| --- | --- | --- |
| 三种主题选项可通过键盘和鼠标选择 | 静态通过 | 原生 `radio`、`label`、可访问设置按钮；`pnpm run check` 通过 |
| 浏览器开发模式安全降级 | 静态通过 | `isTauri()` 分支与 `localStorage` 回退；生产前端构建通过 |
| 主题保存和恢复 | 待桌面运行时复验 | 官方 Store 已注册、Capability 已授权、类型检查通过 |
| 原生窗口主题同步 | 待桌面运行时复验 | 当前窗口 `setTheme` 已编译通过 |
| 日志转发到官方插件 | 待桌面运行时复验 | 官方 Log 已注册；前端日志入口在非 Tauri 模式安全降级 |

## 自动化验证

- `pnpm run check`：通过。
- `pnpm run build`：通过。
- `cargo check --manifest-path src-tauri/Cargo.toml`：通过。
- `cargo test --manifest-path src-tauri/Cargo.toml --no-run`：通过。
- `cargo test -p todo-contracts`：通过。

## 阻塞与风险

- 本机执行完整 `cargo test --manifest-path src-tauri/Cargo.toml export_type_bindings` 时，测试二进制在启动前报 `STATUS_ENTRYPOINT_NOT_FOUND`。需要在修复 Windows 运行库环境后完成真实 Tauri QA，当前不进入 Code Review 或完成状态。
