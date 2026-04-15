-- ClickHouse 初始化：审计、心跳、命令回执明细
-- 以 MergeTree 家族为主；ReplacingMergeTree 用于可重放/去重场景

-- 审计流（从 Postgres 异步复制或直写）
CREATE TABLE IF NOT EXISTS audit_events
(
    id              UUID,
    tenant_id       UUID,
    actor_user_id   Nullable(UUID),
    action          LowCardinality(String),
    resource_type   LowCardinality(String),
    resource_id     String,
    payload         String,  -- JSON 字符串
    created_at      DateTime64(3, 'UTC')
)
ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(created_at)
ORDER BY (tenant_id, created_at, id)
TTL toDateTime(created_at) + INTERVAL 365 DAY;

-- 设备心跳
CREATE TABLE IF NOT EXISTS device_heartbeats
(
    device_id       UUID,
    tenant_id       UUID,
    agent_version   LowCardinality(String),
    telemetry       String,  -- JSON map
    received_at     DateTime64(3, 'UTC') DEFAULT now64(3)
)
ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(received_at)
ORDER BY (tenant_id, device_id, received_at)
TTL toDateTime(received_at) + INTERVAL 90 DAY;

-- 命令回执明细（ReplacingMergeTree 按 updated_at 去重）
CREATE TABLE IF NOT EXISTS command_results
(
    command_id      UUID,
    device_id       UUID,
    tenant_id       UUID,
    status          LowCardinality(String),
    exit_code       Int32,
    stdout_snippet  String,
    stderr_snippet  String,
    evidence_key    Nullable(String),
    reported_at     DateTime64(3, 'UTC') DEFAULT now64(3),
    updated_at      DateTime64(3, 'UTC') DEFAULT now64(3)
)
ENGINE = ReplacingMergeTree(updated_at)
PARTITION BY toYYYYMMDD(reported_at)
ORDER BY (tenant_id, command_id, device_id);

-- 策略漂移事件
CREATE TABLE IF NOT EXISTS policy_drift_events
(
    device_id       UUID,
    tenant_id       UUID,
    policy_id       UUID,
    revision_id     UUID,
    expected_hash   String,
    actual_hash     String,
    detected_at     DateTime64(3, 'UTC') DEFAULT now64(3)
)
ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(detected_at)
ORDER BY (tenant_id, device_id, detected_at)
TTL toDateTime(detected_at) + INTERVAL 180 DAY;
