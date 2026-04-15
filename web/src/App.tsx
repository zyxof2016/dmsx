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
} from "antd";
import {
  DashboardOutlined,
  DesktopOutlined,
  SafetyOutlined,
  AppstoreOutlined,
  CloudServerOutlined,
  AuditOutlined,
  SettingOutlined,
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

const { Header, Sider, Content } = Layout;

const menuItems = [
  { key: "dashboard", icon: <DashboardOutlined />, label: "态势总览" },
  { key: "devices", icon: <DesktopOutlined />, label: "设备管理" },
  { key: "policies", icon: <SafetyOutlined />, label: "策略中心" },
  { key: "commands", icon: <CloudServerOutlined />, label: "远程命令" },
  { key: "artifacts", icon: <AppstoreOutlined />, label: "应用分发" },
  { key: "compliance", icon: <AuditOutlined />, label: "安全合规" },
  { key: "network", icon: <GlobalOutlined />, label: "网络管控" },
  { key: "ai", icon: <RobotOutlined />, label: "AI 智慧中心" },
  { key: "settings", icon: <SettingOutlined />, label: "系统设置" },
];

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
};

const pathToLabel: Record<string, string> = {
  "/": "态势总览",
  "/devices": "设备管理",
  "/policies": "策略中心",
  "/commands": "远程命令",
  "/artifacts": "应用分发",
  "/compliance": "安全合规",
  "/network": "网络管控",
  "/ai": "AI 智慧中心",
  "/settings": "系统设置",
};

export const AppLayout: React.FC = () => {
  const [collapsed, setCollapsed] = React.useState(false);
  const navigate = useNavigate();
  const pathname = useRouterState({
    select: (s) => s.location.pathname,
  });

  const topSegment = "/" + (pathname.split("/")[1] || "");
  const selectedKey =
    Object.entries(keyToPath).find(([, v]) => v === topSegment)?.[0] ??
    "dashboard";

  const breadcrumbLabel = pathToLabel[topSegment] ?? selectedKey;

  const userMenu = {
    items: [
      { key: "profile", label: "个人中心" },
      { key: "logout", label: "退出登录" },
    ],
  };

  return (
    <Layout style={{ minHeight: "100vh" }}>
      <Sider
        collapsible
        collapsed={collapsed}
        onCollapse={setCollapsed}
        theme="dark"
        width={220}
      >
        <div
          style={{
            height: 48,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            color: "#fff",
            fontWeight: 700,
            fontSize: collapsed ? 16 : 20,
            letterSpacing: 2,
            margin: "8px 0",
          }}
        >
          {collapsed ? "DX" : "DMSX 集控"}
        </div>
        <Menu
          theme="dark"
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
            background: "#fff",
            padding: "0 24px",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            borderBottom: "1px solid #f0f0f0",
          }}
        >
          <Breadcrumb
            items={[{ title: "DMSX" }, { title: breadcrumbLabel }]}
          />
          <Space size="large">
            <Badge count={0} size="small">
              <BellOutlined style={{ fontSize: 18, cursor: "pointer" }} />
            </Badge>
            <Dropdown menu={userMenu}>
              <Space style={{ cursor: "pointer" }}>
                <Avatar size="small" icon={<UserOutlined />} />
                管理员
              </Space>
            </Dropdown>
          </Space>
        </Header>

        <Content style={{ margin: 16 }}>
          <div
            style={{
              padding: 24,
              background: "#fff",
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
        tooltip="AI 助手"
        onClick={() => navigate({ to: "/ai" })}
        style={{ insetInlineEnd: 32, insetBlockEnd: 32 }}
      />
    </Layout>
  );
};
