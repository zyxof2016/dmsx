import React from "react";
import {
  AppstoreOutlined,
  AuditOutlined,
  CloudServerOutlined,
  ClusterOutlined,
  DashboardOutlined,
  DesktopOutlined,
  GlobalOutlined,
  RobotOutlined,
  SafetyOutlined,
  SettingOutlined,
  UserOutlined,
} from "@ant-design/icons";

import type { AppMode } from "./appProviders";

export type NavItem = {
  key: string;
  path: string;
  labelKey: string;
  groupKey: string;
  icon: React.ReactNode;
  mode: AppMode;
  platformOnly?: boolean;
  requiredRoles?: string[];
};

export type AccessResult = {
  allowed: boolean;
  reason: "mode" | "role" | "platform" | "unknown";
};

export const NAV_ITEMS: NavItem[] = [
  {
    key: "platformOverview",
    path: "/platform",
    labelKey: "nav.platformOverview",
    groupKey: "nav.group.platformWorkbench",
    icon: <SettingOutlined />,
    mode: "platform",
    platformOnly: true,
  },
  {
    key: "platformPermissions",
    path: "/platform/permissions",
    labelKey: "nav.platformPermissionsOverview",
    groupKey: "nav.group.platformPermissions",
    icon: <SafetyOutlined />,
    mode: "platform",
    platformOnly: true,
  },
  {
    key: "platformPermissionUsers",
    path: "/platform/permissions/users",
    labelKey: "nav.platformPermissionUsers",
    groupKey: "nav.group.platformPermissions",
    icon: <UserOutlined />,
    mode: "platform",
    platformOnly: true,
  },
  {
    key: "platformPermissionRoles",
    path: "/platform/permissions/roles",
    labelKey: "nav.platformPermissionRoles",
    groupKey: "nav.group.platformPermissions",
    icon: <SafetyOutlined />,
    mode: "platform",
    platformOnly: true,
  },
  {
    key: "platformPermissionMenus",
    path: "/platform/permissions/menus",
    labelKey: "nav.platformPermissionMenus",
    groupKey: "nav.group.platformPermissions",
    icon: <AppstoreOutlined />,
    mode: "platform",
    platformOnly: true,
  },
  {
    key: "platformPermissionPolicy",
    path: "/platform/permissions/policy",
    labelKey: "nav.platformPermissionPolicy",
    groupKey: "nav.group.platformPermissions",
    icon: <SettingOutlined />,
    mode: "platform",
    platformOnly: true,
  },
  {
    key: "platformTenants",
    path: "/platform/tenants",
    labelKey: "nav.platformTenants",
    groupKey: "nav.group.platformGovernance",
    icon: <ClusterOutlined />,
    mode: "platform",
    platformOnly: true,
  },
  {
    key: "platformQuotas",
    path: "/platform/quotas",
    labelKey: "nav.platformQuotas",
    groupKey: "nav.group.platformGovernance",
    icon: <AppstoreOutlined />,
    mode: "platform",
    platformOnly: true,
  },
  {
    key: "platformAudit",
    path: "/platform/audit",
    labelKey: "nav.platformAudit",
    groupKey: "nav.group.platformOperations",
    icon: <AuditOutlined />,
    mode: "platform",
    platformOnly: true,
  },
  {
    key: "platformHealth",
    path: "/platform/health",
    labelKey: "nav.platformHealth",
    groupKey: "nav.group.platformOperations",
    icon: <DashboardOutlined />,
    mode: "platform",
    platformOnly: true,
  },
  {
    key: "settings",
    path: "/settings",
    labelKey: "nav.settings",
    groupKey: "nav.group.platformOperations",
    icon: <SettingOutlined />,
    mode: "platform",
    platformOnly: true,
  },
  {
    key: "dashboard",
    path: "/",
    labelKey: "nav.dashboard",
    groupKey: "nav.group.tenantWorkbench",
    icon: <DashboardOutlined />,
    mode: "tenant",
  },
  {
    key: "ai",
    path: "/ai",
    labelKey: "nav.ai",
    groupKey: "nav.group.tenantWorkbench",
    icon: <RobotOutlined />,
    mode: "tenant",
    requiredRoles: ["TenantAdmin", "SiteAdmin", "Operator"],
  },
  {
    key: "devices",
    path: "/devices",
    labelKey: "nav.devices",
    groupKey: "nav.group.tenantDeviceOps",
    icon: <DesktopOutlined />,
    mode: "tenant",
  },
  {
    key: "commands",
    path: "/commands",
    labelKey: "nav.commands",
    groupKey: "nav.group.tenantDeviceOps",
    icon: <CloudServerOutlined />,
    mode: "tenant",
  },
  {
    key: "policies",
    path: "/policies",
    labelKey: "nav.policies",
    groupKey: "nav.group.tenantSecurity",
    icon: <SafetyOutlined />,
    mode: "tenant",
  },
  {
    key: "compliance",
    path: "/compliance",
    labelKey: "nav.compliance",
    groupKey: "nav.group.tenantSecurity",
    icon: <AuditOutlined />,
    mode: "tenant",
  },
  {
    key: "policyEditor",
    path: "/policy-editor",
    labelKey: "nav.policyEditor",
    groupKey: "nav.group.tenantSecurity",
    icon: <SafetyOutlined />,
    mode: "tenant",
    requiredRoles: ["TenantAdmin"],
  },
  {
    key: "auditLogs",
    path: "/audit-logs",
    labelKey: "nav.auditLogs",
    groupKey: "nav.group.tenantSecurity",
    icon: <AuditOutlined />,
    mode: "tenant",
  },
  {
    key: "usersRoles",
    path: "/users",
    labelKey: "nav.usersRoles",
    groupKey: "nav.group.tenantSecurity",
    icon: <UserOutlined />,
    mode: "tenant",
  },
  {
    key: "artifacts",
    path: "/artifacts",
    labelKey: "nav.artifacts",
    groupKey: "nav.group.tenantDelivery",
    icon: <AppstoreOutlined />,
    mode: "tenant",
  },
  {
    key: "network",
    path: "/network",
    labelKey: "nav.network",
    groupKey: "nav.group.tenantDelivery",
    icon: <GlobalOutlined />,
    mode: "tenant",
  },
];

export function itemIsVisible(
  item: NavItem,
  appMode: AppMode,
  effectiveRoles: string[],
  canUsePlatformMode: boolean,
) {
  return evaluateItemAccess(item, appMode, effectiveRoles, canUsePlatformMode).allowed;
}

export function evaluateItemAccess(
  item: NavItem | undefined,
  appMode: AppMode,
  effectiveRoles: string[],
  canUsePlatformMode: boolean,
): AccessResult {
  if (!item) return { allowed: true, reason: "unknown" };
  if (item.mode !== appMode) return { allowed: false, reason: "mode" };
  if (item.platformOnly && !canUsePlatformMode) {
    return { allowed: false, reason: "platform" };
  }
  if (!item.requiredRoles?.length) return { allowed: true, reason: "unknown" };
  return item.requiredRoles.some((role) => effectiveRoles.includes(role))
    ? { allowed: true, reason: "unknown" }
    : { allowed: false, reason: "role" };
}

export function matchNavItem(pathname: string): NavItem | undefined {
  return [...NAV_ITEMS]
    .sort((a, b) => b.path.length - a.path.length)
    .find((item) => {
      if (item.path === "/") return pathname === "/";
      return pathname === item.path || pathname.startsWith(`${item.path}/`);
    });
}

export function navItemLabel(
  item: NavItem | undefined,
  t: (key: string) => string,
) {
  return item ? t(item.labelKey) : t("nav.dashboard");
}

export function buildGroupedMenuItems(items: NavItem[], t: (key: string) => string) {
  const groups: Array<{ key: string; children: NavItem[] }> = [];
  for (const item of items) {
    const group = groups.find((candidate) => candidate.key === item.groupKey);
    if (group) {
      group.children.push(item);
    } else {
      groups.push({ key: item.groupKey, children: [item] });
    }
  }

  return groups.map((group) => ({
    key: group.key,
    type: "group" as const,
    label: t(group.key),
    children: group.children.map((item) => ({
      key: item.key,
      icon: item.icon,
      label: t(item.labelKey),
    })),
  }));
}
