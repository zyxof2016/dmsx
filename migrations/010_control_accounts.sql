CREATE TABLE control_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    display_name TEXT NOT NULL,
    platform_roles JSONB NOT NULL DEFAULT '[]'::jsonb,
    default_tenant_id UUID REFERENCES tenants (id) ON DELETE SET NULL,
    last_tenant_id UUID REFERENCES tenants (id) ON DELETE SET NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE control_account_tenants (
    account_id UUID NOT NULL REFERENCES control_accounts (id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
    roles JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, tenant_id)
);

CREATE INDEX idx_control_account_tenants_tenant
    ON control_account_tenants (tenant_id, account_id);
