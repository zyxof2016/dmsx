## 内测范围与反馈确认（Internal Beta）

本文件用于在进入内测前，明确：

- **本轮内测验收范围（Scope）**
- **不在本轮内测门禁内的延后项（Out of scope / Deferred）**
- **反馈与支持渠道（Feedback）**

> 维护要求：任何会影响内测参与者操作方式、环境变量、端点契约、权限要求的变更，都必须同步更新本文件与 `docs/CHECKLIST.md` 的 DoD 勾选状态。

### 1) 本轮内测目标（Scope）

- **控制面可用**：`dmsx-api` 基础路由可用（健康/就绪、设备、命令、影子、策略等主链路）。
- **Agent 主链路闭环**：注册设备 → 心跳 → 命令执行与结果回传（至少可重复演示 2 轮）。
- **默认认证模式**：内测环境通常使用 `DMSX_API_AUTH_MODE=disabled` 完成冒烟；若启用 `jwt`，须按 `docs/API.md` 中的 JWT 约定（`tenant_id` / `allowed_tenant_ids` / `roles` / `tenant_roles`）执行。
- **横切限制（可选）**：
  - 请求体大小限制：`DMSX_API_REQUEST_BODY_LIMIT_BYTES`（超限 413 ProblemDetails）
  - per-tenant 速率限制：`DMSX_API_RATE_LIMIT_ENABLED` / `DMSX_API_RATE_LIMIT_PER_SECOND` / `DMSX_API_RATE_LIMIT_BURST`（超限 429 ProblemDetails）

### 2) 本轮内测不验收（Out of scope）

- **远程桌面**：不验收 LiveKit 联调、不要求管理台远控面板与 Agent 桌面会话闭环（仓库可保留相关代码与 compose 依赖，但不作为门禁）。
- **公网生产化**：HTTPS、Ingress/HPA/PDB、完整告警规则、OTel SDK 全量接入等。
- **数据面生产化**：RLS、按租户分区、Redis/NATS/ClickHouse/S3 等完整落地（以 `docs/CHECKLIST.md` 为准）。

### 3) 环境与验收方式（How to verify）

- 参考 `docs/CHECKLIST.md` 开篇「内测 DoD」与以下脚本：
  - `scripts/reproduce-dev-env.sh`
  - `scripts/internal-beta-smoke-http.sh`
  - `scripts/internal-beta-verify.sh`

### 4) 反馈与支持（Feedback）

请在开始内测前由项目维护者填写以下信息（保持可复制粘贴）：

- **负责人 / On-call**：`<TODO: name>`
- **反馈渠道**：
  - Issue：`<TODO: repo/issues link 或规范>`
  - 群 / 邮箱：`<TODO: contact>`
- **反馈模板（建议）**：
  - 期望行为 vs 实际行为
  - 复现步骤（含命令/脚本、环境变量）
  - `dmsx-api` / `dmsx-agent` 版本（commit 或 tag）
  - 关键日志（脱敏）

