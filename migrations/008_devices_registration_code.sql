ALTER TABLE devices
ADD COLUMN registration_code TEXT;

UPDATE devices
SET registration_code = CONCAT(
    'DEV-',
    UPPER(SUBSTRING(REPLACE(tenant_id::text, '-', '') FROM 1 FOR 4)),
    '-',
    UPPER(RIGHT(REPLACE(id::text, '-', ''), 12))
)
WHERE registration_code IS NULL;

ALTER TABLE devices
ALTER COLUMN registration_code SET NOT NULL;

CREATE UNIQUE INDEX uq_devices_tenant_registration_code
ON devices (tenant_id, registration_code);

CREATE INDEX idx_devices_tenant_registration_code_search
ON devices (tenant_id, registration_code);
