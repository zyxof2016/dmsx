import {
  createRouter,
  createRoute,
  createRootRoute,
} from "@tanstack/react-router";
import { AppLayout } from "./App";
import { DashboardPage } from "./pages/Dashboard";
import { DevicesPage } from "./pages/Devices";
import { DeviceDetailDrawer } from "./components/DeviceDetail";
import { PoliciesPage } from "./pages/Policies";
import { PolicyDetailDrawer } from "./components/PolicyDetail";
import { CommandsPage } from "./pages/Commands";
import { CommandDetailDrawer } from "./components/CommandDetail";
import { ArtifactsPage } from "./pages/Artifacts";
import { CompliancePage } from "./pages/Compliance";
import { NetworkPage } from "./pages/Network";
import { AiCenterPage } from "./pages/AiCenter";
import { SystemSettingsPage } from "./pages/SystemSettings";
import { PolicyEditorPage } from "./pages/PolicyEditor";
import { AuditLogsPage } from "./pages/AuditLogs";
import { UsersRolesPage } from "./pages/UsersRoles";

const rootRoute = createRootRoute({ component: AppLayout });

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: DashboardPage,
});

// --- Devices (nested: list + detail drawer) ---
const devicesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/devices",
  component: DevicesPage,
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
  component: DeviceDetailDrawer,
});

// --- Policies (nested) ---
const policiesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/policies",
  component: PoliciesPage,
});
const policiesIndexRoute = createRoute({
  getParentRoute: () => policiesRoute,
  path: "/",
});
const policyDetailRoute = createRoute({
  getParentRoute: () => policiesRoute,
  path: "$policyId",
  component: PolicyDetailDrawer,
});

// --- Commands (nested) ---
const commandsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/commands",
  component: CommandsPage,
});
const commandsIndexRoute = createRoute({
  getParentRoute: () => commandsRoute,
  path: "/",
});
const commandDetailRoute = createRoute({
  getParentRoute: () => commandsRoute,
  path: "$commandId",
  component: CommandDetailDrawer,
});

// --- Flat routes ---
const artifactsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/artifacts",
  component: ArtifactsPage,
});
const complianceRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/compliance",
  component: CompliancePage,
});
const networkRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/network",
  component: NetworkPage,
});
const aiRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/ai",
  component: AiCenterPage,
});

const systemSettingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/settings",
  component: SystemSettingsPage,
});

const policyEditorRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/policy-editor",
  component: PolicyEditorPage,
});

const auditLogsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/audit-logs",
  component: AuditLogsPage,
});

const usersRolesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/users",
  component: UsersRolesPage,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
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
]);

export const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
