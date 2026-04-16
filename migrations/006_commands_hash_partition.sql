-- 将 `commands` 转为按 `tenant_id` 的 HASH 分区表（8 分区），与 RLS 并存；物理布局见 docs/POSTGRES_TENANT_PARTITIONING_PLAN.md。
-- 前置：005 RLS 已应用。本迁移在单事务内执行；失败则整批回滚。
--
-- 注意：分区表上主键须包含分区键，故主键为 `(tenant_id, id)`。`command_results` 外键改为引用 `(tenant_id, id)`。

ALTER TABLE command_results
    DROP CONSTRAINT IF EXISTS command_results_command_id_fkey;

ALTER TABLE commands RENAME TO commands_legacy;

CREATE TABLE commands (
    id UUID NOT NULL DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
    idempotency_key TEXT,
    target_device_id UUID NOT NULL REFERENCES devices (id) ON DELETE CASCADE,
    payload JSONB NOT NULL,
    priority SMALLINT NOT NULL DEFAULT 0,
    ttl_seconds INT NOT NULL DEFAULT 3600,
    status command_status NOT NULL DEFAULT 'queued',
    created_by UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, id)
) PARTITION BY HASH (tenant_id);

CREATE TABLE commands_p0 PARTITION OF commands FOR VALUES WITH (MODULUS 8, REMAINDER 0);
CREATE TABLE commands_p1 PARTITION OF commands FOR VALUES WITH (MODULUS 8, REMAINDER 1);
CREATE TABLE commands_p2 PARTITION OF commands FOR VALUES WITH (MODULUS 8, REMAINDER 2);
CREATE TABLE commands_p3 PARTITION OF commands FOR VALUES WITH (MODULUS 8, REMAINDER 3);
CREATE TABLE commands_p4 PARTITION OF commands FOR VALUES WITH (MODULUS 8, REMAINDER 4);
CREATE TABLE commands_p5 PARTITION OF commands FOR VALUES WITH (MODULUS 8, REMAINDER 5);
CREATE TABLE commands_p6 PARTITION OF commands FOR VALUES WITH (MODULUS 8, REMAINDER 6);
CREATE TABLE commands_p7 PARTITION OF commands FOR VALUES WITH (MODULUS 8, REMAINDER 7);

INSERT INTO commands SELECT * FROM commands_legacy;

DO $$
DECLARE
    n_new bigint;
    n_old bigint;
BEGIN
    SELECT COUNT(*) INTO n_new FROM commands;
    SELECT COUNT(*) INTO n_old FROM commands_legacy;
    IF n_new IS DISTINCT FROM n_old THEN
        RAISE EXCEPTION 'commands partition copy mismatch: new=% old=%', n_new, n_old;
    END IF;
END $$;

CREATE UNIQUE INDEX idx_commands_idempotency
    ON commands (tenant_id, idempotency_key)
    WHERE idempotency_key IS NOT NULL;

CREATE INDEX idx_commands_tenant_device ON commands (tenant_id, target_device_id);

DROP TABLE commands_legacy;

ALTER TABLE command_results
    ADD CONSTRAINT command_results_command_fk
    FOREIGN KEY (tenant_id, command_id)
    REFERENCES commands (tenant_id, id)
    ON DELETE CASCADE;

ALTER TABLE commands ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rls_commands_tenant ON commands;
CREATE POLICY rls_commands_tenant ON commands
    USING (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id())
    WITH CHECK (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id());
