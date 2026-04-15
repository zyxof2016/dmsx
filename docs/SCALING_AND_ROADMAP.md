# 容量估算、背压与路线图

## 容量估算方法

记：

- \(N\)：在线设备数
- \(f_h\)：心跳频率（次/秒/设备），例如 1/60
- \(f_c\)：人均命令下发速率（条/秒）
- \(S_r\)：平均回执大小（字节）

**心跳写入（ClickHouse）**

\[
Q_{heartbeat} \approx N \times f_h
\]

**命令与回执（消息总线 + CH）**

\[
Q_{cmd} \approx \lambda_{commands\_in} + N \times p_{exec}
\]

其中 \(p_{exec}\) 为设备侧执行完成产生回执的速率。JetStream / Kafka 分区数按峰值 \(Q_{cmd}\) 与单分区上限（经验 5–15MB/s）估算。

**网关连接**

- 每连接内存：order(数十 KB)（含 buffer）；\(N\) 百万级时需 **多副本 + 四层负载均衡 + 连接粘性（可选）**。

**Postgres**

- 热路径：设备行更新（`last_seen_at`）易成热点 → 使用 **批量合并写入**（秒级聚合）或 **先写 Redis 再刷 PG**。

## 背压与离线

| 场景 | 策略 |
|------|------|
| 网关过载 | 连接限流、租户级 quota、gRPC `RESOURCE_EXHAUSTED` |
| 消息堆积 | JetStream 保留策略 + 消费者 lag 告警；命令 TTL 到期标记 `expired` |
| 设备离线 | Agent 磁盘队列；上线后按 `cursor` 重放 `StreamCommands` |
| 大文件证据 | 分块上传 `UploadEvidence`；直传 S3 multipart |

## 幂等与一致性

- **命令**：部分唯一索引 `UNIQUE (tenant_id, idempotency_key) WHERE idempotency_key IS NOT NULL`——仅当提供幂等键时约束唯一，无键命令可重复创建。网关/消费者重复投递不产生副作用。
- **策略**：`policy_revisions` 只追加；设备侧记录 `last_policy_revision_id`。
- **回执**：`command_id + device_id + attempt` 去重（CH ReplacingMergeTree 或 PG UPSERT）。

## 分阶段路线图（与计划一致）

| 阶段 | 时长（参考） | 交付 |
|------|----------------|------|
| Phase0 | 2–4 周 | 租户/注册、长连接、心跳、命令闭环、基础审计 |
| Phase1 | 4–8 周 | 声明式策略 + Reconcile、分组标签、制品仓库与灰度 |
| Phase2 | 8–12 周 | 合规基线、补丁/漏洞对接、证书轮换/吊销、SIEM/EDR Webhook |
| Phase3 | 持续 | 网络编排增强、工作流审批、SBOM、多区域灾备 |

## 风险与缓解

- **热点行**：心跳合并、异步刷盘、CH 承担明细。
- **租户噪声**：租户级隔离（连接、Topic、配额）。
- **策略错误**：灰度 + 自动回滚（`rollout` 元数据）+ 只读审计。
