# 修复tauri测试manifest

- 级别：直达
- 状态：完成

## 需求（产品）

- 目标与场景：`cargo test --workspace` 在 Windows 上无法通过——`cross-platform-todo` 的 lib 测试二进制以 `STATUS_ENTRYPOINT_NOT_FOUND`（0xC0000139）启动失败，导致 AGENTS.md「应用命令」规定的 Rust 验证标准在任何任务上都无法执行，也导致 `src/bindings/models/` 的 ts-rs 导出（`export_type_bindings` 测试）跑不起来。修复后全仓验证链路方可使用。

  已定位根因（编排者诊断，供参考，实现方案由开发判断）：
  - 测试二进制 `target/debug/deps/cross_platform_todo_lib-*.exe` 的导入表包含 `comctl32.dll` 的 `TaskDialogIndirect`、`SetWindowSubclass`、`RemoveWindowSubclass`、`DefSubclassProc`（经 PE 导入表解析确认）。
  - 这四个导出仅存在于并排程序集 Common-Controls **6.0.0.0**；没有应用程序清单时加载器绑定到 `System32\comctl32.dll`（v5.82），找不到入口点即报 0xC0000139。
  - `src-tauri/build.rs` 只调用 `tauri_build::build()`，它嵌入的清单只作用于应用 bin 目标，测试 harness 可执行文件没有清单。
  - 参考修复方向：在 `build.rs` 中为测试目标追加链接参数嵌入清单（Cargo 提供 `cargo:rustc-link-arg-tests=…`，MSVC 链接器接受 `/MANIFEST:EMBED` 与 `/MANIFESTINPUT:<file>`），清单内容声明 `Microsoft.Windows.Common-Controls` version `6.0.0.0`。仅在 `windows` + `msvc` 下生效，其他平台不受影响。

- 范围（做 / 不做）：
  - 做：让 `cargo test --workspace` 在本机 Windows 上完整通过（含 `cross-platform-todo` 的 3 个测试）；改动限于 `src-tauri/build.rs` 及其新增的清单资源文件（如 `src-tauri/*.manifest`）与必要的 `src-tauri/Cargo.toml` 构建依赖。
  - 不做：任何业务逻辑、命令、UI 改动；不改动测试用例本身的断言；不通过删除/忽略测试或改用 `--exclude` 绕过问题；不改 CI 配置（仓库当前无 CI）。

- 验收标准（逐条可检查）：
  - [ ] 仓库根目录执行 `cargo test --workspace` 全部通过，`cross-platform-todo` 的 lib 测试实际运行且不再报 0xC0000139。
  - [ ] `cargo check --workspace` 通过。
  - [ ] `pnpm run check` 通过。
  - [ ] 改动为跨平台安全：非 Windows 或非 MSVC 目标下 `build.rs` 新增逻辑不生效，不影响 `pnpm run tauri:build` 的既有清单行为。
  - [ ] 未新增 `thiserror`/`anyhow` 依赖（AGENTS.md Rust 规范）。

## 规格（UI/UX）

## 实现记录（开发）

- 改动文件：
  - `src-tauri/build.rs`：新增 `declare_common_controls_for_test_binaries()`，仅在 `windows` + `msvc` 下发出三条链接参数——`/MANIFEST`、`/MANIFESTDEPENDENCY:...Common-Controls 6.0.0.0`（对全部链接目标生效）与 `cargo:rustc-link-arg-bins=/MANIFEST:NO`（对 bin 目标关闭外部清单）。
  - `crates/contracts/src/lib.rs`：`Todo::due_date` 与 `Todo::reminder_at` 的 `#[ts(optional)]` 改为 `#[ts(optional = nullable)]`（说明见下）。
  - `vite.config.ts`：`fmt.ignorePatterns` 增加 `src/bindings/**`（说明见下）。
  - `src/bindings/commands.ts`、`src/bindings/models/*.ts`（10 个文件）：`export_type_bindings` 测试重新生成的产物，按 AGENTS.md 一并纳入改动。

- 方案说明（与「需求」节参考方向的差异）：
  - 参考方向的 `cargo:rustc-link-arg-tests` 不可用：Cargo 该指令只作用于 `tests/` 集成测试目标，本包没有该目标，直接报 `invalid instruction ... does not have a test target`；失败的是 `--lib` 单测二进制，Cargo 没有对应的作用域指令。
  - 改用作用于全部目标的 `cargo:rustc-link-arg` 时，若用 `/MANIFEST:EMBED` + `/MANIFESTINPUT:`，bin 目标会与 tauri-build 已链接的 `resource.lib` 里的清单资源冲突（`CVT1100: duplicate resource. type:MANIFEST, name:1` → `LNK1123`）。
  - 因此改为让链接器把 Common-Controls 依赖写进**外部** side-by-side 清单文件（`<exe>.manifest`）：无嵌入清单的 lib 单测二进制由加载器读取该文件，问题解除；bin 目标另发 `/MANIFEST:NO`，既保留 tauri 嵌入的清单、也不产生任何多余文件，打包行为零变化。
  - 已实测确认：`target/debug/cross-platform-todo.exe` 仍含嵌入的 `Microsoft.Windows.Common-Controls` 清单，且 `target/debug/` 下不生成任何 `.manifest` 文件；外部清单只出现在 `target/debug/deps/` 的测试二进制与 cdylib 旁。

- 超出「范围」声明的两处改动（必要性说明，请评审/编排者裁决）：
  - `crates/contracts/src/lib.rs`：`cargo test` 恢复后 `export_type_bindings` 首次真正重跑，暴露出既有生成产物与 Rust 源已漂移——`Todo` 的 `due_date`/`reminder_at` 标的是 `#[ts(optional)]`（生成 `dueDate?: string`），而仓库里提交的 `Todo.ts` 是 `dueDate?: string | null`，前端 `src/stores/todos.ts` 也确实写 `null`。若原样提交新产物，`pnpm run check` 报 5 处 TS2322。同文件的 `TodoPatch` 对同名字段已用 `#[ts(optional = nullable)]`，AGENTS.md 也规定「允许写入 null 的用 `#[ts(optional = nullable)]`」，故按此修正两个属性——这样重新生成的 `Todo.ts` 类型体与仓库既有内容完全一致，前端零改动。
  - `vite.config.ts`：`cargo test` 每次都会用生成器原始格式覆写 `src/bindings/**`，而 `vp check` 的格式校验要求这些文件是格式化后的样子，二者互斥——先前之所以没暴露，正是因为 lib 单测跑不起来。把 `src/bindings/**` 加入 `fmt.ignorePatterns` 后，`cargo test` 与 `pnpm run check` 可任意顺序反复执行且都通过（lint 与类型检查仍覆盖该目录）。不做此项则「`cargo test --workspace` 通过」与「`pnpm run check` 通过」无法在同一次运行中同时成立。

- 自验结果（命令与结论）：
  - `cargo test --workspace`：通过。`cross-platform-todo` 的 lib 单测实际运行（`tests::export_type_bindings ... ok`），不再出现 0xC0000139；全仓合计 6 个测试通过（cross-platform-todo 1、todo-contracts 1、todo-domain 1、todo-server 3）。
  - `cargo check --workspace`：通过，无警告。
  - `pnpm run check`：通过（39 个文件格式校验通过，34 个文件无 lint/类型错误）；在紧接 `cargo test --workspace` 之后重跑仍通过。
  - `cargo build -p cross-platform-todo`：通过；产物 `target/debug/cross-platform-todo.exe` 内仍含 `Microsoft.Windows.Common-Controls` 嵌入清单，`target/debug/` 无外部 `.manifest`。
  - 未跑 `pnpm run tauri:build`（耗时长且需完整打包链）；清单相关逻辑与 profile 无关，上一条 debug bin 的核对即为该行为的等价证据。

- 验收标准逐条自查：
  - [x] `cargo test --workspace` 全部通过，lib 测试实际运行且不再报 0xC0000139。
  - [x] `cargo check --workspace` 通过。
  - [x] `pnpm run check` 通过。
  - [x] 跨平台安全：`build.rs` 新增逻辑由 `CARGO_CFG_TARGET_OS == "windows"` 且 `CARGO_CFG_TARGET_ENV == "msvc"` 双重守卫，其他目标直接 return；bin 目标的嵌入清单与打包产物不变。
  - [x] 未新增 `thiserror`/`anyhow`（`src-tauri/Cargo.toml` 未改动，未新增任何依赖）。

- 遗留问题：
  - TODO.md 未改动：本次为构建脚本修复，不对应功能点（按 roles/dev.md 步骤 7）。
  - `src/bindings/**` 现在提交的是生成器原始格式（单行类型、tab 缩进、ts-rs 默认头注释），与此前手工格式化过的版本在排版与头注释上不同，类型内容一致。
  - 外部 side-by-side 清单要求 `<exe>.manifest` 与测试二进制同目录；`cargo test` 就地运行 `target/debug/deps/` 下的二进制，满足该前提。若将来把测试二进制单独拷贝到别处运行，需连同 `.manifest` 一起拷贝。

## 评审记录

### 第 1 轮

- 建议 · crates/contracts/src/lib.rs:17,21 · 超出「范围」的契约改动（`#[ts(optional)]` → `#[ts(optional = nullable)]`）判定为「建议，不阻塞」：核查确认它只是修正源与产物的漂移——重新生成的 `Todo.ts` 类型体与仓库既有内容逐字段一致（`dueDate?: string | null`、`reminderAt?: string | null`），前端零改动；`#[ts(...)]` 仅影响 ts-rs 生成的 TypeScript，两个字段的 `#[serde(default, skip_serializing_if = "Option::is_none")]` 未动，Rust 侧序列化/反序列化行为不变；`Todo` 未被 `crates/server` 引用（server 只用 `SyncRequest`/`SyncResponse`/`TodoSyncChange`/`HealthResponse` 等，见 crates/server/src/lib.rs:15-17），服务端语义不变；前端 `src/stores/todos.ts:32-33` 确实写入 `null`，改后标注与同文件 `TodoPatch` 同名字段一致 · AGENTS.md 服务端规范「允许写入 null 的用 `#[ts(optional = nullable)]`」＋任务「范围」条款
  - 处理（编排者）：追认范围扩展，维持不改。该改动是恢复「契约单源」这一 AGENTS.md 硬约束的必要条件——不改则 `cargo test` 生成的产物与仓库既有 `Todo.ts` 冲突、`pnpm run check` 必失败，本任务的核心目标（打通验证链路）无法达成；且经评审核实 Rust 序列化行为与服务端语义均不变，属最小改动。
- 建议 · vite.config.ts:10 · 超出「范围」的配置改动（`fmt.ignorePatterns` 增加 `src/bindings/**`）判定为「建议，不阻塞」：互斥属实——`export_type_bindings` 由 `cargo test`（含 AGENTS.md 规定的 `cargo test --workspace`）触发并以生成器原始格式覆写 `src/bindings/**`，`src-tauri/src/lib.rs:117` 还让 `tauri:dev` 启动时重写 `commands.ts`，格式化结果必被下次生成抹掉；更小替代方案经核查均不成立——在 `types:generate` 脚本后置 `vp fmt` 覆盖不到 `cargo test --workspace`，ts-rs 12 的 `Config` 无格式化与头注释选项（只有需新增 dprint 依赖的 `format` feature，且对 tauri-specta 产出的 `commands.ts` 无效）；检查强度未降低——`lint.ignorePatterns` 未加该目录、`tsconfig.json` include `src/**/*.ts`，实测 lint/类型检查覆盖 34 个文件＝`src/` 全部 32 个（含 bindings 11 个）＋2 个根配置 · AGENTS.md「`src/bindings/` 为生成产物，禁止手改」＋简约至上
  - 处理（编排者）：追认范围扩展，维持不改。理由同上——`cargo test --workspace` 与 `pnpm run check` 二者皆为 AGENTS.md 强制验证命令，不做此项则无法在同一次运行中同时成立；评审已核实 lint 与类型检查仍覆盖 `src/bindings/`，仅豁免格式校验，检查强度损失可接受。后续若引入生成后格式化步骤，可回收此豁免。
- 建议 · src-tauri/build.rs · 验收标准第 4 条中「不影响 `pnpm run tauri:build` 的既有清单行为」未直接实测，仅以 debug 产物做等价核对。评审侧已复核证据且认为风险低：`target/debug/cross-platform-todo.exe` 仍含 tauri 嵌入的 `Microsoft.Windows.Common-Controls 6.0.0.0` RT_MANIFEST 资源，`target/debug/` 下无外部 `.manifest`，外部清单只出现在 `deps/` 的 lib 测试二进制与 cdylib 旁；且 `deps/` 下其他包的测试二进制（todo_server、todo_domain、todo_contracts）均无 `.manifest`，可证链接器默认不产外部清单、`/MANIFEST:NO` 对 bin 目标恰好还原改动前行为；链接参数与 profile 无关。建议下次正式打包时顺带确认 · 任务验收标准第 4 条
  - 处理（编排者）：本次不处理，已知悉。`pnpm run tauri:build` 属发布链路，将在「发布工程/02-安装包与签名」子任务中首次正式执行，届时一并核对嵌入清单；此处不再单独跑一次完整打包。

验证：`cargo test --workspace` 通过（6 个测试全绿，`tests::export_type_bindings ... ok`，不再出现 0xC0000139）；`cargo check --workspace` 通过、无警告；`pnpm run check` 通过（39 个文件格式校验、34 个文件无 lint/类型错误）；`cargo test --workspace` 之后紧接重跑 `pnpm run check` 仍通过且 `git status` 无新增改动（生成幂等）；`git status` 改动清单与「实现记录」逐项一致，无清单遗漏；`src-tauri/build.rs` 守卫经读码核对为 `CARGO_CFG_TARGET_OS == "windows"` 且 `CARGO_CFG_TARGET_ENV == "msvc"` 双条件，`unwrap_or_default()` 使变量缺失时取空串，非 Windows／非 MSVC 直接 return，三条 `println!` 均在守卫之后，跨平台无副作用。
