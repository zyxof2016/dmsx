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
- `deploy/kubernetes/dmsx-rbac.yaml`：Namespace 内最小 RBAC（ServiceAccount + 只读 ConfigMap 的 Role/RoleBinding 模板；默认禁用 SA token 自动挂载）。
- `deploy/kubernetes/dmsx-configmap.example.yaml`：ConfigMap 示例（非敏感配置；敏感值仍用 Secret）。
- `deploy/kubernetes/dmsx-hpa.yaml`：HPA 示例（基于 CPU utilization；依赖 Metrics Server）。
- `deploy/kubernetes/dmsx-pdb.yaml`：PDB 示例（滚动升级/节点维护时保持最小可用副本）。
- `deploy/kubernetes/monitoring/`：可观测性配套模板（Prometheus Operator / Grafana 场景）：
  - `dmsx-api-servicemonitor.yaml`：抓取 `dmsx-api` 的 `/metrics`
  - `dmsx-api-prometheusrule.yaml`：告警规则示例
  - `dmsx-api-grafana-dashboard.yaml`：Grafana Dashboard（ConfigMap）示例

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

# 3.1) （推荐）RBAC（最小）+ 禁用 SA token 自动挂载
kubectl apply -f deploy/kubernetes/dmsx-rbac.yaml

# 3.2) （可选）ConfigMap 示例（非敏感配置）
kubectl apply -f deploy/kubernetes/dmsx-configmap.example.yaml

# 4) （可选）限制入站：只允许 Ingress Controller / Prometheus
kubectl apply -f deploy/kubernetes/dmsx-api-networkpolicy.yaml

# 5) （可选）Ingress（把 host / tls secretName 替换为你的域名与证书 Secret）
kubectl apply -f deploy/kubernetes/dmsx-api-ingress.yaml

# 6) （可选）HPA/PDB（依赖 Metrics Server；按你环境调整副本与阈值）
kubectl apply -f deploy/kubernetes/dmsx-hpa.yaml
kubectl apply -f deploy/kubernetes/dmsx-pdb.yaml
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
| `DMSX_API_PLATFORM_TENANT_LIMIT` | `1000` | `/v1/config/quotas` 中平台租户数配额上限 |
| `DMSX_API_PLATFORM_DEVICE_LIMIT` | `10000` | `/v1/config/quotas` 中平台设备数配额上限 |
| `DMSX_API_PLATFORM_COMMAND_LIMIT` | `100000` | `/v1/config/quotas` 中平台命令数配额上限 |
| `DMSX_API_PLATFORM_ARTIFACT_LIMIT` | `10000` | `/v1/config/quotas` 中平台制品数配额上限 |
| `DMSX_API_CORS_ALLOWED_ORIGINS` | （未设置） | 允许的 CORS 来源（逗号分隔，完整 scheme+host+port，如 `https://admin.example.com,http://localhost:3000`）。未设置且非 `dev` 环境将拒绝所有跨域请求（浏览器侧阻断）。 |
| `DMSX_API_CORS_ALLOW_ALL` | `false` | 是否放开所有 CORS 来源（dev-like）；设置为 `1/true/yes` 时 `DMSX_API_CORS_ALLOWED_ORIGINS` 将被忽略。 |
| `DMSX_API_UPLOAD_TOKEN_HMAC_SECRET` | （未设置） | 控制面签发 `UploadEvidence` token 的 HMAC secret；应与网关 `DMSX_GW_UPLOAD_TOKEN_HMAC_SECRET` 保持一致。若未设置，`POST /v1/tenants/{tid}/commands/{cid}/evidence-upload-token` 返回 **500** |
| `DMSX_CLICKHOUSE_HTTP_URL` | （未设置） | ClickHouse HTTP 接口地址（未设置则不写入 CH）。示例：`http://127.0.0.1:8123` |
| `DMSX_CLICKHOUSE_HTTP_USER` / `DMSX_CLICKHOUSE_HTTP_PASSWORD` | （可选） | ClickHouse HTTP Basic Auth 用户名/密码；仅当上述 URL 配置了且两项都存在时才启用 |
| `DMSX_REDIS_URL` | （未设置） | Redis URL（未设置则不使用缓存/持久化）。当前用于桌面会话映射持久化：`session_id → {tenant_id, device_id}` 与 `device_id → session_id` |
| `DMSX_NATS_URL` | （未设置） | NATS 服务端地址（未设置则不发布命令到 JetStream）。示例：`nats://127.0.0.1:4222` |
| `DMSX_NATS_JETSTREAM_ENABLED` | `true` | 是否启用 JetStream 命令发布；`0`/`false`/`no`/`off` 为关闭 |
| `DMSX_NATS_COMMAND_STREAM` | `DMSX_COMMANDS` | JetStream stream 名称；启动时 `get_or_create`，subjects 固定为 `dmsx.command.>` |
| `DMSX_NATS_RESULT_CONSUMER` | `dmsx-api-result-ingest` | 命令回执 JetStream **durable pull** consumer 名称（`filter_subject=dmsx.command.result.>`）；多副本 `dmsx-api` 共享同一 consumer 时分摊消费 |
| `DMSX_API_METRICS_ENABLED` | `true` | 是否启用 `GET /metrics`（关闭时返回 **404**） |
| `DMSX_API_METRICS_BEARER` | （可选） | 若设置，则访问 `GET /metrics` 必须携带完全匹配的 `Authorization: Bearer ...`（不匹配返回 **401**）；建议配合 Ingress/NetworkPolicy 仅集群内暴露 |

当启用了 OIDC/JWKS（设置了 `DMSX_API_OIDC_DISCOVERY_URL` 或 `DMSX_API_JWKS_URL`）时，`dmsx-api` 会要求同时配置 **`DMSX_API_JWT_ISSUER` 与 `DMSX_API_JWT_AUDIENCE`**，避免接受来自非预期签发方/受众的令牌。

启用 `jwt` 时，管理台或 BFF 签发的访问令牌须与 **OpenAPI `bearerAuth`** 及 **[`API.md`](API.md)** 中的多租户 / 按租户 RBAC 约定一致，否则路径租户或写操作将返回 **403**。

## 环境变量（dmsx-device-gw）

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `DMSX_GW_BIND` | `0.0.0.0:50051` | gRPC 监听地址 |
| `DMSX_GW_METRICS_ENABLED` | `false` | 是否启用网关 Prometheus 指标（HTTP `/metrics`） |
| `DMSX_GW_METRICS_BIND` | `0.0.0.0:9090` | 指标 HTTP 监听地址（启用时生效） |
| `DMSX_GW_CONCURRENCY_PER_CONNECTION` | `64` | 每个 gRPC 连接的并发请求上限（tonic `concurrency_limit_per_connection`） |
| `DMSX_GW_TLS_CERT` / `DMSX_GW_TLS_KEY` | （未设置） | 服务端证书与私钥 PEM 路径；**均设置**时启用 gRPC **TLS**（HTTP/2 over TLS） |
| `DMSX_GW_TLS_CLIENT_CA` | （未设置） | 校验客户端证书的 CA PEM 路径；设置后默认 **要求** 客户端证书（mTLS）；与 `DMSX_GW_TLS_CLIENT_AUTH_OPTIONAL` 联用。若同时配置 Enroll 所需 HMAC/CA，网关会自动允许**无证书客户端仅用于 `Enroll`** |
| `DMSX_GW_TLS_CLIENT_AUTH_OPTIONAL` | `false` | `1`/`true`/`yes`/`on` 时：在已配置 `DMSX_GW_TLS_CLIENT_CA` 的前提下仍允许匿名客户端（**不推荐生产**） |
| `DMSX_GW_RATE_LIMIT_ENABLED` | `false` | 是否启用**按租户**的 gRPC 速率限制（内存内实现；多副本按 Pod 分摊） |
| `DMSX_GW_RATE_LIMIT_PER_SECOND` | `100` | 每租户每秒允许的请求起始配额（下限 1） |
| `DMSX_GW_RATE_LIMIT_BURST` | `200` | 每租户突发容量（下限 1） |
| `DMSX_NATS_URL` | （未设置） | 与 `dmsx-api` 相同的 NATS 地址；**未设置**时 `StreamCommands` 仍返回空流；`ReportResult` 返回 **`accepted=false`**（不落库） |
| `DMSX_NATS_JETSTREAM_ENABLED` | `true` | 与 API 一致；关闭时不从 JetStream 拉命令、不发布回执 |
| `DMSX_NATS_COMMAND_STREAM` | `DMSX_COMMANDS` | 与 API 相同的 stream 名；网关会 `get_or_create_stream`（与 API 幂等） |
| `DMSX_GW_ENROLL_HMAC_SECRET` | （未设置） | **内测 Enroll**：HMAC secret（启用 enrollment token 验证）；未设置则 `Enroll` 返回失败（failed_precondition） |
| `DMSX_GW_ENROLL_CA_CERT` / `DMSX_GW_ENROLL_CA_KEY` | （未设置） | **内测 Enroll**：CA 证书/私钥 PEM 路径；用于签发设备客户端证书（`EnrollRequest.public_key_pem` 需为 **CSR PEM**） |
| `DMSX_GW_ENROLL_CERT_TTL_DAYS` | `30` | **内测 Enroll**：签发证书有效期（天，1–3650） |
| `DMSX_GW_UPLOAD_TOKEN_HMAC_SECRET` | （未设置） | `UploadEvidence` 的 `upload_token` 验签 secret；token 采用 `v1.<payload_b64url>.<sig_b64url>`（payload 至少含 `tenant_id` / `device_id` / `exp`，可选 `content_type`） |
| `DMSX_GW_EVIDENCE_S3_BUCKET` | （未设置） | 启用 `UploadEvidence` 真实持久化；未设置时 `UploadEvidence` 返回 failed_precondition |
| `DMSX_GW_EVIDENCE_S3_REGION` | `us-east-1` | 证据对象存储 region |
| `DMSX_GW_EVIDENCE_S3_ENDPOINT` | （未设置） | S3 兼容 endpoint（例如本地 MinIO 可填 `http://127.0.0.1:9100`） |
| `DMSX_GW_EVIDENCE_S3_ACCESS_KEY` / `DMSX_GW_EVIDENCE_S3_SECRET_KEY` | （未设置） | 显式对象存储凭据；未设置时走默认 AWS credential chain |
| `DMSX_GW_EVIDENCE_S3_PREFIX` | `evidence` | 证据对象 key 前缀 |
| `DMSX_GW_EVIDENCE_S3_FORCE_PATH_STYLE` | `true`（当设置 endpoint 时） | 是否使用 path-style 访问；MinIO 通常需要开启 |

`StreamCommands`：当前实现使用按租户/设备稳定命名的 **durable pull consumer**（名称前缀可通过 `DMSX_GW_COMMAND_CONSUMER_PREFIX` 调整），在解析出 **`tenant_id` + `device_id`** 后使用 **`filter_subject=dmsx.command.{tenant_id}.{device_id}`**（租户在 **mTLS 严格模式**下可由证书 SAN 推出，否则须在 `StreamCommandsRequest.tenant_id` 中显式携带 UUID）。默认 `ack_wait=30s`、`max_deliver=5`、`max_ack_pending=1`，并使用单条 pull（batch 1）保证**单设备单条在途命令**；若 `cursor` 提供 **JetStream stream sequence**，首次创建 consumer 时将从该序号恢复。消息体为 `dmsx-api` 发布的 **`Command` JSON**；当前语义不是“送入 gRPC 缓冲即 ACK”，而是**发出一条命令后等待同设备对应 `ReportResult(command_id)` 发布成功，再 ACK JetStream 并继续下一条**。等待期间网关会发送 progress ack 续租；客户端断开或流被替换时会 **NAK** 让该条命令尽快重投。同一租户/设备仅允许一个活跃 `StreamCommands` 连接，第二个连接会被拒绝。

`ReportResult`：在 NATS/JetStream 可用时将 JSON 发布到 **`dmsx.command.result.{tenant_id}.{device_id}`**（与 `dmsx-api` 后台 ingest 约定一致）。控制面入库时以消息中的 **`status`** 为准更新 `commands.status`，`exit_code/stdout/stderr/evidence_key` 写入 `command_results`。**mTLS 严格模式**（已配置 `DMSX_GW_TLS_CLIENT_CA` 且未开启 `DMSX_GW_TLS_CLIENT_AUTH_OPTIONAL`）下，客户端证书 SAN 须含 URI **`urn:dmsx:tenant:{uuid}:device:{uuid}`**，且与 RPC 中的 `tenant_id` / `device_id` 一致。

`Enroll`（内测实现）：验证 enrollment token（`v1.<payload_b64url>.<sig_b64url>`，HMAC-SHA256 over `payload_b64url`），并使用 CA 签发设备客户端证书；证书 SAN 写入 `urn:dmsx:tenant:{tenant_id}:device:{device_id}`。当前 enrollment token **必须显式绑定 `device_id`**，避免同一 token 重放生成多个设备身份；`EnrollRequest.public_key_pem` 在内测实现中要求为 **PKCS#10 CSR PEM**（历史字段名保留，后续可协议升级为 `csr_pem`）。若同时配置了 `DMSX_GW_TLS_CLIENT_CA` 与 Enroll 所需 HMAC/CA，握手层会允许匿名新设备连入并调用 `Enroll`，但其他 RPC 仍要求设备证书。

`UploadEvidence`：当前实现会将收到的 chunk 聚合后写入 **S3 / MinIO 兼容对象存储**，对象 key 形如 `evidence/{tenant_id}/{device_id}/{uuid}`。租户与设备身份来自**设备证书**或 **`upload_token`**；若两者同时提供，则必须一致。若既没有 mTLS 身份、也没有 `upload_token`，网关会拒绝匿名落盘。控制面现已提供 **`POST /v1/tenants/{tid}/commands/{cid}/evidence-upload-token`** 用于按命令目标设备签发短期 token（可选绑定 `content_type`）；生产内测建议仍优先使用设备证书身份直传，并将 token 作为过渡或跨进程上传场景兜底。

数据面最小闭环联调可使用 `scripts/internal-beta-data-plane-e2e.sh`：脚本会串起**创建设备 -> Enroll -> FetchDesiredState -> StreamCommands -> ReportResult -> 控制面查结果**，默认按 `GW_GRPC_MODE=tls` 运行；若为自签名服务端证书，可配 `GW_TLS_CA_CERT` 或直接 `GW_TLS_INSECURE=1`。设置 **`DMSX_E2E_WITH_EVIDENCE=1`** 时，会额外跑 **evidence-upload-token → `UploadEvidence` → `ReportResult.evidence_object_key` → GET 结果校验 `evidence_key`**（需 GW 已配置 `DMSX_GW_EVIDENCE_S3_*` 桶可写，且 API/GW 的 upload token HMAC 密钥一致）。

## 环境变量（dmsx-agent）

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `DMSX_API_URL` | `http://127.0.0.1:8080` | 控制面 API 地址 |
| `DMSX_TENANT_ID` | `00000000-0000-0000-0000-000000000001` | 租户 ID |
| `DMSX_DEVICE_REGISTRATION_CODE` | （未设置） | Agent 首次绑定时使用的人可见设备注册码；设置后会优先按该码复用已预注册设备 |
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
OTEL_EXPORTER_OTLP_ENDPOINT="http://127.0.0.1:4317" \
OTEL_SERVICE_NAME=dmsx-api \
OTEL_EXPORTER_OTLP_PROTOCOL=grpc \
RUST_LOG="dmsx_api=info,tower_http=info,sqlx=warn" \
cargo run -p dmsx-api
```

随后观察 `otel-collector` 日志，应能看到 traces/metrics/logs 的 debug 输出。

`dmsx-device-gw` 同样支持：设置 `OTEL_EXPORTER_OTLP_ENDPOINT` 后会导出 traces；K8s 示例见 `deploy/kubernetes/dmsx-device-gw.yaml` 的可选 env 段落。

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
