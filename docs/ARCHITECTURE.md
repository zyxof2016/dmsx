# 架构与服务边界

## 控制面 vs 数据面

| 平面 | 职责 | 本仓库对应 |
|------|------|------------|
| **控制面** | 管理台/集成 API、租户与 RBAC、策略编排、审计查询、制品元数据、远程桌面会话 | `dmsx-api`（Axum）→ 未来可拆分为独立微服务 |
| **数据面** | 设备长连接、推送命令与策略、高吞吐回执与遥测入口 | `dmsx-device-gw`（Tonic gRPC）→ 经消息总线投递到消费者 |
| **设备代理** | 采集遥测、接收命令、执行远控操作、屏幕采集与键鼠注入 | `dmsx-agent`（Rust 跨平台，Windows / Linux / Android）|

**原则**：控制面以 **Postgres** 为权威状态；数据面以 **连接 + 消息流** 为主路径，回执与明细写入 **ClickHouse**（异步），关键状态回写 PG。

## 服务拆分（演进式单体 → 微服务）

当前脚手架为 **workspace 多 crate**，便于后续拆仓：

1. **api-gateway**（`dmsx-api`）：REST、限流、租户解析、审计注入、WebSocket 远程桌面中继。
2. **device-gateway**（`dmsx-device-gw`）：mTLS 终端、gRPC streaming、背压。
3. **device-agent**（`dmsx-agent`）：跨平台独立二进制，HTTP/WebSocket 与控制面通信。
4. **device-service** / **policy-service** / **command-service** / **app-repo-service** / **compliance-service** / **network-service**：逻辑可先共库，按 bounded context 分模块，流量上来再独立进程 + gRPC 内部调用。

## 技术栈

| 层 | 选型 | 用途 |
|----|------|------|
| 语言运行时 | Rust + Tokio | 异步 IO、内存安全 |
| 控制面 HTTP | Axum | REST + WebSocket、中间件、OpenAPI（后续 `utoipa`） |
| 数据面 RPC | Tonic + Prost | Agent 双向/流式通信 |
| 远程桌面 | WebSocket 中继 + JPEG 帧流（后续迁移到 LiveKit WebRTC） | 屏幕共享 + 键鼠控制 |
| 屏幕采集 | `scrap`（Windows DXGI / Linux X11） | Agent 侧屏幕采集 |
| 键鼠注入 | `enigo` | Agent 侧输入模拟 |
| 关系库 | PostgreSQL + sqlx | 租户、设备、策略版本、命令状态、设备影子 |
| 缓存/会话 | Redis | 在线快照、分布式锁、速率限制 |
| 消息 | NATS JetStream（推荐）或 Kafka | 命令投递、事件、重放 |
| 分析/审计明细 | ClickHouse | 心跳、回执、遥测、不可变审计副本 |
| 制品存储 | S3 兼容 + CDN | 包体、证据、日志归档 |
| 远程桌面信令 | LiveKit Server（WebRTC，已集成 Docker Compose）| Token 签发备用 |
| 第三方远程桌面 | RustDesk（自建 hbbs/hbbr，可选）| 备选方案 |
| 观测 | OpenTelemetry → OTLP | 指标/日志/追踪统一出口 |

## 数据流（含远程桌面）

```mermaid
flowchart LR
  Admin[AdminUI] --> ApiGW[dmsx_api]
  ApiGW --> PG[(Postgres)]
  ApiGW --> Bus[MessageBus]
  ApiGW <-->|WebSocket 远程桌面中继| Agent[dmsx_agent]
  Agent --> DevGW[dmsx_device_gw]
  DevGW --> Bus
  Bus --> Workers[Consumers]
  Workers --> PG
  Workers --> CH[(ClickHouse)]
  Agent --> S3[(ObjectStorage)]
  Agent -->|屏幕采集 JPEG帧| ApiGW
  Admin -->|键鼠事件| ApiGW
```

## 远程桌面架构

```
管理员浏览器
  │  POST /desktop/session → 获取 room token
  │  WebSocket /desktop/ws/viewer → 接收 JPEG 帧 / 发送键鼠 JSON
  ▼
dmsx-api（WebSocket 中继）
  │  broadcast channel：帧 TX → 多个 viewer RX
  │  broadcast channel：input TX → agent RX
  ▼
dmsx-agent（WebSocket /desktop/ws/agent）
  │  scrap::Capturer → BGRA→JPEG → binary frame
  │  收到键鼠 JSON → enigo 注入
  ▼
被管设备屏幕 / 键鼠
```

**后续迁移路径**：当前使用 WebSocket JPEG 流（低延迟，简单可靠）；LiveKit Server 已就绪，后续可改为 WebRTC 发布视频轨，降低带宽并支持 P2P 穿透。

## 与外部系统集成

- **身份**：OIDC / SAML（企业 SSO）；设备侧 **mTLS + enrollment token**。
- **EDR/SIEM**：Webhook 或 Kafka 出站；合规服务消费告警关联 `device_id`。
- **网络**：本阶段以 **策略下发 + 对接现有 ZTNA/SD-WAN API** 为主，不自研完整数据面。
- **远程桌面**：WebSocket 中继（内置）+ LiveKit WebRTC（可选，Docker Compose 就绪）+ RustDesk（备选）。
