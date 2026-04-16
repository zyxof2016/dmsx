-- Tenant isolation via Postgres RLS.
-- This migration enforces that tenant-scoped tables can only be accessed when
-- the current session sets `dmsx.tenant_id` (UUID), unless `dmsx.is_platform_admin` is true.
--
-- Design notes:
-- - Uses current_setting(..., true) so missing settings don't error (return NULL).
-- - Policies use both USING and WITH CHECK to cover reads and writes.
-- - `tenants` is treated as platform/global and is not RLS-protected here.

CREATE SCHEMA IF NOT EXISTS dmsx;

CREATE OR REPLACE FUNCTION dmsx.current_tenant_id()
RETURNS uuid
LANGUAGE sql
STABLE
AS $$
  SELECT NULLIF(current_setting('dmsx.tenant_id', true), '')::uuid
$$;

CREATE OR REPLACE FUNCTION dmsx.is_platform_admin()
RETURNS boolean
LANGUAGE sql
STABLE
AS $$
  SELECT COALESCE(NULLIF(current_setting('dmsx.is_platform_admin', true), ''), 'false')::boolean
$$;

-- Helper macro pattern:
--   USING      (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id())
--   WITH CHECK (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id())

ALTER TABLE orgs ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rls_orgs_tenant ON orgs;
CREATE POLICY rls_orgs_tenant ON orgs
  USING      (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id())
  WITH CHECK (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id());

ALTER TABLE sites ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rls_sites_tenant ON sites;
CREATE POLICY rls_sites_tenant ON sites
  USING      (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id())
  WITH CHECK (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id());

ALTER TABLE groups ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rls_groups_tenant ON groups;
CREATE POLICY rls_groups_tenant ON groups
  USING      (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id())
  WITH CHECK (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id());

ALTER TABLE devices ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rls_devices_tenant ON devices;
CREATE POLICY rls_devices_tenant ON devices
  USING      (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id())
  WITH CHECK (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id());

ALTER TABLE policies ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rls_policies_tenant ON policies;
CREATE POLICY rls_policies_tenant ON policies
  USING      (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id())
  WITH CHECK (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id());

ALTER TABLE policy_revisions ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rls_policy_revisions_tenant ON policy_revisions;
CREATE POLICY rls_policy_revisions_tenant ON policy_revisions
  USING      (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id())
  WITH CHECK (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id());

ALTER TABLE commands ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rls_commands_tenant ON commands;
CREATE POLICY rls_commands_tenant ON commands
  USING      (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id())
  WITH CHECK (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id());

ALTER TABLE artifacts ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rls_artifacts_tenant ON artifacts;
CREATE POLICY rls_artifacts_tenant ON artifacts
  USING      (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id())
  WITH CHECK (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id());

ALTER TABLE audit_logs ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rls_audit_logs_tenant ON audit_logs;
CREATE POLICY rls_audit_logs_tenant ON audit_logs
  USING      (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id())
  WITH CHECK (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id());

ALTER TABLE compliance_findings ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rls_compliance_findings_tenant ON compliance_findings;
CREATE POLICY rls_compliance_findings_tenant ON compliance_findings
  USING      (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id())
  WITH CHECK (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id());

ALTER TABLE device_shadows ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rls_device_shadows_tenant ON device_shadows;
CREATE POLICY rls_device_shadows_tenant ON device_shadows
  USING      (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id())
  WITH CHECK (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id());

ALTER TABLE command_results ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rls_command_results_tenant ON command_results;
CREATE POLICY rls_command_results_tenant ON command_results
  USING      (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id())
  WITH CHECK (dmsx.is_platform_admin() OR tenant_id = dmsx.current_tenant_id());

