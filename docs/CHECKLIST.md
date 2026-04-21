# DMSX 功能清单与完成状态

> **维护规则**：每次新增功能或完成已有功能时，**必须同步更新本文件**中对应条目的状态标记。
>
> 状态说明：
> - [x] 已完成（可编译 / 可运行 / 有测试覆盖）
> - [~] 骨架 / Stub（代码结构已有，但逻辑为占位或 mock 数据）
> - [ ] 未开始

---

## 内测阶段目标与完成定义（当前）

> **当前目标：先内测**（与 `docs/SCALING_AND_ROADMAP.md` 中「控制面可用 + Agent 主链路闭环」一致）。本节为 **内测门禁 DoD** 与 **明确延后项**，用于收敛范围；公测或多租户生产前再对照下方 §1 起全文逐项加码。
>
> **本轮内测范围**：**不包含远程桌面**（不验收 LiveKit 联调、不要求管理台远控面板与 Agent 桌面会话闭环）。`docker compose` 仍可带 LiveKit 服务，但**不作为本轮内测门禁**。

### 内测 DoD（交付内测用户前建议逐项勾选）

- [x] **环境可复现**：按 `README.md` 与 `deploy/docker-compose.yml` 可在内测机拉起依赖；**仓库侧** 一键校验见 [`scripts/reproduce-dev-env.sh`](../scripts/reproduce-dev-env.sh)（验证 **主机 5432** 上 `postgres://dmsx:dmsx@127.0.0.1:5432/dmsx` 与 README 一致；**2026-04-16** 已通过 compose + 回退检测）。`dmsx-api` / `dmsx-agent` / 前端 `web/` 的**参与者网络说明**仍见未勾「范围与反馈」或发版说明（**本轮不要求**为验桌面而单独拉起或打通 LiveKit）
- [x] **主链路可重复演示**：设备注册 → 心跳 → 命令下发/轮询/状态与结果，在内测环境至少完成 **2 轮**完整走通（含失败重试或刷新后的可理解表现）。**辅助脚本**（curl + `python3`，不启真实 Agent）：[`scripts/internal-beta-smoke-http.sh`](../scripts/internal-beta-smoke-http.sh)（环境变量见脚本头注释；可跑多遍作多轮中的「API 闭环」）；**仓库侧** 2026-04-16 已连续跑通 **2 轮**（见下表）
- **远程桌面（本轮不适用）**：已明确**不纳入**本轮内测；**不要求**完成 LiveKit + `POST/DELETE .../desktop/session` + 浏览器/Agent 联调。后续轮次或专项里程碑再启用验收口径时，可改回可勾选条目。
- [x] **认证约定落地**：内测统一 `DMSX_API_AUTH_MODE`（常见为 `disabled` 或团队共享 JWT）；**本轮冒烟**在 `disabled` 下完成。若启用 `jwt`，JWT 中 `tenant_id` / `allowed_tenant_ids` / `tenant_roles` / `roles` 与 `docs/API.md`、`openapi/dmsx-control-plane.yaml` 一致；**库测** `dmsx-api --lib` 已覆盖 JWT/JWKS 分支（见 `internal-beta-verify.sh`），参与者取令牌方式仍须在发版说明中单独告知
- [x] **自动化基线绿灯**：`cargo test -p dmsx-api --lib`、`cargo test -p dmsx-agent --lib`（或与 CI 等价命令）在内测所用分支/标签上通过（**一键脚本**：[`scripts/internal-beta-verify.sh`](../scripts/internal-beta-verify.sh)；**验证记录**见下表）
- [x] **范围与反馈**：范围与延后项见 [`docs/INTERNAL_BETA_SCOPE_AND_FEEDBACK.md`](INTERNAL_BETA_SCOPE_AND_FEEDBACK.md)；反馈入口（负责人 / Issue / 群）已指定并可被内测参与者直接引用

#### 内测验证记录（仓库侧可复现）

| 日期 | 命令 / 动作 | 结果 |
|------|-------------|------|
| 2026-04-16 | `cargo test -p dmsx-api --lib` | **48 passed** |
| 2026-04-16 | `cargo test -p dmsx-agent --lib` | **6 passed**（Agent lib 测试含 `wiremock` 等依赖，首编译可能较慢） |
| 2026-04-16 | `./scripts/internal-beta-smoke-http.sh`（`DMSX_SMOKE_API=http://127.0.0.1:8080`，本机已起 `dmsx-api` + PG） | **通过**（脚本已改为 `python3` 解析 JSON，无需 jq） |
| 2026-04-16 | `./scripts/internal-beta-smoke-http.sh` **第 2 轮**（同上 `DMSX_SMOKE_API` / `DMSX_API_AUTH_MODE=disabled`） | **通过** |
| 2026-04-16 | `./scripts/internal-beta-verify.sh`（复跑：`dmsx-api` 48 passed、`dmsx-agent` 6 passed） | **通过** |
| 2026-04-16 | `./scripts/reproduce-dev-env.sh`（`REPRODUCE_MINIMAL=1`；本机 5432 占用时回退校验 `dmsx-postgres`） | **通过**；随后 `dmsx-api` + `internal-beta-smoke-http.sh` **通过** |
| 2026-04-16 | `REPRODUCE_MINIMAL=1 ./scripts/reproduce-dev-env.sh` + `DATABASE_URL=... DMSX_API_AUTH_MODE=disabled DMSX_API_BIND=127.0.0.1:8080 cargo run -p dmsx-api` + `./scripts/internal-beta-dod.sh` | **通过**（DoD 一键：库级基线 + 主链路 HTTP 冒烟） |

### 网内测试建议（内测环境）

> 面向「先内测」：**控制面与数据默认只在团队内网 / VPN 可达范围验证**，避免与公网生产要求混淆。

- [ ] **网络边界**：`dmsx-api`、Postgres、管理台等仅绑定内网地址或经 VPN；若必须经公网演示，须单独评审（TLS、**`jwt` 模式**、防火墙白名单），且不把生产密钥写入仓库（**本轮**未验收远程桌面时，**LiveKit 可不作为必起/必通依赖**）
  - 验证（K8s 示例）：`kubectl -n dmsx get svc dmsx-api -o yaml | rg "type:"`（应为空/ClusterIP）；内测阶段默认不 apply `deploy/kubernetes/dmsx-api-ingress.yaml`；如需 Ingress，务必使用 internal IngressClass 或源 IP 白名单
- [ ] **节奏**：每次合并至内测分支或打内测标签前，至少执行 DoD 中的 **`cargo test -p dmsx-api --lib`**、**`cargo test -p dmsx-agent --lib`**，并跑通 **一条**主路径冒烟（可用 **[`scripts/internal-beta-smoke-http.sh`](../scripts/internal-beta-smoke-http.sh)** 代替手工 curl，仍需真实 Agent 的场景另测）。**推荐一键脚本**：[`scripts/internal-beta-dod.sh`](../scripts/internal-beta-dod.sh)
- [ ] **数据与凭据**：内测库、JWT 密钥及（若启用桌面或 LiveKit 时的）LiveKit Secret 视为敏感；截图/录屏中含 token 时须打码；不向网外渠道粘贴完整环境变量

### 内测阶段明确延后（不纳入内测门禁）

以下**不要求**在内测前完成；下一里程碑（扩大用户面或公网暴露）再纳入计划：

- Postgres 按 `tenant_id` **HASH 分区**（除 `commands` 外的热点表仍延后）、独立迁移工具链成熟
- **ClickHouse** 客户端写入审计/遥测、物化视图、归档策略落地
- **Redis**（缓存/在线/锁）、**NATS JetStream**（命令投递）、**S3/MinIO 预签名**等控制面横切生产化
- **`dmsx-device-gw` 作为默认数据面**、设备 **mTLS** 全链路、CA 集成与吊销自动化
- **AI** 四类接口从 stub 到真实引擎、**OpenAPI oauth2** 等文档增强
- **K8s** Ingress/HPA/PDB、应用侧 OTel SDK、完整告警规则等运维深化

---

## 多租户公测（控制面 `jwt` + 双租户数据）

> **定位**：在 **内测 `disabled` 冒烟** 之外，对 **`DMSX_API_AUTH_MODE=jwt`**、**路径租户 ∈ `tenant_id` ∪ `allowed_tenant_ids`**（见 [`API.md`](API.md)）以及 **库内第二租户** 下的主链路做一次可复现的门禁。不等价于公网生产就绪（TLS、配额、审计落库等仍见 §1 起全文与下方未勾选项）。

### 多租户公测 DoD（仓库侧）

- [x] **第二租户种子**：[`migrations/004_second_tenant_seed.sql`](../migrations/004_second_tenant_seed.sql)（租户 B `22222222-2222-2222-2222-222222222222`）。**注意**：`sqlx::migrate!` 在 **编译 `dmsx-api` 时**嵌入迁移文件；新增或修改 `migrations/*.sql` 后须 **`cargo build -p dmsx-api`（或 `cargo run` 触发重编）** 再启动 API，否则数据库不会执行新脚本。
- [x] **HTTP 门禁**：[`scripts/public-beta-multi-tenant-smoke.sh`](../scripts/public-beta-multi-tenant-smoke.sh) — 使用与 API 一致的 **`DMSX_API_JWT_SECRET`**、`python3` 标准库签发 HS256 JWT；在租户 A、B 各执行一遍 [`internal-beta-smoke-http.sh`](../scripts/internal-beta-smoke-http.sh)；**仅含租户 A 的令牌**访问租户 B 的 `GET .../devices` 须 **403**。
- [x] **库级基线**：[`scripts/internal-beta-verify.sh`](../scripts/internal-beta-verify.sh)（与内测相同）。
- [ ] **生产化加码**（本小节不勾选即视为未承诺）：公网 **HTTPS**、**`iss`/`aud` 强制**、**速率限制**、按 **`tenant_roles`** 的细粒度写权限冒烟、真实 IdP **JWKS** 长稳联调等。
  - 参考脚本：[`scripts/oidc-jwks-prod-smoke.sh`](../scripts/oidc-jwks-prod-smoke.sh)（需要你从真实 IdP 获取 5 个不同语义的 JWT：valid / a-only / bad-iss / bad-aud / platform-admin）。
  - 参考清单：`deploy/kubernetes/dmsx-api.yaml` + `deploy/kubernetes/dmsx-api-ingress.yaml` + `deploy/kubernetes/dmsx-api-secrets.example.yaml` + `deploy/kubernetes/dmsx-api-networkpolicy.yaml`（生产建议迁移到 GitOps，并将 Secret 交由 External Secrets 管理；NetworkPolicy 需按集群命名空间/标签调整）。

#### 多租户公测验证记录

| 日期 | 命令 / 动作 | 结果 |
|------|-------------|------|
| 2026-04-16 | 重编并启动 `dmsx-api`（`DMSX_API_AUTH_MODE=jwt`，PG 已应用迁移含 `004_second_tenant_seed`） | **通过**（库内存在租户 A + B） |
| 2026-04-16 | `./scripts/public-beta-multi-tenant-smoke.sh`（`DMSX_SMOKE_API=http://127.0.0.1:8080`，`DMSX_API_JWT_SECRET` 与 API 对齐） | **通过**（A/B 主链路 + 跨租户 403） |
| 2026-04-16 | `./scripts/internal-beta-verify.sh` | **通过**（`dmsx-api` 48、`dmsx-agent` 6） |
| 2026-04-16 | 新增 `scripts/oidc-jwks-prod-smoke.sh`（OIDC/JWKS 生产化冒烟脚本模板） | **已就绪**（待接入真实 IdP token 运行） |

---

## 1. 工程基础

- [x] Rust workspace 搭建（`Cargo.toml`、`Cargo.lock`）
- [x] 多 crate 拆分：`dmsx-core` / `dmsx-ai` / `dmsx-api` / `dmsx-device-gw`
- [x] 统一错误类型 `DmsxError` + RFC 7807 Problem Details
- [x] `DmsxError` 实现 Axum `IntoResponse`（feature gate `axum`）
- [x] `.gitignore` 配置
- [x] CI 流水线（`.github/workflows/ci.yml`：fmt / clippy / build / test / Docker）
- [x] `cargo check` / `cargo clippy` 零错误零警告
- [x] 工程规范与开发约定文档（`docs/ENGINEERING_STANDARDS.md`：已对齐 **同 package lib+bin 可见性**、**sqlx 嵌入迁移 / Postgres DDL 单一来源 / advisory lock 注意点**、**§13.4 本地脚本基线**、**§8.3 国际化与本地化（语言/时间/单位）**；合并前可对照该文档与 `README.md` 验证命令）
- [x] PR / Code Review 检查清单（`docs/PR_REVIEW_CHECKLIST.md`）
- [x] `dmsx-agent` 首轮模块化拆分（`config` / `api` / `telemetry` / `rustdesk` / `command_runner` / `desktop`）
- [x] `dmsx-agent` 第二轮模块化拆分（`app` / `device` / `platform`，`main.rs` 仅保留入口）
- [x] `dmsx-agent` 第三轮模块化拆分（`desktop` 细分为 `capture` / `input` / `session`，停止逻辑收口为方法）
- [x] `dmsx-agent` 远程桌面输入注入稳态增强（坐标按 `remoteWidth/remoteHeight` 缩放 + clamp；修饰键状态同步防粘住；普通按键/鼠标按钮丢释放的超时兜底；滚轮支持 `deltaX` 水平滚动并对超大 delta 归一化；超时阈值可通过 `DMSX_AGENT_DESKTOP_STUCK_KEY_TIMEOUT_SECONDS` / `DMSX_AGENT_DESKTOP_STUCK_MOUSE_TIMEOUT_SECONDS` 调优；输入通道会定期记录 active / idle 健康日志，便于排查“画面存在但长时间无输入”场景；`cargo test -p dmsx-agent --lib`、`cargo check -p dmsx-agent` 已通过）
- [x] `dmsx-agent` 首批测试用例已补充（`device` 注册/心跳 + `script` 参数分支，`cargo test -p dmsx-agent --lib` 已通过）
- [x] `dmsx-agent` `run_script` 超时语义已收紧：超过 `params.timeout` 后 Agent 会主动终止子进程并回报 `124 timeout ... process terminated`，避免控制面已判超时但设备侧脚本仍继续后台运行；已补超时测试覆盖（`cargo test -p dmsx-agent --lib`）
- [x] `dmsx-api` 轻量测试入口已建立（`lib.rs` / `app.rs` / `error.rs`，`cargo test -p dmsx-api --lib` 已通过）
- [x] `dmsx-api` handlers 纯逻辑已下沉到 `helpers` 并补测试（影子 delta / 命令结果状态，`cargo test -p dmsx-api --lib` 已通过 10 项）
- [x] `dmsx-api` desktop 纯构造逻辑已下沉到 `desktop_helpers` 并补测试（LiveKit 可用性 / start-stop desktop command payload，`cargo test -p dmsx-api --lib` 已通过 13 项）
- [x] `dmsx-api` devices / commands / shadow 已下沉到 `services`，handlers 收敛为薄层（`cargo test -p dmsx-api --lib`、`cargo check -p dmsx-api` 已通过）
- [x] `dmsx-api` `db.rs` 首轮拆分为 repo 模块（`repo/devices.rs` / `repo/commands.rs` / `repo/shadow.rs` / `repo/audit.rs`，`cargo test -p dmsx-api --lib`、`cargo check -p dmsx-api` 已通过）
- [x] `dmsx-api` `policies` 已补齐 `repo/service` 分层（`repo/policies.rs` + `services/policies.rs`，handlers 进一步收敛，`cargo test -p dmsx-api --lib`、`cargo check -p dmsx-api` 已通过）
- [x] `dmsx-api` `artifacts` / `compliance` / `stats` / `tenant seed` 已完成 `repo/service` 收口（`repo/artifacts.rs` / `repo/compliance.rs` / `repo/stats.rs` / `repo/tenants.rs` + 对应 services，`app.rs` 启动 seed 下沉，`cargo test -p dmsx-api --lib`、`cargo check -p dmsx-api` 已通过）
- [~] 集成测试框架（`dmsx-agent --lib` 已有实际用例；`dmsx-api` 已补 `build_router()` 路由级 smoke tests：health / livekit config / auth reject / tenant mismatch，workspace / 全量二进制链路待继续扩展）
- [x] `dmsx-api` 指标端点（`GET /metrics`，Prometheus 文本格式；支持 `DMSX_API_METRICS_ENABLED` 关闭与 `DMSX_API_METRICS_BEARER` 固定 Bearer）— `cargo test -p dmsx-api` 通过
  - 验证：`DMSX_API_METRICS_ENABLED=false` 时 `GET /metrics` 返回 404；设置 `DMSX_API_METRICS_BEARER` 后缺失/错误 token 返回 401，正确 token 返回 200（见 `app.rs` 单测）
- [x] 横切面：`X-Request-Id` 透传/生成并在响应返回；全局请求超时（`DMSX_API_REQUEST_TIMEOUT_SECONDS`）可配置；并发上限（`DMSX_API_CONCURRENCY_LIMIT_*`）可选启用
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

- [x] **Postgres DDL 单一来源**：由 `dmsx-api` 启动时 `sqlx`（`crates/dmsx-api/src/migrate_embedded.rs`）应用 `migrations/*.sql`；`deploy/docker-compose.yml` **勿**将同一套 Postgres SQL 再挂到 `docker-entrypoint-initdb.d`（与 `ENGINEERING_STANDARDS.md` §7.3 一致；ClickHouse 的 `migrations/ch` initdb 挂载另计）

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
- [x] RLS（Row Level Security）策略（迁移 `migrations/005_rls_tenant_isolation.sql`；`dmsx-api` 在每条写读业务 SQL 上使用 `BEGIN` + `set_config(..., true)` 绑定 `dmsx.tenant_id` / `dmsx.is_platform_admin`，与连接池复用兼容。**验证**：`cargo test -p dmsx-api`；有 Postgres 且已跑迁移时 `DMSX_TEST_DATABASE_URL=... cargo test -p dmsx-api --test rls_tenant_session`）
- [~] 按 `tenant_id` HASH 分区（**`commands`**：`migrations/006_commands_hash_partition.sql`，`PARTITION BY HASH (tenant_id)` 共 8 分区；`PRIMARY KEY (tenant_id, id)`；`command_results` 外键改为 `(tenant_id, command_id) → (tenant_id, id)`。迁移中会先把 `commands` 旧索引重命名为 `idx_commands_legacy_*`，避免 clean bootstrap 时与新分区表索引重名冲突。其余表顺序见 [`POSTGRES_TENANT_PARTITIONING_PLAN.md`](POSTGRES_TENANT_PARTITIONING_PLAN.md)。**验证**：修改迁移后 `cargo build -p dmsx-api` 再启动以嵌入 `sqlx::migrate!`；库上 `\d commands` 应显示 `Partitioned table`）
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
- [x] `POST /v1/tenants` 创建租户（sqlx；**PlatformAdmin**；`disabled` 开发模式不校验）
- [x] `POST /v1/tenants/{tid}/orgs` 创建组织（sqlx + 审计）
- [x] `POST /v1/tenants/{tid}/orgs/{oid}/sites` 创建站点（sqlx，校验 org 归属租户）
- [x] `POST /v1/tenants/{tid}/sites/{sid}/groups` 创建设备组（sqlx，校验 site 归属租户）
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
- [x] `POST /v1/tenants/{tid}/commands/{cid}/evidence-upload-token` 命令证据上传 token 签发
- [x] `GET /v1/tenants/{tid}/devices/{did}/shadow` 获取设备影子（含 delta 计算）
- [x] `PATCH /v1/tenants/{tid}/devices/{did}/shadow/desired` 更新期望状态
- [x] `POST /v1/tenants/{tid}/devices/{did}/actions` 设备远控操作下发
- [x] `GET /v1/tenants/{tid}/devices/{did}/commands` 设备命令历史
- [x] `GET/POST /v1/tenants/{tid}/artifacts` 制品列表/创建（sqlx 持久化）
- [x] `GET /v1/tenants/{tid}/compliance/findings` 合规发现列表（sqlx 持久化）
- [x] `GET /v1/tenants/{tid}/stats` Dashboard 聚合统计（sqlx）
- [x] `POST /v1/tenants/{tid}/devices/{did}/desktop/session` 创建远程桌面会话（LiveKit Token + Agent `start_desktop`；**2026-04-20** 已在本机对 `dmsx-api` + LiveKit + Redis + 真实 `dmsx-agent` 做最小闭环验证；Agent 现仅在 **屏幕轨发布成功且输入注入器初始化成功** 后才回报 ready，避免“能看屏但无法输入”仍被判成功）
- [x] `DELETE /v1/tenants/{tid}/devices/{did}/desktop/session?session_id=` 终止指定远程桌面会话（`stop_desktop`）
- [x] `GET /v1/config/livekit` LiveKit 配置查询
- [~] `POST /v1/tenants/{tid}/ai/anomalies` AI 异常检测（规则引擎 stub）
- [~] `POST /v1/tenants/{tid}/ai/recommendations` AI 策略推荐（stub）
- [~] `POST /v1/tenants/{tid}/ai/chat` AI 智能助手（stub，待接 LLM）
- [~] `POST /v1/tenants/{tid}/ai/predictions` AI 预测性维护（stub）

### 中间件与横切

- [x] 认证中间件骨架（`auth` 模块 + Bearer/JWT 解析 + `/health` 放行 + 可配置 `DMSX_API_AUTH_MODE`）
- [x] 监听地址可配置（`DMSX_API_BIND` 环境变量）
- [x] `TraceLayer` 日志追踪
- [~] JWT / OIDC 认证实现（JWT `issuer` / `audience` 校验已支持；可选 **`allowed_tenant_ids`** 与 **`tenant_id`** 并集作为路径租户白名单；可选 **`tenant_roles`** 按活动租户覆盖 RBAC（无键回退 **`roles`**）；OIDC discovery -> `jwks_uri` 加载、JWKS 校验、后台 TTL 刷新、未知 `kid` 强制刷新、刷新失败 stale fallback、最大陈旧窗口、启动首刷失败可配置策略已接入；`/ready` 已暴露认证/JWKS 就绪状态；外部 IdP 实机联调与告警/指标后端集成待补）
- [x] RBAC 权限校验（已细化到资源级：全局配置 / devices / policies / commands / shadow / artifacts / compliance / desktop / AI；`TenantAdmin` 与 `PlatformAdmin` 非租户路由权限已区分）
- [x] 路径 `{tenant_id}` 与 JWT 许可集合及 RBAC（`tenant_id` ∪ `allowed_tenant_ids`；`tenant_roles` 按活动租户覆盖 `roles`；见 [`API.md`](API.md)）
- [x] 速率限制（per-tenant：可通过 `DMSX_API_RATE_LIMIT_ENABLED` / `DMSX_API_RATE_LIMIT_PER_SECOND` / `DMSX_API_RATE_LIMIT_BURST` 配置，超限返回 429 ProblemDetails）
- [x] 请求体大小限制（`DMSX_API_REQUEST_BODY_LIMIT_BYTES`，超限返回 413 ProblemDetails）
- [x] CORS 生产配置（按 `DMSX_API_CORS_ALLOWED_ORIGINS`/`DMSX_API_CORS_ALLOW_ALL` 配置 `tower-http CorsLayer`；非 `dev` 且未配置来源将拒绝跨域）

### 持久化

- [x] sqlx 连接池接入（Postgres, PgPoolOptions, 自动迁移）
- [x] 真实 CRUD 替换 stub（设备/策略/命令/制品/合规/统计）
- [x] AppState 注入 + 环境变量配置（`DATABASE_URL` / `LIVEKIT_URL` / `LIVEKIT_API_KEY` / `LIVEKIT_API_SECRET`）
- [x] 启动时自动运行 sqlx migrations
- [x] CORS 中间件（`tower-http CorsLayer`）
- [~] ClickHouse 客户端接入（审计写入 `audit_events`：当配置 `DMSX_CLICKHOUSE_HTTP_URL` 时每次 `audit_logs` 写入会异步写入；遥测/心跳/命令回执明细待补）
- [~] Redis 接入（当前用于桌面会话映射持久化：`session_id → {tenant_id, device_id}` + `device_id → session_id`；缓存/在线状态/分布式锁待扩展）
- [~] NATS JetStream 接入（**API**：命令在 Postgres 提交成功后异步发布 `dmsx.command.{tenant_id}.{device_id}`；**回执 ingest**：消费 `dmsx.command.result.>`（durable consumer，默认名见 `DMSX_NATS_RESULT_CONSUMER`）并与 HTTP `submit_command_result` 对齐写库，**以消息 `status` 为准**更新命令状态，跨租户伪造消息 **TERM**；**网关**：`StreamCommands` pull 过滤 `dmsx.command.{tenant_id}.{device_id}`；`ReportResult` 发布 `dmsx.command.result.{tenant_id}.{device_id}`；**验证**：compose 起 NATS 后配置 `DMSX_NATS_URL`，`cargo check -p dmsx-api -p dmsx-device-gw`，联调 `nats sub 'dmsx.command.>'` / `nats sub 'dmsx.command.result.>'`）
- [ ] S3 / MinIO 接入（制品上传预签名）

---

## 5. 数据面 gRPC（`dmsx-device-gw`）

### RPC

- [~] `Enroll` — 内测 CA 签发证书（HMAC enrollment token + **固定 device_id** + CSR；SAN `urn:dmsx:tenant:{tid}:device:{did}`）；启用 `DMSX_GW_TLS_CLIENT_CA` 且配置 Enroll HMAC/CA 时，允许匿名新设备仅调用 `Enroll`；生产级 CA 集成/吊销待补
- [x] `Heartbeat` — 返回服务器时间
- [~] `FetchDesiredState` — 返回空策略（stub；已做 mTLS device_id 校验）
- [~] `StreamCommands` — JetStream durable pull，`filter_subject=dmsx.command.{tenant_id}.{device_id}`；支持稳定 consumer + `cursor`（stream sequence）恢复；同设备**单活跃流**、**单条在途命令**，以 `ReportResult(command_id)` 发布成功作为 ACK 提交点；mTLS SAN 与 RPC 对齐（见 `docs/DEPLOYMENT.md`）；已补 `scripts/internal-beta-data-plane-e2e.sh` 做最小闭环联调；更完整跨实例恢复语义待补
- [x] `ReportResult` — 发布 JetStream `dmsx.command.result.{tenant_id}.{device_id}`（无 NATS 时 `accepted=false`）
- [x] `UploadEvidence` — 消费流 + 256 MiB 限制 + 首包 `device_id` mTLS / `upload_token` 绑定校验；已支持写入 S3 / MinIO 兼容对象存储（`DMSX_GW_EVIDENCE_S3_*`）；控制面已接 `POST /v1/tenants/{tid}/commands/{cid}/evidence-upload-token` 签发入口；**验证**：`DMSX_E2E_WITH_EVIDENCE=1 ./scripts/internal-beta-data-plane-e2e.sh`（依赖可写桶 + 与 GW 一致的 upload token secret；默认不传此变量则不跑对象存储段）
- [x] gRPC Health Check（`grpc.health.v1.Health`）

### 基础设施

- [x] 监听地址可配置（`DMSX_GW_BIND` 环境变量）
- [x] mTLS 启用（`DMSX_GW_TLS_CERT`/`KEY`/`CLIENT_CA` + tonic `ServerTlsConfig`；可选 `DMSX_GW_TLS_CLIENT_AUTH_OPTIONAL`）
- [~] 连接限流 / 背压（已接 `concurrency_limit_per_connection`：`DMSX_GW_CONCURRENCY_PER_CONNECTION`；更细粒度背压/速率限制待补）
- [~] 按租户速率限制（`DMSX_GW_RATE_LIMIT_*`，内存内 keyed limiter；多副本按 Pod 分摊）
- [x] 设备身份校验（客户端证书 SAN `urn:dmsx:tenant:{uuid}:device:{uuid}` 与 RPC `device_id`/`tenant_id` 交叉校验）
- [~] NATS JetStream 接入（命令 + 回执 subject 已贯通；观测/重放策略与其它 RPC 待补）
- [~] 指标 / 追踪（已提供 Prometheus `/metrics`：`DMSX_GW_METRICS_*`；OpenTelemetry 追踪注入待补）

---

## 6. Proto 契约

- [x] `proto/dmsx/agent.proto` — Agent 服务定义（6 RPC）
- [x] `CommandStatusProto` / `DevicePlatformProto` 枚举（与 `dmsx-core` 对齐）
- [x] 多租户隔离约定注释（`tenant_id` 可选字段 + mTLS SAN `urn:dmsx:tenant:…:device:…` 说明）
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
- [x] 暗色模式（AntD theme darkAlgorithm + 顶部切换；主题偏好落盘 `localStorage`；先覆盖 AppLayout 顶栏/内容容器/侧栏品牌与菜单外观）
- [x] 国际化（i18n）（简版：提供 `AppProviders` + `t()`；顶部语言切换影响导航/面包屑/用户菜单与 Dashboard 主标题；未覆盖的文案默认回退到中文 key）
- [x] 前端会话上下文（`AppProviders` 挂载到实际入口；活动租户落盘 `localStorage['dmsx.tenant_id']`，JWT 落盘 `localStorage['dmsx.jwt']`，顶栏可直接切换）

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
- [x] 远程桌面面板（RemoteDesktop — LiveKit WebRTC 订阅 + Data Channel 键鼠 + 状态反馈 / 重连 + RustDesk 备选）
- [x] 策略详情抽屉（完整信息 + 作用域字段）
- [x] 命令详情抽屉（payload JSON 高亮、状态标签、目标设备信息 + 执行结果展示 exit_code/stdout/stderr）
- [x] 系统设置页面（已接入后端：`GET/PUT /v1/config/settings/{key}`；前端支持在 UI 中配置并注入 `Authorization`）
- [x] 策略编辑器（已接入后端保存：`POST /v1/tenants/{tid}/policies/editor`；前端支持在 UI 中配置活动租户与 `Authorization`）
- [x] 审计日志查看页（已接入后端：`GET /v1/tenants/{tid}/audit-logs`；前端支持在 UI 中配置活动租户与 `Authorization`）
- [~] 用户 / 角色管理页（UI 框架 + 表格渲染；后端仅提供 RBAC 角色清单：`GET /v1/config/rbac/roles`；用户/角色管理（CRUD）仍未接入）

---

## 9. OpenAPI 契约（`openapi/dmsx-control-plane.yaml`）

- [x] OpenAPI `paths` 与 `dmsx-api` 已注册路由对齐（已移除未实现的租户/组织/站点/组 POST 占位路径）
- [x] `GET /v1/config/livekit`、`POST/DELETE .../devices/{did}/desktop/session`（与当前远程桌面主链路一致）
- [x] `GET /v1/tenants/{tid}/audit-logs`、`POST /v1/tenants/{tid}/policies/editor`、`GET/PUT /v1/config/settings/{key}`、`GET /v1/config/rbac/roles` 已写入 OpenAPI
- [x] `ProblemDetails` 错误 schema
- [x] `CommandCreate` / `ListResponseDevice` / `Device` / `Command` / `ShadowResponse` / `Policy` / `PolicyRevision` / `ListResponsePolicy` 等核心 schema（OpenAPI）
- [x] 设备影子、远控动作、设备/租户命令列表、命令状态与结果、统计、策略 CRUD + revision、AI 请求体等已写入 OpenAPI
- [x] 制品列表/创建、合规发现列表已在 OpenAPI 中强类型化（`Artifact` / `ComplianceFinding` 及分页列表）
- [x] OpenAPI 全局 **`bearerAuth`**（`securitySchemes` + 根级 `security`）；`/health`、`/ready` 使用 `security: []`
- [x] `components.responses` 提供 **401 / 403 / 404 / 400 / 409 / 500**（`ProblemDetails`）复用定义；带 `requestBody` 的操作已挂 **400**，部分创建类 POST 已挂 **409**，各操作已挂 **500**（`InternalServerError`）
- [x] 各 `paths` 操作已批量声明 `401` / `403`；`components.responses.NotFound` 及典型 `404`（设备/影子/策略/命令/桌面会话等）已对齐
- [ ] 按需补充 `oauth2` 与更细错误码文档

---

## 10. 安全

- [x] 安全设计文档（`docs/SECURITY.md`）
- [x] 设备 mTLS 架构设计
- [x] RBAC 角色与范围设计
- [x] 审计不可篡改设计（PG + CH + 对象存储）
- [x] 制品签名设计（cosign / sigstore）
- [~] 认证实现（JWT `issuer` / `audience` 校验已支持；OIDC discovery -> `jwks_uri` 加载、JWKS 校验、后台 TTL 刷新与未知 `kid` 强制刷新已接入；外部 IdP 实机联调与更完整轮转/失效策略待补）
- [x] RBAC 中间件实现（按活动租户解析 **`roles`**：`tenant_roles` 有键则用该数组，否则用令牌级 `roles`；资源级路由权限；缺失角色、越权写策略、越权访问全局配置均返回 `403`）
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
- [x] Namespace / RBAC 清单（Namespace：`deploy/kubernetes/namespace-dmsx.yaml`；RBAC：`deploy/kubernetes/dmsx-rbac.yaml`）
- [x] ConfigMap / Secret 示例（ConfigMap：`deploy/kubernetes/dmsx-configmap.example.yaml`；Secret：`deploy/kubernetes/dmsx-api-secrets.example.yaml`）
- [x] Ingress + TLS 清单（`deploy/kubernetes/dmsx-api-ingress.yaml`）
- [x] HPA 自动伸缩配置（`deploy/kubernetes/dmsx-hpa.yaml`）
- [x] PDB 配置（`deploy/kubernetes/dmsx-pdb.yaml`）

### 可观测性

- [x] OTel Collector 配置（`deploy/otel-collector-config.yaml`）
- [x] 应用侧 OpenTelemetry SDK 集成（`dmsx-api`：设置 `OTEL_EXPORTER_OTLP_ENDPOINT` 即导出 traces；未设置则仅本地日志）
- [x] 应用侧 OpenTelemetry SDK 集成（`dmsx-device-gw`：设置 `OTEL_EXPORTER_OTLP_ENDPOINT` 即导出 traces；未设置则仅本地日志）
- [x] Prometheus ServiceMonitor（模板：`deploy/kubernetes/monitoring/dmsx-api-servicemonitor.yaml`）
- [x] Grafana 仪表盘模板（模板：`deploy/kubernetes/monitoring/dmsx-api-grafana-dashboard.yaml`）
- [x] 告警规则（PrometheusRule，模板：`deploy/kubernetes/monitoring/dmsx-api-prometheusrule.yaml`）

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
| [x] 已完成 | 140 |
| [~] 骨架/Stub | 12 |
| [ ] 未开始 | 48 |

> 上表仅统计 **§1 起各功能小节** 中带状态标记的条目；**「内测阶段目标与完成定义」** 中的 DoD 为过程自查项，**未计入**上表「未开始」数量。其中 **「自动化基线绿灯」** 已勾选并附验证记录，亦不纳入上表「已完成」计数。

> 最后更新：2026-04-20（**数据面 e2e**：`DMSX_E2E_WITH_EVIDENCE=1` 覆盖 evidence-upload-token → `UploadEvidence` → `ReportResult` → 控制面 `evidence_key`；其余见前文）
