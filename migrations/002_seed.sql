-- Development seed: default tenant
INSERT INTO tenants (id, name) VALUES
    ('00000000-0000-0000-0000-000000000001', '默认租户')
ON CONFLICT (id) DO NOTHING;
