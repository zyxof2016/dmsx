import React from "react";
import {
  App as AntApp,
  Layout,
  Menu,
  Breadcrumb,
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
  AppstoreOutlined,
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

import { useAppI18n, useThemeMode, useAppSession, type Lang } from "./appProviders";

const AppDeferredTools = React.lazy(async () => {
  const mod = await import("./components/AppDeferredTools");
  return { default: mod.AppDeferredTools };
});

const { Header, Sider, Content } = Layout;

const keyToPath: Record<string, string> = {
  dashboard: "/",
  devices: "/devices",
  policies: "/policies",
  commands: "/commands",
  artifacts: "/artifacts",
  compliance: "/compliance",
  network: "/network",
  ai: "/ai",
  settings: "/settings",
  policyEditor: "/policy-editor",
  auditLogs: "/audit-logs",
  usersRoles: "/users",
};

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

  const topSegment = "/" + (pathname.split("/")[1] || "");
  const selectedKey =
    Object.entries(keyToPath).find(([, v]) => v === topSegment)?.[0] ??
    "dashboard";

  const { t, lang, setLang } = useAppI18n();
  const { themeMode, setThemeMode } = useThemeMode();
  const { tenantId, setTenantId, jwt, setJwt, clearJwt } = useAppSession();
  const { token } = antdTheme.useToken();

  const breadcrumbLabel = t(`nav.${selectedKey}`);

  const menuItems = [
    { key: "dashboard", icon: <DashboardOutlined />, label: t("nav.dashboard") },
    { key: "devices", icon: <DesktopOutlined />, label: t("nav.devices") },
    { key: "policies", icon: <SafetyOutlined />, label: t("nav.policies") },
    { key: "commands", icon: <CloudServerOutlined />, label: t("nav.commands") },
    { key: "artifacts", icon: <AppstoreOutlined />, label: t("nav.artifacts") },
    { key: "compliance", icon: <AuditOutlined />, label: t("nav.compliance") },
    { key: "network", icon: <GlobalOutlined />, label: t("nav.network") },
    { key: "ai", icon: <RobotOutlined />, label: t("nav.ai") },
    { key: "settings", icon: <SafetyOutlined />, label: t("nav.settings") },
    { key: "policyEditor", icon: <SafetyOutlined />, label: t("nav.policyEditor") },
    { key: "auditLogs", icon: <AuditOutlined />, label: t("nav.auditLogs") },
    { key: "usersRoles", icon: <UserOutlined />, label: t("nav.usersRoles") },
  ];

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
        <Menu
          theme={themeMode === "dark" ? "dark" : "light"}
          mode="inline"
          selectedKeys={[selectedKey]}
          items={menuItems}
          onClick={({ key }) => {
            const to = keyToPath[key];
            if (to) navigate({ to });
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
          <Breadcrumb
            items={[{ title: t("brand") }, { title: breadcrumbLabel }]}
          />
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
                jwt={jwt}
                userLabel={t("user.admin")}
                profileLabel={t("user.profile")}
                logoutLabel={t("user.logout")}
                aiTooltip={t("ai.assistant")}
                setTenantId={setTenantId}
                setJwt={setJwt}
                clearJwt={clearJwt}
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
            <Outlet />
          </div>
        </Content>
      </Layout>
    </Layout>
  );
};
