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

## Kubernetes 示例清单（仓库内）

仓库提供一个最小示例清单（仅用于参考，生产请迁移到你的 GitOps 仓库并接入 Secret 管理）：

- `deploy/kubernetes/dmsx-api.yaml`：Service + Deployment（含 `/health`/`/ready` 探针；示例已包含生产化加码常用 env，且通过 `Secret` 引用 `DATABASE_URL`、LiveKit key/secret）。
- `deploy/kubernetes/dmsx-api-ingress.yaml`：Ingress 示例（TLS/HSTS/安全头/匿名探针边界）。
- `deploy/kubernetes/dmsx-api-secrets.example.yaml`：Secret 模板（示例值需要替换；生产建议由 External Secrets/Vault/KMS 生成）。
- `deploy/kubernetes/dmsx-api-networkpolicy.yaml`：NetworkPolicy 示例（限制入站，仅允许 Ingress Controller 与 Prometheus 抓取；需按集群命名空间/标签调整）。
- `deploy/kubernetes/namespace-dmsx.yaml`：Namespace（`dmsx`）示例（含推荐 labels）。

约定（示例默认）：

- 业务命名空间：`dmsx`
- Pod 选择器标签：`app.kubernetes.io/name: <service>`
- `app.kubernetes.io/part-of: dmsx` 用于跨组件聚合检索

最小落地步骤（示例；生产请迁移到你的 GitOps 仓库）：

```bash
# 1) 创建命名空间
kubectl apply -f deploy/kubernetes/namespace-dmsx.yaml

# 2) 创建 Secret（示例文件需要替换内容；生产建议用 External Secrets）
kubectl -n dmsx apply -f deploy/kubernetes/dmsx-api-secrets.example.yaml

# 3) 部署 API（Service + Deployment）
kubectl apply -f deploy/kubernetes/dmsx-api.yaml

# 4) （可选）限制入站：只允许 Ingress Controller / Prometheus
kubectl apply -f deploy/kubernetes/dmsx-api-networkpolicy.yaml

# 5) （可选）Ingress（把 host / tls secretName 替换为你的域名与证书 Secret）
kubectl apply -f deploy/kubernetes/dmsx-api-ingress.yaml
```

## 内测网络边界（推荐默认）

面向「先内测」阶段，建议默认 **不对公网暴露控制面**，只在团队内网 / VPN 可达范围验证。

推荐落地方式（Kubernetes）：

- **对外暴露最小化**：只 apply `deploy/kubernetes/dmsx-api.yaml`（Service 默认为 **ClusterIP**），不要创建 `LoadBalancer` Service。
- **是否需要 Ingress**：
  - 不需要外部访问时：**不要 apply** `deploy/kubernetes/dmsx-api-ingress.yaml`。
  - 需要在内网演示时：使用 **内网 IngressClass**（或 internal LB）并对来源做白名单限制；公网域名不暴露。
- **入站收敛**：建议 apply `deploy/kubernetes/dmsx-api-networkpolicy.yaml`，将 `dmsx-api:8080` 入站限制为 Ingress Controller 与采集器（如 Prometheus）来源（按集群 namespace/label 调整）。

推荐落地方式（本机/裸机）：

- **只绑定回环**：`DMSX_API_BIND=127.0.0.1:8080`（避免误暴露到公网网卡）。
- **通过 VPN 访问**：若需要远程访问，优先通过 VPN/跳板机端口转发，不直接把 `0.0.0.0:8080` 挂公网。

## Ingress / TLS（生产建议）

`dmsx-api` 应用进程默认明文 HTTP 监听（见 `DMSX_API_BIND`），生产环境建议由 **Ingress** 终止 TLS，并在集群边界做最小防护。

建议要点：

- **TLS 终止**：Ingress 配置证书（如 cert-manager）；外部仅开放 HTTPS。
- **HSTS**：仅在确认全站 HTTPS 后开启（避免误伤 HTTP 客户端）。
- **匿名探针边界**：仅允许匿名访问 `GET /health`、`GET /ready`；其余路径必须 Bearer JWT。
- **指标端点**：`GET /metrics` 默认匿名返回 Prometheus 文本格式；建议仅在集群内开放（Ingress / NetworkPolicy 限制），避免公网暴露内部指标。
- **安全响应头**（可选）：`X-Content-Type-Options: nosniff`、`Referrer-Policy: no-referrer` 等。

### /metrics 仅集群内访问（推荐）

建议二选一（或同时）：

- **NetworkPolicy**：只允许 Prometheus（或你的采集器）命名空间/Pod 访问 `dmsx-api:8080`。
- **双 Ingress**：公网域名不暴露 `/metrics`；另建一个仅内网可访问的 Ingress/域名给 Prometheus 抓取。

示例：NetworkPolicy（假设 `dmsx-api` 在命名空间 `dmsx`，Prometheus 在命名空间 `monitoring` 且有标签 `app=prometheus`）：

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: dmsx-api-ingress
  namespace: dmsx
spec:
  podSelector:
    matchLabels:
      app: dmsx-api
  policyTypes: ["Ingress"]
  ingress:
    # 允许来自 Ingress Controller 的流量（按你集群实际标签调整）
    - from:
        - namespaceSelector:
            matchLabels:
              kubernetes.io/metadata.name: ingress-nginx
      ports:
        - protocol: TCP
          port: 8080
    # 允许 Prometheus 抓取 /metrics（按你集群实际标签调整）
    - from:
        - namespaceSelector:
            matchLabels:
              kubernetes.io/metadata.name: monitoring
          podSelector:
            matchLabels:
              app: prometheus
      ports:
        - protocol: TCP
          port: 8080
```

示例（以 NGINX Ingress 为例，关键点可迁移到其他控制器）：

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: dmsx-api
  annotations:
    nginx.ingress.kubernetes.io/force-ssl-redirect: "true"
    nginx.ingress.kubernetes.io/hsts: "true"
    nginx.ingress.kubernetes.io/hsts-max-age: "31536000"
    nginx.ingress.kubernetes.io/hsts-include-subdomains: "true"
    nginx.ingress.kubernetes.io/configuration-snippet: |
      add_header X-Content-Type-Options "nosniff" always;
      add_header Referrer-Policy "no-referrer" always;
      if ($request_uri !~ "^/(health|ready)$") {
        if ($http_authorization = "") { return 401; }
      }
spec:
  tls:
    - hosts: ["api.example.com"]
      secretName: dmsx-api-tls
  rules:
    - host: api.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: dmsx-api
                port:
                  number: 8080
```

## 环境变量（dmsx-api）

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `DATABASE_URL` | `postgres://dmsx:dmsx@127.0.0.1:5432/dmsx` | Postgres 连接字符串 |
| `DMSX_API_BIND` | `0.0.0.0:8080` | HTTP 监听地址 |
| `LIVEKIT_URL` | `ws://127.0.0.1:7880` | LiveKit Server WebSocket 地址 |
| `LIVEKIT_API_KEY` | `dmsx-api-key` | LiveKit API Key（与 livekit.yaml 一致） |
| `LIVEKIT_API_SECRET` | `dmsx-api-secret-that-is-at-least-32-chars` | LiveKit API Secret |
| `DMSX_API_ENV` | `dev` | `dev`/`staging`/`prod`；**非 dev** 时会启用启动期安全闸门（见下方认证相关变量） |
| `DMSX_API_AUTH_MODE` | `disabled` | `jwt` 时校验 `Authorization: Bearer`；JWT 声明（**`tenant_id`**、**`allowed_tenant_ids`**、**`tenant_roles`**、**`roles`**）语义见 [`API.md`](API.md) |
| `DMSX_API_ALLOW_INSECURE_AUTH` | `false` | 仅在 `DMSX_API_ENV!=dev` 时生效：允许（不推荐）使用 `DMSX_API_AUTH_MODE=disabled` 启动；用于过渡内网环境 |
| `DMSX_API_JWT_SECRET` | 开发回退常量 | `jwt` 模式下 HS256 密钥；**生产必须显式配置** |
| `DMSX_API_JWT_ISSUER` / `DMSX_API_JWT_AUDIENCE` | （可选） | 与签发方一致的 `iss` / `aud` 校验 |
| `DMSX_API_OIDC_DISCOVERY_URL` / `DMSX_API_JWKS_URL` | （可选） | OIDC discovery 或直连 JWKS；详见 `crates/dmsx-api` 认证模块与 `docs/CHECKLIST.md` |
| `DMSX_API_REQUEST_BODY_LIMIT_BYTES` | `1048576` | 请求体大小上限（字节）；超限返回 **413**（ProblemDetails） |
| `DMSX_API_RATE_LIMIT_ENABLED` | `false` | 是否启用 per-tenant 速率限制 |
| `DMSX_API_RATE_LIMIT_PER_SECOND` | `50` | 每租户每秒允许请求数（下限 1） |
| `DMSX_API_RATE_LIMIT_BURST` | `100` | 每租户突发容量（下限 1） |
| `DMSX_API_REQUEST_TIMEOUT_SECONDS` | `30` | 全局请求超时（秒）；用于防止下游卡死导致资源耗尽 |
| `DMSX_API_CONCURRENCY_LIMIT_ENABLED` | `false` | 是否启用全局并发上限（建议在公网/大流量入口开启） |
| `DMSX_API_CONCURRENCY_LIMIT` | `1024` | 全局并发上限（启用时生效；下限 1） |
| `DMSX_API_METRICS_ENABLED` | `true` | 是否启用 `GET /metrics`（关闭时返回 **404**） |
| `DMSX_API_METRICS_BEARER` | （可选） | 若设置，则访问 `GET /metrics` 必须携带完全匹配的 `Authorization: Bearer ...`（不匹配返回 **401**）；建议配合 Ingress/NetworkPolicy 仅集群内暴露 |

当启用了 OIDC/JWKS（设置了 `DMSX_API_OIDC_DISCOVERY_URL` 或 `DMSX_API_JWKS_URL`）时，`dmsx-api` 会要求同时配置 **`DMSX_API_JWT_ISSUER` 与 `DMSX_API_JWT_AUDIENCE`**，避免接受来自非预期签发方/受众的令牌。

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

### API 侧最小配置（推荐）

`dmsx-api` 使用 `tracing` 输出结构化日志；建议在 K8s 里通过环境变量控制过滤级别：

- `RUST_LOG`：示例 `dmsx_api=info,tower_http=info,sqlx=warn`

OTLP 导出使用 OpenTelemetry Rust 标准环境变量（若未来引入 SDK，建议沿用这一套；Collector 端见 `deploy/otel-collector-config.yaml`）：

- `OTEL_SERVICE_NAME=dmsx-api`
- `OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-collector:4317`（gRPC）
- `OTEL_EXPORTER_OTLP_PROTOCOL=grpc`
- `OTEL_RESOURCE_ATTRIBUTES=deployment.environment=prod,service.version=0.1.0`

本地验证（使用 compose 自带 `otel-collector` 的 debug exporter 打印到 stdout）：

```bash
OTEL_SERVICE_NAME=dmsx-api \
OTEL_EXPORTER_OTLP_ENDPOINT="http://127.0.0.1:4317" \
OTEL_EXPORTER_OTLP_PROTOCOL=grpc \
RUST_LOG="dmsx_api=info,tower_http=info,sqlx=warn" \
cargo run -p dmsx-api
```

随后观察 `otel-collector` 日志，应能看到 traces/metrics/logs 的 debug 输出。

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
