import React from "react";
import {
  Layout,
  Menu,
  Breadcrumb,
  Avatar,
  Dropdown,
  Space,
  Badge,
  FloatButton,
  Select,
  Switch,
  Modal,
  Input,
  Tag,
  theme as antdTheme,
  message,
} from "antd";
import {
  DashboardOutlined,
  DesktopOutlined,
  SafetyOutlined,
  AppstoreOutlined,
  CloudServerOutlined,
  AuditOutlined,
  RobotOutlined,
  BellOutlined,
  UserOutlined,
  GlobalOutlined,
} from "@ant-design/icons";
import {
  Outlet,
  useNavigate,
  useRouterState,
} from "@tanstack/react-router";

import { useAppI18n, useThemeMode, useAppSession, type Lang } from "./appProviders";

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
  const [collapsed, setCollapsed] = React.useState(false);
  const [jwtModalOpen, setJwtModalOpen] = React.useState(false);
  const [tenantModalOpen, setTenantModalOpen] = React.useState(false);
  const [jwtDraft, setJwtDraft] = React.useState("");
  const [tenantDraft, setTenantDraft] = React.useState("");
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

  const shortTenantId = `${tenantId.slice(0, 8)}...${tenantId.slice(-4)}`;

  const isValidUuid = (value: string) =>
    /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(value);

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

  const userMenu = {
    items: [
      { key: "profile", label: t("user.profile") },
      { key: "set_tenant", label: "设置活动租户" },
      { key: "set_jwt", label: "设置 JWT（用于 jwt 模式联调）" },
      { key: "clear_jwt", label: "清除 JWT" },
      { key: "logout", label: t("user.logout") },
    ],
    onClick: ({ key }: { key: string }) => {
      if (key === "set_tenant") {
        setTenantDraft(tenantId);
        setTenantModalOpen(true);
      } else if (key === "set_jwt") {
        setJwtDraft(jwt);
        setJwtModalOpen(true);
      } else if (key === "clear_jwt") {
        clearJwt();
        message.success("已清除 JWT");
      }
    },
  };

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
            <Tag
              color="blue"
              style={{ cursor: "pointer", marginInlineEnd: 0 }}
              onClick={() => {
                setTenantDraft(tenantId);
                setTenantModalOpen(true);
              }}
            >
              租户 {shortTenantId}
            </Tag>
            <Badge count={0} size="small">
              <BellOutlined style={{ fontSize: 18, cursor: "pointer" }} />
            </Badge>
            <Dropdown menu={userMenu}>
              <Space style={{ cursor: "pointer" }}>
                <Avatar size="small" icon={<UserOutlined />} />
                {t("user.admin")}
              </Space>
            </Dropdown>
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

      <FloatButton
        icon={<RobotOutlined />}
        type="primary"
        tooltip={t("ai.assistant")}
        onClick={() => navigate({ to: "/ai" })}
        style={{ insetInlineEnd: 32, insetBlockEnd: 32 }}
      />

      <Modal
        title="设置 JWT"
        open={jwtModalOpen}
        onCancel={() => setJwtModalOpen(false)}
        onOk={() => {
          const v = jwtDraft.trim();
          if (!v) {
            message.error("JWT 不能为空");
            return;
          }
          setJwt(v);
          setJwtModalOpen(false);
          message.success("JWT 已保存");
        }}
        okText="保存"
        cancelText="取消"
        destroyOnClose
      >
        <Input.TextArea
          value={jwtDraft}
          onChange={(e) => setJwtDraft(e.target.value)}
          rows={6}
          placeholder="粘贴形如：xxxx.yyyy.zzzz 的 JWT（可带或不带 Bearer 前缀）"
        />
      </Modal>

      <Modal
        title="设置活动租户"
        open={tenantModalOpen}
        onCancel={() => setTenantModalOpen(false)}
        onOk={() => {
          const v = tenantDraft.trim();
          if (!isValidUuid(v)) {
            message.error("租户 ID 必须是合法 UUID");
            return;
          }
          setTenantId(v);
          setTenantModalOpen(false);
          message.success("活动租户已更新");
        }}
        okText="保存"
        cancelText="取消"
        destroyOnClose
      >
        <Input
          value={tenantDraft}
          onChange={(e) => setTenantDraft(e.target.value)}
          placeholder="输入租户 UUID，例如 00000000-0000-0000-0000-000000000001"
        />
      </Modal>
    </Layout>
  );
};
