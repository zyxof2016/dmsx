# 部署与可观测性

## Kubernetes 拓扑（建议）

```mermaid
flowchart TB
  subgraph ingress [Ingress]
    IG[IngressController_TLS]
  end
  subgraph apps [Namespace dmsx]
    API[dmsx_api Deployment]
    GW[dmsx_device_gw Deployment]
    LK[LiveKit Server]
    NATS[NATS JetStream StatefulSet]
    REDIS[Redis]
  end
  subgraph data [Managed or in-cluster]
    PG[(PostgreSQL)]
    CH[(ClickHouse)]
    S3[(ObjectStorage)]
  end
  IG --> API
  IG --> GW
  IG -->|WebRTC ICE / TURN| LK
  API --> PG
  API --> NATS
  API --> REDIS
  GW --> NATS
  GW --> PG
  API --> S3
  GW --> S3
  API -.->|"async audit/analytics"| CH
```

远程桌面：**浏览器与 Agent 直连 LiveKit**（Ingress 仅暴露信令/ICE）；`dmsx-api` 负责签发 JWT、维护 `session_id` 生命周期，并通过命令触发 Agent 入房，**不经由 API ↔ LiveKit 的桌面视频 WebSocket**。

- **HPA**：`dmsx-api`、`dmsx-device-gw` 按 CPU / gRPC 并发指标扩展。
- **PDB**：保证滚动升级时最少可用副本。
- **Pod 反亲和**：网关跨节点打散。

## GitOps

- 清单仓库 + **Argo CD** 或 **Flux**；环境分 `dev` / `staging` / `prod`（overlay）。
- 镜像：**semver + digest** 固定；禁止 `:latest` 入生产。

## 配置与密钥

- 非机密：`ConfigMap`（或 Helm values）。
- 机密：**External Secrets** → Vault / 云 KMS；定期轮换 DB 与签名密钥。
- 设备 CA：独立 **offline root** + **online intermediate**（网关只信中间层）。

## 环境变量（dmsx-api）

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `DATABASE_URL` | `postgres://dmsx:dmsx@127.0.0.1:5432/dmsx` | Postgres 连接字符串 |
| `DMSX_API_BIND` | `0.0.0.0:8080` | HTTP 监听地址 |
| `LIVEKIT_URL` | `ws://127.0.0.1:7880` | LiveKit Server WebSocket 地址 |
| `LIVEKIT_API_KEY` | `dmsx-api-key` | LiveKit API Key（与 livekit.yaml 一致） |
| `LIVEKIT_API_SECRET` | `dmsx-api-secret-that-is-at-least-32-chars` | LiveKit API Secret |
| `DMSX_API_AUTH_MODE` | `disabled` | `jwt` 时校验 `Authorization: Bearer`；JWT 声明（**`tenant_id`**、**`allowed_tenant_ids`**、**`tenant_roles`**、**`roles`**）语义见 [`API.md`](API.md) |
| `DMSX_API_JWT_SECRET` | 开发回退常量 | `jwt` 模式下 HS256 密钥；**生产必须显式配置** |
| `DMSX_API_JWT_ISSUER` / `DMSX_API_JWT_AUDIENCE` | （可选） | 与签发方一致的 `iss` / `aud` 校验 |
| `DMSX_API_OIDC_DISCOVERY_URL` / `DMSX_API_JWKS_URL` | （可选） | OIDC discovery 或直连 JWKS；详见 `crates/dmsx-api` 认证模块与 `docs/CHECKLIST.md` |

启用 `jwt` 时，管理台或 BFF 签发的访问令牌须与 **OpenAPI `bearerAuth`** 及 **[`API.md`](API.md)** 中的多租户 / 按租户 RBAC 约定一致，否则路径租户或写操作将返回 **403**。

## 环境变量（dmsx-agent）

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `DMSX_API_URL` | `http://127.0.0.1:8080` | 控制面 API 地址 |
| `DMSX_TENANT_ID` | `00000000-0000-0000-0000-000000000001` | 租户 ID |
| `DMSX_HEARTBEAT_SECS` | `30` | 心跳间隔（秒） |
| `DMSX_POLL_SECS` | `10` | 命令轮询间隔（秒） |
| `DMSX_RUSTDESK_RELAY` | （可选）| RustDesk 自建中继服务器地址 |

## 可观测性（OpenTelemetry）

- 应用：**OTLP gRPC** 导出 → OpenTelemetry Collector（`deploy/otel-collector-config.yaml` 示例）。
- 后端组合（任选托管或自管）：
  - 指标：**Prometheus** + Grafana
  - 日志：**Loki** 或云日志
  - 追踪：**Tempo** / Jaeger
- **SLO 示例**：设备网关可用性 99.95%；命令 `queued → succeeded` P95 延迟；心跳丢失率。

## 本地与 CI

本地开发：**Docker Compose**（`deploy/docker-compose.yml`）拉起全套基础设施：

```bash
cd deploy
docker compose up -d
```

包含服务：
| 服务 | 端口 | 说明 |
|------|------|------|
| postgres | 5432 | 主数据库（自动执行 migrations） |
| redis | 6379 | 缓存 / 分布式锁 |
| nats | 4222, 8222 | 消息总线（JetStream 已启用） |
| clickhouse | 8123, 9000 | 分析数据库 |
| minio | 9100, 9001 | 对象存储（制品 / 证据） |
| rustdesk-hbbs | 21115-21118 | RustDesk 信令服务器 |
| rustdesk-hbbr | 21117, 21119 | RustDesk 中继服务器 |
| livekit | 7880, 7881, 7882 | LiveKit WebRTC 服务器 |
| otel-collector | 4317 | OpenTelemetry 收集器 |

## 构建依赖

### API / gRPC 网关

```bash
sudo apt update && sudo apt install -y build-essential pkg-config libssl-dev protobuf-compiler
```

### Agent（含远程桌面屏幕采集和键鼠注入）

```bash
# X11 屏幕采集（scrap）和键鼠注入（enigo）依赖
sudo apt install -y libxcb1-dev libxcb-shm0-dev libxcb-randr0-dev libxdo-dev
```

Windows / macOS 下无需额外系统库（scrap 使用 DXGI / CGDisplay）。

### Agent 交叉编译（Android）

参考 [docs/ANDROID_DEPLOY.md](ANDROID_DEPLOY.md) 了解 Termux、NDK 交叉编译和原生 App 三种接入方案。

## CI

GitHub Actions（`.github/workflows/ci.yml`）：`cargo fmt`, `cargo clippy`, `cargo test`, Docker build。

CI 矩阵包括：
- Linux x86_64（主要目标）
- Windows x86_64（Agent 编译验证）
