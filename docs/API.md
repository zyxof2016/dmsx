# API 契约

## 控制面 REST（前缀 `/v1`）

基址示例：`https://api.example.com`。租户作用域由 URL 路径中的 `{tenant_id}` 决定；JWT 须证明该租户在本令牌下可访问（见下「多租户 JWT」）。

**远程桌面**：音视频与键鼠控制走 **LiveKit WebRTC**（浏览器 `livekit-client` 订阅、Agent SDK 发布）；`dmsx-api` 负责会话创建、LiveKit JWT 签发，并通过命令队列触发 Agent `start_desktop` / `stop_desktop`。**不再提供**经 `dmsx-api` 的桌面 JPEG WebSocket 中继端点。

**认证**：除 `/health`、`/ready` 外，管理接口需 `Authorization: Bearer <JWT>`（路径 `{tenant_id}` 须被该 JWT 允许；`GET /v1/config/livekit` 需 **PlatformAdmin**）。OpenAPI 根级 **`security: [bearerAuth]`**，并在各操作中声明 **`401` / `403`**；资源类接口另声明 **`404`**；各操作另声明 **`500`**（内部错误）。上述错误均复用 `components.responses` 与 **`ProblemDetails`**（见 `openapi/dmsx-control-plane.yaml`）。

**横切限制（可选）**：控制面可启用 **请求体大小限制** 与 **速率限制**（per-tenant）。当启用时：

- 请求体超限返回 **413**（`Payload Too Large`，ProblemDetails）
- 触发速率限制返回 **429**（`Too Many Requests`，ProblemDetails；并附 `retry-after`/`x-ratelimit-after` 等头）

**多租户 JWT（单用户多租户）**：JWT 声明中含 **`tenant_id`**（UUID，默认/主租户，无 `allowed_tenant_ids` 时即唯一允许租户）与可选数组 **`allowed_tenant_ids`**（UUID）。有效租户集合为 **`tenant_id` ∪ `allowed_tenant_ids`**。对 `/v1/tenants/{tenant_id}/...` 的请求，路径中的 `{tenant_id}` 必须属于该集合，否则 **403**。`AuthContext` 中的活动租户与路径一致，便于前端**切换租户**：更换 URL 中的 `{tenant_id}` 即可；签发方应在成员关系变化时更新 **`allowed_tenant_ids`**。

**按租户 RBAC（`tenant_roles`）**：可选对象 **`tenant_roles`**，键为租户 UUID 字符串、值为该租户下角色字符串数组（与令牌级 **`roles`** 同一套角色名，如 `TenantAdmin`、`ReadOnly`）。对某次请求，**活动租户**为路径中的 `{tenant_id}`（无路径租户时，如 `GET /v1/config/livekit`，则为 JWT 的 **`tenant_id`**）。若 **`tenant_roles` 中存在该活动租户的键**（含空数组 `[]`），则本请求 **仅使用该键对应数组** 做 RBAC；若 **无该键**，则回退使用令牌级 **`roles`**。空数组表示该租户下显式无角色，受保护路由将 **403**（与「未声明 `roles`」行为一致）。

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
| GET | `/v1/config/livekit` | LiveKit 配置查询（`{ enabled, url }`） |

### 租户与组织结构

> **实现状态**：下列 **POST** 已在 `dmsx-api` 注册并与 **OpenAPI** 对齐。`POST /v1/tenants` 与 `GET /v1/config/livekit` 同级 RBAC：**仅 `PlatformAdmin`**（`jwt` 模式；`disabled` 不校验）。其余创建路径在 **`jwt` 模式**下需 **`TenantAdmin`**（或更高）且路径 `{tid}` 属于 JWT 许可租户集合；请求体字段 **`name`** 长度 1–200。站点创建要求 **`org_id`** 属于该租户；设备组创建要求 **`site_id`** 属于该租户（否则 **400**）。名称在同一父级下违反唯一约束时 **409**。

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
| GET | `/v1/tenants/{tid}/devices` | 列表（支持 `search`, `platform`, `enroll_status`, `online_state` 筛选 + 分页） |
| POST | `/v1/tenants/{tid}/devices` | 注册/预置设备 |
| GET/PATCH/DELETE | `/v1/tenants/{tid}/devices/{did}` | 查询/更新标签分组/吊销 |

### 设备影子（Device Shadow）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/v1/tenants/{tid}/devices/{did}/shadow` | 获取设备影子（含 reported / desired / delta） |
| PATCH | `/v1/tenants/{tid}/devices/{did}/shadow/desired` | 更新期望状态（管理员下发） |
| PATCH | `/v1/tenants/{tid}/devices/{did}/shadow/reported` | 更新已报告状态（Agent 心跳上报） |

### 设备远控（Remote Control）

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/v1/tenants/{tid}/devices/{did}/actions` | 下发设备操作（reboot / shutdown / lock_screen / run_script 等） |
| GET | `/v1/tenants/{tid}/devices/{did}/commands` | 查询该设备的命令历史 |

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

### 命令管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET/POST | `/v1/tenants/{tid}/commands` | 列表 / 下发命令（**202 Accepted**；支持 `idempotency_key`） |
| GET | `/v1/tenants/{tid}/commands/{cid}` | 查询命令状态与回执摘要 |
| PATCH | `/v1/tenants/{tid}/commands/{cid}/status` | 更新命令状态（Agent 回报） |
| GET/POST | `/v1/tenants/{tid}/commands/{cid}/result` | 查询 / 提交命令执行结果（exit_code / stdout / stderr） |

### 制品与合规

| 方法 | 路径 | 说明 |
|------|------|------|
| GET/POST | `/v1/tenants/{tid}/artifacts` | 制品列表（分页）/ 创建元数据（**201** 返回完整 `Artifact` 行；非预签名上传 URL） |
| GET | `/v1/tenants/{tid}/compliance/findings` | 合规发现列表（分页；支持 `search` / `severity` / `status`） |

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
| `Enroll` | unary | enrollment token + 设备公钥 → 签发证书 |
| `Heartbeat` | unary | 存活与轻量遥测 |
| `FetchDesiredState` | unary | 拉取当前策略 revision 与 `spec_json` |
| `StreamCommands` | server stream | 服务端推送 `CommandEnvelope` |
| `ReportResult` | unary | 命令执行结果与证据指针 |
| `UploadEvidence` | client stream | 分块上传证据到对象存储（网关签发 `upload_token`） |

认证：**mTLS**（设备证书）+ 可选 per-RPC metadata `authorization: Bearer <session>`。

**多租户隔离约定**：gRPC 消息体不含 `tenant_id`。设备身份通过 mTLS 客户端证书绑定到唯一 `device_id`，`device_id → tenant_id` 映射在 enrollment 时建立并存储在服务端。网关根据证书自动填充 `tenant_id`，不信任客户端传入。

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
