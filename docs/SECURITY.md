# 安全设计

## 设备身份与接入

| 阶段 | 机制 | 说明 |
|------|------|------|
| 注册 | **Enrollment token**（短生命周期、一次性或限量） | 可扫码、批量 CSV、IT 工单系统集成 |
| 常态 | **mTLS** | 每设备独立客户端证书；私钥存 TPM/Secure Enclave（平台相关） |
| 轮换 | **证书 TTL + 自动续期** | Agent 在过期前调用 `Enroll` 子协议或专用 `RotateCertificate`（可后续扩展） |
| 吊销 | **吊销列表 + 连接拒止** | 网关校验 `device_id` + 证书序列号；PG `enroll_status = revoked/blocked` |

## 控制面身份与权限

- **AuthN**：OIDC（Keycloak/Entra ID/Okta 等）；服务账号用 **JWT + mTLS**（双向约束）。
- **AuthZ**：**RBAC** 角色示例：`PlatformAdmin`, `TenantAdmin`, `SiteAdmin`, `Operator`, `Auditor`, `ReadOnly`。
- **范围**：权限绑定在 `tenant_id` + 可选 `site_id` / `group_id`；API 层校验 **URL 路径中的 `{tenant_id}` 必须落在 JWT 允许的租户集合内**（`tenant_id` ∪ `allowed_tenant_ids`）。**RBAC**：可选 **`tenant_roles`** 按活动租户覆盖角色；无键时回退令牌级 **`roles`**。切换租户即换 URL 前缀；签发方维护 `allowed_tenant_ids` 与 `tenant_roles`。（完整约定见 [`API.md`](API.md)。）
- **Postgres RLS（纵深防御）**：租户域表启用 RLS，策略依赖会话变量 `dmsx.tenant_id`（UUID 文本）与 `dmsx.is_platform_admin`（`true`/`false`）。`dmsx-api` 在业务事务内使用 `set_config(..., true)` 做**事务级**绑定，避免连接池复用导致会话串味；`PlatformAdmin` 令牌对应 `dmsx.is_platform_admin=true` 以访问跨租户运维路径。**开发注意**：`DMSX_API_AUTH_MODE=disabled` 时中间件会注入**合成** `AuthContext`（路径含 `{tenant_id}` 时为该租户 `TenantAdmin`，否则为 `PlatformAdmin`），仅用于本地/内网，生产务必启用 JWT/OIDC。
- **OIDC/JWKS 强制 `iss`/`aud`**：当使用 `DMSX_API_OIDC_DISCOVERY_URL` 或 `DMSX_API_JWKS_URL` 验签时，服务端要求同时配置 `DMSX_API_JWT_ISSUER` 与 `DMSX_API_JWT_AUDIENCE` 并校验令牌 `iss`/`aud`；避免接受来自非预期签发方/受众但签名有效的 JWT。
- **ABAC**：`devices.labels` + 策略 `scope_expr`；评估引擎在 PolicyService（后续实现）。

## 通信安全

- 公网：**TLS 1.3**，仅现代套件；**HSTS**。
- 东西向：服务网格 **mTLS**（Istio/Linkerd）或自签 SPIFFE/SVID。
- gRPC：禁止明文；设备网关独立 LB，**按租户速率限制**。

## 审计与不可篡改

- 所有管理 API 写操作产生 **AuditLog**（Postgres 权威）。
- 异步写入 **ClickHouse** 流水表；定期 **对象存储归档**（S3 Object Lock / WORM 桶策略）。
- 可选：**哈希链**（每条审计记录含 `prev_hash`）或 **签名批次**（每日 Merkle root 上链/离线签）。

## 制品与供应链

- **SHA-256** 必填；**签名**（Sigstore/cosign 或自建 KMS 签名）存 `signature_b64`。
- 下载：**预签名 URL + 短 TTL**；Agent 校验哈希与签名后再执行。
- 进阶：**SBOM**（CycloneDX）存 `artifacts.metadata`。

## 密钥管理

- 运行态：**Kubernetes Secrets + External Secrets Operator** 同步 Vault/KMS。
- 数据加密：**PG TDE/磁盘加密**（云厂商）；敏感字段 **应用层信封加密**（KMS 数据密钥）。

## 威胁建模要点（摘要）

- 被盗设备证书 → 吊销 + 设备隔离命令 + 会话失效。
- 内部滥用 → 审计 + 双人审批（高危命令，后续工作流）。
- 供应链投毒 → 签名 + 渠道 + 灰度 + 自动回滚（策略 spec 驱动）。
