# API 契约

## 控制面 REST（前缀 `/v1`）

基址示例：`https://api.example.com`。租户作用域由 URL 路径中的 `{tenant_id}` 决定；JWT 须证明该租户在本令牌下可访问（见下「多租户 JWT」）。

**远程桌面**：音视频与键鼠控制走 **LiveKit WebRTC**（浏览器 `livekit-client` 订阅、Agent SDK 发布）；`dmsx-api` 负责会话创建、LiveKit JWT 签发，并通过命令队列触发 Agent `start_desktop` / `stop_desktop`。**不再提供**经 `dmsx-api` 的桌面 JPEG WebSocket 中继端点。

**认证**：除 `/health`、`/ready` 外，管理接口需 `Authorization: Bearer <JWT>`（路径 `{tenant_id}` 须被该 JWT 允许；平台只读接口 `GET /v1/config/livekit`、`GET /v1/config/rbac/roles`、`GET /v1/config/settings/{key}`、`GET /v1/config/tenants`、`GET /v1/config/audit-logs`、`GET /v1/config/platform-health`、`GET /v1/config/quotas` 需 **PlatformAdmin** 或 **PlatformViewer**；平台写接口如 `PUT /v1/config/settings/{key}`、`POST /v1/tenants` 仅 **PlatformAdmin** 可写）。OpenAPI 根级 **`security: [bearerAuth]`**，并在各操作中声明 **`401` / `403`**；资源类接口另声明 **`404`**；各操作另声明 **`500`**（内部错误）。上述错误均复用 `components.responses` 与 **`ProblemDetails`**（见 `openapi/dmsx-control-plane.yaml`）。

**请求关联**：服务端会为每个请求生成或透传 `X-Request-Id`，并在响应中返回同名 header，便于排障时将客户端错误与服务端日志关联。

**横切限制（可选）**：控制面可启用 **请求体大小限制** 与 **速率限制**（per-tenant）。当启用时：

- 请求体超限返回 **413**（`Payload Too Large`，ProblemDetails）
- 触发速率限制返回 **429**（`Too Many Requests`，ProblemDetails；并附 `retry-after`/`x-ratelimit-after` 等头）

排障建议：

- **413**：优先检查 `Content-Length` 与 `DMSX_API_REQUEST_BODY_LIMIT_BYTES`；对于无 `Content-Length`（chunked）请求，建议在 Ingress/LB 层也配置 body size 限制以尽早拒绝。
- **429**：按租户维度限流（key 为路径 `{tenant_id}`；无路径租户时回退到 `AuthContext.tenant_id`，否则为 `global`）。建议在客户端实现指数退避；服务端可通过 `DMSX_API_RATE_LIMIT_PER_SECOND` / `DMSX_API_RATE_LIMIT_BURST` 调整。

**多租户 JWT（单用户多租户）**：JWT 声明中含 **`tenant_id`**（UUID，默认/主租户，无 `allowed_tenant_ids` 时即唯一允许租户）与可选数组 **`allowed_tenant_ids`**（UUID）。有效租户集合为 **`tenant_id` ∪ `allowed_tenant_ids`**。对 `/v1/tenants/{tenant_id}/...` 的请求，路径中的 `{tenant_id}` 必须属于该集合，否则 **403**。`AuthContext` 中的活动租户与路径一致，便于前端**切换租户**：更换 URL 中的 `{tenant_id}` 即可；签发方应在成员关系变化时更新 **`allowed_tenant_ids`**。

**按租户 RBAC（`tenant_roles`）**：可选对象 **`tenant_roles`**，键为租户 UUID 字符串、值为该租户下角色字符串数组（如 `TenantAdmin`、`ReadOnly`）。对某次**租户路径请求**，活动租户为路径中的 `{tenant_id}`：若 **`tenant_roles` 中存在该活动租户的键**（含空数组 `[]`），则本请求 **仅使用该键对应数组** 做 RBAC；若 **无该键**，则回退使用令牌级 **`roles`**。对**非租户路径请求**（如 `/v1/config/...`），仅使用令牌级 **`roles`**，不会套用 `tenant_roles`。空数组表示该租户下显式无角色，受保护租户路由将 **403**。

示例（节选 payload）：

```json
"tenant_id": "11111111-1111-1111-1111-111111111111",
"roles": ["TenantAdmin"],
"allowed_tenant_ids": ["22222222-2222-2222-2222-222222222222"],
"tenant_roles": {
  "22222222-2222-2222-2222-222222222222": ["ReadOnly"]
}
```

上例中访问 `/v1/tenants/11111111-.../devices` 时使用 **`TenantAdmin`**；访问 `/v1/tenants/22222222-.../devices` 时使用 **`ReadOnly`**。

机器可读 OpenAPI 见文末；**`paths` 与当前 `crates/dmsx-api` 已注册路由对齐**；未实现的 HTTP 路由不会出现在 OpenAPI 中（见下方「租户与组织结构」说明）。

### 通用端点

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/health` | 健康检查 |
| GET | `/ready` | 就绪检查（含认证 / JWKS 状态） |
| GET | `/metrics` | Prometheus 指标（可通过 `DMSX_API_METRICS_ENABLED` 关闭；建议仅集群内访问） |
| GET | `/v1/config/livekit` | LiveKit 配置查询（`{ enabled, url }`） |

### 租户与组织结构

> **实现状态**：平台只读接口（`GET /v1/config/livekit`、`GET /v1/config/rbac/roles`、`GET /v1/config/settings/{key}`、`GET /v1/config/tenants`、`GET /v1/config/audit-logs`、`GET /v1/config/platform-health`、`GET /v1/config/quotas`）支持 **`PlatformAdmin`** 或 **`PlatformViewer`**；平台写接口（`POST /v1/tenants`、`PUT /v1/config/settings/{key}`）仅 **`PlatformAdmin`** 可写（`jwt` 模式；`disabled` 不校验）。其余租户创建路径在 **`jwt` 模式**下需 **`TenantAdmin`**（或更高）且路径 `{tid}` 属于 JWT 许可租户集合；请求体字段 **`name`** 长度 1–200。站点创建要求 **`org_id`** 属于该租户；设备组创建要求 **`site_id`** 属于该租户（否则 **400**）。名称在同一父级下违反唯一约束时 **409**。

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/v1/tenants` | 创建租户（服务端生成 `id`；**PlatformAdmin**） |
| POST | `/v1/tenants/{tid}/orgs` | 创建组织 |
| POST | `/v1/tenants/{tid}/orgs/{oid}/sites` | 创建站点（校验 `org_id` 归属租户） |
| POST | `/v1/tenants/{tid}/sites/{sid}/groups` | 创建设备组（校验 `site_id` 归属租户） |
| GET | `/v1/tenants/{tid}/stats` | Dashboard 聚合统计 |

### 设备管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/v1/tenants/{tid}/devices` | 列表（支持 `search`, `platform`, `enroll_status`, `online_state` 筛选 + 分页；`search` 同时匹配 `hostname` 与 `registration_code`） |
| POST | `/v1/tenants/{tid}/devices` | 注册/预置设备（支持可选 `registration_code`；留空时后端自动生成稳定注册码） |
| POST | `/v1/tenants/{tid}/devices:batch-create` | 批量预注册设备（1-200 台；可选同时签发 enrollment token） |
| GET | `/v1/tenants/{tid}/device-enrollment-batches` | 查询设备 enrollment 批次历史 |
| GET | `/v1/tenants/{tid}/device-enrollment-batches/{batch_id}` | 查询指定设备 enrollment 批次结果 |
| GET/PATCH/DELETE | `/v1/tenants/{tid}/devices/{did}` | 查询/更新标签分组/吊销 |
| POST | `/v1/tenants/{tid}/devices/{did}/registration-code:rotate` | 重置该设备的注册码 |
| POST | `/v1/tenants/{tid}/devices/{did}/enrollment-token` | 为该设备签发短期 enrollment token |
| POST | `/v1/tenants/{tid}/devices/claim-with-enrollment-token` | Agent 首次安装时用 enrollment token 认领已预注册设备 |

补充说明：设备模型支持稳定的人可见绑定标识 `registration_code`。建议平台预注册设备时显式录入该码，后续 Agent 首次启动通过同一注册码精确复用该设备记录，而不是仅靠 `hostname` 模糊匹配。

若希望进一步避免人工录错，可直接对预注册设备签发 enrollment token。该 token 会绑定 `tenant_id + device_id + registration_code + exp`，Agent 首次启动调用 `POST /v1/tenants/{tid}/devices/claim-with-enrollment-token` 后即可精确认领该设备记录。

批量场景建议直接调用 `devices:batch-create`，并为返回结果导出 `registration_code`、`enrollment_token` 与 Agent 启动命令，便于工厂、门店或 IT 运维批量下发。

### 设备影子（Device Shadow）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/v1/tenants/{tid}/devices/{did}/shadow` | 获取设备影子（含 reported / desired / delta） |
| PATCH | `/v1/tenants/{tid}/devices/{did}/shadow/desired` | 更新期望状态（管理员下发） |
| PATCH | `/v1/tenants/{tid}/devices/{did}/shadow/reported` | 更新已报告状态（Agent 心跳上报） |

### 设备远控（Remote Control）

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/v1/tenants/{tid}/devices/{did}/actions` | 下发设备操作（reboot / shutdown / lock_screen / run_script / install_update 等） |
| GET | `/v1/tenants/{tid}/devices/{did}/commands` | 查询该设备的命令历史 |

补充说明：`install_update` 当前已支持最小可用参数集。建议在 `params` 中传入 `download_url`，并尽量同时传入 `sha256`；可选 `expected_version`、`installer_kind`（如 `sh` / `ps1` / `msi` / `exe` / `deb` / `rpm` / `pkg` / `apk`）、`install_command`（支持 `{{file_path}}`、`{{download_url}}`、`{{sha256}}` 占位符）、`interpreter` 与 `timeout`。Agent 会先下载到本地临时文件、校验 SHA256（若提供），再执行默认安装器或自定义安装命令，并在结束后清理临时文件。若传入 `expected_version`，前端可在命令结束后继续基于设备下一次心跳上报的 `agent_version` 做版本确认。

### 远程桌面（Remote Desktop）

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/v1/tenants/{tid}/devices/{did}/desktop/session` | 创建远程桌面会话（**201**）：签发管理员侧 LiveKit JWT、写入会话表、向设备投递 `start_desktop`；响应含 `room`、`token`、`livekit_url`、`session_id` |
| DELETE | `/v1/tenants/{tid}/devices/{did}/desktop/session?session_id={sid}` | 终止指定会话（**204**）：从会话映射移除、向设备投递 `stop_desktop`；`session_id` 必填。未知 `session_id` 或与会话所属租户/设备路径不一致时返回 **404** |

### 策略管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET/POST | `/v1/tenants/{tid}/policies` | 列表 / 创建策略 |
| GET/PATCH/DELETE | `/v1/tenants/{tid}/policies/{pid}` | 查询 / 更新 / 删除单条策略 |
| POST | `/v1/tenants/{tid}/policies/{pid}/revisions` | 发布新版本（body 仅 `spec` 对象会写入 revision；`rollout` 由库表默认值或后续扩展填充，当前请求体可不传） |
| POST | `/v1/tenants/{tid}/policies/editor` | 创建策略并发布新版本（由 `{scope_kind, scope_expr}` 直接生成 revision `spec`） |

### 命令管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET/POST | `/v1/tenants/{tid}/commands` | 列表 / 下发命令（**202 Accepted**；支持 `idempotency_key`） |
| GET | `/v1/tenants/{tid}/commands/{cid}` | 查询命令状态与回执摘要 |
| PATCH | `/v1/tenants/{tid}/commands/{cid}/status` | 更新命令状态（Agent 回报） |
| GET/POST | `/v1/tenants/{tid}/commands/{cid}/result` | 查询 / 提交命令执行结果（exit_code / stdout / stderr） |
| POST | `/v1/tenants/{tid}/commands/{cid}/evidence-upload-token` | 为该命令目标设备签发 `UploadEvidence` 用的短期 token（可绑定 `content_type`） |

当配置了 **`DMSX_NATS_URL`** 且未关闭 **`DMSX_NATS_JETSTREAM_ENABLED`** 时，`dmsx-api` 在命令行成功写入 Postgres 并提交事务后，会将完整 `Command` JSON **异步**发布到 NATS JetStream：subject **`dmsx.command.{tenant_id}.{target_device_id}`**，stream 默认 **`DMSX_COMMANDS`**（subjects **`dmsx.command.>`**）。发布失败不影响 HTTP 成功语义（仅记日志）；环境变量见 [`DEPLOYMENT.md`](DEPLOYMENT.md)。

同一条件下，控制面还会启动 **命令回执 JetStream ingest**：消费 subject **`dmsx.command.result.{tenant_id}.{target_device_id}`** 上的 JSON（字段与 `POST .../commands/{cid}/result` 请求体对齐：`tenant_id`、`device_id`、`command_id`、`exit_code`、`stdout`、`stderr`、`evidence_key`），在校验 `commands` 行与消息中的租户/设备一致后，调用与 HTTP 相同的入库路径。恶意/不匹配消息 **TERM** 丢弃；瞬时 DB 失败 **NAK** 重试。可选环境变量 **`DMSX_NATS_RESULT_CONSUMER`**（默认 `dmsx-api-result-ingest`）指定 durable consumer 名称。

### 制品与合规

| 方法 | 路径 | 说明 |
|------|------|------|
| GET/POST | `/v1/tenants/{tid}/artifacts` | 制品列表（分页）/ 创建元数据（**201** 返回完整 `Artifact` 行；非预签名上传 URL） |
| GET | `/v1/tenants/{tid}/compliance/findings` | 合规发现列表（分页；支持 `search` / `severity` / `status`） |

### 审计日志

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/v1/tenants/{tid}/audit-logs` | 审计日志列表（分页；支持 `action` / `resource_type` 筛选） |

### 系统设置

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/v1/config/settings/{key}` | 获取平台全局 `system_settings`（JSON） |
| PUT | `/v1/config/settings/{key}` | 更新平台全局 `system_settings`（JSON） |

### RBAC 角色清单

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/v1/config/rbac/roles` | 返回后端内置 RBAC 角色定义（`PlatformAdmin` / `PlatformViewer` 可读） |

### 平台管理接口

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/v1/config/tenants` | 平台租户目录汇总（分页；支持 `search` 搜索租户名称或 UUID，并返回跨租户设备数 / 策略数 / 命令数；仅 `PlatformAdmin`） |
| GET | `/v1/config/audit-logs` | 平台全局审计日志（分页；支持 `action` / `resource_type` 筛选；仅 `PlatformAdmin`） |
| GET | `/v1/config/platform-health` | 平台健康摘要（租户 / 设备 / 策略 / 命令 / 制品 / 审计数量，以及 LiveKit / Redis / Command Bus 开关状态；仅 `PlatformAdmin`） |
| GET | `/v1/config/quotas` | 平台配额列表（返回真实已用量；上限可由 `DMSX_API_PLATFORM_*_LIMIT` 环境变量配置；仅 `PlatformAdmin`） |

### AI 智慧管控

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/v1/tenants/{tid}/ai/anomalies` | 异常检测 |
| POST | `/v1/tenants/{tid}/ai/recommendations` | 策略推荐 |
| POST | `/v1/tenants/{tid}/ai/chat` | 智能助手（NL → 内部操作） |
| POST | `/v1/tenants/{tid}/ai/predictions` | 预测性维护 |

---

## 请求/响应示例

### 创建远程桌面会话

```http
POST /v1/tenants/{tid}/devices/{did}/desktop/session
Content-Type: application/json

{ "width": 1920, "height": 1080 }
```

请求体字段均可选；缺省时服务端使用 **1920×1080**（与 Agent 侧采集提示一致）。

响应（**201 Created**）：
```json
{
  "room": "desktop-{device_id}-1776239309",
  "token": "<LiveKit JWT>",
  "livekit_url": "ws://127.0.0.1:7880",
  "session_id": "<uuid>"
}
```

控制面只负责创建会话并投递 `start_desktop`。设备侧命令结果语义为：**只有 Agent 真正连上 LiveKit 并成功发布屏幕轨后，`start_desktop` 才应回报成功**；若 LiveKit 不可达或发布轨失败，命令结果应为失败。

当前删除语义依赖后续投递 `stop_desktop` 收敛会话：若某设备在 `POST /desktop/session` 后立刻收到删除请求，控制面会先清理会话映射并再下发 `stop_desktop`。Agent 当前会按设备命令的创建顺序执行排队命令，因此 `stop_desktop` 不会反向抢在对应 `start_desktop` 之前执行；但**仍不保证设备永远不会短暂执行到先前已下发的 `start_desktop`**。当前最小保证是最终会收敛到停止状态，而不是建立额外的“撤销未消费命令”协议。

### 终止远程桌面会话

```http
DELETE /v1/tenants/{tid}/devices/{did}/desktop/session?session_id=<sid>
```

成功时返回 **204 No Content**。`session_id` 在服务端不存在，或与该 URL 下租户/设备不匹配时返回 **404**（Problem Details）。

### 设备影子（Shadow）

```http
PATCH /v1/tenants/{tid}/devices/{did}/shadow/desired
Content-Type: application/json

{ "desired": { "screen_lock_timeout": 300, "vpn_enabled": true } }
```

响应：
```json
{
  "device_id": "<uuid>",
  "reported": { "os": "Windows 11", "cpu_count": 8 },
  "desired": { "screen_lock_timeout": 300, "vpn_enabled": true },
  "delta": { "screen_lock_timeout": 300, "vpn_enabled": true },
  "version": 3
}
```

### 命令下发

```http
POST /v1/tenants/{tid}/commands
Content-Type: application/json

{
  "target_device_id": "550e8400-e29b-41d4-a716-446655440000",
  "payload": { "action": "run_script", "params": { "script": "echo ok", "interpreter": "bash" } },
  "priority": 0,
  "ttl_seconds": 3600,
  "idempotency_key": "job-2026-04-14-001"
}
```

### 键鼠事件协议（LiveKit Data Channel，UTF-8 JSON）

前端通过 `livekit-client` 的 **Data Channel**（如 `publishData`）发送；Agent 在房间内接收数据消息并解析为输入事件。**不是** `dmsx-api` 的 HTTP/WebSocket 端点。

```json
{"type": "mousemove", "x": 500, "y": 300}
{"type": "mousedown", "button": "left", "x": 500, "y": 300}
{"type": "mouseup", "button": "left", "x": 500, "y": 300}
{"type": "keydown", "key": "a", "code": "KeyA", "modifiers": ["ctrl"]}
{"type": "keyup", "key": "a", "code": "KeyA"}
{"type": "scroll", "x": 500, "y": 300, "deltaX": 0, "deltaY": -120}
```

---

## 数据面 gRPC（`proto/dmsx/agent.proto`）

包名：`dmsx.agent.v1`。服务：`AgentService`。

| RPC | 类型 | 说明 |
|-----|------|------|
| `Enroll` | unary | enrollment token + CSR → 签发证书（当前内测实现要求 token **显式绑定 `device_id`**，且 `public_key_pem` 实际上传 **PKCS#10 CSR PEM**）。若网关同时启用 TLS + `DMSX_GW_TLS_CLIENT_CA` 且已配置 Enroll 所需 HMAC/CA，则握手层会允许**未持证书的新设备仅调用 `Enroll`**；拿到证书后其余 RPC 仍按设备证书身份校验 |
| `Heartbeat` | unary | 存活与轻量遥测 |
| `FetchDesiredState` | unary | 拉取当前策略 revision 与 `spec_json` |
| `StreamCommands` | server stream | 服务端推送 `CommandEnvelope`；当网关配置 **`DMSX_NATS_URL`** 且启用 JetStream 时，从与 `dmsx-api` 相同的 stream（默认 **`DMSX_COMMANDS`**）按 **`dmsx.command.{tenant_id}.{device_id}`** 拉取 `Command` JSON 并映射为 `CommandEnvelope`。当前实现使用按租户/设备稳定命名的 **durable pull consumer**；**同一租户/设备仅允许一个活跃流**，并按“**发出一条 -> 等对应 `ReportResult` 成功 -> ACK JetStream -> 再发下一条**”串行推进，保证单设备有序交付与断线可重投。若 `cursor` 提供 **JetStream stream sequence**，则首次创建 consumer 时会从该序号恢复（未配置 NATS 时流为空，与旧 stub 一致） |
| `ReportResult` | unary | 将执行结果发布到 JetStream **`dmsx.command.result.{tenant_id}.{device_id}`** 供 `dmsx-api` 入库；控制面入库时优先使用消息中的 `status` 更新命令状态，`exit_code` 仅用于结果详情。若该 `command_id` 正是当前活跃 `StreamCommands` 已下发但尚未提交的命令，则网关会在发布成功后推进对应 JetStream ACK；未配置 NATS 时响应 **`accepted=false`** |
| `UploadEvidence` | client stream | 分块上传证据到对象存储（当前实现写入 **S3 / MinIO 兼容桶**）；首个 chunk 必须提供 `device_id`，并满足“**mTLS 设备证书**或**有效 `upload_token`**”至少其一；若二者同时提供，则 `tenant_id` / `device_id` / 可选 `content_type` 必须一致。控制面可通过 **`POST /v1/tenants/{tid}/commands/{cid}/evidence-upload-token`** 为该命令签发短期 token |

认证：**mTLS**（设备证书）+ 可选 per-RPC metadata `authorization: Bearer <session>`。

**多租户与身份**：`ReportResultRequest` / `StreamCommandsRequest` 含可选 **`tenant_id`**。在 **mTLS 严格模式**（网关配置 **`DMSX_GW_TLS_CLIENT_CA`** 且未设置 **`DMSX_GW_TLS_CLIENT_AUTH_OPTIONAL`**）下，客户端证书 SAN 必须包含 URI **`urn:dmsx:tenant:{uuid}:device:{uuid}`**；服务端以证书为准校验 RPC 中的 **`device_id`**（及显式 **`tenant_id`**，若携带）与证书一致。为支持首证签发，若同时配置了 Enroll 所需 HMAC/CA，网关会在 **TLS 握手层**对 `Enroll` 放开“无客户端证书也可连入”，但**应用层**仍仅允许该匿名连接调用 `Enroll`；其余 RPC 必须带证书并完成同样的身份绑定校验。`UploadEvidence` 没有显式 `tenant_id` 字段，因此当前实现要求通过**设备证书**或 **`upload_token`** 推导租户；若两者都没有则拒绝落盘。**未启用 mTLS 时**须在 RPC 中显式提供合法 **`tenant_id` UUID**（开发/过渡场景；生产应走 mTLS）。`Enroll` 的 enrollment token 当前内测实现也要求显式携带 **`device_id`**，避免同一 token 重放生成多个设备身份。

---

## 错误模型

HTTP 使用 **Problem Details**（`application/problem+json`）。当前 `dmsx_core::ProblemDetails` 字段为 **`type`**（URI 字符串，可为 `about:blank`）、**`title`**、**`status`**（HTTP 状态码数值）、**`detail`**（人类可读说明）。OpenAPI 中 `ProblemDetails` schema 设 **`additionalProperties: true`**，与 RFC 7807 扩展字段约定一致；运行时响应以核心四字段为主。

与 `DmsxError` 的对应关系（见 `crates/dmsx-core/src/error.rs`）：

| 错误变体 | HTTP `status` | `title`（典型） |
|----------|---------------|-----------------|
| `Validation` | **400** | Bad Request |
| `Unauthorized` | **401** | Unauthorized |
| `Forbidden` | **403** | Forbidden |
| `NotFound` | **404** | Not Found |
| `Conflict` | **409** | Conflict |
| `Internal` | **500** | Internal Server Error |

**说明**：业务校验失败当前映射为 **400**，**不是 422**；若未来引入 RFC 4918 语义，再在实现与 OpenAPI 中同步演进。

gRPC：标准 `google.rpc.Status` 可附加 `ErrorInfo`（`reason`, `domain`, `metadata`）。

机器可读 OpenAPI：[openapi/dmsx-control-plane.yaml](../openapi/dmsx-control-plane.yaml)（`paths` 与已实现路由一致；全局 **`security: [bearerAuth]`**，`/health` 与 `/ready` 除外；**`bearerAuth`** 说明中含 **`allowed_tenant_ids`**、**`tenant_roles`** 与 **`roles`** 语义；`components.responses` 含 **401 / 403 / 404 / 400 / 409 / 500** 与 `ProblemDetails` schema）。
