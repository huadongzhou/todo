# PNPM-001 QA 报告

## 结果：通过

| 验收标准 | 验证方式 | 结果 | 证据 |
| --- | --- | --- | --- |
| pnpm 固定版本且不调用 npm | 审阅 manifest、Tauri 配置和路径扫描 | 通过 | `packageManager` 为 pnpm 11.5.1；before commands 与 build script 均使用 pnpm |
| lockfile 单一 | 文件清单核对 | 通过 | 仅存在 `pnpm-lock.yaml` |
| pnpm 命令可用 | `pnpm install`、`pnpm run check`、`pnpm run build` | 通过 | 三项命令均成功 |
| 工程规范使用 pnpm | 审阅 `AGENTS.md` | 通过 | 安装、开发、构建、类型导出与移动端命令已更新 |

## 安全与异常处理

- pnpm 11 的依赖构建审批默认阻止未知 postinstall；仅 `vue-demi` 被显式批准。
- pnpm 默认的发布冷却期仍对其他依赖生效；仅对已存在的 Vue 3.5.40 依赖族配置例外，避免迁移期锁定版本被拒绝。
