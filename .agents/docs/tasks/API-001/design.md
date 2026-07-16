# 设计：服务基线

本任务无可见界面变化。服务 API 只暴露 `GET /health` 与 `POST /v1/sync`；同步接口采用 JSON、最多 100 个操作的批次和 cursor 增量返回。错误由 Elysia 的 schema 校验返回，客户端暂不接入网络调用。
