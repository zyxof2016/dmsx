# API 契约

## 控制面 REST（前缀 `/v1`）

基址示例：`https://api.example.com`。所有路径下资源均属于路径中的 `{tenant_id}`（或从 JWT `tenant_id` 推导后与 URL 校验一致）。

### 通用端点

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/health` | 健康检查 |
| GET | `/v1/config/livekit` | LiveKit 配置查询（`{ enabled, url }`） |

### 租户与组织结构

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/v1/tenants` | 创建租户（平台级，需超级管理员） |
| POST | `/v1/tenants/{tid}/orgs` | 创建组织 |
| POST | `/v1/tenants/{tid}/orgs/{oid}/sites` | 创建站点 |
| POST | `/v1/tenants/{tid}/sites/{sid}/groups` | 创建设备组 |
| GET | `/v1/tenants/{tid}/stats` | Dashboard 聚合统计 |

### 设备管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/v1/tenants/{tid}/devices` | 列表（支持 `search`, `platform`, `online_state` 筛选 + 分页） |
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
| POST | `/v1/tenants/{tid}/devices/{did}/desktop/session` | 创建远程桌面会话（签发 LiveKit Token + 下发 start_desktop 命令） |
| DELETE | `/v1/tenants/{tid}/devices/{did}/desktop/session` | 终止远程桌面会话 |
| GET (WebSocket) | `/v1/tenants/{tid}/devices/{did}/desktop/ws/viewer` | 管理员端 WS：接收 JPEG 帧 / 发送键鼠事件 |
| GET (WebSocket) | `/v1/tenants/{tid}/devices/{did}/desktop/ws/agent` | Agent 端 WS：推送 JPEG 帧 / 接收键鼠事件 |

### 策略管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET/POST | `/v1/tenants/{tid}/policies` | 列表 / 创建策略 |
| GET/PATCH/DELETE | `/v1/tenants/{tid}/policies/{pid}` | 查询 / 更新 / 删除单条策略 |
| POST | `/v1/tenants/{tid}/policies/{pid}/revisions` | 发布新版本（body: `spec`, `rollout`） |

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
| GET/POST | `/v1/tenants/{tid}/artifacts` | 制品列表 / 创建记录 |
| GET | `/v1/tenants/{tid}/compliance/findings` | 合规发现列表 |

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

{}
```

响应：
```json
{
  "room": "desktop-{device_id}-1776239309",
  "token": "<LiveKit JWT>",
  "livekit_url": "ws://127.0.0.1:7880",
  "session_id": "<uuid>"
}
```

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

### 键鼠事件协议（WebSocket DataChannel JSON）

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

HTTP 使用 **Problem Details**（`application/problem+json`）：`type`, `title`, `status`, `detail`, `tenant_id`（可选）。

gRPC：标准 `google.rpc.Status` 可附加 `ErrorInfo`（`reason`, `domain`, `metadata`）。

机器可读 OpenAPI：[openapi/dmsx-control-plane.yaml](../openapi/dmsx-control-plane.yaml)。
