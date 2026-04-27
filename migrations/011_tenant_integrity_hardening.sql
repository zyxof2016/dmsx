-- Harden tenant isolation at the database boundary.
--
-- RLS FORCE makes policies apply to table owners too, reducing accidental bypass
-- risk when the application connects as the migration/table owner.
-- Composite foreign keys make child rows prove that their parent belongs to the
-- same tenant, not just that the referenced UUID exists somewhere globally.

ALTER TABLE orgs FORCE ROW LEVEL SECURITY;
ALTER TABLE sites FORCE ROW LEVEL SECURITY;
ALTER TABLE groups FORCE ROW LEVEL SECURITY;
ALTER TABLE devices FORCE ROW LEVEL SECURITY;
ALTER TABLE policies FORCE ROW LEVEL SECURITY;
ALTER TABLE policy_revisions FORCE ROW LEVEL SECURITY;
ALTER TABLE commands FORCE ROW LEVEL SECURITY;
ALTER TABLE artifacts FORCE ROW LEVEL SECURITY;
ALTER TABLE audit_logs FORCE ROW LEVEL SECURITY;
ALTER TABLE compliance_findings FORCE ROW LEVEL SECURITY;
ALTER TABLE device_shadows FORCE ROW LEVEL SECURITY;
ALTER TABLE command_results FORCE ROW LEVEL SECURITY;
ALTER TABLE system_settings FORCE ROW LEVEL SECURITY;

CREATE UNIQUE INDEX IF NOT EXISTS uniq_orgs_tenant_id ON orgs (tenant_id, id);
CREATE UNIQUE INDEX IF NOT EXISTS uniq_sites_tenant_id ON sites (tenant_id, id);
CREATE UNIQUE INDEX IF NOT EXISTS uniq_groups_tenant_id ON groups (tenant_id, id);
CREATE UNIQUE INDEX IF NOT EXISTS uniq_devices_tenant_id ON devices (tenant_id, id);
CREATE UNIQUE INDEX IF NOT EXISTS uniq_policies_tenant_id ON policies (tenant_id, id);

ALTER TABLE sites
    DROP CONSTRAINT IF EXISTS sites_org_id_fkey,
    ADD CONSTRAINT sites_tenant_org_fk
        FOREIGN KEY (tenant_id, org_id)
        REFERENCES orgs (tenant_id, id)
        ON DELETE CASCADE;

ALTER TABLE groups
    DROP CONSTRAINT IF EXISTS groups_site_id_fkey,
    ADD CONSTRAINT groups_tenant_site_fk
        FOREIGN KEY (tenant_id, site_id)
        REFERENCES sites (tenant_id, id)
        ON DELETE CASCADE;

ALTER TABLE devices
    DROP CONSTRAINT IF EXISTS devices_site_id_fkey,
    DROP CONSTRAINT IF EXISTS devices_primary_group_id_fkey,
    ADD CONSTRAINT devices_tenant_site_fk
        FOREIGN KEY (tenant_id, site_id)
        REFERENCES sites (tenant_id, id)
        ON DELETE SET NULL (site_id),
    ADD CONSTRAINT devices_tenant_group_fk
        FOREIGN KEY (tenant_id, primary_group_id)
        REFERENCES groups (tenant_id, id)
        ON DELETE SET NULL (primary_group_id);

ALTER TABLE policies
    DROP CONSTRAINT IF EXISTS policies_scope_org_id_fkey,
    DROP CONSTRAINT IF EXISTS policies_scope_site_id_fkey,
    DROP CONSTRAINT IF EXISTS policies_scope_group_id_fkey,
    ADD CONSTRAINT policies_tenant_scope_org_fk
        FOREIGN KEY (tenant_id, scope_org_id)
        REFERENCES orgs (tenant_id, id)
        ON DELETE CASCADE,
    ADD CONSTRAINT policies_tenant_scope_site_fk
        FOREIGN KEY (tenant_id, scope_site_id)
        REFERENCES sites (tenant_id, id)
        ON DELETE CASCADE,
    ADD CONSTRAINT policies_tenant_scope_group_fk
        FOREIGN KEY (tenant_id, scope_group_id)
        REFERENCES groups (tenant_id, id)
        ON DELETE CASCADE;

ALTER TABLE policy_revisions
    DROP CONSTRAINT IF EXISTS policy_revisions_policy_id_fkey,
    ADD CONSTRAINT policy_revisions_tenant_policy_fk
        FOREIGN KEY (tenant_id, policy_id)
        REFERENCES policies (tenant_id, id)
        ON DELETE CASCADE;

ALTER TABLE commands
    DROP CONSTRAINT IF EXISTS commands_target_device_id_fkey,
    ADD CONSTRAINT commands_tenant_target_device_fk
        FOREIGN KEY (tenant_id, target_device_id)
        REFERENCES devices (tenant_id, id)
        ON DELETE CASCADE;

ALTER TABLE compliance_findings
    DROP CONSTRAINT IF EXISTS compliance_findings_device_id_fkey,
    ADD CONSTRAINT compliance_findings_tenant_device_fk
        FOREIGN KEY (tenant_id, device_id)
        REFERENCES devices (tenant_id, id)
        ON DELETE CASCADE;

ALTER TABLE device_shadows
    DROP CONSTRAINT IF EXISTS device_shadows_device_id_fkey,
    ADD CONSTRAINT device_shadows_tenant_device_fk
        FOREIGN KEY (tenant_id, device_id)
        REFERENCES devices (tenant_id, id)
        ON DELETE CASCADE;
