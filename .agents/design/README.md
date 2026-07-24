# 视觉设计稿产物目录

本目录存放「视觉设计稿」母任务（`.agents/tasks/视觉设计稿/`）产出的设计图，作为后续代码复现的像素参照。

- 方式：以 DESIGN.md 设计令牌为唯一事实源，用 HTML/Artifact 生成高保真视觉稿，渲染后导出 PNG。（用户 2026-07-25 在 View/starter 席位下选 HTML/Artifact 路线替代 Figma 建稿；Figma 账号 `1471570248@qq.com` 仅可读不可建。）
- 结构：每个屏幕/界面一个子目录 `<两位序号>-<短名>/`，内含 `index.html`（可复现的设计源）、`light.png`、`dark.png`（导出的浅色/暗色设计图，含移动端屏时另出 `mobile-*.png`）。
- 用途：设计稿经评审对齐 DESIGN.md 后，作为对应功能模块「复现」阶段的视觉参照；复现时按图实现，不再另写视觉规格。

> 本目录是设计产物，不属产品代码（`src/`、`src-tauri/`、`crates/`）；`pnpm run check`/`cargo test` 不适用于此，验证以 DESIGN.md 令牌一致性 + 可访问性（对比度/触控/浅暗成对）+ 屏幕状态覆盖为准。
