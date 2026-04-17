-- System settings (platform global + optional tenant override).
-- For now the admin UI uses platform-global keys (tenant_id IS NULL).

CREATE TABLE IF NOT EXISTS system_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants (id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_system_settings_tenant_updated
    ON system_settings (tenant_id, updated_at DESC);

-- Unique constraints with NULL semantics handled by partial unique indexes.
CREATE UNIQUE INDEX IF NOT EXISTS uniq_system_settings_global_key
    ON system_settings (key)
    WHERE tenant_id IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS uniq_system_settings_tenant_key
    ON system_settings (tenant_id, key)
    WHERE tenant_id IS NOT NULL;

ALTER TABLE system_settings ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS rls_system_settings_tenant ON system_settings;
CREATE POLICY rls_system_settings_tenant ON system_settings
    USING      (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id())
    WITH CHECK (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id());

