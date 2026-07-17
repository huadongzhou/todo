# ‪目标目录：D:\rust\tarui-demo 目标目录为tarui官方的推荐开源项目中 符合当前产品 提取他们的前…

> ‪目标目录：D:\rust\tarui-demo
> 目标目录为tarui官方的推荐开源项目中 符合当前产品
>
> 提取他们的前端公共能力/客户端公共能力 有什么是平台端可取的 ？
>
> 应用的场景为 ：多端 跨平台 桌面常驻 桌面卡片 主题 通信 日志 设置 日期 等等

<details><summary>7 previous messages</summary>

> Stream error
>
> Reconnecting 5/5
> request timed out
>
> 我会先确认目标项目的实际结构与它关联的官方推荐来源，再按“前端公共能力 / Tauri 客户端能力 / 可复用的平台能力”做一份适配待办应用的取舍清单；本轮只做调研，不改动代码。
>
> <details><summary>Ran 4 commands, called 2 tools, searched the web</summary>
>
> - Ran `if (Test-Path -LiteralPath D:\rust\tarui-demo) { Get-ChildItem -LiteralPath D:\rust\tarui-demo -Force | Select-Object Mode,Length,LastWriteTime,Name; if (Test-Path -LiteralPath D:\rust\tarui-demo\.git) { git -C D:\rust\tarui-demo remote -v; git -C D:\rust\tarui-demo status --short --branch } } else { Write-Output 'TARGET_NOT_FOUND' }`
> - Ran `Get-Content -LiteralPath C:\Users\Administrator\.codex\RTK.md -Raw; Get-Content -LiteralPath .agents\workflows\feature-delivery.md -Raw; Get-ChildItem -LiteralPath .agents\agents -File | Select-Object -ExpandProperty FullName`
> - Ran `rtk Get-Content -LiteralPath .agents\agents\architect.md -Raw`
>
> MCP tool call
>
> codebase-memory-mcp.list_projects
>
> ```json
> {}
> ```
>
> Result: none
>
> - Ran `rtk proxy powershell -NoProfile -Command "Get-Content -LiteralPath '.agents\agents\architect.md' -Raw"`
>
> MCP tool call
>
> codebase-memory-mcp.list_projects
>
> ```json
> {}
> ```
>
> ```text
> {"projects":[{"name":"D-project-cpms-plm","root_path":"D:/project/cpms/plm","nodes":4032,"edges":8605,"size_bytes":9240576},{"name":"D-project-custom-js-console-platform","root_path":"D:/project/custom/js-console-platform","nodes":66807,"edges":204026,"size_bytes":155779072},{"name":"D-project-som","root_path":"D:/project/som","nodes":36551,"edges":96745,"size_bytes":64487424},{"name":"D-project-todo","root_path":"D:/project/todo","nodes":594,"edges":651,"size_bytes":2031616}]}
> ```
>
> <details><summary>Searched the web</summary>
>
> - Searched the web for `site:tauri.app showcase Anarlog Blinko Kanri Spacedrive Voicetypr ...`
>
> </details>
>
> </details>
>
> 目标目录里已有五个来自 Awesome Tauri 的项目副本：Anarlog、Blinko、Kanri、Spacedrive、Voicetypr。它们和当前待办场景最相关的分别是：日程、笔记/任务数据、看板、桌面壳与设备通信、常驻语音输入。接下来我会从实际依赖、Tauri 配置和调用边界中提炼能力，而不是只按项目介绍判断。
>
> <details><summary>Ran 10 commands, called 33 tools, searched the web</summary>
>
> MCP tool call
>
> codebase-memory-mcp.index_repository
>
> ```json
> {
>   "repo_path": "D:\\rust\\tarui-demo\\anarlog-main",
>   "mode": "moderate",
>   "persistence": false
> }
> ```
>
> ```text
> {"project":"D-rust-tarui-demo-anarlog-main","status":"indexed","excluded":{"dirs":[".cargo",".vscode","docs","e2e","examples","scripts","supabase/.branches","supabase/.temp","supabase/migrations","supabase/scripts","plugins/webhook/docs","plugins/transcription/assets","plugins/local-llm/assets","plugins/icon/examples","plugins/deeplink2/examples","packages/api-client/src/generated","packages/agent-support/src/tools","packages/agent-designer/src/tools","packages/agent-core/src/tools","legacy/db-user/assets","crates/docs","crates/vad-masking/examples","crates/transcribe-proxy/tests/fixtures","crates/template-support/assets","crates/template-eval/assets"],"count":62,"truncated":true},"nodes":47846,"edges":140413,"adr_present":false,"adr_hint":"Project indexed. Consider creating an Architecture Decision Record: explore the codebase with get_architecture(aspects=['all']), then use manage_adr(mode='store') to persist architectural insights across sessions.","artifact_present":false}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.index_repository
>
> ```json
> {
>   "repo_path": "D:\\rust\\tarui-demo\\blinko-main",
>   "mode": "moderate",
>   "persistence": false
> }
> ```
>
> ```text
> {"project":"D-rust-tarui-demo-blinko-main","status":"indexed","excluded":{"dirs":["shared/lib/__tests__","server/__tests__","server/aiServer/tools","server/aiServer/providers/__tests__","prisma/migrations","app/public","app/src-tauri/gen","app/src/assets"],"count":8,"truncated":false},"nodes":6727,"edges":41891,"adr_present":false,"adr_hint":"Project indexed. Consider creating an Architecture Decision Record: explore the codebase with get_architecture(aspects=['all']), then use manage_adr(mode='store') to persist architectural insights across sessions.","artifact_present":false}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.index_repository
>
> ```json
> {
>   "repo_path": "D:\\rust\\tarui-demo\\kanri-main",
>   "mode": "moderate",
>   "persistence": false
> }
> ```
>
> ```text
> {"project":"D-rust-tarui-demo-kanri-main","status":"indexed","excluded":{"dirs":[".vscode","assets","i18n","scripts"],"count":4,"truncated":false},"nodes":527,"edges":706,"adr_present":false,"adr_hint":"Project indexed. Consider creating an Architecture Decision Record: explore the codebase with get_architecture(aspects=['all']), then use manage_adr(mode='store') to persist architectural insights across sessions.","artifact_present":false}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.index_repository
>
> ```json
> {
>   "repo_path": "D:\\rust\\tarui-demo\\spacedrive-main",
>   "mode": "moderate",
>   "persistence": false
> }
> ```
>
> ```text
> {"project":"D-rust-tarui-demo-spacedrive-main","status":"indexed","excluded":{"dirs":[".cargo",".vscode","docs","scripts","packages/assets","packages/ts-client/tests/integration","packages/ts-client/src/generated","packages/ts-client/src/__tests__","packages/ts-client/src/hooks/__tests__","extensions/test-extension/.cargo","crates/sd-client/examples","crates/log-analyzer/examples","crates/crypto/examples","core/examples","core/scripts","core/src/bin","core/src/ops/media","core/benchmarks/src/bin","apps/tauri/assets","apps/tauri/scripts","apps/tauri/src-tauri/gen","apps/mobile/scripts",".github/actions/publish-artifacts/dist"],"count":23,"truncated":false},"nodes":24874,"edges":84789,"adr_present":false,"adr_hint":"Project indexed. Consider creating an Architecture Decision Record: explore the codebase with get_architecture(aspects=['all']), then use manage_adr(mode='store') to persist architectural insights across sessions.","artifact_present":false}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.index_repository
>
> ```json
> {
>   "repo_path": "D:\\rust\\tarui-demo\\voicetypr-main",
>   "mode": "moderate",
>   "persistence": false
> }
> ```
>
> ```text
> {"project":"D-rust-tarui-demo-voicetypr-main","status":"indexed","excluded":{"dirs":[".claude",".vscode","docs","public","scripts","tests/fixtures","src-tauri/.cargo","src-tauri/src/media","src-tauri/crates/keytrigger/examples","src/assets","src/lib/__tests__","src/hooks/__tests__","src/components/__tests__","src/components/sections/__tests__",".github/scripts"],"count":15,"truncated":false},"nodes":7331,"edges":29164,"adr_present":false,"adr_hint":"Project indexed. Consider creating an Architecture Decision Record: explore the codebase with get_architecture(aspects=['all']), then use manage_adr(mode='store') to persist architectural insights across sessions.","artifact_present":false}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.get_architecture
>
> ```json
> {
>   "project": "D-rust-tarui-demo-anarlog-main",
>   "aspects": ["packages", "entry_points", "dependencies"]
> }
> ```
>
> ```text
> {"project":"D-rust-tarui-demo-anarlog-main","total_nodes":47846,"total_edges":140413,"edge_types":[{"type":"DEFINES","count":42901},{"type":"USAGE","count":40991},{"type":"CALLS","count":31237},{"type":"DECORATES","count":5915},{"type":"IMPORTS","count":4798},{"type":"SIMILAR_TO","count":4116},{"type":"DEFINES_METHOD","count":3671},{"type":"CONTAINS_FILE","count":3432},{"type":"CONTAINS_FOLDER","count":1093},{"type":"WRITES","count":882},{"type":"CONFIGURES","count":464},{"type":"TESTS","count":285},{"type":"HTTP_CALLS","count":148},{"type":"IMPLEMENTS","count":142},{"type":"RAISES","count":100},{"type":"HANDLES","count":84},{"type":"SEMANTICALLY_RELATED","count":62},{"type":"DEPENDS_ON","count":41},{"type":"LISTENS_ON","count":36},{"type":"INHERITS","count":9},{"type":"EMITS","count":5},{"type":"THROWS","count":1}],"packages":[{"name":"fortio.org/progressbar","node_count":1,"fan_in":0,"fan_out":0},{"name":"github.com/adrg/xdg","node_count":1,"fan_in":0,"fan_out":0},{"name":"github.com/caarlos0/env/v11","node_count":1,"fan_in":0,"fan_out":0},{"name":"github.com/cenkalti/backoff/v5","node_count":1,"fan_in":0,"fan_out":0},{"name":"github.com/cespare/xxhash/v2","node_count":1,"fan_in":0,"fan_out":0},{"name":"github.com/cyberphone/json-canonicalization","node_count":1,"fan_in":0,"fan_out":0},{"name":"github.com/dgraph-io/badger/v4","node_count":1,"fan_in":0,"fan_out":0},{"name":"github.com/dgraph-io/ristretto/v2","node_count":1,"fan_in":0,"fan_out":0},{"name":"github.com/dustin/go-humanize","node_count":1,"fan_in":0,"fan_out":0},{"name":"github.com/go-logr/logr","node_count":1,"fan_in":0,"fan_out":0},{"name":"github.com/go-logr/stdr","node_count":1,"fan_in":0,"fan_out":0},{"name":"github.com/google/flatbuffers","node_count":1,"fan_in":0,"fan_out":0},{"name":"github.com/inconshreveable/mousetrap","node_count":1,"fan_in":0,"fan_out":0},{"name":"github.com/jedib0t/go-pretty/v6","node_count":1,"fan_in":0,"fan_out":0},{"name":"github.com/json-iterator/go","node_count":1,"fan_in":0,"fan_out":0}],"entry_points":[{"name":"main","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.api.src.main.main","file":"apps/api/src/main.rs"},{"name":"main","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.cli.src.main.main","file":"apps/cli/src/main.rs"},{"name":"changelog","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.plugins.changelog.changelog","file":"apps/desktop/plugins/changelog.ts"},{"name":"main","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src-tauri.build.main","file":"apps/desktop/src-tauri/build.rs"},{"name":"main","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src-tauri.src.lib.main","file":"apps/desktop/src-tauri/src/lib.rs"},{"name":"main","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src-tauri.src.main.main","file":"apps/desktop/src-tauri/src/main.rs"},{"name":"createAuthFetch","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.ai.auth-fetch.createAuthFetch","file":"apps/desktop/src/ai/auth-fetch.ts"},{"name":"AITaskProvider","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.ai.contexts.AITaskProvider","file":"apps/desktop/src/ai/contexts.tsx"},{"name":"useAITask","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.ai.contexts.useAITask","file":"apps/desktop/src/ai/contexts.tsx"},{"name":"useAITaskTask","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.ai.hooks.useAITaskTask.useAITaskTask","file":"apps/desktop/src/ai/hooks/useAITaskTask.ts"},{"name":"useLanguageModel","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.ai.hooks.useLLMConnection.useLanguageModel","file":"apps/desktop/src/ai/hooks/useLLMConnection.ts"},{"name":"useLLMConnection","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.ai.hooks.useLLMConnection.useLLMConnection","file":"apps/desktop/src/ai/hooks/useLLMConnection.ts"},{"name":"useLLMConnectionStatus","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.ai.hooks.useLLMConnection.useLLMConnectionStatus","file":"apps/desktop/src/ai/hooks/useLLMConnection.ts"},{"name":"useModelMetadata","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.ai.hooks.useModelMetadata.useModelMetadata","file":"apps/desktop/src/ai/hooks/useModelMetadata.ts"},{"name":"useTitleGenerating","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.ai.hooks.useTitleGenerating.useTitleGenerating","file":"apps/desktop/src/ai/hooks/useTitleGenerating.ts"},{"name":"useTitleGeneration","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.ai.hooks.useTitleGeneration.useTitleGeneration","file":"apps/desktop/src/ai/hooks/useTitleGeneration.ts"},{"name":"deterministicGenerationSettings","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.ai.model-settings.deterministicGenerationSettings","file":"apps/desktop/src/ai/model-settings.ts"},{"name":"isMainAITaskHostWindow","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.ai.task-window-sync.isMainAITaskHostWindow","file":"apps/desktop/src/ai/task-window-sync.tsx"},{"name":"requestMainAITaskCancel","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.ai.task-window-sync.requestMainAITaskCancel","file":"apps/desktop/src/ai/task-window-sync.tsx"},{"name":"requestMainEnhance","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.ai.task-window-sync.requestMainEnhance","file":"apps/desktop/src/ai/task-window-sync.tsx"}]}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.get_architecture
>
> ```json
> {
>   "project": "D-rust-tarui-demo-blinko-main",
>   "aspects": ["packages", "entry_points", "dependencies"]
> }
> ```
>
> ```text
> {"project":"D-rust-tarui-demo-blinko-main","total_nodes":6727,"total_edges":41891,"edge_types":[{"type":"WRITES","count":19804},{"type":"CALLS","count":11444},{"type":"DEFINES","count":6108},{"type":"IMPORTS","count":1544},{"type":"USAGE","count":857},{"type":"SIMILAR_TO","count":697},{"type":"CONTAINS_FILE","count":384},{"type":"DEFINES_METHOD","count":374},{"type":"SEMANTICALLY_RELATED","count":324},{"type":"CONTAINS_FOLDER","count":123},{"type":"RAISES","count":48},{"type":"DECORATES","count":43},{"type":"HTTP_CALLS","count":30},{"type":"THROWS","count":30},{"type":"EMITS","count":29},{"type":"INHERITS","count":22},{"type":"LISTENS_ON","count":22},{"type":"CONFIGURES","count":6},{"type":"DEPENDS_ON","count":1},{"type":"HANDLES","count":1}],"packages":[{"name":"vditor","node_count":3254,"fan_in":0,"fan_out":0},{"name":"src","node_count":878,"fan_in":0,"fan_out":0},{"name":"lib","node_count":82,"fan_in":0,"fan_out":0},{"name":"aiServer","node_count":72,"fan_in":0,"fan_out":0},{"name":"src-tauri","node_count":60,"fan_in":0,"fan_out":0},{"name":"jobs","node_count":53,"fan_in":0,"fan_out":0},{"name":"tauri-plugin-blinko","node_count":41,"fan_in":0,"fan_out":0},{"name":"routerExpress","node_count":33,"fan_in":0,"fan_out":0},{"name":"routerTrpc","node_count":29,"fan_in":0,"fan_out":0},{"name":"schema","node_count":24,"fan_in":0,"fan_out":0},{"name":"seed","node_count":9,"fan_in":0,"fan_out":0},{"name":"templates","node_count":9,"fan_in":0,"fan_out":0},{"name":"server","node_count":8,"fan_in":0,"fan_out":0},{"name":"resetpassword","node_count":2,"fan_in":0,"fan_out":0},{"name":"vite","node_count":2,"fan_in":0,"fan_out":0}],"entry_points":[{"name":"main","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.build.main","file":"app/src-tauri/build.rs"},{"name":"main","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.main.main","file":"app/src-tauri/src/main.rs"},{"name":"setNavigate","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.components.Auth.auth-client.setNavigate","file":"app/src/components/Auth/auth-client.ts"},{"name":"navigate","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.components.Auth.auth-client.navigate","file":"app/src/components/Auth/auth-client.ts"},{"name":"getTokenData","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.components.Auth.auth-client.getTokenData","file":"app/src/components/Auth/auth-client.ts"},{"name":"signIn","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.components.Auth.auth-client.signIn","file":"app/src/components/Auth/auth-client.ts"},{"name":"signOut","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.components.Auth.auth-client.signOut","file":"app/src/components/Auth/auth-client.ts"},{"name":"getToolIcon","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.components.BlinkoAi.ToolComponents.getToolIcon","file":"app/src/components/BlinkoAi/ToolComponents.tsx"},{"name":"getToolDisplayName","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.components.BlinkoAi.ToolComponents.getToolDisplayName","file":"app/src/components/BlinkoAi/ToolComponents.tsx"},{"name":"ToolUsageChip","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.components.BlinkoAi.ToolComponents.ToolUsageChip","file":"app/src/components/BlinkoAi/ToolComponents.tsx"},{"name":"ToolCallBubble","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.components.BlinkoAi.ToolComponents.ToolCallBubble","file":"app/src/components/BlinkoAi/ToolComponents.tsx"},{"name":"StreamToolRenderer","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.components.BlinkoAi.ToolComponents.StreamToolRenderer","file":"app/src/components/BlinkoAi/ToolComponents.tsx"},{"name":"AiEmoji","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.components.BlinkoAi.aiEmoji.AiEmoji","file":"app/src/components/BlinkoAi/aiEmoji.tsx"},{"name":"AiTag","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.components.BlinkoAi.aiTag.AiTag","file":"app/src/components/BlinkoAi/aiTag.tsx"},{"name":"HeatMap","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.components.BlinkoAnalytics.HeatMap.HeatMap","file":"app/src/components/BlinkoAnalytics/HeatMap.tsx"},{"name":"SwipeableCard","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.components.BlinkoCard.SwipeableCard.SwipeableCard","file":"app/src/components/BlinkoCard/SwipeableCard.tsx"},{"name":"CardBlogBox","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.components.BlinkoCard.cardBlogBox.CardBlogBox","file":"app/src/components/BlinkoCard/cardBlogBox.tsx"},{"name":"CardFooter","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.components.BlinkoCard.cardFooter.CardFooter","file":"app/src/components/BlinkoCard/cardFooter.tsx"},{"name":"ConvertTypeButton","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.components.BlinkoCard.cardFooter.ConvertTypeButton","file":"app/src/components/BlinkoCard/cardFooter.tsx"},{"name":"ShowCommentDialog","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.components.BlinkoCard.commentButton.ShowCommentDialog","file":"app/src/components/BlinkoCard/commentButton.tsx"}]}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.get_architecture
>
> ```json
> {
>   "project": "D-rust-tarui-demo-kanri-main",
>   "aspects": ["packages", "entry_points", "dependencies"]
> }
> ```
>
> ```text
> {"project":"D-rust-tarui-demo-kanri-main","total_nodes":527,"total_edges":706,"edge_types":[{"type":"DEFINES","count":408},{"type":"CALLS","count":104},{"type":"CONTAINS_FILE","count":93},{"type":"SEMANTICALLY_RELATED","count":61},{"type":"CONTAINS_FOLDER","count":21},{"type":"USAGE","count":9},{"type":"IMPORTS","count":8},{"type":"DECORATES","count":1},{"type":"SIMILAR_TO","count":1}],"packages":[{"name":"boards","node_count":33,"fan_in":0,"fan_out":0},{"name":"useBoard","node_count":28,"fan_in":0,"fan_out":0},{"name":"settings","node_count":14,"fan_in":0,"fan_out":0},{"name":"colorUtils","node_count":11,"fan_in":0,"fan_out":0},{"name":"useBackgroundImage","node_count":8,"fan_in":0,"fan_out":0},{"name":"Cargo","node_count":6,"fan_in":0,"fan_out":0},{"name":"layout","node_count":5,"fan_in":0,"fan_out":0},{"name":"sorting","node_count":4,"fan_in":0,"fan_out":0},{"name":"theme","node_count":4,"fan_in":0,"fan_out":0},{"name":"iconManager","node_count":2,"fan_in":0,"fan_out":0},{"name":"no-store-outside-stores","node_count":2,"fan_in":0,"fan_out":0},{"name":"src","node_count":2,"fan_in":0,"fan_out":0},{"name":"objects","node_count":1,"fan_in":0,"fan_out":0},{"name":"build","node_count":1,"fan_in":0,"fan_out":0},{"name":"useContextMenuClasses","node_count":1,"fan_in":0,"fan_out":0}],"entry_points":[{"name":"useBackgroundImage","qualified_name":"D-rust-tarui-demo-kanri-main.composables.useBackgroundImage.useBackgroundImage","file":"composables/useBackgroundImage.ts"},{"name":"useBoard","qualified_name":"D-rust-tarui-demo-kanri-main.composables.useBoard.useBoard","file":"composables/useBoard.ts"},{"name":"init","qualified_name":"D-rust-tarui-demo-kanri-main.composables.useBoard.init","file":"composables/useBoard.ts"},{"name":"save","qualified_name":"D-rust-tarui-demo-kanri-main.composables.useBoard.save","file":"composables/useBoard.ts"},{"name":"renameBoard","qualified_name":"D-rust-tarui-demo-kanri-main.composables.useBoard.renameBoard","file":"composables/useBoard.ts"},{"name":"togglePin","qualified_name":"D-rust-tarui-demo-kanri-main.composables.useBoard.togglePin","file":"composables/useBoard.ts"},{"name":"updateBoardPin","qualified_name":"D-rust-tarui-demo-kanri-main.composables.useBoard.updateBoardPin","file":"composables/useBoard.ts"},{"name":"duplicate","qualified_name":"D-rust-tarui-demo-kanri-main.composables.useBoard.duplicate","file":"composables/useBoard.ts"},{"name":"remove","qualified_name":"D-rust-tarui-demo-kanri-main.composables.useBoard.remove","file":"composables/useBoard.ts"},{"name":"addColumn","qualified_name":"D-rust-tarui-demo-kanri-main.composables.useBoard.addColumn","file":"composables/useBoard.ts"},{"name":"removeColumn","qualified_name":"D-rust-tarui-demo-kanri-main.composables.useBoard.removeColumn","file":"composables/useBoard.ts"},{"name":"reorderColumns","qualified_name":"D-rust-tarui-demo-kanri-main.composables.useBoard.reorderColumns","file":"composables/useBoard.ts"},{"name":"setColumnTitle","qualified_name":"D-rust-tarui-demo-kanri-main.composables.useBoard.setColumnTitle","file":"composables/useBoard.ts"},{"name":"createCard","qualified_name":"D-rust-tarui-demo-kanri-main.composables.useBoard.createCard","file":"composables/useBoard.ts"},{"name":"duplicateCard","qualified_name":"D-rust-tarui-demo-kanri-main.composables.useBoard.duplicateCard","file":"composables/useBoard.ts"},{"name":"deleteCard","qualified_name":"D-rust-tarui-demo-kanri-main.composables.useBoard.deleteCard","file":"composables/useBoard.ts"},{"name":"deleteAllColumnCards","qualified_name":"D-rust-tarui-demo-kanri-main.composables.useBoard.deleteAllColumnCards","file":"composables/useBoard.ts"},{"name":"mutateCard","qualified_name":"D-rust-tarui-demo-kanri-main.composables.useBoard.mutateCard","file":"composables/useBoard.ts"},{"name":"reorderCards","qualified_name":"D-rust-tarui-demo-kanri-main.composables.useBoard.reorderCards","file":"composables/useBoard.ts"},{"name":"useContextMenuClasses","qualified_name":"D-rust-tarui-demo-kanri-main.composables.useContextMenuClasses.useContextMenuClasses","file":"composables/useContextMenuClasses.ts"}]}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.get_architecture
>
> ```json
> {
>   "project": "D-rust-tarui-demo-spacedrive-main",
>   "aspects": ["packages", "entry_points", "dependencies"]
> }
> ```
>
> ```text
> {"project":"D-rust-tarui-demo-spacedrive-main","total_nodes":24874,"total_edges":84789,"edge_types":[{"type":"USAGE","count":26337},{"type":"DEFINES","count":22373},{"type":"CALLS","count":19039},{"type":"DEFINES_METHOD","count":5083},{"type":"IMPORTS","count":3324},{"type":"DECORATES","count":2485},{"type":"CONTAINS_FILE","count":1852},{"type":"TESTS","count":1491},{"type":"WRITES","count":1245},{"type":"SIMILAR_TO","count":575},{"type":"CONTAINS_FOLDER","count":443},{"type":"IMPLEMENTS","count":296},{"type":"HTTP_CALLS","count":127},{"type":"CONFIGURES","count":85},{"type":"SEMANTICALLY_RELATED","count":17},{"type":"INHERITS","count":8},{"type":"LISTENS_ON","count":8},{"type":"RAISES","count":1}],"packages":[{"name":"src","node_count":5670,"fan_in":0,"fan_out":0},{"name":"interface","node_count":814,"fan_in":0,"fan_out":0},{"name":"tests","node_count":536,"fan_in":0,"fan_out":0},{"name":"mobile","node_count":320,"fan_in":0,"fan_out":0},{"name":"tauri","node_count":300,"fan_in":0,"fan_out":0},{"name":"cli","node_count":266,"fan_in":0,"fan_out":0},{"name":"task-system","node_count":237,"fan_in":0,"fan_out":0},{"name":"sdk","node_count":196,"fan_in":0,"fan_out":0},{"name":"archive","node_count":179,"fan_in":0,"fan_out":0},{"name":"fs-watcher","node_count":135,"fan_in":0,"fan_out":0},{"name":"ffmpeg","node_count":124,"fan_in":0,"fan_out":0},{"name":"crypto","node_count":120,"fan_in":0,"fan_out":0},{"name":"swift-client","node_count":105,"fan_in":0,"fan_out":0},{"name":"ts-client","node_count":98,"fan_in":0,"fan_out":0},{"name":"log-analyzer","node_count":95,"fan_in":0,"fan_out":0}],"entry_points":[{"name":"main","qualified_name":"D-rust-tarui-demo-spacedrive-main.adapters.apple-notes.sync.main","file":"adapters/apple-notes/sync.py"},{"name":"main","qualified_name":"D-rust-tarui-demo-spacedrive-main.adapters.chrome-bookmarks.sync.main","file":"adapters/chrome-bookmarks/sync.py"},{"name":"main","qualified_name":"D-rust-tarui-demo-spacedrive-main.adapters.chrome-history.sync.main","file":"adapters/chrome-history/sync.py"},{"name":"main","qualified_name":"D-rust-tarui-demo-spacedrive-main.adapters.github.sync.main","file":"adapters/github/sync.py"},{"name":"main","qualified_name":"D-rust-tarui-demo-spacedrive-main.adapters.gmail.sync.main","file":"adapters/gmail/sync.py"},{"name":"main","qualified_name":"D-rust-tarui-demo-spacedrive-main.adapters.macos-calendar.sync.main","file":"adapters/macos-calendar/sync.py"},{"name":"main","qualified_name":"D-rust-tarui-demo-spacedrive-main.adapters.macos-contacts.sync.main","file":"adapters/macos-contacts/sync.py"},{"name":"main","qualified_name":"D-rust-tarui-demo-spacedrive-main.adapters.obsidian.sync.main","file":"adapters/obsidian/sync.py"},{"name":"main","qualified_name":"D-rust-tarui-demo-spacedrive-main.adapters.opencode.sync.main","file":"adapters/opencode/sync.py"},{"name":"main","qualified_name":"D-rust-tarui-demo-spacedrive-main.adapters.safari-history.sync.main","file":"adapters/safari-history/sync.py"},{"name":"main","qualified_name":"D-rust-tarui-demo-spacedrive-main.adapters.slack.sync.main","file":"adapters/slack/sync.py"},{"name":"main","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.cli.src.main.main","file":"apps/cli/src/main.rs"},{"name":"main","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.gpui-photo-grid.src.main.main","file":"apps/gpui-photo-grid/src/main.rs"},{"name":"TabLayout","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.mobile.src.app.(drawer).(tabs)._layout.TabLayout","file":"apps/mobile/src/app/(drawer)/(tabs)/_layout.tsx"},{"name":"NetworkScreen","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.mobile.src.app.(drawer).(tabs).network.NetworkScreen","file":"apps/mobile/src/app/(drawer)/(tabs)/network.tsx"},{"name":"DrawerLayout","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.mobile.src.app.(drawer)._layout.DrawerLayout","file":"apps/mobile/src/app/(drawer)/_layout.tsx"},{"name":"RootLayout","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.mobile.src.app._layout.RootLayout","file":"apps/mobile/src/app/_layout.tsx"},{"name":"Index","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.mobile.src.app.Index","file":"apps/mobile/src/app/index.tsx"},{"name":"JobsScreen","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.mobile.src.app.jobs.JobsScreen","file":"apps/mobile/src/app/jobs.tsx"},{"name":"SearchScreen","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.mobile.src.app.search.SearchScreen","file":"apps/mobile/src/app/search.tsx"}]}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.get_architecture
>
> ```json
> {
>   "project": "D-rust-tarui-demo-voicetypr-main",
>   "aspects": ["packages", "entry_points", "dependencies"]
> }
> ```
>
> ```text
> {"project":"D-rust-tarui-demo-voicetypr-main","total_nodes":7331,"total_edges":29164,"edge_types":[{"type":"USAGE","count":8988},{"type":"CALLS","count":7793},{"type":"DEFINES","count":6815},{"type":"DECORATES","count":1767},{"type":"TESTS","count":854},{"type":"IMPORTS","count":751},{"type":"DEFINES_METHOD","count":602},{"type":"SEMANTICALLY_RELATED","count":601},{"type":"CONTAINS_FILE","count":385},{"type":"WRITES","count":204},{"type":"HTTP_CALLS","count":171},{"type":"SIMILAR_TO","count":149},{"type":"CONTAINS_FOLDER","count":55},{"type":"IMPLEMENTS","count":18},{"type":"INHERITS","count":4},{"type":"CONFIGURES","count":2},{"type":"LISTENS_ON","count":2},{"type":"RAISES","count":2},{"type":"THROWS","count":1}],"packages":[{"name":"src","node_count":2928,"fan_in":0,"fan_out":0},{"name":"components","node_count":466,"fan_in":0,"fan_out":0},{"name":"crates","node_count":173,"fan_in":0,"fan_out":0},{"name":"lib","node_count":58,"fan_in":0,"fan_out":0},{"name":"hooks","node_count":48,"fan_in":0,"fan_out":0},{"name":"parakeet-swift","node_count":35,"fan_in":0,"fan_out":0},{"name":"utils","node_count":24,"fan_in":0,"fan_out":0},{"name":"whisper-vulkan","node_count":23,"fan_in":0,"fan_out":0},{"name":"contexts","node_count":21,"fan_in":0,"fan_out":0},{"name":"services","node_count":17,"fan_in":0,"fan_out":0},{"name":"Cargo","node_count":13,"fan_in":0,"fan_out":0},{"name":"types","node_count":11,"fan_in":0,"fan_out":0},{"name":"catalog","node_count":8,"fan_in":0,"fan_out":0},{"name":"test","node_count":5,"fan_in":0,"fan_out":0},{"name":"test-manifest","node_count":4,"fan_in":0,"fan_out":0}],"entry_points":[{"name":"main","qualified_name":"D-rust-tarui-demo-voicetypr-main.sidecar.whisper-vulkan.src.main.main","file":"sidecar/whisper-vulkan/src/main.rs"},{"name":"main","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.build.main","file":"src-tauri/build.rs"},{"name":"main","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.catalog.generate.main","file":"src-tauri/catalog/generate.py"},{"name":"main","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.main.main","file":"src-tauri/src/main.rs"},{"name":"App","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.App.App","file":"src/App.tsx"},{"name":"ActivityGraph","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.components.ActivityGraph.ActivityGraph","file":"src/components/ActivityGraph.tsx"},{"name":"ApiKeyModal","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.components.ApiKeyModal.ApiKeyModal","file":"src/components/ApiKeyModal.tsx"},{"name":"AppContainer","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.components.AppContainer.AppContainer","file":"src/components/AppContainer.tsx"},{"name":"AppShell","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.components.AppShell.AppShell","file":"src/components/AppShell.tsx"},{"name":"AudioBars","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.components.AudioBars.AudioBars","file":"src/components/AudioBars.tsx"},{"name":"Brandmark","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.components.Brandmark.Brandmark","file":"src/components/Brandmark.tsx"},{"name":"CrashReportDialog","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.components.CrashReportDialog.CrashReportDialog","file":"src/components/CrashReportDialog.tsx"},{"name":"EnhancementSettings","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.components.EnhancementSettings.EnhancementSettings","file":"src/components/EnhancementSettings.tsx"},{"name":"AppErrorBoundary","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.components.ErrorBoundary.AppErrorBoundary","file":"src/components/ErrorBoundary.tsx"},{"name":"RecordingErrorBoundary","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.components.ErrorBoundary.RecordingErrorBoundary","file":"src/components/ErrorBoundary.tsx"},{"name":"SettingsErrorBoundary","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.components.ErrorBoundary.SettingsErrorBoundary","file":"src/components/ErrorBoundary.tsx"},{"name":"ModelManagementErrorBoundary","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.components.ErrorBoundary.ModelManagementErrorBoundary","file":"src/components/ErrorBoundary.tsx"},{"name":"FeedbackToast","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.components.FeedbackToast.FeedbackToast","file":"src/components/FeedbackToast.tsx"},{"name":"LanguageSelection","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.components.LanguageSelection.LanguageSelection","file":"src/components/LanguageSelection.tsx"},{"name":"MicrophoneSelection","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.components.MicrophoneSelection.MicrophoneSelection","file":"src/components/MicrophoneSelection.tsx"}]}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.search_graph
>
> ```json
> {
>   "project": "D-rust-tarui-demo-anarlog-main",
>   "query": "calendar settings theme notifications tray autostart log event IPC",
>   "limit": 40,
>   "include_connected": true
> }
> ```
>
> ```text
> {"total":1104,"search_mode":"bm25","results":[{"name":"useSettingsThemeReady","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.shared.theme.use-settings-theme-ready.useSettingsThemeReady","label":"Function","file_path":"apps/desktop/src/shared/theme/use-settings-theme-ready.ts","start_line":3,"end_line":5,"rank":-28.07929305518853},{"name":"ThemeSelector","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.settings.general.theme.ThemeSelector","label":"Function","file_path":"apps/desktop/src/settings/general/theme.tsx","start_line":22,"end_line":68,"rank":-26.861436535701415},{"name":"log_probe_event","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.audio-actual.src.capture.stream.AecReferenceAligner.log_probe_event","label":"Method","file_path":"crates/audio-actual/src/capture/stream.rs","start_line":345,"end_line":398,"rank":-25.57676369429916},{"name":"loadThemeFromSettings","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.shared.theme.apply.loadThemeFromSettings","label":"Function","file_path":"apps/desktop/src/shared/theme/apply.ts","start_line":46,"end_line":57,"rank":-25.093190943709246},{"name":"bootstrapThemeFromSettings","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.shared.theme.apply.bootstrapThemeFromSettings","label":"Function","file_path":"apps/desktop/src/shared/theme/apply.ts","start_line":59,"end_line":75,"rank":-25.093190943709246},{"name":"calendar_notifications_default_to_app_plus_calendar_overlay","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.notification-interface.src.lib.calendar_notifications_default_to_app_plus_calendar_overlay","label":"Function","file_path":"crates/notification-interface/src/lib.rs","start_line":339,"end_line":351,"rank":-24.638784033906774},{"name":"build","qualified_name":"D-rust-tarui-demo-anarlog-main.plugins.tray.src.menu_items.tray_settings.TraySettings.build","label":"Method","file_path":"plugins/tray/src/menu_items/tray_settings.rs","start_line":15,"end_line":18,"rank":-24.292937844947275},{"name":"handle","qualified_name":"D-rust-tarui-demo-anarlog-main.plugins.tray.src.menu_items.tray_settings.TraySettings.handle","label":"Method","file_path":"plugins/tray/src/menu_items/tray_settings.rs","start_line":20,"end_line":29,"rank":-24.292937844947275},{"name":"SettingsNotifications","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.settings.general.SettingsNotifications","label":"Function","file_path":"apps/desktop/src/settings/general/index.tsx","start_line":311,"end_line":318,"rank":-23.129566505510887},{"name":"setup_change_notification","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.apple-calendar.src.apple.notifications.setup_change_notification","label":"Function","file_path":"crates/apple-calendar/src/apple/notifications.rs","start_line":16,"end_line":49,"rank":-23.06083589666558},{"name":"checkEventNotifications","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.services.event-notification.checkEventNotifications","label":"Function","file_path":"apps/desktop/src/services/event-notification/index.ts","start_line":22,"end_line":86,"rank":-22.763983583635564},{"name":"extract_event_calendar_ref","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.apple-calendar.src.apple.transforms.event.extract_event_calendar_ref","label":"Function","file_path":"crates/apple-calendar/src/apple/transforms/event.rs","start_line":83,"end_line":89,"rank":-22.148054454970584},{"name":"applyTheme","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.shared.theme.provider.applyTheme","label":"Function","file_path":"apps/desktop/src/shared/theme/provider.tsx","start_line":20,"end_line":23,"rank":-21.450684513123747},{"name":"normalizeThemePreference","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.shared.theme.apply.normalizeThemePreference","label":"Function","file_path":"apps/desktop/src/shared/theme/apply.ts","start_line":15,"end_line":22,"rank":-21.35350827071148},{"name":"applyDocumentTheme","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.shared.theme.apply.applyDocumentTheme","label":"Function","file_path":"apps/desktop/src/shared/theme/apply.ts","start_line":39,"end_line":44,"rank":-21.35350827071148},{"name":"AppThemeProvider","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.shared.theme.provider.AppThemeProvider","label":"Function","file_path":"apps/desktop/src/shared/theme/provider.tsx","start_line":9,"end_line":36,"rank":-21.35350827071148},{"name":"calendar_event","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.notification-interface.src.lib.NotificationKey.calendar_event","label":"Method","file_path":"crates/notification-interface/src/lib.rs","start_line":32,"end_line":36,"rank":-21.32579469288925},{"name":"readStoredThemePreference","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.shared.theme.apply.readStoredThemePreference","label":"Function","file_path":"apps/desktop/src/shared/theme/apply.ts","start_line":10,"end_line":13,"rank":-21.257967521190796},{"name":"writeStoredThemePreference","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.shared.theme.apply.writeStoredThemePreference","label":"Function","file_path":"apps/desktop/src/shared/theme/apply.ts","start_line":31,"end_line":37,"rank":-21.257967521190796},{"name":"transform_event","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.apple-calendar.src.apple.transforms.event.transform_event","label":"Function","file_path":"crates/apple-calendar/src/apple/transforms/event.rs","start_line":16,"end_line":63,"rank":-21.227782952241302},{"name":"create_event","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.calendar.src.lib.create_event","label":"Function","file_path":"crates/calendar/src/lib.rs","start_line":193,"end_line":204,"rank":-21.010175170729536},{"name":"create_event","qualified_name":"D-rust-tarui-demo-anarlog-main.plugins.calendar.src.commands.create_event","label":"Function","file_path":"plugins/calendar/src/commands.rs","start_line":95,"end_line":101,"rank":-21.010175170729536},{"name":"extract_event_identifiers","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.apple-calendar.src.apple.transforms.event.extract_event_identifiers","label":"Function","file_path":"crates/apple-calendar/src/apple/transforms/event.rs","start_line":71,"end_line":81,"rank":-20.97736634124862},{"name":"extract_event_dates","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.apple-calendar.src.apple.transforms.event.extract_event_dates","label":"Function","file_path":"crates/apple-calendar/src/apple/transforms/event.rs","start_line":128,"end_line":134,"rank":-20.97736634124862},{"name":"extract_event_flags","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.apple-calendar.src.apple.transforms.event.extract_event_flags","label":"Function","file_path":"crates/apple-calendar/src/apple/transforms/event.rs","start_line":155,"end_line":174,"rank":-20.97736634124862},{"name":"extract_event_participants","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.apple-calendar.src.apple.transforms.event.extract_event_participants","label":"Function","file_path":"crates/apple-calendar/src/apple/transforms/event.rs","start_line":181,"end_line":195,"rank":-20.97736634124862},{"name":"useNotifications","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.contexts.notifications.useNotifications","label":"Function","file_path":"apps/desktop/src/contexts/notifications.tsx","start_line":185,"end_line":188,"rank":-20.760774699948968},{"name":"clear_notifications","qualified_name":"D-rust-tarui-demo-anarlog-main.plugins.notification.src.commands.clear_notifications","label":"Function","file_path":"plugins/notification/src/commands.rs","start_line":14,"end_line":18,"rank":-20.760774699948968},{"name":"extract_event_basic_info","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.apple-calendar.src.apple.transforms.event.extract_event_basic_info","label":"Function","file_path":"crates/apple-calendar/src/apple/transforms/event.rs","start_line":101,"end_line":120,"rank":-20.738945885021156},{"name":"extract_event_status_info","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.apple-calendar.src.apple.transforms.event.extract_event_status_info","label":"Function","file_path":"crates/apple-calendar/src/apple/transforms/event.rs","start_line":141,"end_line":146,"rank":-20.738945885021156},{"name":"extract_event_location_info","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.apple-calendar.src.apple.transforms.event.extract_event_location_info","label":"Function","file_path":"crates/apple-calendar/src/apple/transforms/event.rs","start_line":201,"end_line":208,"rank":-20.738945885021156},{"name":"extract_event_recurrence_info","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.apple-calendar.src.apple.transforms.event.extract_event_recurrence_info","label":"Function","file_path":"crates/apple-calendar/src/apple/transforms/event.rs","start_line":216,"end_line":225,"rank":-20.738945885021156},{"name":"extract_event_alarm_info","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.apple-calendar.src.apple.transforms.event.extract_event_alarm_info","label":"Function","file_path":"crates/apple-calendar/src/apple/transforms/event.rs","start_line":231,"end_line":240,"rank":-20.738945885021156},{"name":"extract_event_birthday_info","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.apple-calendar.src.apple.transforms.event.extract_event_birthday_info","label":"Function","file_path":"crates/apple-calendar/src/apple/transforms/event.rs","start_line":247,"end_line":260,"rank":-20.738945885021156},{"name":"convert_google_event","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.calendar.src.convert.convert_google_event","label":"Function","file_path":"crates/calendar/src/convert.rs","start_line":125,"end_line":188,"rank":-20.693023210250196},{"name":"convert_outlook_event","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.calendar.src.convert.convert_outlook_event","label":"Function","file_path":"crates/calendar/src/convert.rs","start_line":190,"end_line":256,"rank":-20.693023210250196},{"name":"convert_apple_event","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.calendar.src.convert.convert_apple_event","label":"Function","file_path":"crates/calendar/src/convert.rs","start_line":258,"end_line":305,"rank":-20.693023210250196},{"name":"create_apple_event","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.calendar.src.lib.create_apple_event","label":"Function","file_path":"crates/calendar/src/lib.rs","start_line":387,"end_line":391,"rank":-20.693023210250196},{"name":"create_event","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.google-calendar.src.client.GoogleCalendarClient.create_event","label":"Method","file_path":"crates/google-calendar/src/client.rs","start_line":112,"end_line":124,"rank":-20.693023210250196},{"name":"create_event","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.outlook-calendar.src.client.OutlookCalendarClient.create_event","label":"Method","file_path":"crates/outlook-calendar/src/client.rs","start_line":75,"end_line":87,"rank":-20.693023210250196}],"has_more":true}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.search_graph
>
> ```json
> {
>   "project": "D-rust-tarui-demo-blinko-main",
>   "query": "theme settings communication notification log date Tauri",
>   "limit": 40,
>   "include_connected": true
> }
> ```
>
> ```text
> {"total":151,"search_mode":"bm25","results":[{"name":"set_desktop_theme","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.window.set_desktop_theme","label":"Function","file_path":"app/src-tauri/src/desktop/window.rs","start_line":240,"end_line":274,"rank":-22.236276430625253},{"name":"open_app_settings","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.src.commands.open_app_settings","label":"Function","file_path":"app/tauri-plugin-blinko/src/commands.rs","start_line":16,"end_line":20,"rank":-20.567266056701776},{"name":"open_app_settings","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.src.desktop.Blinko.open_app_settings","label":"Method","file_path":"app/tauri-plugin-blinko/src/desktop.rs","start_line":21,"end_line":25,"rank":-20.465948945393563},{"name":"open_app_settings","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.src.mobile.Blinko.open_app_settings","label":"Method","file_path":"app/tauri-plugin-blinko/src/mobile.rs","start_line":35,"end_line":40,"rank":-20.465948945393563},{"name":"CreateNotification","qualified_name":"D-rust-tarui-demo-blinko-main.server.routerTrpc.notification.CreateNotification","label":"Function","file_path":"server/routerTrpc/notification.ts","start_line":6,"end_line":30,"rank":-19.992475228149218},{"name":"setTauriTheme","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.lib.tauriHelper.setTauriTheme","label":"Function","file_path":"app/src/lib/tauriHelper.ts","start_line":126,"end_line":138,"rank":-18.891102383194674},{"name":"logTransportError","qualified_name":"D-rust-tarui-demo-blinko-main.server.routerExpress.mcp.logTransportError","label":"Function","file_path":"server/routerExpress/mcp.ts","start_line":117,"end_line":120,"rank":-18.478387012578757},{"name":"saveThemePreference","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.android.src.main.java.Blinko.Blinko.saveThemePreference","label":"Method","file_path":"app/tauri-plugin-blinko/android/src/main/java/Blinko.kt","start_line":84,"end_line":96,"rank":-18.361847860824337},{"name":"getSavedTheme","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.android.src.main.java.Blinko.Blinko.getSavedTheme","label":"Method","file_path":"app/tauri-plugin-blinko/android/src/main/java/Blinko.kt","start_line":98,"end_line":115,"rank":-18.361847860824337},{"name":"applyStartupTheme","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.android.src.main.java.Blinko.Blinko.applyStartupTheme","label":"Method","file_path":"app/tauri-plugin-blinko/android/src/main/java/Blinko.kt","start_line":117,"end_line":126,"rank":-18.361847860824337},{"name":"logOAuthRequest","qualified_name":"D-rust-tarui-demo-blinko-main.server.routerExpress.auth.logOAuthRequest","label":"Function","file_path":"server/routerExpress/auth/index.ts","start_line":29,"end_line":35,"rank":-18.16418017516707},{"name":"openAppSettings","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.dist-js.openAppSettings","label":"Function","file_path":"app/tauri-plugin-blinko/dist-js/index.js","start_line":11,"end_line":13,"rank":-18.051538395172948},{"name":"openAppSettings","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.guest-js.openAppSettings","label":"Function","file_path":"app/tauri-plugin-blinko/guest-js/index.ts","start_line":12,"end_line":14,"rank":-18.051538395172948},{"name":"getVisibleSettings","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.pages.settings.getVisibleSettings","label":"Function","file_path":"app/src/pages/settings.tsx","start_line":158,"end_line":182,"rank":-17.80936655788617},{"name":"openAppSettings","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.android.src.main.java.Blinko.Blinko.openAppSettings","label":"Method","file_path":"app/tauri-plugin-blinko/android/src/main/java/Blinko.kt","start_line":128,"end_line":144,"rank":-17.396880561079048},{"name":"openAppSettings","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.android.src.main.java.BlinkoPlugin.BlinkoPlugin.openAppSettings","label":"Method","file_path":"app/tauri-plugin-blinko/android/src/main/java/BlinkoPlugin.kt","start_line":28,"end_line":32,"rank":-17.396880561079048},{"name":"getCurrentComponent","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.pages.settings.getCurrentComponent","label":"Function","file_path":"app/src/pages/settings.tsx","start_line":184,"end_line":187,"rank":-16.870589976310065},{"name":"applyThemeToEditor","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.components.Common.Editor.hooks.useEditor.applyThemeToEditor","label":"Function","file_path":"app/src/components/Common/Editor/hooks/useEditor.ts","start_line":81,"end_line":92,"rank":-15.49805474949816},{"name":"setSettings","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.components.BlinkoShareDialog.setSettings","label":"Function","file_path":"app/src/components/BlinkoShareDialog/index.tsx","start_line":98,"end_line":100,"rank":-15.04954725468125},{"name":"initializeSettings","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.user.UserStore.initializeSettings","label":"Method","file_path":"app/src/store/user.ts","start_line":205,"end_line":335,"rank":-15.04954725468125},{"name":"ThemeSwitcherProps","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.components.Common.Theme.ThemeSwitcher.ThemeSwitcherProps","label":"Interface","file_path":"app/src/components/Common/Theme/ThemeSwitcher.tsx","start_line":8,"end_line":10,"rank":-14.252390194192628},{"name":"main","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.build.main","label":"Function","file_path":"app/src-tauri/build.rs","start_line":1,"end_line":3,"rank":-13.740676609060964},{"name":"run","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.lib.run","label":"Function","file_path":"app/src-tauri/src/lib.rs","start_line":8,"end_line":105,"rank":-13.646365217368864},{"name":"main","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.main.main","label":"Function","file_path":"app/src-tauri/src/main.rs","start_line":4,"end_line":6,"rank":-13.646365217368864},{"name":"main","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.build.main","label":"Function","file_path":"app/tauri-plugin-blinko/build.rs","start_line":3,"end_line":8,"rank":-13.646365217368864},{"name":"setcolor","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.src.commands.setcolor","label":"Function","file_path":"app/tauri-plugin-blinko/src/commands.rs","start_line":8,"end_line":13,"rank":-13.55669250439031},{"name":"init","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.src.desktop.init","label":"Function","file_path":"app/tauri-plugin-blinko/src/desktop.rs","start_line":6,"end_line":11,"rank":-13.55669250439031},{"name":"init","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.src.lib.init","label":"Function","file_path":"app/tauri-plugin-blinko/src/lib.rs","start_line":36,"end_line":51,"rank":-13.55669250439031},{"name":"init","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.src.mobile.init","label":"Function","file_path":"app/tauri-plugin-blinko/src/mobile.rs","start_line":13,"end_line":22,"rank":-13.55669250439031},{"name":"default","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.hotkey.HotkeyConfig.default","label":"Method","file_path":"app/src-tauri/src/desktop/hotkey.rs","start_line":32,"end_line":41,"rank":-13.51349000518816},{"name":"default","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.hotkey.WindowConfig.default","label":"Method","file_path":"app/src-tauri/src/desktop/hotkey.rs","start_line":45,"end_line":56,"rank":-13.51349000518816},{"name":"setcolor","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.src.desktop.Blinko.setcolor","label":"Method","file_path":"app/tauri-plugin-blinko/src/desktop.rs","start_line":17,"end_line":19,"rank":-13.51349000518816},{"name":"serialize","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.src.error.Error.serialize","label":"Method","file_path":"app/tauri-plugin-blinko/src/error.rs","start_line":15,"end_line":20,"rank":-13.51349000518816},{"name":"blinko","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.src.lib.BlinkoExt.blinko","label":"Method","file_path":"app/tauri-plugin-blinko/src/lib.rs","start_line":26,"end_line":26,"rank":-13.51349000518816},{"name":"blinko","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.src.lib.T.blinko","label":"Method","file_path":"app/tauri-plugin-blinko/src/lib.rs","start_line":30,"end_line":32,"rank":-13.51349000518816},{"name":"setcolor","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.src.mobile.Blinko.setcolor","label":"Method","file_path":"app/tauri-plugin-blinko/src/mobile.rs","start_line":28,"end_line":33,"rank":-13.51349000518816},{"name":"register_hotkey","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.hotkey.register_hotkey","label":"Function","file_path":"app/src-tauri/src/desktop/hotkey.rs","start_line":66,"end_line":93,"rank":-13.4301589666807},{"name":"unregister_hotkey","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.hotkey.unregister_hotkey","label":"Function","file_path":"app/src-tauri/src/desktop/hotkey.rs","start_line":96,"end_line":120,"rank":-13.4301589666807},{"name":"setup_app","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.setup.setup_app","label":"Function","file_path":"app/src-tauri/src/desktop/setup.rs","start_line":10,"end_line":65,"rank":-13.4301589666807},{"name":"shortcuts_match","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.setup.shortcuts_match","label":"Function","file_path":"app/src-tauri/src/desktop/setup.rs","start_line":68,"end_line":106,"rank":-13.4301589666807}],"has_more":true}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.search_graph
>
> ```json
> {
>   "project": "D-rust-tarui-demo-kanri-main",
>   "query": "kanban theme settings shortcut tray Tauri",
>   "limit": 40,
>   "include_connected": true
> }
> ```
>
> ```text
> {"total":29,"search_mode":"bm25","results":[{"name":"loadThemeSettings","qualified_name":"D-rust-tarui-demo-kanri-main.stores.theme.loadThemeSettings","label":"Function","file_path":"stores/theme.ts","start_line":36,"end_line":48,"rank":-19.529348477299127},{"name":"setTheme","qualified_name":"D-rust-tarui-demo-kanri-main.stores.theme.setTheme","label":"Function","file_path":"stores/theme.ts","start_line":50,"end_line":65,"rank":-16.321023545929634},{"name":"toggleAutoTheme","qualified_name":"D-rust-tarui-demo-kanri-main.stores.theme.toggleAutoTheme","label":"Function","file_path":"stores/theme.ts","start_line":67,"end_line":80,"rank":-16.247181668053557},{"name":"state","qualified_name":"D-rust-tarui-demo-kanri-main.stores.theme.state","label":"Function","file_path":"stores/theme.ts","start_line":27,"end_line":34,"rank":-15.675756149429315},{"name":"loadSettings","qualified_name":"D-rust-tarui-demo-kanri-main.stores.settings.loadSettings","label":"Function","file_path":"stores/settings.ts","start_line":59,"end_line":91,"rank":-15.123901235124983},{"name":"state","qualified_name":"D-rust-tarui-demo-kanri-main.stores.settings.state","label":"Function","file_path":"stores/settings.ts","start_line":25,"end_line":56,"rank":-14.600839362963008},{"name":"setLocale","qualified_name":"D-rust-tarui-demo-kanri-main.stores.settings.setLocale","label":"Function","file_path":"stores/settings.ts","start_line":117,"end_line":120,"rank":-14.528741284875686},{"name":"deleteAllData","qualified_name":"D-rust-tarui-demo-kanri-main.stores.settings.deleteAllData","label":"Function","file_path":"stores/settings.ts","start_line":111,"end_line":115,"rank":-14.45886798849343},{"name":"setAnimationsEnabled","qualified_name":"D-rust-tarui-demo-kanri-main.stores.settings.setAnimationsEnabled","label":"Function","file_path":"stores/settings.ts","start_line":122,"end_line":125,"rank":-14.45886798849343},{"name":"setAutostartEnabled","qualified_name":"D-rust-tarui-demo-kanri-main.stores.settings.setAutostartEnabled","label":"Function","file_path":"stores/settings.ts","start_line":127,"end_line":133,"rank":-14.45886798849343},{"name":"setDisableSpellcheck","qualified_name":"D-rust-tarui-demo-kanri-main.stores.settings.setDisableSpellcheck","label":"Function","file_path":"stores/settings.ts","start_line":135,"end_line":138,"rank":-14.45886798849343},{"name":"setReverseSorting","qualified_name":"D-rust-tarui-demo-kanri-main.stores.settings.setReverseSorting","label":"Function","file_path":"stores/settings.ts","start_line":165,"end_line":168,"rank":-14.45886798849343},{"name":"loadBoardSortingOptions","qualified_name":"D-rust-tarui-demo-kanri-main.stores.settings.loadBoardSortingOptions","label":"Function","file_path":"stores/settings.ts","start_line":93,"end_line":109,"rank":-14.391118060991541},{"name":"setColumnZoomLevel","qualified_name":"D-rust-tarui-demo-kanri-main.stores.settings.setColumnZoomLevel","label":"Function","file_path":"stores/settings.ts","start_line":140,"end_line":143,"rank":-14.391118060991541},{"name":"setBoardSortingOption","qualified_name":"D-rust-tarui-demo-kanri-main.stores.settings.setBoardSortingOption","label":"Function","file_path":"stores/settings.ts","start_line":160,"end_line":163,"rank":-14.391118060991541},{"name":"setDisplayColumnCardCountEnabled","qualified_name":"D-rust-tarui-demo-kanri-main.stores.settings.setDisplayColumnCardCountEnabled","label":"Function","file_path":"stores/settings.ts","start_line":150,"end_line":153,"rank":-14.261612570609234},{"name":"setDefaultRelativeDueDatesEnabled","qualified_name":"D-rust-tarui-demo-kanri-main.stores.settings.setDefaultRelativeDueDatesEnabled","label":"Function","file_path":"stores/settings.ts","start_line":155,"end_line":158,"rank":-14.261612570609234},{"name":"setAddToTopOfColumnButtonEnabled","qualified_name":"D-rust-tarui-demo-kanri-main.stores.settings.setAddToTopOfColumnButtonEnabled","label":"Function","file_path":"stores/settings.ts","start_line":145,"end_line":148,"rank":-14.139527150422358},{"name":"loadLayoutSettings","qualified_name":"D-rust-tarui-demo-kanri-main.stores.layout.loadLayoutSettings","label":"Function","file_path":"stores/layout.ts","start_line":46,"end_line":53,"rank":-13.28216680924557},{"name":"main","qualified_name":"D-rust-tarui-demo-kanri-main.src-tauri.build.main","label":"Function","file_path":"src-tauri/build.rs","start_line":7,"end_line":9,"rank":-11.674345960990252},{"name":"run","qualified_name":"D-rust-tarui-demo-kanri-main.src-tauri.src.lib.run","label":"Function","file_path":"src-tauri/src/lib.rs","start_line":17,"end_line":62,"rank":-11.624226061508864},{"name":"main","qualified_name":"D-rust-tarui-demo-kanri-main.src-tauri.src.main.main","label":"Function","file_path":"src-tauri/src/main.rs","start_line":9,"end_line":11,"rank":-11.624226061508864},{"name":"https://github.com/trobonox/kanri","qualified_name":"__route__infra__https://github.com/trobonox/kanri","label":"Route","file_path":"src-tauri/Cargo.toml","start_line":0,"end_line":0,"rank":-9.106784544984162},{"name":"package","qualified_name":"D-rust-tarui-demo-kanri-main.src-tauri.Cargo.package","label":"Class","file_path":"src-tauri/Cargo.toml","start_line":5,"end_line":16,"rank":-6.6743459609902525},{"name":"lib","qualified_name":"D-rust-tarui-demo-kanri-main.src-tauri.Cargo.lib","label":"Class","file_path":"src-tauri/Cargo.toml","start_line":16,"end_line":22,"rank":-6.6743459609902525},{"name":"dependencies","qualified_name":"D-rust-tarui-demo-kanri-main.src-tauri.Cargo.dependencies","label":"Class","file_path":"src-tauri/Cargo.toml","start_line":25,"end_line":38,"rank":-6.6743459609902525},{"name":"features","qualified_name":"D-rust-tarui-demo-kanri-main.src-tauri.Cargo.features","label":"Class","file_path":"src-tauri/Cargo.toml","start_line":38,"end_line":46,"rank":-6.6743459609902525},{"name":"build-dependencies","qualified_name":"D-rust-tarui-demo-kanri-main.src-tauri.Cargo.build-dependencies","label":"Class","file_path":"src-tauri/Cargo.toml","start_line":22,"end_line":25,"rank":-6.600274736394647},{"name":"target.\"cfg(not(any(target_os = \\\"android\\\", target_os = \\\"ios\\\")))\".dependencies","qualified_name":"D-rust-tarui-demo-kanri-main.src-tauri.Cargo.target.\"cfg(not(any(target_os = \\\"android\\\", target_os = \\\"ios\\\")))\".dependencies","label":"Class","file_path":"src-tauri/Cargo.toml","start_line":46,"end_line":50,"rank":-6.144565758612888}],"has_more":false}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.search_graph
>
> ```json
> {
>   "project": "D-rust-tarui-demo-spacedrive-main",
>   "query": "desktop tray theme settings communication logs date",
>   "limit": 40,
>   "include_connected": true
> }
> ```
>
> ```text
> {"total":173,"search_mode":"bm25","results":[{"name":"handleThemeChange","qualified_name":"D-rust-tarui-demo-spacedrive-main.packages.interface.src.Settings.pages.AppearanceSettings.handleThemeChange","label":"Function","file_path":"packages/interface/src/Settings/pages/AppearanceSettings.tsx","start_line":39,"end_line":46,"rank":-24.64146139775579},{"name":"dev-desktop","qualified_name":"D-rust-tarui-demo-spacedrive-main.justfile.dev-desktop","label":"Function","file_path":"justfile","start_line":13,"end_line":16,"rank":-22.025941200624217},{"name":"run_logs_show","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.cli.src.domains.logs.mod.run_logs_show","label":"Function","file_path":"apps/cli/src/domains/logs/mod.rs","start_line":20,"end_line":45,"rank":-20.950415566931774},{"name":"run_logs_follow","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.cli.src.domains.logs.mod.run_logs_follow","label":"Function","file_path":"apps/cli/src/domains/logs/mod.rs","start_line":48,"end_line":132,"rank":-20.950415566931774},{"name":"show_recent_historical_logs","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.cli.src.domains.logs.mod.show_recent_historical_logs","label":"Function","file_path":"apps/cli/src/domains/logs/mod.rs","start_line":300,"end_line":334,"rank":-20.77223387973012},{"name":"/Desktop/file.txt","qualified_name":"__route__ANY__/Desktop/file.txt","label":"Route","file_path":"","start_line":0,"end_line":0,"rank":-20.363522538325252},{"name":"subscribe_logs","qualified_name":"D-rust-tarui-demo-spacedrive-main.core.src.client.mod.CoreClient.subscribe_logs","label":"Method","file_path":"core/src/client/mod.rs","start_line":103,"end_line":120,"rank":-20.109670623290544},{"name":"stream_logs","qualified_name":"D-rust-tarui-demo-spacedrive-main.core.src.client.mod.LogStream.stream_logs","label":"Method","file_path":"core/src/client/mod.rs","start_line":210,"end_line":224,"rank":-20.109670623290544},{"name":"collapse_logs","qualified_name":"D-rust-tarui-demo-spacedrive-main.crates.log-analyzer.src.collapse.collapse_logs","label":"Function","file_path":"crates/log-analyzer/src/collapse.rs","start_line":11,"end_line":49,"rank":-20.024623570746506},{"name":"logs_dir","qualified_name":"D-rust-tarui-demo-spacedrive-main.core.src.config.app_config.AppConfig.logs_dir","label":"Method","file_path":"core/src/config/app_config.rs","start_line":297,"end_line":299,"rank":-19.940995488570533},{"name":"stream_logs","qualified_name":"D-rust-tarui-demo-spacedrive-main.core.src.infra.daemon.client.DaemonClient.stream_logs","label":"Method","file_path":"core/src/infra/daemon/client.rs","start_line":111,"end_line":166,"rank":-19.940995488570533},{"name":"copy_logs","qualified_name":"D-rust-tarui-demo-spacedrive-main.core.tests.helpers.sync_harness.SnapshotCapture.copy_logs","label":"Method","file_path":"core/tests/helpers/sync_harness.rs","start_line":611,"end_line":631,"rank":-19.940995488570533},{"name":"logs_path","qualified_name":"D-rust-tarui-demo-spacedrive-main.core.tests.helpers.test_data.TestDataDir.logs_path","label":"Method","file_path":"core/tests/helpers/test_data.rs","start_line":163,"end_line":165,"rank":-19.940995488570533},{"name":"job_logs_dir","qualified_name":"D-rust-tarui-demo-spacedrive-main.core.src.library.mod.Library.job_logs_dir","label":"Method","file_path":"core/src/library/mod.rs","start_line":452,"end_line":454,"rank":-19.858751158277393},{"name":"/Users/jamie/Desktop/file.txt","qualified_name":"__route__ANY__/Users/jamie/Desktop/file.txt","label":"Route","file_path":"","start_line":0,"end_line":0,"rank":-19.706304942449556},{"name":"parse_date","qualified_name":"D-rust-tarui-demo-spacedrive-main.adapters.gmail.sync.parse_date","label":"Function","file_path":"adapters/gmail/sync.py","start_line":163,"end_line":171,"rank":-19.684691318885587},{"name":"normalize_date","qualified_name":"D-rust-tarui-demo-spacedrive-main.adapters.obsidian.sync.normalize_date","label":"Function","file_path":"adapters/obsidian/sync.py","start_line":25,"end_line":56,"rank":-19.684691318885587},{"name":"Settings","qualified_name":"D-rust-tarui-demo-spacedrive-main.packages.interface.src.routes.settings.Settings","label":"Function","file_path":"packages/interface/src/routes/settings/index.tsx","start_line":133,"end_line":154,"rank":-19.512366318013196},{"name":"categorize_date","qualified_name":"D-rust-tarui-demo-spacedrive-main.core.src.ops.search.facets.FacetBuilder.categorize_date","label":"Method","file_path":"core/src/ops/search/facets.rs","start_line":57,"end_line":72,"rank":-19.280729935323947},{"name":"date_range","qualified_name":"D-rust-tarui-demo-spacedrive-main.core.src.ops.search.filters.FilterBuilder.date_range","label":"Method","file_path":"core/src/ops/search/filters.rs","start_line":41,"end_line":58,"rank":-19.280729935323947},{"name":"categorize_date","qualified_name":"D-rust-tarui-demo-spacedrive-main.core.src.ops.search.output.SearchFacets.categorize_date","label":"Method","file_path":"core/src/ops/search/output.rs","start_line":305,"end_line":320,"rank":-19.280729935323947},{"name":"run","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.cli.src.domains.logs.mod.run","label":"Function","file_path":"apps/cli/src/domains/logs/mod.rs","start_line":12,"end_line":17,"rank":-18.913882425850687},{"name":"test_ephemeral_search_date_filter","qualified_name":"D-rust-tarui-demo-spacedrive-main.core.tests.search_test.test_ephemeral_search_date_filter","label":"Function","file_path":"core/tests/search_test.rs","start_line":584,"end_line":641,"rank":-18.838339242873964},{"name":"level_matches","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.cli.src.domains.logs.mod.level_matches","label":"Function","file_path":"apps/cli/src/domains/logs/mod.rs","start_line":135,"end_line":146,"rank":-18.6235990544426},{"name":"format_timestamp","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.cli.src.domains.logs.mod.format_timestamp","label":"Function","file_path":"apps/cli/src/domains/logs/mod.rs","start_line":149,"end_line":155,"rank":-18.6235990544426},{"name":"colorize_level","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.cli.src.domains.logs.mod.colorize_level","label":"Function","file_path":"apps/cli/src/domains/logs/mod.rs","start_line":158,"end_line":167,"rank":-18.6235990544426},{"name":"SettingsSidebar","qualified_name":"D-rust-tarui-demo-spacedrive-main.packages.interface.src.routes.settings.SettingsSidebar","label":"Function","file_path":"packages/interface/src/routes/settings/index.tsx","start_line":32,"end_line":57,"rank":-18.443086265103688},{"name":"SettingsContent","qualified_name":"D-rust-tarui-demo-spacedrive-main.packages.interface.src.routes.settings.SettingsContent","label":"Function","file_path":"packages/interface/src/routes/settings/index.tsx","start_line":63,"end_line":84,"rank":-18.443086265103688},{"name":"SettingsScreen","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.mobile.src.screens.settings.SettingsScreen.SettingsScreen","label":"Function","file_path":"apps/mobile/src/screens/settings/SettingsScreen.tsx","start_line":20,"end_line":696,"rank":-18.372059262545577},{"name":"AboutSettings","qualified_name":"D-rust-tarui-demo-spacedrive-main.packages.interface.src.Settings.pages.AboutSettings.AboutSettings","label":"Function","file_path":"packages/interface/src/Settings/pages/AboutSettings.tsx","start_line":8,"end_line":144,"rank":-18.372059262545577},{"name":"AdvancedSettings","qualified_name":"D-rust-tarui-demo-spacedrive-main.packages.interface.src.Settings.pages.AdvancedSettings.AdvancedSettings","label":"Function","file_path":"packages/interface/src/Settings/pages/AdvancedSettings.tsx","start_line":10,"end_line":107,"rank":-18.372059262545577},{"name":"AppearanceSettings","qualified_name":"D-rust-tarui-demo-spacedrive-main.packages.interface.src.Settings.pages.AppearanceSettings.AppearanceSettings","label":"Function","file_path":"packages/interface/src/Settings/pages/AppearanceSettings.tsx","start_line":23,"end_line":155,"rank":-18.372059262545577},{"name":"GeneralSettings","qualified_name":"D-rust-tarui-demo-spacedrive-main.packages.interface.src.Settings.pages.GeneralSettings.GeneralSettings","label":"Function","file_path":"packages/interface/src/Settings/pages/GeneralSettings.tsx","start_line":9,"end_line":152,"rank":-18.372059262545577},{"name":"IndexerSettings","qualified_name":"D-rust-tarui-demo-spacedrive-main.packages.interface.src.Settings.pages.IndexerSettings.IndexerSettings","label":"Function","file_path":"packages/interface/src/Settings/pages/IndexerSettings.tsx","start_line":13,"end_line":166,"rank":-18.372059262545577},{"name":"LibrarySettings","qualified_name":"D-rust-tarui-demo-spacedrive-main.packages.interface.src.Settings.pages.LibrarySettings.LibrarySettings","label":"Function","file_path":"packages/interface/src/Settings/pages/LibrarySettings.tsx","start_line":14,"end_line":191,"rank":-18.372059262545577},{"name":"PrivacySettings","qualified_name":"D-rust-tarui-demo-spacedrive-main.packages.interface.src.Settings.pages.PrivacySettings.PrivacySettings","label":"Function","file_path":"packages/interface/src/Settings/pages/PrivacySettings.tsx","start_line":8,"end_line":78,"rank":-18.372059262545577},{"name":"ServicesSettings","qualified_name":"D-rust-tarui-demo-spacedrive-main.packages.interface.src.Settings.pages.ServicesSettings.ServicesSettings","label":"Function","file_path":"packages/interface/src/Settings/pages/ServicesSettings.tsx","start_line":11,"end_line":111,"rank":-18.372059262545577},{"name":"SettingsContentWrapper","qualified_name":"D-rust-tarui-demo-spacedrive-main.packages.interface.src.routes.settings.SettingsContentWrapper","label":"Function","file_path":"packages/interface/src/routes/settings/index.tsx","start_line":86,"end_line":127,"rank":-18.372059262545577},{"name":"read_last_lines","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.cli.src.domains.logs.mod.read_last_lines","label":"Function","file_path":"apps/cli/src/domains/logs/mod.rs","start_line":228,"end_line":247,"rank":-18.351625747753502},{"name":"format_log_line","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.cli.src.domains.logs.mod.format_log_line","label":"Function","file_path":"apps/cli/src/domains/logs/mod.rs","start_line":250,"end_line":297,"rank":-18.351625747753502}],"has_more":true}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.search_graph
>
> ```json
> {
>   "project": "D-rust-tarui-demo-voicetypr-main",
>   "query": "tray settings theme log communication hotkey notifications",
>   "limit": 40,
>   "include_connected": true
> }
> ```
>
> ```text
> {"total":477,"search_mode":"bm25","results":[{"name":"apply_tray_theme_icon","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.lib.apply_tray_theme_icon","label":"Function","file_path":"src-tauri/src/lib.rs","start_line":104,"end_line":116,"rank":-28.727862657459156},{"name":"test_theme_values","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.tests.settings_commands.test_theme_values","label":"Function","file_path":"src-tauri/src/tests/settings_commands.rs","start_line":189,"end_line":199,"rank":-26.046811668996},{"name":"log_hotkey_registration","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.utils.onboarding_logger.OnboardingLogger.log_hotkey_registration","label":"Method","file_path":"src-tauri/src/utils/onboarding_logger.rs","start_line":181,"end_line":202,"rank":-23.836774258596698},{"name":"test_valid_hotkey_formats","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.tests.settings_commands.test_valid_hotkey_formats","label":"Function","file_path":"src-tauri/src/tests/settings_commands.rs","start_line":169,"end_line":186,"rank":-21.827104655084334},{"name":"test_ptt_hotkey_none","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.tests.settings_commands.test_ptt_hotkey_none","label":"Function","file_path":"src-tauri/src/tests/settings_commands.rs","start_line":611,"end_line":618,"rank":-21.827104655084334},{"name":"test_hotkey_validation_edge_cases","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.tests.settings_commands.test_hotkey_validation_edge_cases","label":"Function","file_path":"src-tauri/src/tests/settings_commands.rs","start_line":285,"end_line":300,"rank":-21.53817076641992},{"name":"update_tray_menu","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.settings.update_tray_menu","label":"Function","file_path":"src-tauri/src/commands/settings.rs","start_line":1297,"end_line":1299,"rank":-21.10990733279279},{"name":"set_model_from_tray","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.settings.set_model_from_tray","label":"Function","file_path":"src-tauri/src/commands/settings.rs","start_line":1066,"end_line":1283,"rank":-20.823573582492717},{"name":"next_tray_menu_generation","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.settings.next_tray_menu_generation","label":"Function","file_path":"src-tauri/src/commands/settings.rs","start_line":1287,"end_line":1289,"rank":-20.823573582492717},{"name":"current_tray_menu_generation","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.settings.current_tray_menu_generation","label":"Function","file_path":"src-tauri/src/commands/settings.rs","start_line":1292,"end_line":1294,"rank":-20.823573582492717},{"name":"frontend_log","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.settings.frontend_log","label":"Function","file_path":"src-tauri/src/commands/settings.rs","start_line":192,"end_line":194,"rank":-20.65993450952316},{"name":"update_tray_menu_with_generation","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.settings.update_tray_menu_with_generation","label":"Function","file_path":"src-tauri/src/commands/settings.rs","start_line":1303,"end_line":1362,"rank":-20.551867657248167},{"name":"saveHotkeySettings","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.components.onboarding.OnboardingDesktop.saveHotkeySettings","label":"Function","file_path":"src/components/onboarding/OnboardingDesktop.tsx","start_line":611,"end_line":693,"rank":-19.078006557445363},{"name":"canonicalizeHotkey","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.lib.hotkey-conflicts.canonicalizeHotkey","label":"Function","file_path":"src/lib/hotkey-conflicts.ts","start_line":82,"end_line":98,"rank":-18.535832826539064},{"name":"formatHotkey","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.lib.hotkey-utils.formatHotkey","label":"Function","file_path":"src/lib/hotkey-utils.tsx","start_line":8,"end_line":96,"rank":-18.535832826539064},{"name":"simulate_hotkey_registration","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.tests.panic_prevention_tests.simulate_hotkey_registration","label":"Function","file_path":"src-tauri/src/tests/panic_prevention_tests.rs","start_line":352,"end_line":358,"rank":-17.819216576607666},{"name":"duplicate_existing_hotkey_rejected","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.tests.shortcut_bindings.duplicate_existing_hotkey_rejected","label":"Function","file_path":"src-tauri/src/tests/shortcut_bindings.rs","start_line":117,"end_line":141,"rank":-17.759991001911423},{"name":"build_tray_menu","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.menu.tray.build_tray_menu","label":"Function","file_path":"src-tauri/src/menu/tray.rs","start_line":109,"end_line":584,"rank":-17.708676233871792},{"name":"theme","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.settings.Settings.theme","label":"Field","file_path":"src-tauri/src/commands/settings.rs","start_line":43,"end_line":43,"rank":-17.621468347239215},{"name":"format_tray_model_label","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.menu.tray.format_tray_model_label","label":"Function","file_path":"src-tauri/src/menu/tray.rs","start_line":70,"end_line":81,"rank":-17.58737217139223},{"name":"getAllConflicts","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.lib.hotkey-conflicts.getAllConflicts","label":"Function","file_path":"src/lib/hotkey-conflicts.ts","start_line":158,"end_line":185,"rank":-17.529219584479698},{"name":"formatConflictMessage","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.lib.hotkey-conflicts.formatConflictMessage","label":"Function","file_path":"src/lib/hotkey-conflicts.ts","start_line":192,"end_line":198,"rank":-17.529219584479698},{"name":"test_app_continues_without_hotkey","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.tests.panic_prevention_tests.test_app_continues_without_hotkey","label":"Function","file_path":"src-tauri/src/tests/panic_prevention_tests.rs","start_line":38,"end_line":60,"rank":-17.476829929973903},{"name":"checkForSystemConflict","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.lib.hotkey-conflicts.checkForSystemConflict","label":"Function","file_path":"src/lib/hotkey-conflicts.ts","start_line":105,"end_line":151,"rank":-17.447124966283848},{"name":"should_include_remote_connection_in_tray","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.menu.tray.should_include_remote_connection_in_tray","label":"Function","file_path":"src-tauri/src/menu/tray.rs","start_line":11,"end_line":13,"rank":-17.355867668119856},{"name":"nonempty_hotkey_disables_stale_isolated_tap_primary","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.trigger.engine_host.nonempty_hotkey_disables_stale_isolated_tap_primary","label":"Function","file_path":"src-tauri/src/trigger/engine_host.rs","start_line":555,"end_line":577,"rank":-17.264757867055753},{"name":"empty_hotkey_with_bare_primary_synthesizes_no_combo","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.trigger.engine_host.empty_hotkey_with_bare_primary_synthesizes_no_combo","label":"Function","file_path":"src-tauri/src/trigger/engine_host.rs","start_line":583,"end_line":605,"rank":-17.11343430791625},{"name":"stale_primary_candidate_returns_none_for_empty_hotkey","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.trigger.engine_host.stale_primary_candidate_returns_none_for_empty_hotkey","label":"Function","file_path":"src-tauri/src/trigger/engine_host.rs","start_line":675,"end_line":685,"rank":-17.11343430791625},{"name":"test_self_connection_is_hidden_from_tray","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.tests.tray_menu_tests.test_self_connection_is_hidden_from_tray","label":"Function","file_path":"src-tauri/src/tests/tray_menu_tests.rs","start_line":256,"end_line":260,"rank":-17.103020601261417},{"name":"test_online_connection_is_shown_in_tray","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.tests.tray_menu_tests.test_online_connection_is_shown_in_tray","label":"Function","file_path":"src-tauri/src/tests/tray_menu_tests.rs","start_line":263,"end_line":267,"rank":-17.103020601261417},{"name":"test_latest_log_attachment_no_log_state","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.tests.log_commands_tests.test_latest_log_attachment_no_log_state","label":"Function","file_path":"src-tauri/src/tests/log_commands_tests.rs","start_line":302,"end_line":314,"rank":-17.012191842658012},{"name":"nonempty_hotkey_disables_stale_bare_primary_and_synthesizes_combo","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.trigger.engine_host.nonempty_hotkey_disables_stale_bare_primary_and_synthesizes_combo","label":"Function","file_path":"src-tauri/src/trigger/engine_host.rs","start_line":507,"end_line":533,"rank":-16.96828619864971},{"name":"test_tray_label_when_onboarding_not_completed_returns_none","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.tests.tray_onboarding_tests.test_tray_label_when_onboarding_not_completed_returns_none","label":"Function","file_path":"src-tauri/src/tests/tray_onboarding_tests.rs","start_line":4,"end_line":11,"rank":-16.899733629095667},{"name":"test_tray_label_when_onboarding_complete_uses_display_name","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.tests.tray_onboarding_tests.test_tray_label_when_onboarding_complete_uses_display_name","label":"Function","file_path":"src-tauri/src/tests/tray_onboarding_tests.rs","start_line":24,"end_line":31,"rank":-16.899733629095667},{"name":"test_tray_label_humanizes_known_model_id_when_display_name_missing","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.tests.tray_onboarding_tests.test_tray_label_humanizes_known_model_id_when_display_name_missing","label":"Function","file_path":"src-tauri/src/tests/tray_onboarding_tests.rs","start_line":34,"end_line":37,"rank":-16.70775896280727},{"name":"test_latest_log_attachment_serialization","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.tests.log_commands_tests.test_latest_log_attachment_serialization","label":"Function","file_path":"src-tauri/src/tests/log_commands_tests.rs","start_line":285,"end_line":299,"rank":-16.531990497623447},{"name":"test_read_log_tail_empty_file","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.tests.log_commands_tests.test_read_log_tail_empty_file","label":"Function","file_path":"src-tauri/src/tests/log_commands_tests.rs","start_line":112,"end_line":121,"rank":-16.434325672463537},{"name":"log","qualified_name":"D-rust-tarui-demo-voicetypr-main.sidecar.parakeet-swift.Sources.main.log","label":"Function","file_path":"sidecar/parakeet-swift/Sources/main.swift","start_line":36,"end_line":39,"rank":-16.400446016283105},{"name":"log_transcription","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.telemetry.log_transcription","label":"Function","file_path":"src-tauri/src/telemetry.rs","start_line":586,"end_line":588,"rank":-16.347993802612365},{"name":"scrub_log","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.telemetry.scrub_log","label":"Function","file_path":"src-tauri/src/telemetry.rs","start_line":668,"end_line":705,"rank":-16.347993802612365}],"has_more":true}
> ```
>
> - Ran `rtk proxy git -C D:\rust\tarui-demo\anarlog-main remote -v`
> - Ran `rtk proxy git -C D:\rust\tarui-demo\voicetypr-main remote -v`
> - Ran `rtk proxy git -C D:\rust\tarui-demo\kanri-main remote -v`
> - Ran `rtk proxy git -C D:\rust\tarui-demo\blinko-main remote -v`
> - Ran `rtk proxy git -C D:\rust\tarui-demo\spacedrive-main remote -v`
> - Ran `rtk git -C D:\rust\tarui-demo\kanri-main remote -v`
> - Ran `rtk git -C D:\rust\tarui-demo\blinko-main remote -v`
> - Ran `rtk git -C D:\rust\tarui-demo\anarlog-main remote -v`
> - Ran `rtk git -C D:\rust\tarui-demo\voicetypr-main remote -v`
> - Ran `rtk git -C D:\rust\tarui-demo\spacedrive-main remote -v`
>
> MCP tool call
>
> codebase-memory-mcp.search_graph
>
> ```json
> {
>   "project": "D-rust-tarui-demo-blinko-main",
>   "query": "Tauri system tray desktop notification",
>   "limit": 12,
>   "include_connected": true
> }
> ```
>
> ```text
> {"total":143,"search_mode":"bm25","results":[{"name":"setup_system_tray","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.tray.setup_system_tray","label":"Function","file_path":"app/src-tauri/src/desktop/tray.rs","start_line":14,"end_line":79,"rank":-40.38759526117133},{"name":"system_tray_enabled","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.hotkey.HotkeyConfig.system_tray_enabled","label":"Field","file_path":"app/src-tauri/src/desktop/hotkey.rs","start_line":18,"end_line":18,"rank":-28.60412245454493},{"name":"set_desktop_theme","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.window.set_desktop_theme","label":"Function","file_path":"app/src-tauri/src/desktop/window.rs","start_line":240,"end_line":274,"rank":-20.206853724918005},{"name":"set_desktop_colors","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.window.set_desktop_colors","label":"Function","file_path":"app/src-tauri/src/desktop/window.rs","start_line":277,"end_line":314,"rank":-20.206853724918005},{"name":"CreateNotification","qualified_name":"D-rust-tarui-demo-blinko-main.server.routerTrpc.notification.CreateNotification","label":"Function","file_path":"server/routerTrpc/notification.ts","start_line":6,"end_line":30,"rank":-19.992475228149218},{"name":"SystemTrayConfig","qualified_name":"D-rust-tarui-demo-blinko-main.shared.lib.types.SystemTrayConfig","label":"Interface","file_path":"shared/lib/types.ts","start_line":319,"end_line":324,"rank":-19.750089556897798},{"name":"init","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.src.desktop.init","label":"Function","file_path":"app/tauri-plugin-blinko/src/desktop.rs","start_line":6,"end_line":11,"rank":-19.097801005012652},{"name":"default","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.hotkey.HotkeyConfig.default","label":"Method","file_path":"app/src-tauri/src/desktop/hotkey.rs","start_line":32,"end_line":41,"rank":-18.987291665176492},{"name":"default","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.hotkey.WindowConfig.default","label":"Method","file_path":"app/src-tauri/src/desktop/hotkey.rs","start_line":45,"end_line":56,"rank":-18.987291665176492},{"name":"setcolor","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.src.desktop.Blinko.setcolor","label":"Method","file_path":"app/tauri-plugin-blinko/src/desktop.rs","start_line":17,"end_line":19,"rank":-18.987291665176492},{"name":"register_hotkey","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.hotkey.register_hotkey","label":"Function","file_path":"app/src-tauri/src/desktop/hotkey.rs","start_line":66,"end_line":93,"rank":-18.774135986144334},{"name":"unregister_hotkey","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.hotkey.unregister_hotkey","label":"Function","file_path":"app/src-tauri/src/desktop/hotkey.rs","start_line":96,"end_line":120,"rank":-18.774135986144334}],"has_more":true}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.search_graph
>
> ```json
> {
>   "project": "D-rust-tarui-demo-blinko-main",
>   "query": "settings theme user preference",
>   "limit": 12,
>   "include_connected": true
> }
> ```
>
> ```text
> {"total":64,"search_mode":"bm25","results":[{"name":"saveThemePreference","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.android.src.main.java.Blinko.Blinko.saveThemePreference","label":"Method","file_path":"app/tauri-plugin-blinko/android/src/main/java/Blinko.kt","start_line":84,"end_line":96,"rank":-22.380184535385492},{"name":"initializeSettings","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.user.UserStore.initializeSettings","label":"Method","file_path":"app/src/store/user.ts","start_line":205,"end_line":335,"rank":-21.573000637125418},{"name":"set_desktop_theme","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.window.set_desktop_theme","label":"Function","file_path":"app/src-tauri/src/desktop/window.rs","start_line":240,"end_line":274,"rank":-18.923957136014472},{"name":"getVisibleSettings","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.pages.settings.getVisibleSettings","label":"Function","file_path":"app/src/pages/settings.tsx","start_line":158,"end_line":182,"rank":-17.80936655788617},{"name":"open_app_settings","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.src.commands.open_app_settings","label":"Function","file_path":"app/tauri-plugin-blinko/src/commands.rs","start_line":16,"end_line":20,"rank":-17.254946762090995},{"name":"open_app_settings","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.src.desktop.Blinko.open_app_settings","label":"Method","file_path":"app/tauri-plugin-blinko/src/desktop.rs","start_line":21,"end_line":25,"rank":-17.191130683419864},{"name":"open_app_settings","qualified_name":"D-rust-tarui-demo-blinko-main.app.tauri-plugin-blinko.src.mobile.Blinko.open_app_settings","label":"Method","file_path":"app/tauri-plugin-blinko/src/mobile.rs","start_line":35,"end_line":40,"rank":-17.191130683419864},{"name":"getCurrentComponent","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.pages.settings.getCurrentComponent","label":"Function","file_path":"app/src/pages/settings.tsx","start_line":184,"end_line":187,"rank":-16.870589976310065},{"name":"constructor","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.user.UserStore.constructor","label":"Method","file_path":"app/src/store/user.ts","start_line":26,"end_line":28,"rank":-16.60673922021154},{"name":"id","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.user.UserStore.id","label":"Method","file_path":"app/src/store/user.ts","start_line":37,"end_line":39,"rank":-16.60673922021154},{"name":"name","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.user.UserStore.name","label":"Method","file_path":"app/src/store/user.ts","start_line":41,"end_line":43,"rank":-16.60673922021154},{"name":"nickname","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.user.UserStore.nickname","label":"Method","file_path":"app/src/store/user.ts","start_line":45,"end_line":47,"rank":-16.60673922021154}],"has_more":true}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.search_graph
>
> ```json
> {
>   "project": "D-rust-tarui-demo-kanri-main",
>   "query": "kanban board task drag drop date theme settings",
>   "limit": 12,
>   "include_connected": true
> }
> ```
>
> ```text
> {"total":33,"search_mode":"bm25","results":[{"name":"applyDrag","qualified_name":"D-rust-tarui-demo-kanri-main.utils.drag-n-drop.applyDrag","label":"Function","file_path":"utils/drag-n-drop.js","start_line":5,"end_line":56,"rank":-24.51495826585983},{"name":"loadThemeSettings","qualified_name":"D-rust-tarui-demo-kanri-main.stores.theme.loadThemeSettings","label":"Function","file_path":"stores/theme.ts","start_line":36,"end_line":48,"rank":-19.529348477299127},{"name":"loadBoardSortingOptions","qualified_name":"D-rust-tarui-demo-kanri-main.stores.settings.loadBoardSortingOptions","label":"Function","file_path":"stores/settings.ts","start_line":93,"end_line":109,"rank":-17.419019932980916},{"name":"setBoardSortingOption","qualified_name":"D-rust-tarui-demo-kanri-main.stores.settings.setBoardSortingOption","label":"Function","file_path":"stores/settings.ts","start_line":160,"end_line":163,"rank":-17.419019932980916},{"name":"setTheme","qualified_name":"D-rust-tarui-demo-kanri-main.stores.theme.setTheme","label":"Function","file_path":"stores/theme.ts","start_line":50,"end_line":65,"rank":-16.321023545929634},{"name":"toggleAutoTheme","qualified_name":"D-rust-tarui-demo-kanri-main.stores.theme.toggleAutoTheme","label":"Function","file_path":"stores/theme.ts","start_line":67,"end_line":80,"rank":-16.247181668053557},{"name":"setCardDueDate","qualified_name":"D-rust-tarui-demo-kanri-main.composables.useBoard.setCardDueDate","label":"Function","file_path":"composables/useBoard.ts","start_line":174,"end_line":189,"rank":-15.953162888984592},{"name":"state","qualified_name":"D-rust-tarui-demo-kanri-main.stores.theme.state","label":"Function","file_path":"stores/theme.ts","start_line":27,"end_line":34,"rank":-15.675756149429315},{"name":"loadSettings","qualified_name":"D-rust-tarui-demo-kanri-main.stores.settings.loadSettings","label":"Function","file_path":"stores/settings.ts","start_line":59,"end_line":91,"rank":-15.123901235124983},{"name":"state","qualified_name":"D-rust-tarui-demo-kanri-main.stores.settings.state","label":"Function","file_path":"stores/settings.ts","start_line":25,"end_line":56,"rank":-14.600839362963008},{"name":"setLocale","qualified_name":"D-rust-tarui-demo-kanri-main.stores.settings.setLocale","label":"Function","file_path":"stores/settings.ts","start_line":117,"end_line":120,"rank":-14.528741284875686},{"name":"deleteAllData","qualified_name":"D-rust-tarui-demo-kanri-main.stores.settings.deleteAllData","label":"Function","file_path":"stores/settings.ts","start_line":111,"end_line":115,"rank":-14.45886798849343}],"has_more":true}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.search_graph
>
> ```json
> {
>   "project": "D-rust-tarui-demo-kanri-main",
>   "query": "Tauri invoke event notification",
>   "limit": 12,
>   "include_connected": true
> }
> ```
>
> ```text
> {"total":10,"search_mode":"bm25","results":[{"name":"main","qualified_name":"D-rust-tarui-demo-kanri-main.src-tauri.build.main","label":"Function","file_path":"src-tauri/build.rs","start_line":7,"end_line":9,"rank":-11.674345960990252},{"name":"run","qualified_name":"D-rust-tarui-demo-kanri-main.src-tauri.src.lib.run","label":"Function","file_path":"src-tauri/src/lib.rs","start_line":17,"end_line":62,"rank":-11.624226061508864},{"name":"main","qualified_name":"D-rust-tarui-demo-kanri-main.src-tauri.src.main.main","label":"Function","file_path":"src-tauri/src/main.rs","start_line":9,"end_line":11,"rank":-11.624226061508864},{"name":"https://github.com/trobonox/kanri","qualified_name":"__route__infra__https://github.com/trobonox/kanri","label":"Route","file_path":"src-tauri/Cargo.toml","start_line":0,"end_line":0,"rank":-9.106784544984162},{"name":"package","qualified_name":"D-rust-tarui-demo-kanri-main.src-tauri.Cargo.package","label":"Class","file_path":"src-tauri/Cargo.toml","start_line":5,"end_line":16,"rank":-6.6743459609902525},{"name":"lib","qualified_name":"D-rust-tarui-demo-kanri-main.src-tauri.Cargo.lib","label":"Class","file_path":"src-tauri/Cargo.toml","start_line":16,"end_line":22,"rank":-6.6743459609902525},{"name":"dependencies","qualified_name":"D-rust-tarui-demo-kanri-main.src-tauri.Cargo.dependencies","label":"Class","file_path":"src-tauri/Cargo.toml","start_line":25,"end_line":38,"rank":-6.6743459609902525},{"name":"features","qualified_name":"D-rust-tarui-demo-kanri-main.src-tauri.Cargo.features","label":"Class","file_path":"src-tauri/Cargo.toml","start_line":38,"end_line":46,"rank":-6.6743459609902525},{"name":"build-dependencies","qualified_name":"D-rust-tarui-demo-kanri-main.src-tauri.Cargo.build-dependencies","label":"Class","file_path":"src-tauri/Cargo.toml","start_line":22,"end_line":25,"rank":-6.600274736394647},{"name":"target.\"cfg(not(any(target_os = \\\"android\\\", target_os = \\\"ios\\\")))\".dependencies","qualified_name":"D-rust-tarui-demo-kanri-main.src-tauri.Cargo.target.\"cfg(not(any(target_os = \\\"android\\\", target_os = \\\"ios\\\")))\".dependencies","label":"Class","file_path":"src-tauri/Cargo.toml","start_line":46,"end_line":50,"rank":-6.144565758612888}],"has_more":false}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.search_graph
>
> ```json
> {
>   "project": "D-rust-tarui-demo-spacedrive-main",
>   "query": "desktop native window system tray multi window event IPC",
>   "limit": 12,
>   "include_connected": true
> }
> ```
>
> ```text
> {"total":1029,"search_mode":"bm25","results":[{"name":"create_window","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.tauri.src-tauri.src.windows.create_window","label":"Function","file_path":"apps/tauri/src-tauri/src/windows.rs","start_line":404,"end_line":449,"rank":-29.335721144792707},{"name":"show_window","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.tauri.src-tauri.src.windows.show_window","label":"Function","file_path":"apps/tauri/src-tauri/src/windows.rs","start_line":487,"end_line":491,"rank":-29.335721144792707},{"name":"close_window","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.tauri.src-tauri.src.windows.close_window","label":"Function","file_path":"apps/tauri/src-tauri/src/windows.rs","start_line":495,"end_line":500,"rank":-29.335721144792707},{"name":"enter_window","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.tauri.src-tauri.src.drag.mod.DragCoordinator.enter_window","label":"Method","file_path":"apps/tauri/src-tauri/src/drag/mod.rs","start_line":111,"end_line":122,"rank":-28.867436156781064},{"name":"leave_window","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.tauri.src-tauri.src.drag.mod.DragCoordinator.leave_window","label":"Method","file_path":"apps/tauri/src-tauri/src/drag/mod.rs","start_line":125,"end_line":136,"rank":-28.867436156781064},{"name":"position_overlay_window","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.tauri.src-tauri.src.windows.position_overlay_window","label":"Function","file_path":"apps/tauri/src-tauri/src/windows.rs","start_line":461,"end_line":483,"rank":-28.867436156781064},{"name":"resize_overlay_window","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.tauri.src-tauri.src.windows.resize_overlay_window","label":"Function","file_path":"apps/tauri/src-tauri/src/windows.rs","start_line":526,"end_line":545,"rank":-28.867436156781064},{"name":"onWindowEvent","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.tauri.src.platform.onWindowEvent","label":"Function","file_path":"apps/tauri/src/platform.ts","start_line":180,"end_line":185,"rank":-27.265274953697105},{"name":"shareItems","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.tauri.crates.macos.src-swift.window.shareItems","label":"Function","file_path":"apps/tauri/crates/macos/src-swift/window.swift","start_line":77,"end_line":108,"rank":-26.913248331783134},{"name":"disableAppNap","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.tauri.crates.macos.src-swift.window.disableAppNap","label":"Function","file_path":"apps/tauri/crates/macos/src-swift/window.swift","start_line":15,"end_line":29,"rank":-26.731624798822455},{"name":"enableAppNap","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.tauri.crates.macos.src-swift.window.enableAppNap","label":"Function","file_path":"apps/tauri/crates/macos/src-swift/window.swift","start_line":31,"end_line":43,"rank":-26.731624798822455},{"name":"lockAppTheme","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.tauri.crates.macos.src-swift.window.lockAppTheme","label":"Function","file_path":"apps/tauri/crates/macos/src-swift/window.swift","start_line":45,"end_line":59,"rank":-26.731624798822455}],"has_more":true}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.search_graph
>
> ```json
> {
>   "project": "D-rust-tarui-demo-spacedrive-main",
>   "query": "settings persisted theme appearance",
>   "limit": 12,
>   "include_connected": true
> }
> ```
>
> ```text
> {"total":86,"search_mode":"bm25","results":[{"name":"AppearanceSettings","qualified_name":"D-rust-tarui-demo-spacedrive-main.packages.interface.src.Settings.pages.AppearanceSettings.AppearanceSettings","label":"Function","file_path":"packages/interface/src/Settings/pages/AppearanceSettings.tsx","start_line":23,"end_line":155,"rank":-27.252550837266206},{"name":"handleThemeChange","qualified_name":"D-rust-tarui-demo-spacedrive-main.packages.interface.src.Settings.pages.AppearanceSettings.handleThemeChange","label":"Function","file_path":"packages/interface/src/Settings/pages/AppearanceSettings.tsx","start_line":39,"end_line":46,"rank":-24.64146139775579},{"name":"get_persisted_device","qualified_name":"D-rust-tarui-demo-spacedrive-main.core.src.service.network.device.registry.DeviceRegistry.get_persisted_device","label":"Method","file_path":"core/src/service/network/device/registry.rs","start_line":618,"end_line":623,"rank":-22.3177450868017},{"name":"load_persisted_sessions","qualified_name":"D-rust-tarui-demo-spacedrive-main.core.src.service.network.protocol.pairing.mod.PairingProtocolHandler.load_persisted_sessions","label":"Method","file_path":"core/src/service/network/protocol/pairing/mod.rs","start_line":180,"end_line":195,"rank":-22.12356837758364},{"name":"AppearanceSettingsForm","qualified_name":"D-rust-tarui-demo-spacedrive-main.packages.interface.src.Settings.pages.AppearanceSettings.AppearanceSettingsForm","label":"Interface","file_path":"packages/interface/src/Settings/pages/AppearanceSettings.tsx","start_line":7,"end_line":10,"rank":-22.04033312860139},{"name":"Settings","qualified_name":"D-rust-tarui-demo-spacedrive-main.packages.interface.src.routes.settings.Settings","label":"Function","file_path":"packages/interface/src/routes/settings/index.tsx","start_line":133,"end_line":154,"rank":-19.512366318013196},{"name":"SettingsSidebar","qualified_name":"D-rust-tarui-demo-spacedrive-main.packages.interface.src.routes.settings.SettingsSidebar","label":"Function","file_path":"packages/interface/src/routes/settings/index.tsx","start_line":32,"end_line":57,"rank":-18.443086265103688},{"name":"SettingsContent","qualified_name":"D-rust-tarui-demo-spacedrive-main.packages.interface.src.routes.settings.SettingsContent","label":"Function","file_path":"packages/interface/src/routes/settings/index.tsx","start_line":63,"end_line":84,"rank":-18.443086265103688},{"name":"SettingsScreen","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.mobile.src.screens.settings.SettingsScreen.SettingsScreen","label":"Function","file_path":"apps/mobile/src/screens/settings/SettingsScreen.tsx","start_line":20,"end_line":696,"rank":-18.372059262545577},{"name":"AboutSettings","qualified_name":"D-rust-tarui-demo-spacedrive-main.packages.interface.src.Settings.pages.AboutSettings.AboutSettings","label":"Function","file_path":"packages/interface/src/Settings/pages/AboutSettings.tsx","start_line":8,"end_line":144,"rank":-18.372059262545577},{"name":"AdvancedSettings","qualified_name":"D-rust-tarui-demo-spacedrive-main.packages.interface.src.Settings.pages.AdvancedSettings.AdvancedSettings","label":"Function","file_path":"packages/interface/src/Settings/pages/AdvancedSettings.tsx","start_line":10,"end_line":107,"rank":-18.372059262545577},{"name":"GeneralSettings","qualified_name":"D-rust-tarui-demo-spacedrive-main.packages.interface.src.Settings.pages.GeneralSettings.GeneralSettings","label":"Function","file_path":"packages/interface/src/Settings/pages/GeneralSettings.tsx","start_line":9,"end_line":152,"rank":-18.372059262545577}],"has_more":true}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.search_graph
>
> ```json
> {
>   "project": "D-rust-tarui-demo-anarlog-main",
>   "query": "desktop tray window event plugin",
>   "limit": 12,
>   "include_connected": true
> }
> ```
>
> ```text
> {"total":1117,"search_mode":"bm25","results":[{"name":"on_window_event","qualified_name":"D-rust-tarui-demo-anarlog-main.plugins.windows.src.events.on_window_event","label":"Function","file_path":"plugins/windows/src/events.rs","start_line":8,"end_line":26,"rank":-23.645667767230577},{"name":"openStandaloneNoteWindow","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.session.window.openStandaloneNoteWindow","label":"Function","file_path":"apps/desktop/src/session/window.ts","start_line":3,"end_line":12,"rank":-20.463188562775542},{"name":"StandaloneWindowShell","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.shared.window-shell.StandaloneWindowShell","label":"Function","file_path":"apps/desktop/src/shared/window-shell.tsx","start_line":1,"end_line":20,"rank":-20.366441437909714},{"name":"isMainWebviewWindow","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.stt.window-control.isMainWebviewWindow","label":"Function","file_path":"apps/desktop/src/stt/window-control.tsx","start_line":19,"end_line":21,"rank":-20.271496434508002},{"name":"AITaskWindowSyncBridge","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.ai.task-window-sync.AITaskWindowSyncBridge","label":"Function","file_path":"apps/desktop/src/ai/task-window-sync.tsx","start_line":58,"end_line":66,"rank":-19.996976115603616},{"name":"isMainAITaskHostWindow","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.ai.task-window-sync.isMainAITaskHostWindow","label":"Function","file_path":"apps/desktop/src/ai/task-window-sync.tsx","start_line":38,"end_line":40,"rank":-19.90875080687384},{"name":"MainAITaskWindowSyncBridge","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.ai.task-window-sync.MainAITaskWindowSyncBridge","label":"Function","file_path":"apps/desktop/src/ai/task-window-sync.tsx","start_line":68,"end_line":148,"rank":-19.90875080687384},{"name":"RemoteAITaskWindowSyncBridge","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.ai.task-window-sync.RemoteAITaskWindowSyncBridge","label":"Function","file_path":"apps/desktop/src/ai/task-window-sync.tsx","start_line":150,"end_line":184,"rank":-19.90875080687384},{"name":"tray","qualified_name":"D-rust-tarui-demo-anarlog-main.plugins.tray.src.ext.TrayPluginExt.tray","label":"Method","file_path":"plugins/tray/src/ext.rs","start_line":252,"end_line":254,"rank":-19.779214762133},{"name":"tray","qualified_name":"D-rust-tarui-demo-anarlog-main.plugins.tray.src.ext.T.tray","label":"Method","file_path":"plugins/tray/src/ext.rs","start_line":258,"end_line":266,"rank":-19.779214762133},{"name":"create_tray_menu","qualified_name":"D-rust-tarui-demo-anarlog-main.plugins.tray.src.ext.Tray.create_tray_menu","label":"Method","file_path":"plugins/tray/src/ext.rs","start_line":104,"end_line":133,"rank":-19.764196492420098},{"name":"run","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.stt.window-control.run","label":"Function","file_path":"apps/desktop/src/stt/window-control.tsx","start_line":98,"end_line":108,"rank":-19.660797515891076}],"has_more":true}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.search_graph
>
> ```json
> {
>   "project": "D-rust-tarui-demo-voicetypr-main",
>   "query": "window close hide tray settings persistence notification",
>   "limit": 12,
>   "include_connected": true
> }
> ```
>
> ```text
> {"total":436,"search_mode":"bm25","results":[{"name":"close_pill_window","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.window_manager.WindowManager.close_pill_window","label":"Method","file_path":"src-tauri/src/window_manager.rs","start_line":451,"end_line":473,"rank":-28.656219446767007},{"name":"hide_toast_window","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.window.hide_toast_window","label":"Function","file_path":"src-tauri/src/commands/window.rs","start_line":153,"end_line":159,"rank":-28.349208220627833},{"name":"hide_pill_window","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.window_manager.WindowManager.hide_pill_window","label":"Method","file_path":"src-tauri/src/window_manager.rs","start_line":382,"end_line":448,"rank":-28.22674874611998},{"name":"hide_main_window","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.lib.hide_main_window","label":"Function","file_path":"src-tauri/src/lib.rs","start_line":68,"end_line":84,"rank":-27.682773381400843},{"name":"close_pill_widget","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.window.close_pill_widget","label":"Function","file_path":"src-tauri/src/commands/window.rs","start_line":122,"end_line":145,"rank":-26.920606412935996},{"name":"hide_pill_widget","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.window.hide_pill_widget","label":"Function","file_path":"src-tauri/src/commands/window.rs","start_line":64,"end_line":119,"rank":-26.487754700836792},{"name":"update_tray_menu","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.settings.update_tray_menu","label":"Function","file_path":"src-tauri/src/commands/settings.rs","start_line":1297,"end_line":1299,"rank":-21.10990733279279},{"name":"set_model_from_tray","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.settings.set_model_from_tray","label":"Function","file_path":"src-tauri/src/commands/settings.rs","start_line":1066,"end_line":1283,"rank":-20.823573582492717},{"name":"next_tray_menu_generation","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.settings.next_tray_menu_generation","label":"Function","file_path":"src-tauri/src/commands/settings.rs","start_line":1287,"end_line":1289,"rank":-20.823573582492717},{"name":"current_tray_menu_generation","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.settings.current_tray_menu_generation","label":"Function","file_path":"src-tauri/src/commands/settings.rs","start_line":1292,"end_line":1294,"rank":-20.823573582492717},{"name":"update_tray_menu_with_generation","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.settings.update_tray_menu_with_generation","label":"Function","file_path":"src-tauri/src/commands/settings.rs","start_line":1303,"end_line":1362,"rank":-20.551867657248167},{"name":"settings_reads_window_manager_via_app_state_not_tauri_state","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.tests.regression_tests.settings_reads_window_manager_via_app_state_not_tauri_state","label":"Function","file_path":"src-tauri/src/tests/regression_tests.rs","start_line":118,"end_line":126,"rank":-20.374825409838227}],"has_more":true}
> ```
>
> <details><summary>Searched the web</summary>
>
> - Searched the web for `site:v2.tauri.app plugin autostart notification log store global shortcut ...`
>
> </details>
>
> MCP tool call
>
> codebase-memory-mcp.get_code_snippet
>
> ```json
> {
>   "project": "D-rust-tarui-demo-blinko-main",
>   "qualified_name": "D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.tray.setup_system_tray",
>   "include_neighbors": true
> }
> ```
>
> ```text
> {"name":"setup_system_tray","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.tray.setup_system_tray","label":"Function","file_path":"D:/rust/tarui-demo/blinko-main/app/src-tauri/src/desktop/tray.rs","start_line":14,"end_line":79,"source":"pub fn setup_system_tray(app: &AppHandle) -> Result<TrayIcon, Box<dyn std::error::Error>> {\n    let icon_bytes = include_bytes!(\"../../icons/32x32.png\");\n    let image = Image::from_bytes(icon_bytes)?;\n    \n    // Create system tray menu\n    let quick_note_item = MenuItem::with_id(app, \"quicknote\", \"Quick Note\", true, None::<&str>)?;\n    let separator1 = PredefinedMenuItem::separator(app)?;\n    let toggle_item = MenuItem::with_id(app, \"toggle\", \"Show/Hide Window\", true, None::<&str>)?;\n    let settings_item = MenuItem::with_id(app, \"settings\", \"Settings\", true, None::<&str>)?;\n    let separator2 = PredefinedMenuItem::separator(app)?;\n    let quit_item = MenuItem::with_id(app, \"quit\", \"Quit\", true, None::<&str>)?;\n    \n    let tray_menu = MenuBuilder::new(app)\n        .items(&[\n            &quick_note_item,\n            &separator1,\n            &toggle_item,\n            &settings_item,\n            &separator2,\n            &quit_item,\n        ])\n        .build()?;\n    \n    let tray_icon = TrayIconBuilder::with_id(\"blinko-tray\")\n        .icon(image)\n        .menu(&tray_menu)\n        .tooltip(\"Blinko - Quick Note\")\n        .on_tray_icon_event(|tray, event| {\n            match event {\n                TrayIconEvent::Click {\n                    button: MouseButton::Left,\n                    button_state: MouseButtonState::Up,\n                    ..\n                } => {\n                    // Left click to toggle window visibility\n                    let app = tray.app_handle();\n                    let _ = toggle_editor_window(app.clone());\n                }\n                _ => {}\n            }\n        })\n        .on_menu_event(|app, event| {\n            match event.id().as_ref() {\n                \"quicknote\" => {\n                    let _ = toggle_quicknote_window(app.clone());\n                }\n                \"toggle\" => {\n                    let _ = toggle_editor_window(app.clone());\n                }\n                \"settings\" => {\n                    if let Some(window) = app.get_webview_window(\"main\") {\n                        let _ = window.show();\n                        let _ = window.set_focus();\n                        let _ = window.emit(\"navigate-to-settings\", ());\n                    }\n                }\n                \"quit\" => {\n                    app.exit(0);\n                }\n                _ => {}\n            }\n        })\n        .build(app)?;\n\n    Ok(tray_icon)\n}","complexity":10,"cognitive":19,"loop_count":0,"loop_depth":0,"self_recursive":false,"param_count":1,"max_access_depth":1,"linear_scan_in_loop":0,"alloc_in_loop":0,"recursion_in_loop":false,"unguarded_recursion":false,"lines":66,"is_exported":true,"is_test":false,"is_entry_point":false,"signature":"(app: &AppHandle)","return_type":"Result<TrayIcon, Box<dyn std::error::Error>>","fp":"0037262a0120495201fafe3d007d966702e375aa0038ff1600faf83c007f70410047353200d4a3c400999ec000066cb90016f05b028d803f006fe4e5002db02c01eeec7b0086926c0103c03401c8a0e0000d92bc00860de600de565903ba8d5400ed7ee1013d0a71006c2647000836cf0137f25b001d72e20128a2fb003d72c1003e16250144f706004eeede00fb08490141ffd70158c76d0028319f017088cc00417ad300df0c16012dd08300abd52d00decc88021a57300070d97f007dd680005d9b1a014c951000543f8d0101736200c548e0002a21a600279b610037b8ac0086531f010e50a400430155000872fa011bb7c900514b6d00223d99005f69e3","sp":"1,0,0,2,9,0,21,102,0,0,0,0,17,1,0,1,0,0,0,4,4,44,101,66,101","bt":"icon_bytes include_bytes image Image from_bytes quick_note_item MenuItem with_id app None separator1 PredefinedMenuItem separator toggle_item settings_item separator2 quit_item tray_menu MenuBuilder new items build tray_icon TrayIconBuilder icon menu tooltip on_tray_icon_event tray event TrayIconEvent button MouseButton Left button_state MouseButtonState Up app_handle toggle_editor_window clone","transitive_loop_depth":9,"recursive":false,"callers":1,"callees":9,"caller_names":["setup_app"],"callee_names":["build","exit","icon","id","items","new","show","toggle_editor_window","toggle_quicknote_window"]}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.get_code_snippet
>
> ```json
> {
>   "project": "D-rust-tarui-demo-kanri-main",
>   "qualified_name": "D-rust-tarui-demo-kanri-main.stores.theme.loadThemeSettings",
>   "include_neighbors": true
> }
> ```
>
> ```text
> {"name":"loadThemeSettings","qualified_name":"D-rust-tarui-demo-kanri-main.stores.theme.loadThemeSettings","label":"Function","file_path":"D:/rust/tarui-demo/kanri-main/stores/theme.ts","start_line":36,"end_line":48,"source":"    async loadThemeSettings() {\n      const store = useTauriStore().store;\n\n      const activeThemeSaved: string = await store.get(\"activeTheme\") ?? \"dark\";\n      const colorsSaved: Theme | null = await store.get(\"colors\") ?? dark;\n      const savedCustomThemeSaved: Theme | null = await store.get(\"savedCustomTheme\") ?? null;\n      const autoThemeEnabledSaved: boolean = await store.get(\"activeTheme\") === \"auto\" ? true : false;\n\n      this.activeTheme = activeThemeSaved;\n      this.colors = colorsSaved;\n      this.savedCustomTheme = savedCustomThemeSaved;\n      this.autoThemeEnabled = autoThemeEnabledSaved;\n    },\n","complexity":0,"cognitive":0,"loop_count":0,"loop_depth":0,"self_recursive":false,"param_count":0,"max_access_depth":1,"linear_scan_in_loop":0,"alloc_in_loop":0,"recursion_in_loop":false,"unguarded_recursion":false,"lines":13,"is_exported":true,"is_test":false,"is_entry_point":false,"signature":"()","fp":"02501a8b0231420106ebf8d7002173b20246b1f2021cb69f015a85db01c40baa04ed5ad00110239d00106c76025e1c0302522f3600530dc50654f9af024cbd2d0231ab46016ff795013cdb4d011525a3003cb1ac01afc83d00bdb3d2002613490469063b00629db6010e8ed103cb56ed06bd501b016fe8eb00be880101014ef6008f1b8c018767d0004eeede001a706800edd18901bbce9306293b600530f68d005d44e70247841d04121e0e01f2cdc300c595a200190aee00b2bb2b007dd6800210ecb20071c37600543f8d005bab130482702001371f9202ce64550168f5b2089f33480218741000d83cdb001b07ac0318d3d7002063b5004586a7039440d8","sp":"0,0,0,0,0,0,9,42,4,0,0,4,6,0,2,0,0,0,4,4,5,22,28,13,28","bt":"store useTauriStore activeThemeSaved get colorsSaved dark savedCustomThemeSaved autoThemeEnabledSaved activeTheme colors savedCustomTheme autoThemeEnabled","transitive_loop_depth":0,"recursive":true,"callers":1,"callees":2,"caller_names":["loadThemeSettings"],"callee_names":["loadThemeSettings","useTauriStore"]}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.get_code_snippet
>
> ```json
> {
>   "project": "D-rust-tarui-demo-voicetypr-main",
>   "qualified_name": "D-rust-tarui-demo-voicetypr-main.src-tauri.src.menu.tray.build_tray_menu",
>   "include_neighbors": true
> }
> ```
>
> ```text
> {"name":"build_tray_menu","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.menu.tray.build_tray_menu","label":"Function","file_path":"D:/rust/tarui-demo/voicetypr-main/src-tauri/src/menu/tray.rs","start_line":109,"end_line":584,"source":"pub async fn build_tray_menu<R: tauri::Runtime>(\n    app: &tauri::AppHandle<R>,\n) -> Result<tauri::menu::Menu<R>, Box<dyn std::error::Error>> {\n    use std::time::Instant;\n    let build_start = Instant::now();\n    log::debug!(\"⏱️ [TRAY BUILD TIMING] build_tray_menu called\");\n\n    let (current_model, selected_microphone, onboarding_done) = {\n        match app.store(\"settings\") {\n            Ok(store) => {\n                let model = store\n                    .get(\"current_model\")\n                    .and_then(|v| v.as_str().map(|s| s.to_string()))\n                    .unwrap_or_default();\n                let microphone = store\n                    .get(\"selected_microphone\")\n                    .and_then(|v| v.as_str().map(|s| s.to_string()));\n                let onboarding_done = store\n                    .get(\"onboarding_completed\")\n                    .and_then(|v| v.as_bool())\n                    .unwrap_or(false);\n                (model, microphone, onboarding_done)\n            }\n            Err(_) => (\"\".to_string(), None, false),\n        }\n    };\n    log::debug!(\n        \"⏱️ [TRAY BUILD TIMING] Settings loaded (+{}ms)\",\n        build_start.elapsed().as_millis()\n    );\n\n    // Get remote server info (active connection and saved connections)\n    // Use CACHED data - do NOT make HTTP calls here (that would block tray menu for seconds)\n    // The frontend/background tasks handle status polling separately\n    let (effective_active_id, active_remote_display, active_remote_model, remote_connections) = {\n        if let Some(remote_state) = app.try_state::<AsyncMutex<RemoteSettings>>() {\n            log::debug!(\n                \"⏱️ [TRAY BUILD TIMING] Acquiring remote_settings lock... (+{}ms)\",\n                build_start.elapsed().as_millis()\n            );\n            let settings = remote_state.lock().await;\n            log::debug!(\n                \"⏱️ [TRAY BUILD TIMING] remote_settings lock acquired (+{}ms)\",\n                build_start.elapsed().as_millis()\n            );\n            // Build connections list using cached data (no HTTP calls!)\n            // Include all servers with cached model info - let the user see what's available\n            let mut connections: Vec<(String, String, Option<String>)> = Vec::new();\n            for conn in settings.saved_connections.iter() {\n                if should_include_remote_connection_in_tray(&conn.status) {\n                    connections.push((conn.id.clone(), conn.display_name(), conn.model.clone()));\n                }\n            }\n            log::debug!(\n                \"⏱️ [TRAY BUILD TIMING] Loaded {} cached connections (+{}ms)\",\n                connections.len(),\n                build_start.elapsed().as_millis()\n            );\n\n            let effective_active_id =\n                effective_active_remote_id(settings.active_connection_id.as_deref(), &connections);\n\n            // Get effective active connection info\n            let active_conn_info = effective_active_id\n                .as_ref()\n                .and_then(|id| connections.iter().find(|(cid, _, _)| cid == id));\n            let active_display = active_conn_info.map(|(_, name, _)| name.clone());\n            let active_model = active_conn_info.and_then(|(_, _, model)| model.clone());\n\n            (\n                effective_active_id,\n                active_display,\n                active_model,\n                connections,\n            )\n        } else {\n            (None, None, None, Vec::new())\n        }\n    };\n    log::debug!(\n        \"⏱️ [TRAY BUILD TIMING] Remote server info collected (+{}ms)\",\n        build_start.elapsed().as_millis()\n    );\n\n    let (available_models, whisper_models_info) = {\n        // Store (name, display_name, accuracy_score, speed_score) for sorting to match UI order\n        let mut models: Vec<(String, String, u8, u8)> = Vec::new();\n        let mut whisper_all = std::collections::HashMap::new();\n\n        if let Some(whisper_state) =\n            app.try_state::<AsyncRwLock<whisper::manager::WhisperManager>>()\n        {\n            let manager = whisper_state.read().await;\n            whisper_all = manager.get_models_status();\n            for (name, info) in whisper_all.iter() {\n                if info.downloaded {\n                    models.push((\n                        name.clone(),\n                        info.display_name.clone(),\n                        info.accuracy_score,\n                        info.speed_score,\n                    ));\n                }\n            }\n        } else {\n            log::warn!(\"WhisperManager not available for tray menu\");\n        }\n\n        if let Some(parakeet_manager) = app.try_state::<crate::parakeet::ParakeetManager>() {\n            for m in parakeet_manager.list_models().into_iter() {\n                if m.downloaded {\n                    models.push((\n                        m.name.clone(),\n                        m.display_name.clone(),\n                        m.accuracy_score,\n                        m.speed_score,\n                    ));\n                }\n            }\n        } else {\n            log::warn!(\"ParakeetManager not available for tray menu\");\n        }\n\n        for provider in crate::cloud_stt::CloudProvider::ALL {\n            if crate::secure_store::secure_has(app, provider.key_name()).unwrap_or(false) {\n                models.push((\n                    provider.id().to_string(),\n                    provider.cloud_label(),\n                    u8::MAX,\n                    0,\n                ));\n            }\n        }\n\n        // Sort by accuracy_score (descending), then by speed_score (descending) as tiebreaker\n        // Higher accuracy_score = better accuracy = shown first\n        // Higher speed_score = faster = shown first within same accuracy\n        models.sort_by(|a, b| {\n            match b.2.cmp(&a.2) {\n                std::cmp::Ordering::Equal => b.3.cmp(&a.3), // speed descending\n                other => other,\n            }\n        });\n\n        // Convert back to (name, display_name) pairs\n        let models: Vec<(String, String)> = models.into_iter().map(|(n, d, _, _)| (n, d)).collect();\n\n        // NOTE: Remote servers are added separately below with a separator\n        (models, whisper_all)\n    };\n\n    let model_submenu = if !available_models.is_empty() || !remote_connections.is_empty() {\n        let mut model_items: Vec<&dyn tauri::menu::IsMenuItem<_>> = Vec::new();\n        let mut model_check_items = Vec::new();\n        let mut separator_items = Vec::new();\n        let mut remote_header_items = Vec::new();\n        let mut remote_check_items = Vec::new();\n\n        // Add local models first\n        for (model_name, display_name) in &available_models {\n            // Local model - only selected if no remote is active\n            let is_selected = effective_active_id.is_none()\n                && should_mark_model_selected(onboarding_done, model_name, &current_model);\n\n            let model_item = CheckMenuItem::with_id(\n                app,\n                format!(\"model_{}\", model_name),\n                display_name,\n                true,\n                is_selected,\n                None::<&str>,\n            )?;\n            model_check_items.push(model_item);\n        }\n\n        for item in &model_check_items {\n            model_items.push(item);\n        }\n\n        // Add remote servers section if any exist\n        if !remote_connections.is_empty() {\n            // Add separator\n            let sep = PredefinedMenuItem::separator(app)?;\n            separator_items.push(sep);\n            for item in &separator_items {\n                model_items.push(item);\n            }\n\n            // Add \"Remote Voicetypr\" header (disabled item)\n            let header = MenuItem::with_id(\n                app,\n                \"remote_header\",\n                \"Remote Voicetypr\",\n                false,\n                None::<&str>,\n            )?;\n            remote_header_items.push(header);\n            for item in &remote_header_items {\n                model_items.push(item);\n            }\n\n            // Add remote server items with format \"ServerName - ModelName\"\n            for (conn_id, conn_display, conn_model) in &remote_connections {\n                let model_id = format!(\"remote_{}\", conn_id);\n                let display = if let Some(model) = conn_model {\n                    format!(\"{} - {}\", conn_display, model)\n                } else {\n                    conn_display.clone()\n                };\n                let is_selected = effective_active_id.as_deref() == Some(conn_id.as_str());\n\n                let remote_item = CheckMenuItem::with_id(\n                    app,\n                    format!(\"model_{}\", model_id),\n                    &display,\n                    true,\n                    is_selected,\n                    None::<&str>,\n                )?;\n                remote_check_items.push(remote_item);\n            }\n\n            for item in &remote_check_items {\n                model_items.push(item);\n            }\n        }\n\n        // Resolve display name for the menu label - prioritize active remote server\n        let (effective_model, resolved_display_name) = if let Some(ref remote_display) =\n            active_remote_display\n        {\n            // Remote server is active - show \"ServerName - ModelName\"\n            let display = if let Some(ref model) = active_remote_model {\n                format!(\"{} - {}\", remote_display, model)\n            } else {\n                remote_display.clone()\n            };\n            (\"remote\".to_string(), Some(display))\n        } else if onboarding_done && !current_model.is_empty() {\n            let display = if let Some(info) = whisper_models_info.get(&current_model) {\n                Some(info.display_name.clone())\n            } else if let Some(parakeet_manager) =\n                app.try_state::<crate::parakeet::ParakeetManager>()\n            {\n                if let Some(pm) = parakeet_manager\n                    .list_models()\n                    .into_iter()\n                    .find(|m| m.name == current_model)\n                {\n                    Some(pm.display_name)\n                } else if let Some(p) = crate::cloud_stt::CloudProvider::from_id(&current_model) {\n                    Some(p.cloud_label())\n                } else {\n                    Some(humanize_model_id(&current_model))\n                }\n            } else if let Some(p) = crate::cloud_stt::CloudProvider::from_id(&current_model) {\n                Some(p.cloud_label())\n            } else {\n                Some(humanize_model_id(&current_model))\n            };\n            (current_model.clone(), display)\n        } else {\n            (String::new(), None)\n        };\n\n        let current_model_display = format_tray_model_label(\n            onboarding_done || effective_active_id.is_some(),\n            &effective_model,\n            resolved_display_name,\n        );\n\n        Some(Submenu::with_id_and_items(\n            app,\n            \"models\",\n            &current_model_display,\n            true,\n            &model_items,\n        )?)\n    } else {\n        None\n    };\n\n    let available_devices = if onboarding_done {\n        audio::recorder::AudioRecorder::get_devices()\n    } else {\n        Vec::new()\n    };\n\n    let microphone_submenu = if onboarding_done && !available_devices.is_empty() {\n        let mut mic_items: Vec<&dyn tauri::menu::IsMenuItem<_>> = Vec::new();\n        let mut mic_check_items = Vec::new();\n\n        let default_item = CheckMenuItem::with_id(\n            app,\n            \"microphone_default\",\n            \"System Default\",\n            true,\n            selected_microphone.is_none(),\n            None::<&str>,\n        )?;\n        mic_check_items.push(default_item);\n\n        for device_name in &available_devices {\n            let is_selected = selected_microphone.as_ref() == Some(device_name);\n            let mic_item = CheckMenuItem::with_id(\n                app,\n                format!(\"microphone_{}\", device_name),\n                device_name,\n                true,\n                is_selected,\n                None::<&str>,\n            )?;\n            mic_check_items.push(mic_item);\n        }\n\n        for item in &mic_check_items {\n            mic_items.push(item);\n        }\n\n        let current_mic_display = if let Some(ref mic_name) = selected_microphone {\n            format!(\"Microphone: {}\", mic_name)\n        } else {\n            \"Microphone: Default\".to_string()\n        };\n\n        Some(Submenu::with_id_and_items(\n            app,\n            \"microphones\",\n            &current_mic_display,\n            true,\n            &mic_items,\n        )?)\n    } else {\n        None\n    };\n\n    let mut recent_owned: Vec<tauri::menu::MenuItem<R>> = Vec::new();\n    let mut latest_copyable_id: Option<String> = None;\n    {\n        if let Ok(store) = app.store(\"transcriptions\") {\n            let mut entries: Vec<(String, serde_json::Value)> = Vec::new();\n            for key in store.keys() {\n                if let Some(value) = store.get(&key) {\n                    entries.push((key.to_string(), value));\n                }\n            }\n            entries.sort_by(|a, b| b.0.cmp(&a.0));\n            latest_copyable_id = latest_copyable_transcription_id(&entries);\n            entries.truncate(5);\n\n            for (ts, entry) in entries {\n                let mut label = entry\n                    .get(\"text\")\n                    .and_then(|v| v.as_str())\n                    .map(|s| {\n                        let first_line = s.lines().next().unwrap_or(\"\").trim();\n                        let char_count = first_line.chars().count();\n                        let mut preview: String = first_line.chars().take(40).collect();\n                        if char_count > 40 {\n                            preview.push('\\u{2026}');\n                        }\n                        if preview.is_empty() {\n                            \"(empty)\".to_string()\n                        } else {\n                            preview\n                        }\n                    })\n                    .unwrap_or_else(|| \"(unknown)\".to_string());\n\n                if label.is_empty() {\n                    label = \"(empty)\".to_string();\n                }\n\n                let item = tauri::menu::MenuItem::with_id(\n                    app,\n                    format!(\"recent_copy_{}\", ts),\n                    label,\n                    true,\n                    None::<&str>,\n                )?;\n                recent_owned.push(item);\n            }\n        }\n    }\n    let mut recent_refs: Vec<&dyn tauri::menu::IsMenuItem<_>> = Vec::new();\n    for item in &recent_owned {\n        recent_refs.push(item);\n    }\n\n    let (toggle_item, ptt_item) = {\n        let recording_mode = match app.store(\"settings\") {\n            Ok(store) => store\n                .get(\"recording_mode\")\n                .and_then(|v| v.as_str().map(|s| s.to_string()))\n                .unwrap_or_else(|| \"toggle\".to_string()),\n            Err(_) => \"toggle\".to_string(),\n        };\n\n        let toggle = tauri::menu::CheckMenuItem::with_id(\n            app,\n            \"recording_mode_toggle\",\n            \"Toggle\",\n            true,\n            recording_mode == \"toggle\",\n            None::<&str>,\n        )?;\n        let ptt = tauri::menu::CheckMenuItem::with_id(\n            app,\n            \"recording_mode_push_to_talk\",\n            \"Push-to-Talk\",\n            true,\n            recording_mode == \"push_to_talk\",\n            None::<&str>,\n        )?;\n        (toggle, ptt)\n    };\n\n    let copy_last_i = MenuItem::with_id(\n        app,\n        \"copy_last_transcription\",\n        \"Copy Last Transcription\",\n        latest_copyable_id.is_some(),\n        None::<&str>,\n    )?;\n    let separator1 = PredefinedMenuItem::separator(app)?;\n    let settings_i = MenuItem::with_id(app, \"dashboard\", \"Dashboard\", true, None::<&str>)?;\n    let check_updates_i = if crate::commands::distribution::is_store_install() {\n        None\n    } else {\n        Some(MenuItem::with_id(\n            app,\n            \"check_updates\",\n            \"Check for Updates\",\n            true,\n            None::<&str>,\n        )?)\n    };\n    let separator2 = PredefinedMenuItem::separator(app)?;\n    let quit_i = MenuItem::with_id(app, \"quit\", \"Quit Voicetypr\", true, None::<&str>)?;\n\n    let mut menu_builder = MenuBuilder::new(app);\n\n    if let Some(model_submenu) = model_submenu {\n        menu_builder = menu_builder.item(&model_submenu);\n    }\n\n    if let Some(microphone_submenu) = microphone_submenu {\n        menu_builder = menu_builder.item(&microphone_submenu);\n    }\n\n    menu_builder = menu_builder.item(&copy_last_i);\n    if !recent_refs.is_empty() {\n        let recent_submenu =\n            Submenu::with_id_and_items(app, \"recent\", \"Recent Transcriptions\", true, &recent_refs)?;\n        menu_builder = menu_builder.item(&recent_submenu);\n    }\n\n    let mode_items: Vec<&dyn tauri::menu::IsMenuItem<_>> = vec![&toggle_item, &ptt_item];\n    let mode_submenu =\n        Submenu::with_id_and_items(app, \"recording_mode\", \"Recording Mode\", true, &mode_items)?;\n    menu_builder = menu_builder.item(&mode_submenu);\n\n    let mut menu_builder = menu_builder.item(&separator1).item(&settings_i);\n\n    if let Some(check_updates_i) = &check_updates_i {\n        menu_builder = menu_builder.item(check_updates_i);\n    }\n\n    let menu = menu_builder.item(&separator2).item(&quit_i).build()?;\n\n    log::debug!(\n        \"⏱️ [TRAY BUILD TIMING] build_tray_menu COMPLETE - total: {}ms\",\n        build_start.elapsed().as_millis()\n    );\n    Ok(menu)\n}\n","complexity":55,"cognitive":126,"loop_count":15,"loop_depth":1,"self_recursive":false,"param_count":1,"max_access_depth":3,"linear_scan_in_loop":1,"alloc_in_loop":16,"recursion_in_loop":false,"unguarded_recursion":false,"lines":476,"is_exported":true,"is_test":false,"is_entry_point":false,"docstring":"/// Build the tray menu with all submenus (models, microphones, recent transcriptions, recording mode)\n","signature":"(\n    app: &tauri::AppHandle<R>,\n)","return_type":"Result<tauri::menu::Menu<R>, Box<dyn std::error::Error>>","fp":"0037262a005aab5e0009a3a3000d7cb7004c6a47002b11f40082c84c000db32a0014f0cc009c8ca90098f029005b4e040016f05b0010bf1b002cf825004db28c003c176c000ed48e000fcdb101446ea8000d92bc00df5c28000be3f4000b699e005778e900b4679100168525000836cf01104f41001d72e200114b85001169e5003e1625009536d3004eeede00227183000ef37200a650f3000197c4005ead62004008d20061c488002ebe53005fb32a001e178e00303a33006fc9320006625a0006d11d00264e6c001632c600355f2c00113341001017650023d02a0019b8f5001138c100c8dfc20023abe5000872fa002a4885003c91ec00223d99008b7326","sp":"31,15,0,3,20,0,25,96,12,6,0,9,58,10,0,1,0,0,9,8,4,303,835,476,835","bt":"std time Instant build_start now log debug current_model selected_microphone onboarding_done app store Ok model get and_then v as_str map s to_string unwrap_or_default microphone as_bool unwrap_or Err None elapsed as_millis effective_active_id active_remote_display active_remote_model remote_connections Some remote_state try_state settings lock connections Vec","transitive_loop_depth":1,"recursive":false,"callers":2,"callees":23,"caller_names":["run","update_tray_menu_with_generation"],"callee_names":["as_str","build","cloud_label","display_name","effective_active_remote_id","format_tray_model_label","from_id","get","get_devices","get_models_status"]}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.get_code_snippet
>
> ```json
> {
>   "project": "D-rust-tarui-demo-spacedrive-main",
>   "qualified_name": "D-rust-tarui-demo-spacedrive-main.apps.tauri.src-tauri.src.windows.position_overlay_window",
>   "include_neighbors": true
> }
> ```
>
> ```text
> {"name":"position_overlay_window","qualified_name":"D-rust-tarui-demo-spacedrive-main.apps.tauri.src-tauri.src.windows.position_overlay_window","label":"Function","file_path":"D:/rust/tarui-demo/spacedrive-main/apps/tauri/src-tauri/src/windows.rs","start_line":461,"end_line":483,"source":"fn position_overlay_window(window: &WebviewWindow, width: f64, height: f64) -> Result<(), String> {\n\tuse tauri::{PhysicalPosition, Position};\n\n\tlet monitor = window\n\t\t.current_monitor()\n\t\t.map_err(|e| e.to_string())?\n\t\t.ok_or(\"No monitor found\")?;\n\n\tlet monitor_size = monitor.size();\n\tlet monitor_position = monitor.position();\n\tlet scale_factor = window.scale_factor().map_err(|e| e.to_string())?;\n\n\tlet physical_width = (width * scale_factor).round() as i32;\n\tlet physical_height = (height * scale_factor).round() as i32;\n\tlet bottom_margin = (24.0 * scale_factor).round() as i32;\n\n\tlet x = monitor_position.x + (monitor_size.width as i32 - physical_width) / 2;\n\tlet y = monitor_position.y + monitor_size.height as i32 - physical_height - bottom_margin;\n\n\twindow\n\t\t.set_position(Position::Physical(PhysicalPosition::new(x, y)))\n\t\t.map_err(|e| e.to_string())\n}\n","complexity":0,"cognitive":0,"loop_count":0,"loop_depth":0,"self_recursive":false,"param_count":3,"max_access_depth":1,"linear_scan_in_loop":0,"alloc_in_loop":0,"recursion_in_loop":false,"unguarded_recursion":false,"lines":23,"is_exported":true,"is_test":false,"is_entry_point":false,"signature":"(window: &WebviewWindow, width: f64, height: f64)","return_type":"Result<(), String>","fp":"0037262a00b149de01f3ae7d010d834200b530bc00d1764c0082c84c003c94780304153801a027f200999ec00086ca430016f05b00bdffc2023c9bb60034149c0015bfbe024499a603d1283404916704002e899a00e0ed2200de56590013df6501efc61102a72fa10130f3ce017b25f004020780002c78df012fdb89003d72c100fd007e0144f706004eeede00a4d09e021bc6fb04653a240028319f017088cc0192303300df0c16015032ed031483ca05b61c6501384fb2011a0e0002d030910074ef56010a1d8600a00f8302df9a03013271d400ec671700279b61067d3a3f001138c103a761450342892a06cfe65c0062eae702386e6500223d9901524f28","sp":"0,0,0,0,3,0,11,48,9,0,0,0,1,2,0,3,0,0,0,3,4,29,62,23,62","bt":"tauri PhysicalPosition Position monitor window current_monitor map_err e to_string ok_or monitor_size size monitor_position position scale_factor physical_width width round physical_height height bottom_margin x y set_position Physical new","transitive_loop_depth":0,"recursive":false,"callers":2,"callees":3,"caller_names":["resize_overlay_window","windows.rs"],"callee_names":["position","size","to_string"]}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.get_code_snippet
>
> ```json
> {
>   "project": "D-rust-tarui-demo-anarlog-main",
>   "qualified_name": "D-rust-tarui-demo-anarlog-main.apps.desktop.src.shared.theme.apply.bootstrapThemeFromSettings",
>   "include_neighbors": true
> }
> ```
>
> ```text
> {"name":"bootstrapThemeFromSettings","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src.shared.theme.apply.bootstrapThemeFromSettings","label":"Function","file_path":"D:/rust/tarui-demo/anarlog-main/apps/desktop/src/shared/theme/apply.ts","start_line":59,"end_line":75,"source":"export async function bootstrapThemeFromSettings({\n  timeoutMs = THEME_BOOTSTRAP_TIMEOUT_MS,\n}: {\n  timeoutMs?: number;\n} = {}): Promise<void> {\n  const themeLoad = loadThemeFromSettings();\n\n  if (timeoutMs <= 0) {\n    await themeLoad;\n    return;\n  }\n\n  await Promise.race([\n    themeLoad,\n    new Promise<void>((resolve) => setTimeout(resolve, timeoutMs)),\n  ]);\n}\n","complexity":1,"cognitive":1,"loop_count":0,"loop_depth":0,"self_recursive":false,"param_count":1,"max_access_depth":1,"linear_scan_in_loop":0,"alloc_in_loop":0,"recursion_in_loop":false,"unguarded_recursion":false,"lines":17,"is_exported":true,"is_test":false,"is_entry_point":true,"signature":"({\n  timeoutMs = THEME_BOOTSTRAP_TIMEOUT_MS,\n}: {\n  timeoutMs?: number;\n} = {})","return_type":": Promise<void>","fp":"02501a8b0099a0bb018238d2002173b2005fbe3601750a4000712ae30093520a00e62de900ee9a490655667e032d5d130250845e02b133fc04daf69601247c7d08226931008b31350140025c0047083b003cb1ac01afc83d00bdb3d2002909d2034a2b86018a2701033a07d3079e3d110439b7bb03deb1ef0269d9cb0545cbf400ba9c2f0144f70601aa0b4601b77dde00edd1890158c76d00b78754017088cc0557bb380061c4880215720c01f2cdc3000f4d53029feafc0070d97f007dd680013362a600583caf00543f8d043340a8087864c701530a2100e598fa025072a70906c0e0010e50a401196410001b07ac0048f494033cfa7a00d2a76301945bd1","sp":"1,0,0,0,0,1,11,51,1,0,0,0,0,1,0,0,0,0,0,5,3,7,13,17,13","bt":"themeLoad loadThemeFromSettings timeoutMs Promise race resolve setTimeout","transitive_loop_depth":1,"recursive":false,"callers":1,"callees":1,"caller_names":["renderApp"],"callee_names":["loadThemeFromSettings"]}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.get_code_snippet
>
> ```json
> {
>   "project": "D-rust-tarui-demo-blinko-main",
>   "qualified_name": "D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.tray.setup_system_tray",
>   "include_neighbors": true
> }
> ```
>
> ```text
> {"name":"setup_system_tray","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.tray.setup_system_tray","label":"Function","file_path":"D:/rust/tarui-demo/blinko-main/app/src-tauri/src/desktop/tray.rs","start_line":14,"end_line":79,"source":"pub fn setup_system_tray(app: &AppHandle) -> Result<TrayIcon, Box<dyn std::error::Error>> {\n    let icon_bytes = include_bytes!(\"../../icons/32x32.png\");\n    let image = Image::from_bytes(icon_bytes)?;\n    \n    // Create system tray menu\n    let quick_note_item = MenuItem::with_id(app, \"quicknote\", \"Quick Note\", true, None::<&str>)?;\n    let separator1 = PredefinedMenuItem::separator(app)?;\n    let toggle_item = MenuItem::with_id(app, \"toggle\", \"Show/Hide Window\", true, None::<&str>)?;\n    let settings_item = MenuItem::with_id(app, \"settings\", \"Settings\", true, None::<&str>)?;\n    let separator2 = PredefinedMenuItem::separator(app)?;\n    let quit_item = MenuItem::with_id(app, \"quit\", \"Quit\", true, None::<&str>)?;\n    \n    let tray_menu = MenuBuilder::new(app)\n        .items(&[\n            &quick_note_item,\n            &separator1,\n            &toggle_item,\n            &settings_item,\n            &separator2,\n            &quit_item,\n        ])\n        .build()?;\n    \n    let tray_icon = TrayIconBuilder::with_id(\"blinko-tray\")\n        .icon(image)\n        .menu(&tray_menu)\n        .tooltip(\"Blinko - Quick Note\")\n        .on_tray_icon_event(|tray, event| {\n            match event {\n                TrayIconEvent::Click {\n                    button: MouseButton::Left,\n                    button_state: MouseButtonState::Up,\n                    ..\n                } => {\n                    // Left click to toggle window visibility\n                    let app = tray.app_handle();\n                    let _ = toggle_editor_window(app.clone());\n                }\n                _ => {}\n            }\n        })\n        .on_menu_event(|app, event| {\n            match event.id().as_ref() {\n                \"quicknote\" => {\n                    let _ = toggle_quicknote_window(app.clone());\n                }\n                \"toggle\" => {\n                    let _ = toggle_editor_window(app.clone());\n                }\n                \"settings\" => {\n                    if let Some(window) = app.get_webview_window(\"main\") {\n                        let _ = window.show();\n                        let _ = window.set_focus();\n                        let _ = window.emit(\"navigate-to-settings\", ());\n                    }\n                }\n                \"quit\" => {\n                    app.exit(0);\n                }\n                _ => {}\n            }\n        })\n        .build(app)?;\n\n    Ok(tray_icon)\n}","complexity":10,"cognitive":19,"loop_count":0,"loop_depth":0,"self_recursive":false,"param_count":1,"max_access_depth":1,"linear_scan_in_loop":0,"alloc_in_loop":0,"recursion_in_loop":false,"unguarded_recursion":false,"lines":66,"is_exported":true,"is_test":false,"is_entry_point":false,"signature":"(app: &AppHandle)","return_type":"Result<TrayIcon, Box<dyn std::error::Error>>","fp":"0037262a0120495201fafe3d007d966702e375aa0038ff1600faf83c007f70410047353200d4a3c400999ec000066cb90016f05b028d803f006fe4e5002db02c01eeec7b0086926c0103c03401c8a0e0000d92bc00860de600de565903ba8d5400ed7ee1013d0a71006c2647000836cf0137f25b001d72e20128a2fb003d72c1003e16250144f706004eeede00fb08490141ffd70158c76d0028319f017088cc00417ad300df0c16012dd08300abd52d00decc88021a57300070d97f007dd680005d9b1a014c951000543f8d0101736200c548e0002a21a600279b610037b8ac0086531f010e50a400430155000872fa011bb7c900514b6d00223d99005f69e3","sp":"1,0,0,2,9,0,21,102,0,0,0,0,17,1,0,1,0,0,0,4,4,44,101,66,101","bt":"icon_bytes include_bytes image Image from_bytes quick_note_item MenuItem with_id app None separator1 PredefinedMenuItem separator toggle_item settings_item separator2 quit_item tray_menu MenuBuilder new items build tray_icon TrayIconBuilder icon menu tooltip on_tray_icon_event tray event TrayIconEvent button MouseButton Left button_state MouseButtonState Up app_handle toggle_editor_window clone","transitive_loop_depth":9,"recursive":false,"callers":1,"callees":9,"caller_names":["setup_app"],"callee_names":["build","exit","icon","id","items","new","show","toggle_editor_window","toggle_quicknote_window"]}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.get_code_snippet
>
> ```json
> {
>   "project": "D-rust-tarui-demo-kanri-main",
>   "qualified_name": "D-rust-tarui-demo-kanri-main.stores.theme.loadThemeSettings",
>   "include_neighbors": true
> }
> ```
>
> ```text
> {"name":"loadThemeSettings","qualified_name":"D-rust-tarui-demo-kanri-main.stores.theme.loadThemeSettings","label":"Function","file_path":"D:/rust/tarui-demo/kanri-main/stores/theme.ts","start_line":36,"end_line":48,"source":"    async loadThemeSettings() {\n      const store = useTauriStore().store;\n\n      const activeThemeSaved: string = await store.get(\"activeTheme\") ?? \"dark\";\n      const colorsSaved: Theme | null = await store.get(\"colors\") ?? dark;\n      const savedCustomThemeSaved: Theme | null = await store.get(\"savedCustomTheme\") ?? null;\n      const autoThemeEnabledSaved: boolean = await store.get(\"activeTheme\") === \"auto\" ? true : false;\n\n      this.activeTheme = activeThemeSaved;\n      this.colors = colorsSaved;\n      this.savedCustomTheme = savedCustomThemeSaved;\n      this.autoThemeEnabled = autoThemeEnabledSaved;\n    },\n","complexity":0,"cognitive":0,"loop_count":0,"loop_depth":0,"self_recursive":false,"param_count":0,"max_access_depth":1,"linear_scan_in_loop":0,"alloc_in_loop":0,"recursion_in_loop":false,"unguarded_recursion":false,"lines":13,"is_exported":true,"is_test":false,"is_entry_point":false,"signature":"()","fp":"02501a8b0231420106ebf8d7002173b20246b1f2021cb69f015a85db01c40baa04ed5ad00110239d00106c76025e1c0302522f3600530dc50654f9af024cbd2d0231ab46016ff795013cdb4d011525a3003cb1ac01afc83d00bdb3d2002613490469063b00629db6010e8ed103cb56ed06bd501b016fe8eb00be880101014ef6008f1b8c018767d0004eeede001a706800edd18901bbce9306293b600530f68d005d44e70247841d04121e0e01f2cdc300c595a200190aee00b2bb2b007dd6800210ecb20071c37600543f8d005bab130482702001371f9202ce64550168f5b2089f33480218741000d83cdb001b07ac0318d3d7002063b5004586a7039440d8","sp":"0,0,0,0,0,0,9,42,4,0,0,4,6,0,2,0,0,0,4,4,5,22,28,13,28","bt":"store useTauriStore activeThemeSaved get colorsSaved dark savedCustomThemeSaved autoThemeEnabledSaved activeTheme colors savedCustomTheme autoThemeEnabled","transitive_loop_depth":0,"recursive":true,"callers":1,"callees":2,"caller_names":["loadThemeSettings"],"callee_names":["loadThemeSettings","useTauriStore"]}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.get_code_snippet
>
> ```json
> {
>   "project": "D-rust-tarui-demo-kanri-main",
>   "qualified_name": "D-rust-tarui-demo-kanri-main.composables.useBoard.setCardDueDate",
>   "include_neighbors": true
> }
> ```
>
> ```text
> {"name":"setCardDueDate","qualified_name":"D-rust-tarui-demo-kanri-main.composables.useBoard.setCardDueDate","label":"Function","file_path":"D:/rust/tarui-demo/kanri-main/composables/useBoard.ts","start_line":174,"end_line":189,"source":"  const setCardDueDate = (\n    columnId: string,\n    cardId: string | undefined,\n    dueDate: Date | null,\n    isCounterRelative: boolean,\n    isCompleted: boolean\n  ) => {\n    if (!board.value) return;\n    if (!cardId) return;\n\n    mutateCard(columnId, cardId, (card) => {\n      card.dueDate = dueDate;\n      card.isDueDateCounterRelative = isCounterRelative;\n      card.isDueDateCompleted = isCompleted;\n    });\n  };\n","complexity":2,"cognitive":2,"loop_count":0,"loop_depth":0,"self_recursive":false,"param_count":5,"max_access_depth":1,"linear_scan_in_loop":0,"alloc_in_loop":0,"recursion_in_loop":false,"unguarded_recursion":false,"lines":16,"is_exported":true,"is_test":false,"is_entry_point":false,"signature":"(\n    columnId: string,\n    cardId: string | undefined,\n    dueDate: Date | null,\n    isCounterRelative: boolean,\n    isCompleted: boolean\n  )","fp":"03b0b20b0099a0bb01c5c8f9074db96d062339160814ad6108c306af01c40baa00e62de901a027f2021330770189571202247b460230f0ff0f5196910190c3b60231ab46016ff7950a56066a05476c73003cb1ac00df5c28011ab472002909d200e7d61407ee0ea802369d1e03a98a15055492fd011097a607606af000fb103400c71d0d0144f7060433f39e0294ad000213e76900bf32ff04afa705010e29a9005d44e702d880410183f3ae031483ca05d4519702922e9b012b19da09b38f49013362a603f71efc00a00f8301fb1bc600c3b50c0cbf44010100db61035097b10085e1db010e50a405003787073540be014a230a04aec82400a72b4201945bd1","sp":"2,0,0,0,0,2,9,52,0,2,0,3,0,0,0,5,0,0,3,6,2,14,16,16,16","bt":"board value cardId mutateCard columnId card dueDate isDueDateCounterRelative isCounterRelative isDueDateCompleted isCompleted","transitive_loop_depth":0,"recursive":false,"callers":0,"callees":1,"callee_names":["mutateCard"]}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.get_code_snippet
>
> ```json
> {
>   "project": "D-rust-tarui-demo-blinko-main",
>   "qualified_name": "D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.window.set_desktop_theme",
>   "include_neighbors": true
> }
> ```
>
> ```text
> {"name":"set_desktop_theme","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.window.set_desktop_theme","label":"Function","file_path":"D:/rust/tarui-demo/blinko-main/app/src-tauri/src/desktop/window.rs","start_line":240,"end_line":274,"source":"pub fn set_desktop_theme<R: tauri::Runtime>(app: AppHandle<R>, theme: String) -> Result<(), String> {\n    use tauri::{Theme, window::Color};\n\n    let tauri_theme = match theme.as_str() {\n        \"light\" => Theme::Light,\n        \"dark\" => Theme::Dark,\n        _ => return Err(format!(\"Invalid theme: {}\", theme))\n    };\n\n    // Define background colors based on theme\n    let background_color = match theme.as_str() {\n        \"light\" => Color(248, 248, 248, 255), // #F8F8F8\n        \"dark\" => Color(28, 28, 30, 255),     // #1C1C1E\n        _ => Color(248, 248, 248, 255),\n    };\n\n    // Set theme for main window only\n    if let Some(window) = app.get_webview_window(\"main\") {\n        // Set system theme\n        if let Err(e) = window.set_theme(Some(tauri_theme)) {\n            eprintln!(\"Failed to set theme for main window: {}\", e);\n        } else {\n            println!(\"Set main window theme to: {}\", theme);\n        }\n\n        // Set window background color\n        if let Err(e) = window.set_background_color(Some(background_color)) {\n            eprintln!(\"Failed to set background color for main window: {}\", e);\n        } else {\n            println!(\"Set main window background color to: {:?}\", background_color);\n        }\n    }\n\n    Ok(())\n}\n","complexity":11,"cognitive":19,"loop_count":0,"loop_depth":0,"self_recursive":false,"param_count":2,"max_access_depth":1,"linear_scan_in_loop":0,"alloc_in_loop":0,"recursion_in_loop":false,"unguarded_recursion":false,"lines":35,"is_exported":true,"is_test":false,"is_entry_point":false,"signature":"(app: AppHandle<R>, theme: String)","return_type":"Result<(), String>","fp":"00df32680159cdd901fafe3d004d49f2005fbe3600b272d40082c84c0093520a00593871029e48fe014a986e01edd5690016f05b00bdffc2001ef0140034149c0299a2f00012a24f017c51a801474d56009c9f5000e0ed2203efb6a001b34b1b005778e90378ebfb00b8b41b000836cf0329e00902e2b6ca03ee5d11012ef3ee00c71d0d0144f706005fb4e0010671120144c34300b8fe1f006c0803000d4609004dcca600a7349b00475f4f020380240280ce8300303a33006fc9320279de0d00cb1af6008a451e00a00f8300cda2a200a10af0002a21a60320bd9c0019b8f5001138c1010e50a400a9bf0102d00c05003e8ff00260c7ee005c8f0d008b7326","sp":"3,0,0,2,0,1,12,62,0,0,0,0,10,12,0,2,0,0,0,4,3,18,57,35,57","bt":"tauri Theme window Color tauri_theme theme as_str Light Dark Err format background_color Some app get_webview_window e set_theme eprintln println set_background_color Ok","transitive_loop_depth":0,"recursive":false,"callers":0,"callees":0}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.get_code_snippet
>
> ```json
> {
>   "project": "D-rust-tarui-demo-voicetypr-main",
>   "qualified_name": "D-rust-tarui-demo-voicetypr-main.src-tauri.src.lib.hide_main_window",
>   "include_neighbors": true
> }
> ```
>
> ```text
> {"name":"hide_main_window","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.lib.hide_main_window","label":"Function","file_path":"D:/rust/tarui-demo/voicetypr-main/src-tauri/src/lib.rs","start_line":68,"end_line":84,"source":"fn hide_main_window(app: &tauri::AppHandle) {\n    // If the system tray failed to create (e.g. the Windows shell notification\n    // area wasn't ready at startup), hiding the window would leave the app\n    // running with no way to bring it back. Keep the window visible instead.\n    if app.tray_by_id(\"main\").is_none() {\n        log::warn!(\"No tray icon present; keeping main window visible instead of hiding to tray\");\n        return;\n    }\n    if let Some(window) = app.get_webview_window(\"main\") {\n        if let Err(e) = window.hide() {\n            log::error!(\"Failed to hide main window: {}\", e);\n            return;\n        }\n        #[cfg(target_os = \"macos\")]\n        hide_dock_icon(app);\n    }\n}\n","complexity":3,"cognitive":4,"loop_count":0,"loop_depth":0,"self_recursive":false,"param_count":1,"max_access_depth":1,"linear_scan_in_loop":0,"alloc_in_loop":0,"recursion_in_loop":false,"unguarded_recursion":false,"lines":17,"is_exported":true,"is_test":false,"is_entry_point":false,"docstring":"/// Hide the main window and the macOS Dock icon — back to the menubar/tray.\n","signature":"(app: &tauri::AppHandle)","fp":"02732deb0159cdd900119ebb01e9673d0053fb1d021cb69f0201fbc200a74ed20059387100d4a3c400999ec0016eb4950163f52807240fb3006fe4e501247c7d048a311f0331aba0052d19ba02f23a5a00bf545d01f353f001df47320388f6e500ed7ee10463c6f204516179000836cf01cddd8906f3b9ba008b5d9b006794d300b579d100208e45004eeede03353ddd00ad6f320158c76d016bb85d034c9ac8004dcca601409cc1035482e0031483ca001e178e00303a33012b19da01d812c700cb1af60283445000a00f8303b53469017f4df4002a21a6071b4e010037b8ac039a4099010e50a4018c641200b6066c002d02e700514b6d0605992101945bd1","sp":"3,0,0,0,0,2,11,58,0,0,0,0,5,0,0,1,0,0,0,3,2,10,20,17,20","bt":"app tray_by_id is_none log warn Some window get_webview_window Err e hide error cfg target_os hide_dock_icon","transitive_loop_depth":0,"recursive":false,"callers":2,"callees":1,"caller_names":["perform_startup_checks","run"],"callee_names":["hide_dock_icon"]}
> ```
>
> <details><summary>Searched the web</summary>
>
> - Searched the web for `"Anarlog" GitHub Tauri calendar ...`
> - Searched the web for `site:github.com/blinko-space/blinko "License" ...`
>
> </details>
>
> </details>
>
> 已确认最值得复用的是“模式与边界”，不是整块拷贝：例如 Blinko 的托盘事件闭环、Kanri 的 Pinia 日期/主题状态、Voicetypr 的隐藏到托盘保护、Spacedrive 的多窗口定位。这些都能收敛为当前 Vue + Pinia + Tauri 的小型平台层。还在核对许可证，避免把源码可见误当作可直接复制。
>
> <details><summary>Ran 3 commands, called 2 tools</summary>
>
> - Ran `rtk proxy powershell -NoProfile -Command "Get-ChildItem -LiteralPath 'D:\rust\tarui-demo' -Recurse -File -Include LICENSE,LICENSE.md,LICENSE.txt,COPYING | Where-Object { $_.FullName -match '(anarlog-main|blinko-main|kanri-main|spacedrive-main|voicetypr-main)' } | ForEach-Object { Write-Output ('FILE: ' + $_.FullName); Get-Content -LiteralPath $_.FullName -TotalCount 4 }"`
> - Ran `rtk proxy powershell -NoProfile -Command "if (Test-Path 'D:\rust\tarui-demo\anarlog-main\LICENSE') { Write-Output 'ANARLOG'; Get-Content 'D:\rust\tarui-demo\anarlog-main\LICENSE' -TotalCount 4 }; if (Test-Path 'D:\rust\tarui-demo\blinko-main\LICENSE') { Write-Output 'BLINKO'; Get-Content 'D:\rust\tarui-demo\blinko-main\LICENSE' -TotalCount 4 }; if (Test-Path 'D:\rust\tarui-demo\kanri-main\LICENSE') { Write-Output 'KANRI'; Get-Content 'D:\rust\tarui-demo\kanri-main\LICENSE' -TotalCount 4 }; if (Test-Path 'D:\rust\tarui-demo\spacedrive-main\LICENSE') { Write-Output 'SPACEDRIVE'; Get-Content 'D:\rust\tarui-demo\spacedrive-main\LICENSE' -TotalCount 4 }; if (Test-Path 'D:\rust\tarui-demo\voicetypr-main\LICENSE') { Write-Output 'VOICETYPR'; Get-Content 'D:\rust\tarui-demo\voicetypr-main\LICENSE' -TotalCount 4 }"`
> - Ran `rtk proxy powershell -NoProfile -Command "Get-ChildItem -LiteralPath 'D:\rust\tarui-demo\voicetypr-main' -Force | Where-Object { $_.Name -match 'license|copying' } | Select-Object -ExpandProperty FullName"`
>
> MCP tool call
>
> codebase-memory-mcp.get_architecture
>
> ```json
> {
>   "project": "D-project-todo",
>   "aspects": ["packages", "entry_points", "dependencies"]
> }
> ```
>
> ```text
> {"project":"D-project-todo","total_nodes":594,"total_edges":651,"edge_types":[{"type":"DEFINES","count":469},{"type":"CONTAINS_FILE","count":77},{"type":"USAGE","count":27},{"type":"CONTAINS_FOLDER","count":25},{"type":"CALLS","count":23},{"type":"IMPORTS","count":17},{"type":"DECORATES","count":11},{"type":"DEFINES_METHOD","count":1},{"type":"TESTS_FILE","count":1}],"packages":[{"name":"desktop","node_count":24,"fan_in":0,"fan_out":0},{"name":"api","node_count":3,"fan_in":0,"fan_out":0},{"name":"domain","node_count":2,"fan_in":0,"fan_out":0}],"entry_points":[{"name":"createApp","qualified_name":"D-project-todo.apps.api.src.app.createApp","file":"apps/api/src/app.ts"},{"name":"main","qualified_name":"D-project-todo.apps.desktop.src-tauri.build.main","file":"apps/desktop/src-tauri/build.rs"},{"name":"main","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.main.main","file":"apps/desktop/src-tauri/src/main.rs"},{"name":"cn","qualified_name":"D-project-todo.apps.desktop.src.lib.utils.cn","file":"apps/desktop/src/lib/utils.ts"},{"name":"nextSyncCursor","qualified_name":"D-project-todo.packages.domain.src.sync.nextSyncCursor","file":"packages/domain/src/sync.ts"},{"name":"isDateOnly","qualified_name":"D-project-todo.packages.domain.src.sync.isDateOnly","file":"packages/domain/src/sync.ts"}]}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.search_graph
>
> ```json
> {
>   "project": "D-project-todo",
>   "query": "theme settings task todo date notification Tauri Pinia",
>   "limit": 30,
>   "include_connected": true
> }
> ```
>
> ```text
> {"total":49,"search_mode":"bm25","results":[{"name":"isDateOnly","qualified_name":"D-project-todo.packages.domain.src.sync.isDateOnly","label":"Function","file_path":"packages/domain/src/sync.ts","start_line":8,"end_line":10,"rank":-14.781241648240282},{"name":"main","qualified_name":"D-project-todo.apps.desktop.src-tauri.build.main","label":"Function","file_path":"apps/desktop/src-tauri/build.rs","start_line":1,"end_line":3,"rank":-12.275819246399578},{"name":"run","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.lib.run","label":"Function","file_path":"apps/desktop/src-tauri/src/lib.rs","start_line":53,"end_line":64,"rank":-12.209521496498743},{"name":"main","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.main.main","label":"Function","file_path":"apps/desktop/src-tauri/src/main.rs","start_line":1,"end_line":3,"rank":-12.209521496498743},{"name":"ipc_builder","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.lib.ipc_builder","label":"Function","file_path":"apps/desktop/src-tauri/src/lib.rs","start_line":41,"end_line":43,"rank":-12.117014187443996},{"name":"get_runtime_info","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.lib.get_runtime_info","label":"Function","file_path":"apps/desktop/src-tauri/src/lib.rs","start_line":34,"end_line":39,"rank":-12.031941713966596},{"name":"export_type_bindings","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.lib.export_type_bindings","label":"Function","file_path":"apps/desktop/src-tauri/src/lib.rs","start_line":79,"end_line":89,"rank":-12.031941713966596},{"name":"frontend_bindings_path","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.lib.frontend_bindings_path","label":"Function","file_path":"apps/desktop/src-tauri/src/lib.rs","start_line":72,"end_line":76,"rank":-12.031941713966596},{"name":"createApp","qualified_name":"D-project-todo.apps.api.src.app.createApp","label":"Function","file_path":"apps/api/src/app.ts","start_line":13,"end_line":41,"rank":-10.00000104581356},{"name":"cn","qualified_name":"D-project-todo.apps.desktop.src.lib.utils.cn","label":"Function","file_path":"apps/desktop/src/lib/utils.ts","start_line":4,"end_line":6,"rank":-10.000001022828835},{"name":"add","qualified_name":"D-project-todo.apps.desktop.src.stores.todos.add","label":"Function","file_path":"apps/desktop/src/stores/todos.ts","start_line":12,"end_line":24,"rank":-10.000001022828835},{"name":"toggle","qualified_name":"D-project-todo.apps.desktop.src.stores.todos.toggle","label":"Function","file_path":"apps/desktop/src/stores/todos.ts","start_line":26,"end_line":33,"rank":-10.000001022828835},{"name":"remove","qualified_name":"D-project-todo.apps.desktop.src.stores.todos.remove","label":"Function","file_path":"apps/desktop/src/stores/todos.ts","start_line":35,"end_line":37,"rank":-10.000001022828835},{"name":"nextSyncCursor","qualified_name":"D-project-todo.packages.domain.src.sync.nextSyncCursor","label":"Function","file_path":"packages/domain/src/sync.ts","start_line":4,"end_line":6,"rank":-10.000001022828835},{"name":"newId","qualified_name":"D-project-todo.apps.desktop.src.stores.todos.newId","label":"Function","file_path":"apps/desktop/src/stores/todos.ts","start_line":5,"end_line":5,"rank":-10.000001000832693},{"name":"getRuntimeInfo","qualified_name":"D-project-todo.apps.desktop.src.bindings.commands.getRuntimeInfo","label":"Function","file_path":"apps/desktop/src/bindings/commands.ts","start_line":6,"end_line":6,"rank":-10.000000979762698},{"name":"sync","qualified_name":"D-project-todo.apps.api.src.modules.sync.service.SyncService.sync","label":"Method","file_path":"apps/api/src/modules/sync/service.ts","start_line":9,"end_line":30,"rank":-10.00000095956156},{"name":"package","qualified_name":"D-project-todo.apps.desktop.src-tauri.Cargo.package","label":"Class","file_path":"apps/desktop/src-tauri/Cargo.toml","start_line":1,"end_line":9,"rank":-7.275819246399578},{"name":"lib","qualified_name":"D-project-todo.apps.desktop.src-tauri.Cargo.lib","label":"Class","file_path":"apps/desktop/src-tauri/Cargo.toml","start_line":9,"end_line":13,"rank":-7.275819246399578},{"name":"dependencies","qualified_name":"D-project-todo.apps.desktop.src-tauri.Cargo.dependencies","label":"Class","file_path":"apps/desktop/src-tauri/Cargo.toml","start_line":16,"end_line":25,"rank":-7.275819246399578},{"name":"Todo","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.lib.Todo","label":"Class","file_path":"apps/desktop/src-tauri/src/lib.rs","start_line":9,"end_line":15,"rank":-7.2095221943831405},{"name":"TodoStatus","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.lib.TodoStatus","label":"Enum","file_path":"apps/desktop/src-tauri/src/lib.rs","start_line":19,"end_line":22,"rank":-7.177800711362212},{"name":"build-dependencies","qualified_name":"D-project-todo.apps.desktop.src-tauri.Cargo.build-dependencies","label":"Class","file_path":"apps/desktop/src-tauri/Cargo.toml","start_line":13,"end_line":16,"rank":-7.177800334639992},{"name":"RuntimeInfo","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.lib.RuntimeInfo","label":"Class","file_path":"apps/desktop/src-tauri/src/lib.rs","start_line":26,"end_line":29,"rank":-7.177800334639992},{"name":"resources","qualified_name":"D-project-todo.apps.desktop.src-tauri.icons.android.values.ic_launcher_background.resources","label":"Class","file_path":"apps/desktop/src-tauri/icons/android/values/ic_launcher_background.xml","start_line":2,"end_line":4,"rank":-6.978926122388473},{"name":"color","qualified_name":"D-project-todo.apps.desktop.src-tauri.icons.android.values.ic_launcher_background.color","label":"Class","file_path":"apps/desktop/src-tauri/icons/android/values/ic_launcher_background.xml","start_line":3,"end_line":3,"rank":-6.978926122388473},{"name":"foreground","qualified_name":"D-project-todo.apps.desktop.src-tauri.icons.android.mipmap-anydpi-v26.ic_launcher.foreground","label":"Class","file_path":"apps/desktop/src-tauri/icons/android/mipmap-anydpi-v26/ic_launcher.xml","start_line":3,"end_line":3,"rank":-6.928606656152907},{"name":"background","qualified_name":"D-project-todo.apps.desktop.src-tauri.icons.android.mipmap-anydpi-v26.ic_launcher.background","label":"Class","file_path":"apps/desktop/src-tauri/icons/android/mipmap-anydpi-v26/ic_launcher.xml","start_line":4,"end_line":4,"rank":-6.928606656152907},{"name":"adaptive-icon","qualified_name":"D-project-todo.apps.desktop.src-tauri.icons.android.mipmap-anydpi-v26.ic_launcher.adaptive-icon","label":"Class","file_path":"apps/desktop/src-tauri/icons/android/mipmap-anydpi-v26/ic_launcher.xml","start_line":2,"end_line":5,"rank":-6.857749321315009},{"name":"Todo","qualified_name":"D-project-todo.apps.desktop.src.bindings.models.Todo.Todo","label":"Type","file_path":"apps/desktop/src/bindings/models/Todo.ts","start_line":4,"end_line":10,"rank":-5.000001821835103}],"has_more":true}
> ```
>
> </details>

</details>

结论：这 5 个项目适合“借鉴架构与交互模式”，不适合直接整包提取。当前待办应用应优先建立一层小型平台适配层，再逐项接入 Tauri 官方插件。

这批项目实际来自 Awesome Tauri 社区清单；Tauri 官方已将旧 Showcase 改为指向该清单，而不是逐个“官方推荐”。[说明](https://github.com/orgs/tauri-apps/discussions/3969)

| 项目       | 最可取能力                                                    | 对待办应用的取舍                                          |
| ---------- | ------------------------------------------------------------- | --------------------------------------------------------- |
| Anarlog    | 启动时限时加载主题、防闪烁；日历/通知的领域边界；独立窗口通信 | 取主题初始化、日期/通知建模；不取录音、AI、日历供应商实现 |
| Blinko     | 托盘：快速新建、显示/隐藏、设置、退出；主题同步到原生窗口     | 这是“桌面常驻待办”的最佳参考                              |
| Kanri      | 看板拖拽、卡片截止日期、主题与排序 Pinia 状态                 | 适合提炼待办领域与交互；代码不能直接复制                  |
| Spacedrive | 多窗口和桌面卡片定位；设置页信息架构；设备间通信分层          | 只取窗口卡片和“核心服务/客户端”边界，绝不引入其完整内核   |
| Voicetypr  | 隐藏到托盘前先确认托盘存在；动态托盘菜单；快捷键、日志脱敏    | 非常适合常驻应用的健壮性细节                              |

建议落到当前工程的公共能力：

- 前端 `platform/appearance`：`light / dark / system`，先读取已保存设置，再渲染；变化时同步 Web 主题和原生窗口主题。
- 前端 `platform/settings`：集中保存语言、周起始日、时区、主题、通知、开机启动、常驻、快捷键等；移动端自动隐藏桌面专属选项。
- 前端 `domain/date`：明确区分 `date-only`（截止日）与 `datetime`（提醒时刻），避免用 UTC 转换造成“日期倒退一天”。
- 客户端 `tray`：托盘菜单固定包含“快速新增 / 打开今日 / 设置 / 退出”；关闭主窗口默认隐藏，不退出。
- 客户端 `window-cards`：把桌面卡片实现为单独的轻量 Tauri 窗口，按显示器缩放和工作区定位，而不是前端绝对定位。
- 客户端 `notifications`：通知点击携带 `todoId`，打开主窗口并定位到任务。
- 客户端 `diagnostics`：Rust 与前端日志统一落盘；前端只通过受限 command 读取日志尾部，上传或导出前脱敏。
- 通信：普通请求用 type-safe command；进度、同步流等有序高频数据用 `Channel`；只把轻量 UI 广播放进 event。Tauri 明确建议大数据/流式通信优先用 Channel，而不是 event。[通信模型](https://v2.tauri.app/es/develop/calling-frontend/)

桌面常驻的关键防护，建议直接采用 Voicetypr 的思路：托盘未成功创建时，不能隐藏主窗口，否则用户会失去重新打开应用的入口。托盘本身可通过 Tauri 原生 API 创建菜单、监听点击、恢复并聚焦窗口。[Tauri 托盘文档](https://v2.tauri.app/learn/system-tray/)

优先级建议：

1. P0：设置、主题、日期模型、日志。
2. P1：托盘常驻、关闭隐藏、通知、桌面卡片、全局快捷键。
3. P2：多端同步与设备通信；此时参考 Spacedrive 的“核心服务与客户端解耦”，但不引入其复杂的 P2P/VDFS。

许可证必须先处理：

- Anarlog：MIT，可在保留版权声明前提下复用少量实现。
- Blinko、Kanri：GPL-3.0，只能借鉴模式，不应复制进非 GPL 产品。
- Spacedrive：本地副本为 FSL 1.1，先仅借鉴设计。
- Voicetypr：AGPL-3.0，同样不应直接复制到本项目。[其仓库说明](https://github.com/moinulmoin/voicetypr)

Tauri 的 `autostart`、`global-shortcut`、`log`、`notification`、`store`、`window-state` 等官方插件可覆盖大半平台能力；实施时应按平台能力开关，不把桌面能力假定为移动端能力。[官方插件列表](https://v2.tauri.app/ja/plugin/)

本轮仅完成了调研与本地代码结构核对，没有修改文件。

> 不考虑许可证 这不是商业项目
>
> 优先考虑 是采用官方插件能力还是推荐项目的第三方实现
>
> 推荐应用哪些公共能力和设计

<details><summary>2 previous messages</summary>

> 我会按“官方插件优先、推荐项目补足产品级模式”的原则重新排序，并把每项能力明确到该用官方能力、该参考哪个项目，或需要自行封装。重点会放在当前待办的桌面常驻、卡片、设置、日期、通信和日志。
>
> <details><summary>Called 3 tools, searched the web</summary>
>
> <details><summary>Searched the web</summary>
>
> - Searched the web for `site:v2.tauri.app plugin store documentation ...`
>
> </details>
>
> MCP tool call
>
> codebase-memory-mcp.search_graph
>
> ```json
> {
>   "project": "D-rust-tarui-demo-voicetypr-main",
>   "query": "tauri plugin store autostart global shortcut log notification",
>   "limit": 24,
>   "include_connected": true
> }
> ```
>
> ```text
> {"total":1239,"search_mode":"bm25","results":[{"name":"set_global_shortcut","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.settings.set_global_shortcut","label":"Function","file_path":"src-tauri/src/commands/settings.rs","start_line":975,"end_line":1041,"rank":-27.752292416697976},{"name":"set_autostart","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.settings.set_autostart","label":"Function","file_path":"src-tauri/src/commands/settings.rs","start_line":1436,"end_line":1461,"rank":-21.33491321696213},{"name":"get_autostart_status","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.settings.get_autostart_status","label":"Function","file_path":"src-tauri/src/commands/settings.rs","start_line":1423,"end_line":1430,"rank":-21.067210627382376},{"name":"test_autostart_commands_exist","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.settings.test_autostart_commands_exist","label":"Function","file_path":"src-tauri/src/commands/settings.rs","start_line":1594,"end_line":1600,"rank":-20.811861230996293},{"name":"test_effective_enhancement_options_keeps_global_personal_without_override","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.ai.tests.test_effective_enhancement_options_keeps_global_personal_without_override","label":"Function","file_path":"src-tauri/src/ai/tests.rs","start_line":296,"end_line":304,"rank":-19.32315652045005},{"name":"forced_message_preset_uses_message_transform_with_global_personal","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.ai.tests.forced_message_preset_uses_message_transform_with_global_personal","label":"Function","file_path":"src-tauri/src/ai/tests.rs","start_line":309,"end_line":317,"rank":-19.32315652045005},{"name":"test_app_rule_message_overrides_global_personal_for_effective_mode","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.writing.test_app_rule_message_overrides_global_personal_for_effective_mode","label":"Function","file_path":"src-tauri/src/writing.rs","start_line":2337,"end_line":2356,"rank":-19.259309919781813},{"name":"test_resolve_effective_writing_preset_app_rule_message_overrides_global_personal","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.writing.test_resolve_effective_writing_preset_app_rule_message_overrides_global_personal","label":"Function","file_path":"src-tauri/src/writing.rs","start_line":3833,"end_line":3864,"rank":-19.072911286737686},{"name":"test_resolve_effective_writing_preset_falls_back_to_global_without_active_app","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.writing.test_resolve_effective_writing_preset_falls_back_to_global_without_active_app","label":"Function","file_path":"src-tauri/src/writing.rs","start_line":3867,"end_line":3886,"rank":-18.893869317405127},{"name":"requestNotificationPermission","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.services.updateService.UpdateService.requestNotificationPermission","label":"Method","file_path":"src/services/updateService.ts","start_line":171,"end_line":194,"rank":-18.583804475148945},{"name":"sendSystemNotification","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.services.updateService.UpdateService.sendSystemNotification","label":"Method","file_path":"src/services/updateService.ts","start_line":199,"end_line":212,"rank":-18.583804475148945},{"name":"checkAutostart","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.components.sections.GeneralSettings.checkAutostart","label":"Function","file_path":"src/components/sections/GeneralSettings.tsx","start_line":144,"end_line":151,"rank":-17.888448021305738},{"name":"shortcut_settings_default_empty","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.tests.shortcut_bindings.shortcut_settings_default_empty","label":"Function","file_path":"src-tauri/src/tests/shortcut_bindings.rs","start_line":27,"end_line":29,"rank":-17.81132848817749},{"name":"handleAutostartToggle","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.components.sections.GeneralSettings.handleAutostartToggle","label":"Function","file_path":"src/components/sections/GeneralSettings.tsx","start_line":316,"end_line":335,"rank":-17.75785219317116},{"name":"is_single_key_shortcut_detection","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.tests.shortcut_bindings.is_single_key_shortcut_detection","label":"Function","file_path":"src-tauri/src/tests/shortcut_bindings.rs","start_line":470,"end_line":482,"rank":-17.691550454658714},{"name":"normalized_custom_shortcut_conflict_ignores_disabled_bindings","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.tests.shortcut_bindings.normalized_custom_shortcut_conflict_ignores_disabled_bindings","label":"Function","file_path":"src-tauri/src/tests/shortcut_bindings.rs","start_line":195,"end_line":215,"rank":-17.462686464161337},{"name":"eventToShortcut","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.lib.shortcut-event-match.eventToShortcut","label":"Function","file_path":"src/lib/shortcut-event-match.ts","start_line":43,"end_line":64,"rank":-17.339105291839324},{"name":"eventMatchesShortcut","qualified_name":"D-rust-tarui-demo-voicetypr-main.src.lib.shortcut-event-match.eventMatchesShortcut","label":"Function","file_path":"src/lib/shortcut-event-match.ts","start_line":70,"end_line":75,"rank":-17.339105291839324},{"name":"hold_shortcut_transition","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.shortcuts.hold_shortcut_transition","label":"Function","file_path":"src-tauri/src/commands/shortcuts.rs","start_line":130,"end_line":151,"rank":-17.107998015407446},{"name":"get_shortcut_settings","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.shortcuts.get_shortcut_settings","label":"Function","file_path":"src-tauri/src/commands/shortcuts.rs","start_line":160,"end_line":162,"rank":-17.107998015407446},{"name":"list_shortcut_actions","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.shortcuts.list_shortcut_actions","label":"Function","file_path":"src-tauri/src/commands/shortcuts.rs","start_line":165,"end_line":167,"rank":-17.107998015407446},{"name":"update_shortcut_settings","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.shortcuts.update_shortcut_settings","label":"Function","file_path":"src-tauri/src/commands/shortcuts.rs","start_line":170,"end_line":193,"rank":-17.107998015407446},{"name":"validate_shortcut_settings","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.shortcuts.validate_shortcut_settings","label":"Function","file_path":"src-tauri/src/commands/shortcuts.rs","start_line":323,"end_line":329,"rank":-17.107998015407446},{"name":"prepare_shortcut_settings","qualified_name":"D-rust-tarui-demo-voicetypr-main.src-tauri.src.commands.shortcuts.prepare_shortcut_settings","label":"Function","file_path":"src-tauri/src/commands/shortcuts.rs","start_line":331,"end_line":398,"rank":-17.107998015407446}],"has_more":true}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.search_graph
>
> ```json
> {
>   "project": "D-rust-tarui-demo-blinko-main",
>   "query": "tauri plugin store global shortcut autostart notification log",
>   "limit": 24,
>   "include_connected": true
> }
> ```
>
> ```text
> {"total":422,"search_mode":"bm25","results":[{"name":"create_global_shortcut_handler","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.setup.create_global_shortcut_handler","label":"Function","file_path":"app/src-tauri/src/desktop/setup.rs","start_line":109,"end_line":227,"rank":-32.227399496771994},{"name":"register_shortcut_command","qualified_name":"D-rust-tarui-demo-blinko-main.app.src-tauri.src.desktop.hotkey.register_shortcut_command","label":"Function","file_path":"app/src-tauri/src/desktop/hotkey.rs","start_line":127,"end_line":130,"rank":-24.168744334758898},{"name":"CreateNotification","qualified_name":"D-rust-tarui-demo-blinko-main.server.routerTrpc.notification.CreateNotification","label":"Function","file_path":"server/routerTrpc/notification.ts","start_line":6,"end_line":30,"rank":-19.992475228149218},{"name":"loadPlugin","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.plugin.pluginManagerStore.PluginManagerStore.loadPlugin","label":"Method","file_path":"app/src/store/plugin/pluginManagerStore.ts","start_line":407,"end_line":440,"rank":-18.982215948417757},{"name":"destroyPlugin","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.plugin.pluginManagerStore.PluginManagerStore.destroyPlugin","label":"Method","file_path":"app/src/store/plugin/pluginManagerStore.ts","start_line":509,"end_line":528,"rank":-18.982215948417757},{"name":"installPlugin","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.plugin.pluginManagerStore.PluginManagerStore.installPlugin","label":"Method","file_path":"app/src/store/plugin/pluginManagerStore.ts","start_line":530,"end_line":540,"rank":-18.982215948417757},{"name":"uninstallPlugin","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.plugin.pluginManagerStore.PluginManagerStore.uninstallPlugin","label":"Method","file_path":"app/src/store/plugin/pluginManagerStore.ts","start_line":542,"end_line":551,"rank":-18.982215948417757},{"name":"initPlugin","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.plugin.pluginManagerStore.PluginManagerStore.initPlugin","label":"Method","file_path":"app/src/store/plugin/pluginManagerStore.ts","start_line":556,"end_line":575,"rank":-18.982215948417757},{"name":"connectDevPlugin","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.plugin.pluginManagerStore.PluginManagerStore.connectDevPlugin","label":"Method","file_path":"app/src/store/plugin/pluginManagerStore.ts","start_line":134,"end_line":166,"rank":-18.887112387493225},{"name":"loadDevPlugin","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.plugin.pluginManagerStore.PluginManagerStore.loadDevPlugin","label":"Method","file_path":"app/src/store/plugin/pluginManagerStore.ts","start_line":385,"end_line":395,"rank":-18.887112387493225},{"name":"disconnectDevPlugin","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.plugin.pluginManagerStore.PluginManagerStore.disconnectDevPlugin","label":"Method","file_path":"app/src/store/plugin/pluginManagerStore.ts","start_line":397,"end_line":405,"rank":-18.887112387493225},{"name":"autoConnectDevPlugin","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.plugin.pluginManagerStore.PluginManagerStore.autoConnectDevPlugin","label":"Method","file_path":"app/src/store/plugin/pluginManagerStore.ts","start_line":43,"end_line":50,"rank":-18.79403694524897},{"name":"handleLegacyDevPlugin","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.plugin.pluginManagerStore.PluginManagerStore.handleLegacyDevPlugin","label":"Method","file_path":"app/src/store/plugin/pluginManagerStore.ts","start_line":241,"end_line":248,"rank":-18.79403694524897},{"name":"processDevPluginFiles","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.plugin.pluginManagerStore.PluginManagerStore.processDevPluginFiles","label":"Method","file_path":"app/src/store/plugin/pluginManagerStore.ts","start_line":253,"end_line":270,"rank":-18.79403694524897},{"name":"handlePluginNotFound","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.plugin.pluginManagerStore.PluginManagerStore.handlePluginNotFound","label":"Method","file_path":"app/src/store/plugin/pluginManagerStore.ts","start_line":445,"end_line":503,"rank":-18.79403694524897},{"name":"getActiveEditorStore","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.plugin.pluginApiStore.PluginApiStore.getActiveEditorStore","label":"Method","file_path":"app/src/store/plugin/pluginApiStore.tsx","start_line":80,"end_line":86,"rank":-18.765710984399863},{"name":"handleMultiFileDevPlugin","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.plugin.pluginManagerStore.PluginManagerStore.handleMultiFileDevPlugin","label":"Method","file_path":"app/src/store/plugin/pluginManagerStore.ts","start_line":214,"end_line":236,"rank":-18.702924349014665},{"name":"getPluginInstanceByName","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.plugin.pluginManagerStore.PluginManagerStore.getPluginInstanceByName","label":"Method","file_path":"app/src/store/plugin/pluginManagerStore.ts","start_line":505,"end_line":507,"rank":-18.702924349014665},{"name":"isIntalledPluginWithSettingPanel","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.plugin.pluginManagerStore.PluginManagerStore.isIntalledPluginWithSettingPanel","label":"Method","file_path":"app/src/store/plugin/pluginManagerStore.ts","start_line":66,"end_line":68,"rank":-18.613712140315698},{"name":"constructor","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.plugin.BasePlugin.constructor","label":"Method","file_path":"app/src/store/plugin/index.ts","start_line":93,"end_line":95,"rank":-18.57912020760441},{"name":"logTransportError","qualified_name":"D-rust-tarui-demo-blinko-main.server.routerExpress.mcp.logTransportError","label":"Function","file_path":"server/routerExpress/mcp.ts","start_line":117,"end_line":120,"rank":-18.478387012578757},{"name":"constructor","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.plugin.pluginApiStore.PluginApiStore.constructor","label":"Method","file_path":"app/src/store/plugin/pluginApiStore.tsx","start_line":76,"end_line":78,"rank":-18.47231653981468},{"name":"constructor","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.plugin.pluginManagerStore.PluginManagerStore.constructor","label":"Method","file_path":"app/src/store/plugin/pluginManagerStore.ts","start_line":37,"end_line":41,"rank":-18.47231653981468},{"name":"showDialog","qualified_name":"D-rust-tarui-demo-blinko-main.app.src.store.plugin.pluginApiStore.PluginApiStore.showDialog","label":"Method","file_path":"app/src/store/plugin/pluginApiStore.tsx","start_line":138,"end_line":145,"rank":-18.368139425887726}],"has_more":true}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.search_graph
>
> ```json
> {
>   "project": "D-rust-tarui-demo-anarlog-main",
>   "query": "tauri plugin store global shortcut autostart notification log",
>   "limit": 24,
>   "include_connected": true
> }
> ```
>
> ```text
> {"total":804,"search_mode":"bm25","results":[{"name":"remove_notification_global","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.notification-linux.src.impl.NotificationManager.remove_notification_global","label":"Method","file_path":"crates/notification-linux/src/impl.rs","start_line":317,"end_line":321,"rank":-27.16511306289963},{"name":"store_context","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.notification.src.lib.store_context","label":"Function","file_path":"crates/notification/src/lib.rs","start_line":35,"end_line":43,"rank":-22.022318345980057},{"name":"desktop_store","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src-tauri.src.ext.AppExt.desktop_store","label":"Method","file_path":"apps/desktop/src-tauri/src/ext.rs","start_line":4,"end_line":4,"rank":-21.81718864876112},{"name":"desktop_store","qualified_name":"D-rust-tarui-demo-anarlog-main.apps.desktop.src-tauri.src.ext.T.desktop_store","label":"Method","file_path":"apps/desktop/src-tauri/src/ext.rs","start_line":23,"end_line":27,"rank":-21.81718864876112},{"name":"ensureGlobalMouseMonitor","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.notification-macos.swift-lib.src.NotificationManager+MouseTracking.NotificationManager.ensureGlobalMouseMonitor","label":"Method","file_path":"crates/notification-macos/swift-lib/src/NotificationManager+MouseTracking.swift","start_line":4,"end_line":24,"rank":-21.188129585874037},{"name":"global_base","qualified_name":"D-rust-tarui-demo-anarlog-main.plugins.settings.src.commands.global_base","label":"Function","file_path":"plugins/settings/src/commands.rs","start_line":8,"end_line":15,"rank":-21.0797673212385},{"name":"global_base","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.storage.src.runtime.StorageRuntime.global_base","label":"Method","file_path":"crates/storage/src/runtime.rs","start_line":4,"end_line":4,"rank":-20.98411536708742},{"name":"global_base","qualified_name":"D-rust-tarui-demo-anarlog-main.plugins.settings.src.ext.Settings.global_base","label":"Method","file_path":"plugins/settings/src/ext.rs","start_line":26,"end_line":28,"rank":-20.98411536708742},{"name":"global_base","qualified_name":"D-rust-tarui-demo-anarlog-main.plugins.transcription.src.listener.runtime.TauriRuntime.global_base","label":"Method","file_path":"plugins/transcription/src/listener/runtime.rs","start_line":16,"end_line":22,"rank":-20.79768196105426},{"name":"track_global_restart","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.supervisor.src.dynamic.DynamicSupervisorState.track_global_restart","label":"Method","file_path":"crates/supervisor/src/dynamic.rs","start_line":207,"end_line":233,"rank":-20.706818537780386},{"name":"global_config_path","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.owhisper-config.src.lib.global_config_path","label":"Function","file_path":"crates/owhisper-config/src/lib.rs","start_line":60,"end_line":62,"rank":-20.617471600207338},{"name":"log","qualified_name":"D-rust-tarui-demo-anarlog-main.plugins.git.src.commands.log","label":"Function","file_path":"plugins/git/src/commands.rs","start_line":65,"end_line":71,"rank":-20.54096423866168},{"name":"log","qualified_name":"D-rust-tarui-demo-anarlog-main.plugins.git.src.ext.Git.log","label":"Method","file_path":"plugins/git/src/ext.rs","start_line":36,"end_line":38,"rank":-20.447544295960476},{"name":"global_base","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.listener-core.src.actors.listener.adapters.NoopRuntime.global_base","label":"Method","file_path":"crates/listener-core/src/actors/listener/adapters.rs","start_line":425,"end_line":427,"rank":-20.443177816642162},{"name":"global_base","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.listener-core.src.actors.session.supervisor.TestRuntime.global_base","label":"Method","file_path":"crates/listener-core/src/actors/session/supervisor.rs","start_line":498,"end_line":500,"rank":-20.443177816642162},{"name":"global_base","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.listener-core.src.actors.session.supervisor.RecordingRuntime.global_base","label":"Method","file_path":"crates/listener-core/src/actors/session/supervisor.rs","start_line":609,"end_line":611,"rank":-20.443177816642162},{"name":"global_base","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.listener-core.src.actors.source.pipeline.TestRuntime.global_base","label":"Method","file_path":"crates/listener-core/src/actors/source/pipeline.rs","start_line":407,"end_line":409,"rank":-20.443177816642162},{"name":"log","qualified_name":"D-rust-tarui-demo-anarlog-main.plugins.git.js.bindings.gen.log","label":"Function","file_path":"plugins/git/js/bindings.gen.ts","start_line":57,"end_line":64,"rank":-20.35576568679499},{"name":"log","qualified_name":"D-rust-tarui-demo-anarlog-main.plugins.git.src.operations.local.log","label":"Function","file_path":"plugins/git/src/operations/local.rs","start_line":375,"end_line":420,"rank":-20.35576568679499},{"name":"do_log","qualified_name":"D-rust-tarui-demo-anarlog-main.plugins.tracing.src.commands.do_log","label":"Function","file_path":"plugins/tracing/src/commands.rs","start_line":13,"end_line":19,"rank":-20.265585531946954},{"name":"log_content","qualified_name":"D-rust-tarui-demo-anarlog-main.plugins.tracing.src.commands.log_content","label":"Function","file_path":"plugins/tracing/src/commands.rs","start_line":23,"end_line":27,"rank":-20.265585531946954},{"name":"do_log","qualified_name":"D-rust-tarui-demo-anarlog-main.plugins.tracing.src.ext.Tracing.do_log","label":"Method","file_path":"plugins/tracing/src/ext.rs","start_line":20,"end_line":39,"rank":-20.176962432908258},{"name":"log_content","qualified_name":"D-rust-tarui-demo-anarlog-main.plugins.tracing.src.ext.Tracing.log_content","label":"Method","file_path":"plugins/tracing/src/ext.rs","start_line":41,"end_line":72,"rank":-20.176962432908258},{"name":"attach_log_analysis","qualified_name":"D-rust-tarui-demo-anarlog-main.crates.api-support.src.github.attach_log_analysis","label":"Function","file_path":"crates/api-support/src/github.rs","start_line":293,"end_line":315,"rank":-19.837260989769838}],"has_more":true}
> ```
>
> </details>

</details>

优先级应明确为：官方 Tauri 能力做底座，推荐项目只提供“产品设计、边界和健壮性实现”。不要用推荐项目的自定义实现去替代已有官方插件。

建议比例：官方能力约 80%，项目级封装与交互模式约 20%。

| 场景           | 首选方案                                    | 参考项目              | 应采用的设计                                                                           |
| -------------- | ------------------------------------------- | --------------------- | -------------------------------------------------------------------------------------- |
| 桌面常驻、托盘 | Tauri Core `tray-icon`                      | Blinko、Voicetypr     | 托盘固定有“快速新增、今日待办、设置、退出”；关闭窗口变隐藏而非退出                     |
| 开机启动       | `tauri-plugin-autostart`                    | Voicetypr             | 设置页开关；启动后检查真实注册状态，不只相信前端缓存                                   |
| 全局快捷键     | `tauri-plugin-global-shortcut`              | Blinko、Voicetypr     | “快速新增待办”默认快捷键；冲突检测、失败提示、用户可改                                 |
| 主题           | 前端 UnoCSS/主题令牌 + Tauri `setTheme`     | Anarlog、Blinko       | `light/dark/system` 三态；启动时先恢复主题，避免白屏闪烁；同步原生窗口背景色和托盘图标 |
| 设置持久化     | `tauri-plugin-store` + Pinia                | Kanri、Spacedrive     | Pinia 是运行态，Store 是持久化；只存偏好，不把任务数据塞进设置文件                     |
| 日志           | `tauri-plugin-log`                          | Voicetypr、Anarlog    | Rust/前端统一日志；设置页可查看最近日志和导出诊断包；任务正文、令牌等默认脱敏          |
| 通知提醒       | `tauri-plugin-notification`                 | Anarlog               | 通知载荷仅含 `todoId`；点击后显示主窗口并定位任务；提醒调度仍由领域层管理              |
| 窗口记忆       | `tauri-plugin-window-state`                 | Spacedrive            | 记忆主窗口位置、尺寸、最大化状态；不建议恢复桌面卡片的临时位置                         |
| 单实例         | `tauri-plugin-single-instance`              | Blinko                | 重复启动时唤醒已有主窗口或触发“快速新增”，不再启动第二个进程                           |
| 桌面卡片       | Tauri 多 `WebviewWindow`，不依赖插件        | Spacedrive、Voicetypr | 每张卡片是独立轻量窗口；按显示器 DPI、工作区和边距定位；可关闭、固定、点击打开对应任务 |
| 前后端通信     | 当前 `tauri-specta` command bindings        | Anarlog、Spacedrive   | 查询/操作走 command；高频进度、同步流走 `Channel`；只把轻量 UI 广播放 event            |
| HTTP / 同步    | Axum 契约 + 官方 `http` 或 `websocket` 插件 | Spacedrive            | 传输层替换不影响任务领域；同步、冲突、重试不能散在 Vue 组件中                          |
| 日期与截止日   | 领域模型 + 前端日期工具，不需要 Tauri 插件  | Kanri                 | `date-only` 与 `datetime` 分开：截止日是日期，提醒是带时区的时刻                       |

官方插件覆盖了 autostart、store、log、notification、global-shortcut、single-instance、window-state 等基础能力；并且 capability 权限默认是收紧的，按需授权即可。[官方插件清单](https://v2.tauri.app/ja/plugin/) [Autostart 权限说明](https://v2.tauri.app/plugin/autostart/)

推荐采用的公共设计如下：

```text
Vue 页面 / 组件
        ↓ 只依赖
Pinia stores + domain composables
        ↓
platform 接口层
  ├─ appearance
  ├─ settings
  ├─ tray
  ├─ windows
  ├─ notifications
  ├─ diagnostics
  └─ shortcuts
        ↓
Tauri 官方插件 / Tauri Core / 浏览器降级实现
```

核心原则：

- Vue 组件不直接调用 `@tauri-apps/*`；只调用 `platform` 接口，移动端和 Web 端可替换为降级实现。
- `settings` 是一份跨端设置模型，但以能力开关决定是否显示桌面项。例如 Android/iOS 不显示“开机启动、托盘、全局快捷键、桌面卡片”。
- 托盘是“桌面控制中心”，不是完整主界面：快速新增、未完成数、今日任务、暂停提醒、设置、退出足够。
- 桌面卡片先做一个“今日待办”窗口，不要一开始就做每任务一窗口；后续再支持固定单任务卡片。
- 主题的权威状态在 Pinia/settings；原生窗口、托盘图标、通知样式都监听同一主题变化。
- 日期应先确定产品规则：截止日按用户本地日期保存；提醒按带时区的具体时刻保存。Kanri 的卡片截止日设计值得参考，但需补齐时区规则。

推荐项目的具体借鉴定位：

- Blinko：作为托盘、快捷新增、显示/隐藏主窗口的直接实现参考。
- Voicetypr：作为“关闭隐藏、托盘失败保护、动态托盘菜单、快捷键冲突、日志诊断”的可靠性参考。
- Anarlog：作为主题启动、设置加载超时、通知与多窗口通信分层参考。
- Kanri：作为待办卡片、看板拖拽、截止日期、排序与主题 Pinia 状态参考。
- Spacedrive：只作为多窗口卡片、设置导航与跨设备架构参考，不引入其复杂的文件系统/P2P 内核。

通信方面，应坚持 command / Channel / event 三层分工；Tauri 官方说明中，`Channel` 比 event 更适合有序、高频、流式数据。[通信建议](https://v2.tauri.app/es/develop/calling-frontend/)

最值得先实现的公共能力顺序是：

1. `settings + appearance + log`
2. `tray + close-to-hide + single-instance`
3. `notification + due-date/reminder`
4. `global shortcut + quick add`
5. `today desktop card`
6. `sync transport + device communication`

这样既最大化使用官方能力，也能把五个推荐项目中最成熟的产品设计吸收进来。

> 推进

<details><summary>13 previous messages</summary>

> 我会把“推进”按第一里程碑执行：先交付可跨端复用的 `settings + appearance + diagnostics/log` 基础层；托盘、常驻和桌面卡片放到下一里程碑，避免一次引入多窗口与平台行为。现在先按标准流程建立任务上下文、核对当前分支和现有 Tauri/前端结构。
>
> <details><summary>Ran 3 commands, called 2 tools</summary>
>
> - Ran `rtk git status --short --branch`
> - Ran `rtk proxy powershell -NoProfile -Command "Get-Content -LiteralPath '.agents\agents\product.md' -Raw; Get-Content -LiteralPath '.agents\agents\ux-ui.md' -Raw; Get-Content -LiteralPath '.agents\agents\developer.md' -Raw; Get-Content -LiteralPath '.agents\agents\qa.md' -Raw; Get-Content -LiteralPath '.agents\agents\reviewer.md' -Raw; Get-Content -LiteralPath '.agents\agents\release.md' -Raw; Get-ChildItem -LiteralPath '.agents\docs\tasks\_template' -Recurse -File | Select-Object -ExpandProperty FullName"`
>
> MCP tool call
>
> codebase-memory-mcp.search_graph
>
> ```json
> {
>   "project": "D-project-todo",
>   "query": "main app component Pinia UnoCSS Tauri configuration capability commands",
>   "limit": 100,
>   "include_connected": true
> }
> ```
>
> ```text
> {"total":29,"search_mode":"bm25","results":[{"name":"main","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.main.main","label":"Function","file_path":"apps/desktop/src-tauri/src/main.rs","start_line":1,"end_line":3,"rank":-19.6669869234374},{"name":"main","qualified_name":"D-project-todo.apps.desktop.src-tauri.build.main","label":"Function","file_path":"apps/desktop/src-tauri/build.rs","start_line":1,"end_line":3,"rank":-19.008121978999974},{"name":"getRuntimeInfo","qualified_name":"D-project-todo.apps.desktop.src.bindings.commands.getRuntimeInfo","label":"Function","file_path":"apps/desktop/src/bindings/commands.ts","start_line":6,"end_line":6,"rank":-16.337478582928274},{"name":"createApp","qualified_name":"D-project-todo.apps.api.src.app.createApp","label":"Function","file_path":"apps/api/src/app.ts","start_line":13,"end_line":41,"rank":-15.281294156472667},{"name":"run","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.lib.run","label":"Function","file_path":"apps/desktop/src-tauri/src/lib.rs","start_line":53,"end_line":64,"rank":-12.209520516736045},{"name":"ipc_builder","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.lib.ipc_builder","label":"Function","file_path":"apps/desktop/src-tauri/src/lib.rs","start_line":41,"end_line":43,"rank":-12.11701326588459},{"name":"get_runtime_info","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.lib.get_runtime_info","label":"Function","file_path":"apps/desktop/src-tauri/src/lib.rs","start_line":34,"end_line":39,"rank":-12.03194084408306},{"name":"export_type_bindings","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.lib.export_type_bindings","label":"Function","file_path":"apps/desktop/src-tauri/src/lib.rs","start_line":79,"end_line":89,"rank":-12.03194084408306},{"name":"frontend_bindings_path","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.lib.frontend_bindings_path","label":"Function","file_path":"apps/desktop/src-tauri/src/lib.rs","start_line":72,"end_line":76,"rank":-12.03194084408306},{"name":"package","qualified_name":"D-project-todo.apps.desktop.src-tauri.Cargo.package","label":"Class","file_path":"apps/desktop/src-tauri/Cargo.toml","start_line":1,"end_line":9,"rank":-7.275818223570743},{"name":"lib","qualified_name":"D-project-todo.apps.desktop.src-tauri.Cargo.lib","label":"Class","file_path":"apps/desktop/src-tauri/Cargo.toml","start_line":9,"end_line":13,"rank":-7.275818223570743},{"name":"dependencies","qualified_name":"D-project-todo.apps.desktop.src-tauri.Cargo.dependencies","label":"Class","file_path":"apps/desktop/src-tauri/Cargo.toml","start_line":16,"end_line":25,"rank":-7.275818223570743},{"name":"Todo","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.lib.Todo","label":"Class","file_path":"apps/desktop/src-tauri/src/lib.rs","start_line":9,"end_line":15,"rank":-7.209520516736046},{"name":"build-dependencies","qualified_name":"D-project-todo.apps.desktop.src-tauri.Cargo.build-dependencies","label":"Class","file_path":"apps/desktop/src-tauri/Cargo.toml","start_line":13,"end_line":16,"rank":-7.177799375078433},{"name":"TodoStatus","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.lib.TodoStatus","label":"Enum","file_path":"apps/desktop/src-tauri/src/lib.rs","start_line":19,"end_line":22,"rank":-7.177799375078433},{"name":"RuntimeInfo","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.lib.RuntimeInfo","label":"Class","file_path":"apps/desktop/src-tauri/src/lib.rs","start_line":26,"end_line":29,"rank":-7.177799375078433},{"name":"resources","qualified_name":"D-project-todo.apps.desktop.src-tauri.icons.android.values.ic_launcher_background.resources","label":"Class","file_path":"apps/desktop/src-tauri/icons/android/values/ic_launcher_background.xml","start_line":2,"end_line":4,"rank":-6.97892528385188},{"name":"color","qualified_name":"D-project-todo.apps.desktop.src-tauri.icons.android.values.ic_launcher_background.color","label":"Class","file_path":"apps/desktop/src-tauri/icons/android/values/ic_launcher_background.xml","start_line":3,"end_line":3,"rank":-6.97892528385188},{"name":"app_version","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.lib.RuntimeInfo.app_version","label":"Field","file_path":"apps/desktop/src-tauri/src/lib.rs","start_line":28,"end_line":28,"rank":-6.97507921557425},{"name":"foreground","qualified_name":"D-project-todo.apps.desktop.src-tauri.icons.android.mipmap-anydpi-v26.ic_launcher.foreground","label":"Class","file_path":"apps/desktop/src-tauri/icons/android/mipmap-anydpi-v26/ic_launcher.xml","start_line":3,"end_line":3,"rank":-6.928605846782615},{"name":"background","qualified_name":"D-project-todo.apps.desktop.src-tauri.icons.android.mipmap-anydpi-v26.ic_launcher.background","label":"Class","file_path":"apps/desktop/src-tauri/icons/android/mipmap-anydpi-v26/ic_launcher.xml","start_line":4,"end_line":4,"rank":-6.928605846782615},{"name":"adaptive-icon","qualified_name":"D-project-todo.apps.desktop.src-tauri.icons.android.mipmap-anydpi-v26.ic_launcher.adaptive-icon","label":"Class","file_path":"apps/desktop/src-tauri/icons/android/mipmap-anydpi-v26/ic_launcher.xml","start_line":2,"end_line":5,"rank":-6.857748552078532},{"name":"#[tauri::command]","qualified_name":"<decorator:#[tauri::command]>","label":"Decorator","file_path":"","start_line":0,"end_line":0,"rank":-2.924357890793989},{"name":"id","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.lib.Todo.id","label":"Field","file_path":"apps/desktop/src-tauri/src/lib.rs","start_line":10,"end_line":10,"rank":-2.177799375078432},{"name":"title","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.lib.Todo.title","label":"Field","file_path":"apps/desktop/src-tauri/src/lib.rs","start_line":11,"end_line":11,"rank":-2.177799375078432},{"name":"status","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.lib.Todo.status","label":"Field","file_path":"apps/desktop/src-tauri/src/lib.rs","start_line":12,"end_line":12,"rank":-2.177799375078432},{"name":"platform","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.lib.RuntimeInfo.platform","label":"Field","file_path":"apps/desktop/src-tauri/src/lib.rs","start_line":27,"end_line":27,"rank":-2.177799375078432},{"name":"created_at","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.lib.Todo.created_at","label":"Field","file_path":"apps/desktop/src-tauri/src/lib.rs","start_line":13,"end_line":13,"rank":-2.0878751804044713},{"name":"completed_at","qualified_name":"D-project-todo.apps.desktop.src-tauri.src.lib.Todo.completed_at","label":"Field","file_path":"apps/desktop/src-tauri/src/lib.rs","start_line":14,"end_line":14,"rank":-2.0878751804044713}],"has_more":false}
> ```
>
> MCP tool call
>
> codebase-memory-mcp.detect_changes
>
> ```json
> {
>   "project": "D-project-todo",
>   "scope": "working_tree",
>   "depth": 2
> }
> ```
>
> ```text
> {"changed_files":[],"changed_count":0,"impacted_symbols":[],"depth":2}
> ```
>
> - Ran `rtk proxy powershell -NoProfile -Command "Get-Content -LiteralPath '.agents\docs\tasks\_template\task.json' -Raw; Get-Content -LiteralPath '.agents\docs\tasks\_template\handoff.md' -Raw; Get-Content -LiteralPath 'package.json' -Raw; Get-Content -LiteralPath 'src-tauri\Cargo.toml' -Raw; Get-Content -LiteralPath 'src-tauri\tauri.conf.json' -Raw; Get-ChildItem -LiteralPath 'src' -Recurse -File | Select-Object -ExpandProperty FullName"`
>
> </details>
>
> 当前工作区的基础代码尚未纳入 Git（除已有 `.gitignore` 修改外，大部分文件是未跟踪基线）。我会只新增本里程碑相关的文件，不会整理或提交这些既有未跟踪内容。设置页属于本次可见 UI，我会使用现有 UI/UX 设计指导来保证其桌面与移动端行为一致。
>
> <details><summary>Created 9 files, edited 6 files, deleted a file, ran 5 commands, called a tool</summary>
>
> - Ran `rtk proxy powershell -NoProfile -Command "Get-Content -LiteralPath 'C:\Users\Administrator\.agents\skills\designing-beautiful-websites\SKILL.md' -Raw"`
>
> MCP tool call
>
> codebase-memory-mcp.index_repository
>
> ```json
> {
>   "repo_path": "D:\\project\\todo",
>   "mode": "moderate",
>   "persistence": false
> }
> ```
>
> ```text
> {"project":"D-project-todo","status":"indexed","excluded":{"dirs":[".codebase-memory",".git","dist","node_modules","target","src-tauri/gen","src-tauri/target","packages/contracts/node_modules",".agents/docs",".agents/temp"],"count":10,"truncated":false},"nodes":594,"edges":651,"adr_present":false,"adr_hint":"Project indexed. Consider creating an Architecture Decision Record: explore the codebase with get_architecture(aspects=['all']), then use manage_adr(mode='store') to persist architectural insights across sessions.","artifact_present":true}
> ```
>
> - Ran `rtk proxy powershell -NoProfile -Command "Get-Content -LiteralPath 'src\App.vue' -Raw; Get-Content -LiteralPath 'src\main.ts' -Raw; Get-Content -LiteralPath 'src\styles\main.css' -Raw; Get-Content -LiteralPath 'src\lib\native.ts' -Raw; Get-Content -LiteralPath 'src-tauri\src\lib.rs' -Raw; Get-ChildItem -LiteralPath 'src-tauri\capabilities' -File | ForEach-Object { Get-Content -LiteralPath $_.FullName -Raw }"`
> - Ran `rtk pnpm view @tauri-apps/plugin-store version`
> - Ran `rtk proxy powershell -NoProfile -Command "Get-Content -LiteralPath 'crates\contracts\src\lib.rs' -Raw; Get-Content -LiteralPath 'Cargo.toml' -Raw; Get-Content -LiteralPath 'src\components\ui\button\Button.vue' -Raw; Get-Content -LiteralPath 'uno.config.ts' -Raw"`
> - Ran `rtk proxy powershell -NoProfile -Command "Get-ChildItem -LiteralPath 'src-tauri' -Force | Select-Object Name,Mode; if (Test-Path 'src-tauri\capabilities\default.json') { Get-Content -LiteralPath 'src-tauri\capabilities\default.json' -Raw }"`
> - Wrote `./.agents/docs/tasks/PLATFORM-FOUNDATION-001/architecture.md` (+25 -0)
> - Wrote `./.agents/docs/tasks/PLATFORM-FOUNDATION-001/design.md` (+20 -0)
> - Wrote `./.agents/docs/tasks/PLATFORM-FOUNDATION-001/handoff.md` (+13 -0)
> - Wrote `./.agents/docs/tasks/PLATFORM-FOUNDATION-001/prd.md` (+24 -0)
> - Wrote `./.agents/docs/tasks/PLATFORM-FOUNDATION-001/task.json` (+75 -0)
> - Wrote `./package.json` (+1 -0)
> - Wrote `./src-tauri/Cargo.toml` (+1 -0)
> - Wrote `./src-tauri/capabilities/default.json` (+1 -1)
> - Wrote `./src-tauri/src/lib.rs` (+1 -0)
> - Wrote `./src/lib/appearance.ts` (+24 -0)
> - Wrote `./src/lib/diagnostics.ts` (+44 -0)
> - Wrote `./src/lib/settings-storage.ts` (+57 -0)
> - Wrote `./src/stores/settings.ts` (+43 -0)
> - Wrote `./src/main.ts` (+5 -1)
> - Wrote `./src/styles/main.css` (+7 -0)
> - Deleted `./src/App.vue` (+0 -85)
>
> </details>
>
> Stream error
>
> Reconnecting 1/5
> stream disconnected before completion: Transport error: network error: error decoding response body
>
> <details><summary>Created a file, edited a file</summary>
>
> - Wrote `./src/App.vue` (+162 -0)
> - Wrote `./src/App.vue` (+1 -1)
>
> </details>
>
> 基础实现已落到专用平台层：设置不再散在组件里，主题可在浏览器模式降级，桌面端走官方 Store 与当前窗口主题 API；日志统一经过一个可脱敏的入口。现在正在更新依赖锁定并执行类型检查，预计会根据官方 Store 的实际类型声明做一次小修正。
>
> <details><summary>Edited a file, ran 6 commands</summary>
>
> - Ran `rtk pnpm install --lockfile-only`
> - Ran `rtk pnpm install --frozen-lockfile`
> - Ran `rtk pnpm run check`
> - Ran `rtk pnpm exec vp check --fix`
> - Wrote `./src/lib/settings-storage.ts` (+2 -2)
> - Ran `rtk pnpm run check`
> - Ran `rtk cargo test --manifest-path src-tauri/Cargo.toml export_type_bindings`
>
> </details>
>
> 前端静态检查已进入完整验证阶段；Rust 正在首次解析新增的官方 Store 依赖并运行类型绑定测试。上一轮已经修正了 Store API 要求显式提供默认值的类型问题。
>
> <details><summary>Ran 4 commands</summary>
>
> - Ran `rtk proxy powershell -NoProfile -Command "Get-Content -LiteralPath 'C:\Users\Administrator\AppData\Local\rtk\tee\1784228280_cargo_test.log' -Tail 160"`
> - Ran `rtk cargo test --manifest-path src-tauri/Cargo.toml --no-run`
> - Ran `rtk cargo check --manifest-path src-tauri/Cargo.toml`
> - Ran `rtk pnpm run check`
>
> </details>
>
> 验证结果：前端格式、lint、类型检查通过；Tauri Rust 编译和测试二进制构建通过。完整 Tauri 测试实际启动时被本机 Windows 动态链接环境阻断（`STATUS_ENTRYPOINT_NOT_FOUND`），与新增 Store 代码无关；我会补跑不依赖 Tauri 窗口运行时的 contracts 测试与前端生产构建，并把该环境限制记录到 QA。
>
> <details><summary>Created 2 files, edited 4 files, ran 4 commands</summary>
>
> - Ran `rtk pnpm run build`
> - Ran `rtk cargo test -p todo-contracts`
> - Ran `rtk proxy powershell -NoProfile -Command "Get-Content -LiteralPath 'src\lib\appearance.ts' -Raw; Get-Content -LiteralPath 'src\stores\settings.ts' -Raw"`
> - Wrote `./src/lib/appearance.ts` (+6 -0)
> - Wrote `./src/stores/settings.ts` (+12 -1)
> - Ran `rtk pnpm run check`
> - Wrote `./.agents/docs/tasks/PLATFORM-FOUNDATION-001/handoff.md` (+12 -0)
> - Wrote `./.agents/docs/tasks/PLATFORM-FOUNDATION-001/implementation.md` (+21 -0)
> - Wrote `./.agents/docs/tasks/PLATFORM-FOUNDATION-001/qa-report.md` (+23 -0)
> - Wrote `./.agents/docs/tasks/PLATFORM-FOUNDATION-001/task.json` (+10 -2)
>
> </details>
>
> <details><summary>Plan</summary>
>
> 实现与静态验证已完成；完整 Tauri 运行时测试受本机动态链接环境阻断，QA 需要保留该限制。
>
> - [x] 建立 PLATFORM-FOUNDATION-001 的标准流程文档与验收标准
> - [x] 实现设置、主题和前端诊断日志基础层
> - [x] 接入官方 Store 与原生日志，并生成类型绑定
> - [ ] 运行检查、完成 QA 与审查记录
>
> </details>

</details>

第一里程碑已实现：设置、主题与诊断日志基础层已接入。

- 主题支持浅色、深色、跟随系统；启动恢复偏好，系统主题变化会同步 Web 界面。
- 桌面端使用官方 Store 持久化，浏览器开发模式自动降级到 `localStorage`。
- 当前 Tauri 窗口同步主题；前端日志统一转发到官方 Log 插件。
- 主界面新增可访问的设置面板。

关键文件：

- [App.vue](./src/App.vue)
- [settings.ts](./src/stores/settings.ts)
- [appearance.ts](./src/lib/appearance.ts)
- [settings-storage.ts](./src/lib/settings-storage.ts)
- [diagnostics.ts](./src/lib/diagnostics.ts)
- [Tauri 初始化](./src-tauri/src/lib.rs)
- [任务记录](./.agents/docs/tasks/PLATFORM-FOUNDATION-001/task.json)

验证已通过：

- `pnpm run check`
- `pnpm run build`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml --no-run`
- `cargo test -p todo-contracts`

尚未标记完成：完整 Tauri 测试进程在本机启动前报 `STATUS_ENTRYPOINT_NOT_FOUND`，需要修复 Windows 运行库/动态链接环境后，复验 Store 持久化、原生窗口主题与日志落盘。QA 已记录在 [qa-report.md](./.agents/docs/tasks/PLATFORM-FOUNDATION-001/qa-report.md)。

第二里程碑已实现：托盘、关闭隐藏与桌面行为设置。

- 桌面端通过 `tauri::tray::TrayIconBuilder` 创建原生系统托盘；菜单包含"显示 / 隐藏主界面"、"今日 N 项待完成"、"退出"；双击托盘切换主界面显隐。
- 关闭主界面默认隐藏到托盘（Rust 侧 `WindowEvent::CloseRequested` 拦截并 `prevent_close` + `hide`），可在设置内"关闭按钮隐藏到桌面"开关关闭并恢复原生退出。
- `behavior.closeToTray` 持久化到官方 Store，浏览器模式回退 `localStorage`；桌面行为设置组仅在桌面端可见。
- 托盘丢失时降级为原生退出并写 `warn` 诊断日志，不阻塞启动或崩溃。
- 单实例因 `tauri-plugin-single-instance = "2"` 不在本仓库离线 Cargo 缓存中而推迟，留待联网或纯 Rust 自实现。记录在 [architecture.md](./.agents/docs/tasks/PLATFORM-FOUNDATION-002/architecture.md)。

关键文件：

- [lib.rs](./src-tauri/src/lib.rs)（`setup_tray` / `build_tray` / `close_to_tray_enabled` / 关闭拦截）
- [Cargo.toml](./src-tauri/Cargo.toml)（`image-png` + `tray-icon` feature、`log` 0.4 依赖）
- [settings.ts](./src/stores/settings.ts)（`closeToTray` / `isDesktop` / `setCloseToTray`）
- [settings-storage.ts](./src/lib/settings-storage.ts)（`loadBooleanPreference` / `saveBooleanPreference`）
- [App.vue](./src/App.vue)（桌面行为设置分组）
- [任务记录](./.agents/docs/tasks/PLATFORM-FOUNDATION-002/task.json)

验证已通过：

- `pnpm run check`
- `pnpm run build`
- `cargo check --manifest-path src-tauri/Cargo.toml --offline`
- `cargo test --manifest-path src-tauri/Cargo.toml --no-run --offline`

尚未标记完成：完整 Tauri 桌面运行时测试受本机动态链接环境阻断，待复验桌面实际创建托盘、关闭隐藏与设置保持。QA 已记录在 [qa-report.md](./.agents/docs/tasks/PLATFORM-FOUNDATION-002/qa-report.md)。

第三里程碑已实现：截止日与提醒通知。

- 待办数据结构包含 `dueDate`（本地日历日期，`YYYY-MM-DD`）与 `reminderAt`（带时区 ISO8601 绝对时刻），均为 `Option<String>`，老数据不破坏。
- 表单新增可展开的截止日 / 提醒时间控件；列表项新增截止日徽标（今天 / 明天 / 逾期 N 天 / N 天后），逾期红色、当日/明天强调色。
- 通过 `tauri-plugin-notification 2.3.3` 到点推送系统通知；首次推送静默请求权限，被拒绝写 `warn` 诊断日志、不抛错、不阻塞使用。
- 应用启动时 `rescheduleAll` 重调度所有未完成提醒；toggle / remove 自动取消提醒。
- 当前调度基于前端 `setTimeout`（应用运行期有效）；应用关闭期间的提醒待后续叠加 Rust `NotificationBuilder::schedule` 后台调度。

关键文件：

- [contracts/lib.rs](./crates/contracts/src/lib.rs)（Todo 新字段）
- [Todo.ts](./src/bindings/models/Todo.ts)（同步 TS 绑定）
- [Cargo.toml](./src-tauri/Cargo.toml)（`tauri-plugin-notification = "2"`）
- [default.json](./src-tauri/capabilities/default.json)（`notification:default`）
- [lib.rs](./src-tauri/src/lib.rs)（plugin 初始化）
- [dueDate.ts](./src/lib/dueDate.ts) / [notifications.ts](./src/lib/notifications.ts)（前端平台层）
- [todos.ts](./src/stores/todos.ts)（调度 / 取消 / rescheduleAll）
- [App.vue](./src/App.vue)（表单控件 + 徽标 UI）
- [任务记录](./.agents/docs/tasks/PLATFORM-FOUNDATION-003/task.json)

验证已通过：

- `cargo check`
- `cargo test -p todo-contracts`
- `pnpm run check`
- `pnpm run build`

尚未标记完成：完整 Tauri 桌面运行时测试受本机动态链接环境阻断，待复验提醒到点通知、权限请求与降级、徽标文案。QA 已记录在 [qa-report.md](./.agents/docs/tasks/PLATFORM-FOUNDATION-003/qa-report.md)。

第四里程碑已实现：全局快捷键与快速新增。

- 桌面端通过 `tauri-plugin-global-shortcut 2.3.2` 运行时注册 `CommandOrControl+Shift+A`；按下后主界面可见（含从托盘唤醒）、焦点落到待办输入框。
- 启动时注册、退出时注销，不遗留 OS 级快捷键占用；注册失败写 `warn` 诊断日志、不阻塞启动。
- 快速新增复用主界面 draft 输入框，不引入独立窗口；与"关闭隐藏到托盘"架构一致。
- 浏览器 / 移动端整段安全跳过；UI 显示当前快捷键提示。

关键文件：

- [Cargo.toml](./src-tauri/Cargo.toml)（`tauri-plugin-global-shortcut = "2"`）
- [default.json](./src-tauri/capabilities/default.json)（`global-shortcut:default`）
- [lib.rs](./src-tauri/src/lib.rs)（`Builder::default().build()` 初始化）
- [shortcuts.ts](./src/lib/shortcuts.ts)（`registerShortcuts` / `unregisterShortcuts` 平台层）
- [App.vue](./src/App.vue)（启动注册 / 退出注销 / 聚焦输入框 / UI 提示）
- [任务记录](./.agents/docs/tasks/PLATFORM-FOUNDATION-004/task.json)

验证已通过：

- `cargo check`
- `pnpm run check`
- `pnpm run build`

尚未标记完成：完整 Tauri 桌面运行时测试受本机动态链接环境阻断，待复验快捷键唤醒、从托盘唤起、焦点落地。QA 已记录在 [qa-report.md](./.agents/docs/tasks/PLATFORM-FOUNDATION-004/qa-report.md)。
