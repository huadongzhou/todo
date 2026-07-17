# 实现记录

## 变更范围

- 新增 `@tauri-apps/plugin-store` 与 Rust `tauri-plugin-store` 2.4.3，注册插件并只向 `main` 窗口授予 `store:default`。
- 新增 `settings` Pinia store、偏好持久化适配器、主题适配器和诊断日志入口。
- 启动时恢复 `appearance.theme`；Tauri 环境写入 `settings.json`，浏览器开发模式回退到 `localStorage`。
- 使用 Tauri Core 当前窗口 API 同步浅色、深色或系统主题；系统主题变更时同步 Web 视图。
- 主界面新增可访问设置面板及三种主题选项。

## 验证

- `pnpm run check`：通过。
- `pnpm run build`：通过，生产前端构建成功。
- `cargo check --manifest-path src-tauri/Cargo.toml`：通过。
- `cargo test --manifest-path src-tauri/Cargo.toml --no-run`：通过。
- `cargo test -p todo-contracts`：通过，1 个测试通过。

## 已知限制

完整 Tauri 单元测试启动时，Windows 进程在进入测试前以 `STATUS_ENTRYPOINT_NOT_FOUND` 退出；编译与测试二进制构建均成功。该问题需要在具备正确系统运行库的桌面环境中复验主题持久化与原生窗口同步。
