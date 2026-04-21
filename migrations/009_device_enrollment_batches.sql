CREATE TABLE device_enrollment_batches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
    actor_subject TEXT,
    item_count BIGINT NOT NULL,
    result JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_device_enrollment_batches_tenant_created_at
ON device_enrollment_batches (tenant_id, created_at DESC);
