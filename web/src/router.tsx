import React from "react";
import {
  createRouter,
  createRoute,
  createRootRoute,
} from "@tanstack/react-router";

const AppLayout = React.lazy(async () => {
  const mod = await import("./App");
  return { default: mod.AppLayout };
});

const DashboardPage = React.lazy(async () => {
  const mod = await import("./pages/Dashboard");
  return { default: mod.DashboardPage };
});

const PlatformOverviewPage = React.lazy(async () => {
  const mod = await import("./pages/PlatformOverview");
  return { default: mod.PlatformOverviewPage };
});

const PlatformTenantsPage = React.lazy(async () => {
  const mod = await import("./pages/PlatformTenants");
  return { default: mod.PlatformTenantsPage };
});

const PlatformQuotasPage = React.lazy(async () => {
  const mod = await import("./pages/PlatformQuotas");
  return { default: mod.PlatformQuotasPage };
});

const PlatformAuditLogsPage = React.lazy(async () => {
  const mod = await import("./pages/PlatformAuditLogs");
  return { default: mod.PlatformAuditLogsPage };
});

const PlatformHealthPage = React.lazy(async () => {
  const mod = await import("./pages/PlatformHealth");
  return { default: mod.PlatformHealthPage };
});

const DevicesPage = React.lazy(async () => {
  const mod = await import("./pages/Devices");
  return { default: mod.DevicesPage };
});

const DeviceDetailDrawer = React.lazy(async () => {
  const mod = await import("./components/DeviceDetail");
  return { default: mod.DeviceDetailDrawer };
});

const PoliciesPage = React.lazy(async () => {
  const mod = await import("./pages/Policies");
  return { default: mod.PoliciesPage };
});

const PolicyDetailDrawer = React.lazy(async () => {
  const mod = await import("./components/PolicyDetail");
  return { default: mod.PolicyDetailDrawer };
});

const CommandsPage = React.lazy(async () => {
  const mod = await import("./pages/Commands");
  return { default: mod.CommandsPage };
});

const CommandDetailDrawer = React.lazy(async () => {
  const mod = await import("./components/CommandDetail");
  return { default: mod.CommandDetailDrawer };
});

const ArtifactsPage = React.lazy(async () => {
  const mod = await import("./pages/Artifacts");
  return { default: mod.ArtifactsPage };
});

const CompliancePage = React.lazy(async () => {
  const mod = await import("./pages/Compliance");
  return { default: mod.CompliancePage };
});

const NetworkPage = React.lazy(async () => {
  const mod = await import("./pages/Network");
  return { default: mod.NetworkPage };
});

const AiCenterPage = React.lazy(async () => {
  const mod = await import("./pages/AiCenter");
  return { default: mod.AiCenterPage };
});

const SystemSettingsPage = React.lazy(async () => {
  const mod = await import("./pages/SystemSettings");
  return { default: mod.SystemSettingsPage };
});

const PolicyEditorPage = React.lazy(async () => {
  const mod = await import("./pages/PolicyEditor");
  return { default: mod.PolicyEditorPage };
});

const AuditLogsPage = React.lazy(async () => {
  const mod = await import("./pages/AuditLogs");
  return { default: mod.AuditLogsPage };
});

const UsersRolesPage = React.lazy(async () => {
  const mod = await import("./pages/UsersRoles");
  return { default: mod.UsersRolesPage };
});

const ZeroTouchEnrollPage = React.lazy(async () => {
  const mod = await import("./pages/ZeroTouchEnroll");
  return { default: mod.ZeroTouchEnrollPage };
});

function withLazyBoundary(Component: React.LazyExoticComponent<React.ComponentType>) {
  return function LazyRouteComponent() {
    return (
      <React.Suspense
        fallback={
          <div style={{ minHeight: 240, display: "grid", placeItems: "center" }}>
            <div
              aria-label="loading"
              style={{
                width: 32,
                height: 32,
                borderRadius: "50%",
                border: "3px solid rgba(22, 119, 255, 0.18)",
                borderTopColor: "#1677ff",
                animation: "dmsx-spin 0.8s linear infinite",
              }}
            />
            <style>{"@keyframes dmsx-spin { to { transform: rotate(360deg); } }"}</style>
          </div>
        }
      >
        <Component />
      </React.Suspense>
    );
  };
}

const rootRoute = createRootRoute({ component: withLazyBoundary(AppLayout) });

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: withLazyBoundary(DashboardPage),
});

const platformRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/platform",
  component: withLazyBoundary(PlatformOverviewPage),
});

const platformTenantsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/platform/tenants",
  component: withLazyBoundary(PlatformTenantsPage),
});

const platformQuotasRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/platform/quotas",
  component: withLazyBoundary(PlatformQuotasPage),
});

const platformAuditLogsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/platform/audit",
  component: withLazyBoundary(PlatformAuditLogsPage),
});

const platformHealthRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/platform/health",
  component: withLazyBoundary(PlatformHealthPage),
});

// --- Devices (nested: list + detail drawer) ---
const devicesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/devices",
  component: withLazyBoundary(DevicesPage),
});
const devicesIndexRoute = createRoute({
  getParentRoute: () => devicesRoute,
  path: "/",
});
const deviceDetailRoute = createRoute({
  getParentRoute: () => devicesRoute,
  path: "$deviceId",
  validateSearch: (search: Record<string, unknown>) => ({
    tab: typeof search.tab === "string" ? search.tab : undefined,
  }),
  component: withLazyBoundary(DeviceDetailDrawer),
});

// --- Policies (nested) ---
const policiesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/policies",
  component: withLazyBoundary(PoliciesPage),
});
const policiesIndexRoute = createRoute({
  getParentRoute: () => policiesRoute,
  path: "/",
});
const policyDetailRoute = createRoute({
  getParentRoute: () => policiesRoute,
  path: "$policyId",
  component: withLazyBoundary(PolicyDetailDrawer),
});

// --- Commands (nested) ---
const commandsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/commands",
  component: withLazyBoundary(CommandsPage),
});
const commandsIndexRoute = createRoute({
  getParentRoute: () => commandsRoute,
  path: "/",
});
const commandDetailRoute = createRoute({
  getParentRoute: () => commandsRoute,
  path: "$commandId",
  component: withLazyBoundary(CommandDetailDrawer),
});

// --- Flat routes ---
const artifactsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/artifacts",
  component: withLazyBoundary(ArtifactsPage),
});
const complianceRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/compliance",
  component: withLazyBoundary(CompliancePage),
});
const networkRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/network",
  component: withLazyBoundary(NetworkPage),
});
const aiRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/ai",
  component: withLazyBoundary(AiCenterPage),
});

const systemSettingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/settings",
  component: withLazyBoundary(SystemSettingsPage),
});

const policyEditorRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/policy-editor",
  component: withLazyBoundary(PolicyEditorPage),
});

const auditLogsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/audit-logs",
  component: withLazyBoundary(AuditLogsPage),
});

const usersRolesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/users",
  component: withLazyBoundary(UsersRolesPage),
});

const zeroTouchEnrollRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/zero-touch-enroll",
  component: withLazyBoundary(ZeroTouchEnrollPage),
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  platformRoute,
  platformTenantsRoute,
  platformQuotasRoute,
  platformAuditLogsRoute,
  platformHealthRoute,
  devicesRoute.addChildren([devicesIndexRoute, deviceDetailRoute]),
  policiesRoute.addChildren([policiesIndexRoute, policyDetailRoute]),
  commandsRoute.addChildren([commandsIndexRoute, commandDetailRoute]),
  artifactsRoute,
  complianceRoute,
  networkRoute,
  aiRoute,
  systemSettingsRoute,
  policyEditorRoute,
  auditLogsRoute,
  usersRolesRoute,
  zeroTouchEnrollRoute,
]);

export const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
