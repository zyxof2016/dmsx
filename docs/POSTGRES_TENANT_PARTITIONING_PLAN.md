## Postgres 按 tenant_id 分区（HASH）演进方案

目标：在不破坏内测节奏的前提下，把“热点表”的数据物理隔离为按 `tenant_id` 的 HASH 分区，为后续规模化与运维（vacuum/冷热/迁移）打基础。

> 重要：分区属于“结构性变更”，应以**小步迁移**方式推进；不要在一次迁移中对所有表同时重建。

### 1) 适合优先分区的表（建议顺序）

1. `commands`（写入高、查询按租户+时间/状态多）
2. `devices`（更新频繁但行较小；热点风险更偏“单租户热点”）
3. `audit_logs` / `compliance_findings`（数据增长快，分区利于归档与 vacuum）
4. `device_shadows` / `command_results`（体量取决于设备与命令规模）

### 2) 推荐分区键与分区数

- 分区键：`tenant_id`
- 分区类型：`PARTITION BY HASH (tenant_id)`
- 初始分区数：
  - 内测/小规模：8 或 16
  - 预计中规模：32
  - 可扩展：后续通过“重分区迁移”扩大（需要新表 + 回填 + 切换）

### 3) 迁移策略（小步、安全）

对每个目标表，采用以下步骤（示例以 `commands` 为例）：

1. 新建分区父表 `commands_p`（结构一致，含约束/索引）
2. 创建 N 个分区 `commands_p_0..commands_p_{N-1}`
3. 创建触发器或双写（可选）以支持迁移窗口
4. 批量回填数据（按 tenant 或按时间切片）
5. 校验行数与关键约束
6. 窗口期切换（rename + view 或直接替换）
7. 清理旧表

### 4) 与 RLS 的关系

- 分区并不替代 RLS；分区解决的是**物理布局与运维**，RLS解决的是**越权隔离**。
- 建议顺序：先把 **RLS+应用连接上下文**跑通，再分区；否则排障维度会叠加。

### 5) 回滚策略

- 任何“切换”步骤必须可逆：保留旧表只读副本一段时间；必要时可回切。
- 迁移脚本要支持幂等（`IF EXISTS` / `IF NOT EXISTS`）并写明危险步骤。

### 6) 已落地（仓库状态）

| 迁移 | 表 | 说明 |
|------|-----|------|
| [`migrations/006_commands_hash_partition.sql`](../migrations/006_commands_hash_partition.sql) | `commands` | `PARTITION BY HASH (tenant_id)`，`MODULUS 8`；主键 `(tenant_id, id)`（分区表要求）；`command_results` 外键改为复合引用 `commands(tenant_id, id)`；迁移后重建 `rls_commands_tenant`。与 RLS 并存。 |

后续表（`devices`、`audit_logs` 等）仍按 §1 顺序单独开迁移，避免单次停机面过大。

