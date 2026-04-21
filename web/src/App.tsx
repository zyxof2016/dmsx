import React from "react";
import {
  App as AntApp,
  Layout,
  Menu,
  Breadcrumb,
  Segmented,
  Space,
  Select,
  Switch,
  ConfigProvider,
  theme as antdTheme,
} from "antd";
import zhCN from "antd/locale/zh_CN";
import enUS from "antd/locale/en_US";
import {
  DashboardOutlined,
  DesktopOutlined,
  SafetyOutlined,
  SettingOutlined,
  AppstoreOutlined,
  ClusterOutlined,
  CloudServerOutlined,
  AuditOutlined,
  RobotOutlined,
  UserOutlined,
  GlobalOutlined,
} from "@ant-design/icons";
import {
  Outlet,
  useNavigate,
  useRouterState,
} from "@tanstack/react-router";

import {
  useAppI18n,
  useThemeMode,
  useAppSession,
  type AppMode,
  type Lang,
} from "./appProviders";
import { AccessGate } from "./components/AccessGate";

const AppDeferredTools = React.lazy(async () => {
  const mod = await import("./components/AppDeferredTools");
  return { default: mod.AppDeferredTools };
});

const { Header, Sider, Content } = Layout;

type NavItem = {
  key: string;
  path: string;
  labelKey: string;
  icon: React.ReactNode;
  mode: AppMode;
  requiresPlatformAdmin?: boolean;
  requiredRoles?: string[];
};

type AccessResult = {
  allowed: boolean;
  reason: "mode" | "role" | "platform" | "unknown";
};

const NAV_ITEMS: NavItem[] = [
  {
    key: "platformOverview",
    path: "/platform",
    labelKey: "mode.platform",
    icon: <SettingOutlined />,
    mode: "platform",
    requiresPlatformAdmin: true,
  },
  {
    key: "platformTenants",
    path: "/platform/tenants",
    labelKey: "nav.platformTenants",
    icon: <ClusterOutlined />,
    mode: "platform",
    requiresPlatformAdmin: true,
  },
  {
    key: "platformQuotas",
    path: "/platform/quotas",
    labelKey: "nav.platformQuotas",
    icon: <AppstoreOutlined />,
    mode: "platform",
    requiresPlatformAdmin: true,
  },
  {
    key: "platformAudit",
    path: "/platform/audit",
    labelKey: "nav.platformAudit",
    icon: <AuditOutlined />,
    mode: "platform",
    requiresPlatformAdmin: true,
  },
  {
    key: "platformHealth",
    path: "/platform/health",
    labelKey: "nav.platformHealth",
    icon: <DashboardOutlined />,
    mode: "platform",
    requiresPlatformAdmin: true,
  },
  {
    key: "dashboard",
    path: "/",
    labelKey: "nav.dashboard",
    icon: <DashboardOutlined />,
    mode: "tenant",
  },
  {
    key: "devices",
    path: "/devices",
    labelKey: "nav.devices",
    icon: <DesktopOutlined />,
    mode: "tenant",
  },
  {
    key: "policies",
    path: "/policies",
    labelKey: "nav.policies",
    icon: <SafetyOutlined />,
    mode: "tenant",
  },
  {
    key: "commands",
    path: "/commands",
    labelKey: "nav.commands",
    icon: <CloudServerOutlined />,
    mode: "tenant",
  },
  {
    key: "artifacts",
    path: "/artifacts",
    labelKey: "nav.artifacts",
    icon: <AppstoreOutlined />,
    mode: "tenant",
  },
  {
    key: "compliance",
    path: "/compliance",
    labelKey: "nav.compliance",
    icon: <AuditOutlined />,
    mode: "tenant",
  },
  {
    key: "network",
    path: "/network",
    labelKey: "nav.network",
    icon: <GlobalOutlined />,
    mode: "tenant",
  },
  {
    key: "ai",
    path: "/ai",
    labelKey: "nav.ai",
    icon: <RobotOutlined />,
    mode: "tenant",
    requiredRoles: ["PlatformAdmin", "TenantAdmin", "SiteAdmin", "Operator"],
  },
  {
    key: "policyEditor",
    path: "/policy-editor",
    labelKey: "nav.policyEditor",
    icon: <SafetyOutlined />,
    mode: "tenant",
    requiredRoles: ["PlatformAdmin", "TenantAdmin"],
  },
  {
    key: "auditLogs",
    path: "/audit-logs",
    labelKey: "nav.auditLogs",
    icon: <AuditOutlined />,
    mode: "tenant",
  },
  {
    key: "settings",
    path: "/settings",
    labelKey: "nav.settings",
    icon: <SafetyOutlined />,
    mode: "platform",
    requiresPlatformAdmin: true,
  },
  {
    key: "usersRoles",
    path: "/users",
    labelKey: "nav.usersRoles",
    icon: <UserOutlined />,
    mode: "platform",
    requiresPlatformAdmin: true,
  },
];

function itemIsVisible(
  item: NavItem,
  appMode: AppMode,
  effectiveRoles: string[],
  canUsePlatformMode: boolean,
) {
  return evaluateItemAccess(item, appMode, effectiveRoles, canUsePlatformMode).allowed;
}

function evaluateItemAccess(
  item: NavItem | undefined,
  appMode: AppMode,
  effectiveRoles: string[],
  canUsePlatformMode: boolean,
): AccessResult {
  if (!item) return { allowed: true, reason: "unknown" };
  if (item.mode !== appMode) return { allowed: false, reason: "mode" };
  if (item.requiresPlatformAdmin && !canUsePlatformMode) {
    return { allowed: false, reason: "platform" };
  }
  if (!item.requiredRoles?.length) return { allowed: true, reason: "unknown" };
  return item.requiredRoles.some((role) => effectiveRoles.includes(role))
    ? { allowed: true, reason: "unknown" }
    : { allowed: false, reason: "role" };
}

function matchNavItem(pathname: string): NavItem | undefined {
  return [...NAV_ITEMS]
    .sort((a, b) => b.path.length - a.path.length)
    .find((item) => {
      if (item.path === "/") return pathname === "/";
      return pathname === item.path || pathname.startsWith(`${item.path}/`);
    });
}

export const AppLayout: React.FC = () => {
  const { lang } = useAppI18n();
  const { themeMode } = useThemeMode();
  const locale = lang === "zh" ? zhCN : enUS;
  const algorithm =
    themeMode === "dark" ? antdTheme.darkAlgorithm : antdTheme.defaultAlgorithm;

  return (
    <ConfigProvider
      locale={locale}
      theme={{
        algorithm,
        token: { colorPrimary: "#1677ff", borderRadius: 6 },
      }}
    >
      <AntApp>
        <AppShell />
      </AntApp>
    </ConfigProvider>
  );
};

const AppShell: React.FC = () => {
  const [collapsed, setCollapsed] = React.useState(false);
  const navigate = useNavigate();
  const pathname = useRouterState({
    select: (s) => s.location.pathname,
  });

  const selectedNavItem = matchNavItem(pathname);
  const selectedKey = selectedNavItem?.key ?? "dashboard";

  const { t, lang, setLang } = useAppI18n();
  const { themeMode, setThemeMode } = useThemeMode();
  const {
    tenantId,
    setTenantId,
    jwt,
    setJwt,
    clearJwt,
    appMode,
    setAppMode,
    effectiveRoles,
    canUsePlatformMode,
    subject,
    tenantOptions,
  } = useAppSession();
  const { token } = antdTheme.useToken();

  const visibleNavItems = React.useMemo(
    () =>
      NAV_ITEMS.filter((item) =>
        itemIsVisible(item, appMode, effectiveRoles, canUsePlatformMode),
      ),
    [appMode, canUsePlatformMode, effectiveRoles],
  );
  const breadcrumbLabel = selectedNavItem ? t(selectedNavItem.labelKey) : t("nav.dashboard");
  const modeLabel = t(`mode.${appMode}`);
  const defaultModePath = visibleNavItems[0]?.path ?? "/";
  const selectedAccess = evaluateItemAccess(
    selectedNavItem,
    appMode,
    effectiveRoles,
    canUsePlatformMode,
  );
  const hasAccess = selectedAccess.allowed;

  const accessDescription =
    selectedAccess.reason === "mode"
      ? "当前页面属于另一种工作模式。请切换模式，或返回当前模式首页。"
      : selectedAccess.reason === "platform"
        ? "当前 JWT 不具备 PlatformAdmin，不能进入平台级页面。"
        : selectedAccess.reason === "role"
          ? "当前角色不足以展示该页面入口，请检查 JWT 中的 roles / tenant_roles。"
          : "当前页面不可访问。";

  React.useEffect(() => {
    if (
      !selectedNavItem &&
      !visibleNavItems.some((item) => item.key === selectedKey) &&
      pathname !== defaultModePath
    ) {
      navigate({ to: defaultModePath, replace: true });
    }
  }, [defaultModePath, navigate, pathname, selectedKey, selectedNavItem, visibleNavItems]);

  return (
    <Layout style={{ minHeight: "100vh" }}>
      <Sider
        collapsible
        collapsed={collapsed}
        onCollapse={setCollapsed}
        theme={themeMode === "dark" ? "dark" : "light"}
        width={220}
      >
        <div
          style={{
            height: 48,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            color: token.colorText,
            fontWeight: 700,
            fontSize: collapsed ? 16 : 20,
            letterSpacing: 2,
            margin: "8px 0",
          }}
        >
          {collapsed ? "DX" : t("brand.full")}
        </div>
        {!collapsed && (
          <div
            style={{
              margin: "0 16px 12px",
              color: token.colorTextSecondary,
              fontSize: 12,
              textAlign: "center",
            }}
          >
            {modeLabel}
          </div>
        )}
        <Menu
          theme={themeMode === "dark" ? "dark" : "light"}
          mode="inline"
          selectedKeys={[selectedKey]}
          items={visibleNavItems.map((item) => ({
            key: item.key,
            icon: item.icon,
            label: t(item.labelKey),
          }))}
          onClick={({ key }) => {
            const target = visibleNavItems.find((item) => item.key === key)?.path;
            if (target) navigate({ to: target });
          }}
        />
      </Sider>

      <Layout>
        <Header
          style={{
            background: token.colorBgElevated,
            padding: "0 24px",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            borderBottom: `1px solid ${token.colorBorderSecondary}`,
          }}
        >
          <Space size="middle">
            <Breadcrumb
              items={[
                { title: t("brand") },
                { title: modeLabel },
                { title: breadcrumbLabel },
              ]}
            />
            <Segmented
              size="small"
              value={appMode}
              options={[
                { value: "tenant", label: t("mode.tenantShort") },
                {
                  value: "platform",
                  label: t("mode.platformShort"),
                  disabled: !canUsePlatformMode,
                },
              ]}
              onChange={(value) => {
                const nextMode = value as AppMode;
                setAppMode(nextMode);
                const nextPath = NAV_ITEMS.find((item) =>
                  itemIsVisible(item, nextMode, effectiveRoles, canUsePlatformMode),
                )?.path;
                if (nextPath && !NAV_ITEMS.some((item) => item.key === selectedKey && item.path === nextPath)) {
                  navigate({ to: nextPath });
                }
              }}
            />
          </Space>
          <Space size="large">
            <Select
              size="small"
              value={lang}
              onChange={(v) => setLang(v as Lang)}
              options={[
                { value: "zh", label: "中文" },
                { value: "en", label: "English" },
              ]}
              style={{ width: 120 }}
            />
            <Switch
              size="small"
              checked={themeMode === "dark"}
              onChange={(checked) => setThemeMode(checked ? "dark" : "light")}
              checkedChildren={t("theme.dark")}
              unCheckedChildren={t("theme.light")}
            />
            <React.Suspense fallback={null}>
              <AppDeferredTools
                tenantId={tenantId}
                tenantOptions={tenantOptions}
                jwt={jwt}
                userLabel={subject ?? t("user.admin")}
                profileLabel={t("user.profile")}
                logoutLabel={t("user.logout")}
                aiTooltip={t("ai.assistant")}
                setTenantId={setTenantId}
                setJwt={setJwt}
                clearJwt={clearJwt}
                showTenantShortcut={appMode === "tenant"}
                onOpenAi={() => navigate({ to: "/ai" })}
              />
            </React.Suspense>
          </Space>
        </Header>

        <Content style={{ margin: 16 }}>
          <div
            style={{
              padding: 24,
              background: token.colorBgContainer,
              borderRadius: 8,
              minHeight: 600,
            }}
          >
            {!hasAccess ? (
              <AccessGate
                title="当前页面无访问权限"
                description={accessDescription}
                roles={effectiveRoles}
                modeLabel={modeLabel}
                onGoDefault={() => navigate({ to: defaultModePath })}
                onSwitchMode={
                  canUsePlatformMode || appMode === "platform"
                    ? () => {
                        const nextMode: AppMode = appMode === "tenant" ? "platform" : "tenant";
                        setAppMode(nextMode);
                        const nextPath = NAV_ITEMS.find((item) =>
                          itemIsVisible(item, nextMode, effectiveRoles, canUsePlatformMode),
                        )?.path;
                        navigate({ to: nextPath ?? "/" });
                      }
                    : undefined
                }
                switchModeLabel={appMode === "tenant" ? "切换到平台模式" : "切换到租户模式"}
              />
            ) : (
              <Outlet />
            )}
          </div>
        </Content>
      </Layout>
    </Layout>
  );
};
