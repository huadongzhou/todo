# 全局快捷键与快速新增设计规格

## 交互

- 默认快捷键：`CommandOrControl+Shift+A`。
- 按下后：
  - 若主界面被隐藏（托盘），先显示；
  - 关闭设置面板与展开的日期区；
  - 聚焦到待办输入框。
- 输入框下方显示一条轻量快捷键提示（kbd 标签）。

## 组件与状态

- `activeShortcut`：当前已注册的快捷键字符串，用于 UI 提示。
- 启动时注册、退出时注销。

## 权限与降级

- 桌面端通过 `tauri-plugin-global-shortcut` 注册；Capability 授予 `global-shortcut:default`。
- 浏览器 / 移动端：`isTauri()` 以外为空实现。
- 注册失败（例如被其他应用占用）写 `warn` 诊断日志，不阻塞启动。
