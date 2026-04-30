import React from "react";
import { Card, Descriptions, Drawer, Space, Table, Typography } from "antd";

const { Title, Text } = Typography;

type MenuAccessRow = {
  menu: string;
  path: string;
  permission: string;
  role: string;
};

const MENU_ACCESS_ROWS: MenuAccessRow[] = [
  { menu: "平台总览", path: "/platform", permission: "platform.read", role: "PlatformAdmin / PlatformViewer" },
  { menu: "权限总览", path: "/platform/permissions", permission: "platform.read", role: "PlatformAdmin / PlatformViewer" },
  { menu: "角色管理", path: "/platform/permissions/roles", permission: "platform.read", role: "PlatformAdmin / PlatformViewer" },
  { menu: "用户管理", path: "/platform/permissions/users", permission: "platform.read", role: "PlatformAdmin / PlatformViewer" },
  { menu: "菜单管理", path: "/platform/permissions/menus", permission: "platform.read", role: "PlatformAdmin / PlatformViewer" },
  { menu: "权限策略", path: "/platform/permissions/policy", permission: "platform.write", role: "PlatformAdmin" },
  { menu: "租户管理", path: "/platform/tenants", permission: "platform.write", role: "PlatformAdmin" },
  { menu: "配额治理", path: "/platform/quotas", permission: "platform.write", role: "PlatformAdmin" },
  { menu: "全局审计", path: "/platform/audit", permission: "platform.read", role: "PlatformAdmin / PlatformViewer" },
  { menu: "运行健康", path: "/platform/health", permission: "platform.read", role: "PlatformAdmin / PlatformViewer" },
  { menu: "系统设置", path: "/settings", permission: "platform.write", role: "PlatformAdmin" },
];

export const PlatformPermissionMenusPage: React.FC = () => {
  const [activeMenu, setActiveMenu] = React.useState<MenuAccessRow | null>(null);

  return (
    <Space direction="vertical" style={{ width: "100%" }} size="large">
      <div>
        <Title level={4} style={{ marginBottom: 4 }}>菜单管理</Title>
        <Text type="secondary">只维护菜单信息。当前菜单权限来自前端导航配置，先以查看和审计为主。</Text>
      </div>

      <Card title="平台菜单列表">
        <Table<MenuAccessRow>
          rowKey="path"
          dataSource={MENU_ACCESS_ROWS}
          pagination={false}
          columns={[
            { title: "菜单", dataIndex: "menu", width: 160 },
            { title: "路径", dataIndex: "path", render: (value: string) => <Typography.Text code>{value}</Typography.Text> },
            { title: "权限点", dataIndex: "permission", width: 150 },
            { title: "允许角色", dataIndex: "role", width: 220 },
            { title: "操作", width: 120, render: (_, row) => <a onClick={() => setActiveMenu(row)}>详情</a> },
          ]}
        />
      </Card>

      <Drawer title={activeMenu?.menu} open={Boolean(activeMenu)} width={560} onClose={() => setActiveMenu(null)}>
        {activeMenu && (
          <Descriptions bordered column={1} size="small">
            <Descriptions.Item label="菜单">{activeMenu.menu}</Descriptions.Item>
            <Descriptions.Item label="路径">{activeMenu.path}</Descriptions.Item>
            <Descriptions.Item label="权限点">{activeMenu.permission}</Descriptions.Item>
            <Descriptions.Item label="允许角色">{activeMenu.role}</Descriptions.Item>
          </Descriptions>
        )}
      </Drawer>
    </Space>
  );
};
