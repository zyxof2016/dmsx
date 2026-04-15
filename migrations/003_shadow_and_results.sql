-- Device Shadow: reported (from heartbeat) vs desired (from admin)
CREATE TABLE device_shadows (
    device_id    UUID PRIMARY KEY REFERENCES devices(id) ON DELETE CASCADE,
    tenant_id    UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    reported     JSONB NOT NULL DEFAULT '{}',
    desired      JSONB NOT NULL DEFAULT '{}',
    reported_at  TIMESTAMPTZ,
    desired_at   TIMESTAMPTZ,
    version      BIGINT NOT NULL DEFAULT 0
);
CREATE INDEX idx_shadows_tenant ON device_shadows (tenant_id);

-- Command execution results (stdout/stderr/exit_code)
CREATE TABLE command_results (
    command_id   UUID PRIMARY KEY REFERENCES commands(id) ON DELETE CASCADE,
    tenant_id    UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    exit_code    INT,
    stdout       TEXT NOT NULL DEFAULT '',
    stderr       TEXT NOT NULL DEFAULT '',
    evidence_key TEXT,
    reported_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_cmd_results_tenant ON command_results (tenant_id);
