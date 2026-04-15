-- DMSX 初始 schema：多租户行级隔离（tenant_id 必填 + 索引）
-- 生产环境建议：按 tenant_id HASH 分区（PostgreSQL 声明式分区）、RLS 策略

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- 资源层级
CREATE TABLE tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE orgs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, name)
);
CREATE INDEX idx_orgs_tenant ON orgs (tenant_id);

CREATE TABLE sites (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
    org_id UUID NOT NULL REFERENCES orgs (id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, org_id, name)
);
CREATE INDEX idx_sites_tenant ON sites (tenant_id);

CREATE TABLE groups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
    site_id UUID NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, site_id, name)
);
CREATE INDEX idx_groups_tenant ON groups (tenant_id);

-- 设备
CREATE TYPE device_platform AS ENUM (
    'windows', 'linux', 'macos', 'ios', 'android', 'edge', 'other'
);
CREATE TYPE enroll_status AS ENUM ('pending', 'active', 'revoked', 'blocked');
CREATE TYPE online_state AS ENUM ('unknown', 'online', 'offline');

CREATE TABLE devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
    site_id UUID REFERENCES sites (id) ON DELETE SET NULL,
    primary_group_id UUID REFERENCES groups (id) ON DELETE SET NULL,
    platform device_platform NOT NULL DEFAULT 'other',
    hostname TEXT,
    os_version TEXT,
    agent_version TEXT,
    enroll_status enroll_status NOT NULL DEFAULT 'pending',
    online_state online_state NOT NULL DEFAULT 'unknown',
    last_seen_at TIMESTAMPTZ,
    labels JSONB NOT NULL DEFAULT '{}',
    capabilities JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_devices_tenant ON devices (tenant_id);
CREATE INDEX idx_devices_tenant_labels ON devices USING gin (labels jsonb_path_ops);

-- 策略与不可变版本
CREATE TYPE policy_scope_kind AS ENUM (
    'tenant', 'org', 'site', 'group', 'label'
);

CREATE TABLE policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    scope_kind policy_scope_kind NOT NULL,
    scope_org_id UUID REFERENCES orgs (id) ON DELETE CASCADE,
    scope_site_id UUID REFERENCES sites (id) ON DELETE CASCADE,
    scope_group_id UUID REFERENCES groups (id) ON DELETE CASCADE,
    scope_expr TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, name)
);
CREATE INDEX idx_policies_tenant ON policies (tenant_id);

CREATE TABLE policy_revisions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
    policy_id UUID NOT NULL REFERENCES policies (id) ON DELETE CASCADE,
    version INT NOT NULL,
    spec JSONB NOT NULL,
    rollout JSONB NOT NULL DEFAULT '{}',
    published_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    published_by UUID,
    UNIQUE (policy_id, version)
);
CREATE INDEX idx_policy_revisions_tenant ON policy_revisions (tenant_id);

-- 命令（幂等键 + 状态机）
CREATE TYPE command_status AS ENUM (
    'queued', 'delivered', 'acked', 'running', 'succeeded', 'failed', 'expired', 'cancelled'
);

CREATE TABLE commands (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
    idempotency_key TEXT,
    target_device_id UUID NOT NULL REFERENCES devices (id) ON DELETE CASCADE,
    payload JSONB NOT NULL,
    priority SMALLINT NOT NULL DEFAULT 0,
    ttl_seconds INT NOT NULL DEFAULT 3600,
    status command_status NOT NULL DEFAULT 'queued',
    created_by UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- 部分唯一索引：仅当 idempotency_key 非空时约束唯一——NULL 不参与 PG UNIQUE 比较
CREATE UNIQUE INDEX idx_commands_idempotency
    ON commands (tenant_id, idempotency_key)
    WHERE idempotency_key IS NOT NULL;
CREATE INDEX idx_commands_tenant_device ON commands (tenant_id, target_device_id);

-- 制品元数据（二进制在对象存储）
CREATE TABLE artifacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    signature_b64 TEXT,
    channel TEXT NOT NULL DEFAULT 'stable',
    object_key TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, name, version, channel)
);
CREATE INDEX idx_artifacts_tenant ON artifacts (tenant_id);

-- 管理审计（权威）；异步复制到 ClickHouse
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
    actor_user_id UUID,
    action TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_audit_tenant_time ON audit_logs (tenant_id, created_at DESC);

-- 合规发现
CREATE TYPE finding_severity AS ENUM ('info', 'low', 'medium', 'high', 'critical');
CREATE TYPE finding_status AS ENUM ('open', 'accepted', 'fixed', 'false_positive');

CREATE TABLE compliance_findings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
    device_id UUID NOT NULL REFERENCES devices (id) ON DELETE CASCADE,
    rule_id TEXT NOT NULL,
    title TEXT NOT NULL,
    severity finding_severity NOT NULL,
    status finding_status NOT NULL DEFAULT 'open',
    evidence_object_key TEXT,
    details JSONB NOT NULL DEFAULT '{}',
    detected_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_findings_tenant_device ON compliance_findings (tenant_id, device_id);
