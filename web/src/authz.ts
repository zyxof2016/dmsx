import { useAppSession } from "./appProviders";

export const WRITE_DISABLED_REASON =
  "当前有效角色仅允许查看，不能执行写操作。请检查 JWT 中的 roles / tenant_roles。";

export type FrontendResourceKind =
  | "platformRead"
  | "platformWrite"
  | "stats"
  | "devices"
  | "policies"
  | "commands"
  | "deviceShadow"
  | "artifacts"
  | "compliance"
  | "remoteDesktop"
  | "aiAssist"
  | "genericTenantResource";

function siteAdminAllows(resource: FrontendResourceKind, readOnly: boolean) {
  switch (resource) {
    case "platformRead":
    case "platformWrite":
      return false;
    case "policies":
    case "artifacts":
    case "aiAssist":
    case "compliance":
    case "stats":
      return readOnly;
    default:
      return true;
  }
}

function operatorAllows(resource: FrontendResourceKind, readOnly: boolean) {
  switch (resource) {
    case "platformRead":
    case "platformWrite":
      return false;
    case "policies":
    case "artifacts":
    case "aiAssist":
    case "compliance":
    case "stats":
      return readOnly;
    case "genericTenantResource":
      return readOnly;
    default:
      return true;
  }
}

function auditorAllows(resource: FrontendResourceKind, readOnly: boolean) {
  return readOnly && !["platformRead", "platformWrite", "remoteDesktop", "aiAssist"].includes(resource);
}

function readOnlyAllows(resource: FrontendResourceKind, readOnly: boolean) {
  return readOnly && !["platformRead", "platformWrite", "remoteDesktop", "aiAssist"].includes(resource);
}

function isRoleAllowed(role: string, resource: FrontendResourceKind, readOnly: boolean) {
  switch (role) {
    case "PlatformAdmin":
      return true;
    case "PlatformViewer":
      return readOnly && resource === "platformRead";
    case "TenantAdmin":
      return !["platformRead", "platformWrite"].includes(resource);
    case "SiteAdmin":
      return siteAdminAllows(resource, readOnly);
    case "Operator":
      return operatorAllows(resource, readOnly);
    case "Auditor":
      return auditorAllows(resource, readOnly);
    case "ReadOnly":
      return readOnlyAllows(resource, readOnly);
    default:
      return false;
  }
}

function canAccessResource(
  roles: string[],
  resource: FrontendResourceKind,
  readOnly: boolean,
) {
  return roles.some((role) => isRoleAllowed(role, resource, readOnly));
}

export function useResourceAccess(resource: FrontendResourceKind) {
  const { effectiveRoles } = useAppSession();

  return {
    canRead: canAccessResource(effectiveRoles, resource, true),
    canWrite: canAccessResource(effectiveRoles, resource, false),
  };
}
