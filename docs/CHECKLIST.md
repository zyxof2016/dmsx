# DMSX 功能清单与完成状态

> **维护规则**：每次新增功能或完成已有功能时，**必须同步更新本文件**中对应条目的状态标记。
>
> 状态说明：
> - [x] 已完成（可编译 / 可运行 / 有测试覆盖）
> - [~] 骨架 / Stub（代码结构已有，但逻辑为占位或 mock 数据）
> - [ ] 未开始

---

## 1. 工程基础

- [x] Rust workspace 搭建（`Cargo.toml`、`Cargo.lock`）
- [x] 多 crate 拆分：`dmsx-core` / `dmsx-ai` / `dmsx-api` / `dmsx-device-gw`
- [x] 统一错误类型 `DmsxError` + RFC 7807 Problem Details
- [x] `DmsxError` 实现 Axum `IntoResponse`（feature gate `axum`）
- [x] `.gitignore` 配置
- [x] CI 流水线（`.github/workflows/ci.yml`：fmt / clippy / build / test / Docker）
- [x] `cargo check` / `cargo clippy` 零错误零警告
- [x] 工程规范与开发约定文档（`docs/ENGINEERING_STANDARDS.md`）
- [x] PR / Code Review 检查清单（`docs/PR_REVIEW_CHECKLIST.md`）
- [x] `dmsx-agent` 首轮模块化拆分（`config` / `api` / `telemetry` / `rustdesk` / `command_runner` / `desktop`）
- [x] `dmsx-agent` 第二轮模块化拆分（`app` / `device` / `platform`，`main.rs` 仅保留入口）
- [x] `dmsx-agent` 第三轮模块化拆分（`desktop` 细分为 `capture` / `input` / `session`，停止逻辑收口为方法）
- [x] `dmsx-agent` 首批测试用例已补充（`device` 注册/心跳 + `script` 参数分支，`cargo test -p dmsx-agent --lib` 已通过）
- [x] `dmsx-api` 轻量测试入口已建立（`lib.rs` / `app.rs` / `error.rs`，`cargo test -p dmsx-api --lib` 已通过）
- [x] `dmsx-api` handlers 纯逻辑已下沉到 `helpers` 并补测试（影子 delta / 命令结果状态，`cargo test -p dmsx-api --lib` 已通过 10 项）
- [~] 集成测试框架（`dmsx-agent --lib` 已有实际用例，workspace / 全量二进制链路待继续扩展）
- [ ] `cargo bench` 性能基准

---

## 2. 领域模型（`dmsx-core`）

- [x] 多租户资源层级 ID 新类型：`TenantId` / `OrgId` / `SiteId` / `GroupId` / `DeviceId`
- [x] 业务枚举：`DevicePlatform` / `EnrollStatus` / `OnlineState` / `CommandStatus` / `PolicyScopeKind` / `Severity` / `FindingStatus`
- [x] 实体结构体：`Tenant` / `Org` / `Site` / `Group` / `Device`
- [x] 实体结构体：`Policy` / `PolicyRevision`
- [x] 实体结构体：`Command`
- [x] 实体结构体：`Artifact`
- [x] 实体结构体：`AuditLog` / `ComplianceFinding`
- [x] 实体结构体：`DeviceShadow` / `CommandResult`
- [x] `sqlx` feature gate — 所有 ID/枚举/实体支持 `sqlx::Type` / `FromRow`
- [x] DTO 独立模块 + 输入校验 validate()（hostname/sha256/priority/ttl 等）

---

## 3. 数据库迁移

### Postgres（`migrations/001_init.sql` + `003_shadow_and_results.sql`）

- [x] `tenants` / `orgs` / `sites` / `groups` 表
- [x] `devices` 表（含 JSONB `labels` / `capabilities`、GIN 索引）
- [x] `policies` / `policy_revisions` 表
- [x] `commands` 表 + 部分唯一索引（幂等键 `WHERE idempotency_key IS NOT NULL`）
- [x] `artifacts` 表
- [x] `audit_logs` 表
- [x] `compliance_findings` 表
- [x] `device_shadows` 表（设备影子 — reported/desired/version 乐观并发控制）
- [x] `command_results` 表（命令执行结果 — exit_code/stdout/stderr）
- [x] 所有枚举类型（`device_platform` / `enroll_status` 等）
- [ ] RLS（Row Level Security）策略
- [ ] 按 `tenant_id` HASH 分区
- [ ] 数据库版本迁移工具（sqlx-migrate / refinery）

### ClickHouse（`migrations/ch/001_init.sql`）

- [x] `audit_events` 表（MergeTree、365 天 TTL）
- [x] `device_heartbeats` 表（90 天 TTL）
- [x] `command_results` 表（ReplacingMergeTree 去重）
- [x] `policy_drift_events` 表（180 天 TTL）
- [ ] 物化视图（实时聚合仪表盘指标）

---

## 4. 控制面 API（`dmsx-api`）

### 路由

- [x] `GET /health` 健康检查
- [~] `POST /v1/tenants` 创建租户（stub, 启动时自动 seed）
- [~] `POST /v1/tenants/{tid}/orgs` 创建组织（stub）
- [~] `POST /v1/tenants/{tid}/orgs/{oid}/sites` 创建站点（stub）
- [~] `POST /v1/tenants/{tid}/sites/{sid}/groups` 创建设备组（stub）
- [x] `GET /v1/tenants/{tid}/devices` 设备列表（sqlx 持久化）
- [x] `POST /v1/tenants/{tid}/devices` 注册设备（sqlx 持久化）
- [x] `GET/PATCH/DELETE /v1/tenants/{tid}/devices/{did}` 设备 CRUD（sqlx 持久化）
- [x] `GET/POST /v1/tenants/{tid}/policies` 策略列表/创建（sqlx 持久化）
- [x] `GET/PATCH/DELETE /v1/tenants/{tid}/policies/{pid}` 策略 CRUD（sqlx 持久化）
- [x] `POST /v1/tenants/{tid}/policies/{pid}/revisions` 发布策略版本（sqlx 持久化）
- [x] `GET/POST /v1/tenants/{tid}/commands` 命令列表/下发 — 202（sqlx 持久化）
- [x] `GET /v1/tenants/{tid}/commands/{cid}` 查询命令状态（sqlx 持久化）
- [x] `PATCH /v1/tenants/{tid}/commands/{cid}/status` 更新命令状态
- [x] `GET/POST /v1/tenants/{tid}/commands/{cid}/result` 命令结果查询/提交
- [x] `GET /v1/tenants/{tid}/devices/{did}/shadow` 获取设备影子（含 delta 计算）
- [x] `PATCH /v1/tenants/{tid}/devices/{did}/shadow/desired` 更新期望状态
- [x] `POST /v1/tenants/{tid}/devices/{did}/actions` 设备远控操作下发
- [x] `GET /v1/tenants/{tid}/devices/{did}/commands` 设备命令历史
- [x] `GET/POST /v1/tenants/{tid}/artifacts` 制品列表/创建（sqlx 持久化）
- [x] `GET /v1/tenants/{tid}/compliance/findings` 合规发现列表（sqlx 持久化）
- [x] `GET /v1/tenants/{tid}/stats` Dashboard 聚合统计（sqlx）
- [x] `POST /v1/tenants/{tid}/devices/{did}/desktop/session` 创建远程桌面会话（LiveKit Token + Agent 命令）
- [x] `DELETE /v1/tenants/{tid}/devices/{did}/desktop/session` 终止远程桌面会话
- [x] `GET /v1/tenants/{tid}/devices/{did}/desktop/ws/viewer` WebSocket 视频帧订阅（管理员端）
- [x] `GET /v1/tenants/{tid}/devices/{did}/desktop/ws/agent` WebSocket 视频帧推送（Agent 端）
- [x] `GET /v1/config/livekit` LiveKit 配置查询
- [~] `POST /v1/tenants/{tid}/ai/anomalies` AI 异常检测（规则引擎 stub）
- [~] `POST /v1/tenants/{tid}/ai/recommendations` AI 策略推荐（stub）
- [~] `POST /v1/tenants/{tid}/ai/chat` AI 智能助手（stub，待接 LLM）
- [~] `POST /v1/tenants/{tid}/ai/predictions` AI 预测性维护（stub）

### 中间件与横切

- [~] 认证中间件骨架（`auth_middleware`，当前直接放行）
- [x] 监听地址可配置（`DMSX_API_BIND` 环境变量）
- [x] `TraceLayer` 日志追踪
- [ ] JWT / OIDC 认证实现
- [ ] RBAC 权限校验
- [ ] 租户 URL ↔ JWT `tenant_id` 一致性校验
- [ ] 速率限制（per-tenant）
- [ ] 请求体大小限制
- [ ] CORS 生产配置

### 持久化

- [x] sqlx 连接池接入（Postgres, PgPoolOptions, 自动迁移）
- [x] 真实 CRUD 替换 stub（设备/策略/命令/制品/合规/统计）
- [x] AppState 注入 + 环境变量配置（`DATABASE_URL` / `LIVEKIT_URL` / `LIVEKIT_API_KEY` / `LIVEKIT_API_SECRET`）
- [x] 启动时自动运行 sqlx migrations
- [x] CORS 中间件（`tower-http CorsLayer`）
- [ ] ClickHouse 客户端接入（审计/遥测写入）
- [ ] Redis 接入（缓存 / 在线状态 / 分布式锁）
- [ ] NATS JetStream 接入（命令投递 / 回执流）
- [ ] S3 / MinIO 接入（制品上传预签名）

---

## 5. 数据面 gRPC（`dmsx-device-gw`）

### RPC

- [~] `Enroll` — 返回 `UNIMPLEMENTED`（待接 CA）
- [x] `Heartbeat` — 返回服务器时间
- [~] `FetchDesiredState` — 返回空策略（stub）
- [~] `StreamCommands` — 返回空流（stub）
- [x] `ReportResult` — 接收并记录日志
- [~] `UploadEvidence` — 消费流 + 256 MiB 限制（未持久化）
- [x] gRPC Health Check（`grpc.health.v1.Health`）

### 基础设施

- [x] 监听地址可配置（`DMSX_GW_BIND` 环境变量）
- [ ] mTLS 启用
- [ ] 连接限流 / 背压
- [ ] 设备身份校验（证书 → device_id → tenant_id）
- [ ] NATS JetStream 接入（命令推送 / 回执转发）
- [ ] OpenTelemetry 追踪注入

---

## 6. Proto 契约

- [x] `proto/dmsx/agent.proto` — Agent 服务定义（6 RPC）
- [x] `CommandStatusProto` / `DevicePlatformProto` 枚举（与 `dmsx-core` 对齐）
- [x] 多租户隔离约定注释（`device_id → tenant_id` 服务端映射）
- [x] `proto/grpc/health/v1/health.proto` — 标准 gRPC 健康检查

---

## 7. AI 智慧管控（`dmsx-ai`）

### 引擎抽象

- [x] `AiEngine` trait（四大能力统一接口）
- [x] `AiError` 错误类型
- [x] AI 领域类型（Request / Response / DTO）

### 异常检测

- [~] `RuleBasedAnomalyDetector`（返回固定"正常"报告，待接 CH 查询）
- [ ] 统计阈值检测（Z-score / 滑动窗口）
- [ ] 时序异常模型（Isolation Forest / AutoEncoder）
- [ ] LLM 辅助归因与摘要

### 策略推荐

- [~] `PolicyRecommender` 骨架（空实现）
- [ ] 规则模板推荐（设备画像 → 策略 spec）
- [ ] LLM JSON 生成 + RAG

### 自然语言助手

- [~] `LlmAssistant` 骨架（返回 `ModelUnavailable`）
- [ ] OpenAI 兼容 API 对接
- [ ] 本地模型对接（Ollama / vLLM）
- [ ] 系统提示词 + function calling → 内部 API
- [ ] 对话历史管理

### 预测性维护

- [~] `MaintenancePredictor` 骨架（空实现）
- [ ] 滑动窗口 + 线性外推
- [ ] ONNX Runtime 时序预测
- [ ] LLM 辅助建议

---

## 8. 前端管理台（`web/`）

### 基础

- [x] Vite 6 + React 19 + TypeScript 构建
- [x] Ant Design 5 中文本地化 + 主题配置
- [x] TanStack Query 集成
- [x] API 客户端封装（`api/client.ts`）
- [x] Vite 开发代理（`/v1` → 后端）
- [x] 侧栏 + 顶栏 + 内容区布局
- [x] 菜单导航与页面切换
- [x] TypeScript `tsc --noEmit` 零错误
- [x] TypeScript 类型定义（`api/types.ts` — 完整 DTO 类型）
- [x] TanStack Query hooks（`api/hooks.ts` — 设备/策略/命令/制品/合规/统计）
- [x] 真实 API 数据对接（所有核心页面替换 mock，CRUD 完整闭环）
- [x] TanStack Router URL 路由（替换 useState 页面切换，支持浏览器前进/后退/链接分享）
- [x] 面包屑自动从当前路由生成
- [x] 服务端分页/筛选/排序（ListParams → 后端 LIMIT/OFFSET/WHERE）
- [x] CSV 导出（前端 Blob 下载）
- [x] 批量操作（设备/策略多选批量删除）
- [x] 空状态引导组件（Empty + 操作提示）
- [x] 列表轮询（设备 10s / 命令 10s / 统计 15s）
- [ ] 暗色模式
- [ ] 国际化（i18n）

### 页面

- [x] 态势总览（Dashboard）— KPI 卡片 + AI 洞察 + 安全态势 + recharts 图表（饼图/柱状图/折线图）
- [x] 设备管理（Devices）— 表格 + 服务端筛选 + 搜索 + 创建/删除 + 批量操作 + CSV 导出
- [x] 策略中心（Policies）— 列表 + 服务端搜索 + 创建/删除 + 批量操作 + CSV 导出
- [x] 远程命令（Commands）— 列表 + 状态筛选 + 下发(含二次确认) + CSV 导出
- [x] 应用分发（Artifacts）— 列表 + 搜索 + 上传 + SHA256 校验 + CSV 导出
- [x] 安全合规（Compliance）— 统计 + 严重度/状态筛选 + 合规率(按设备去重) + CSV 导出
- [x] 网络管控（Network）— 站点 + 隧道 + 带宽 + ⚠️演示数据标记
- [x] AI 智慧中心（AiCenter）— 四 Tab + ⚠️演示数据标记
- [x] 全局 AI 悬浮按钮
- [x] 设备详情抽屉（Tabs：基本信息 / 设备影子 / 远控面板 / 远程桌面）
- [x] 设备影子面板（ShadowPanel — 三列对比 Reported/Desired/Delta + JSON 编辑器 + 乐观并发）
- [x] 远控面板（RemoteControl — 快捷操作网格 + 脚本执行器 + 操作历史 + 结果查看 + 擦除三重确认）
- [x] 远程桌面面板（RemoteDesktop — WebSocket 实时画面 + 键鼠控制 + 坐标映射 + 全屏 + RustDesk 备选）
- [x] 策略详情抽屉（完整信息 + 作用域字段）
- [x] 命令详情抽屉（payload JSON 高亮、状态标签、目标设备信息 + 执行结果展示 exit_code/stdout/stderr）
- [ ] 系统设置页面
- [ ] 策略编辑器（JSON Schema 表单 / Monaco Editor）
- [ ] 审计日志查看页
- [ ] 用户 / 角色管理页

---

## 9. OpenAPI 契约（`openapi/dmsx-control-plane.yaml`）

- [x] 全部控制面路由定义（含策略单资源 CRUD）
- [x] `ProblemDetails` 错误 schema
- [x] `CommandCreate` / `StubEntity` / `DeviceList` schema
- [ ] 完善所有请求/响应 schema（替换 `type: object` 占位）
- [ ] 认证安全方案（`bearerAuth` / `oauth2`）
- [ ] 所有端点 4xx/5xx 错误响应引用 `ProblemDetails`

---

## 10. 安全

- [x] 安全设计文档（`docs/SECURITY.md`）
- [x] 设备 mTLS 架构设计
- [x] RBAC 角色与范围设计
- [x] 审计不可篡改设计（PG + CH + 对象存储）
- [x] 制品签名设计（cosign / sigstore）
- [ ] 认证实现（JWT / OIDC）
- [ ] RBAC 中间件实现
- [ ] 设备证书签发（CA 集成）
- [ ] 证书轮换 / 吊销实现
- [x] 审计日志自动写入（所有 create/update/delete/publish 操作写入 audit_logs）
- [ ] 制品签名校验实现

---

## 11. 部署与运维

### Docker

- [x] `deploy/docker-compose.yml`（Postgres / Redis / NATS / ClickHouse / MinIO / OTel / RustDesk / LiveKit）
- [x] `deploy/Dockerfile.api`（多阶段构建）
- [x] `deploy/Dockerfile.device-gw`（多阶段构建）
- [ ] `deploy/Dockerfile.web`（前端 Nginx 静态服务）

### Kubernetes

- [x] `dmsx-api` Deployment + Service（探针 / resources / securityContext）
- [x] `dmsx-device-gw` Deployment + Service（gRPC 探针 / resources / securityContext）
- [ ] Namespace / RBAC 清单
- [ ] ConfigMap / Secret 示例
- [ ] Ingress + TLS 清单
- [ ] HPA 自动伸缩配置
- [ ] PDB 配置

### 可观测性

- [x] OTel Collector 配置（`deploy/otel-collector-config.yaml`）
- [ ] 应用侧 OpenTelemetry SDK 集成
- [ ] Prometheus ServiceMonitor
- [ ] Grafana 仪表盘模板
- [ ] 告警规则（PrometheusRule）

---

## 12. 文档

- [x] `README.md` — 项目总览与快速开始
- [x] `docs/ARCHITECTURE.md` — 架构与服务边界
- [x] `docs/DOMAIN_MODEL.md` — 领域模型与存储
- [x] `docs/API.md` — API 契约（控制面 REST + 数据面 gRPC）
- [x] `docs/SECURITY.md` — 安全设计
- [x] `docs/AI_DESIGN.md` — AI 智慧管控设计
- [x] `docs/FRONTEND.md` — 前端架构
- [x] `docs/DEPLOYMENT.md` — 部署与可观测性
- [x] `docs/SCALING_AND_ROADMAP.md` — 容量估算与路线图
- [x] `docs/CHECKLIST.md` — 本文件（功能清单与完成状态）

---

## 统计摘要

| 状态 | 数量 |
|------|------|
| [x] 已完成 | 135 |
| [~] 骨架/Stub | 17 |
| [ ] 未开始 | 48 |

> 最后更新：2026-04-15（LiveKit 远程桌面集成完成）
