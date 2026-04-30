import React from "react";
import { Card, Descriptions, Drawer, Space, Table, Tag, Typography } from "antd";
import { useAppSession } from "../appProviders";

const { Title, Text } = Typography;

type PlatformUserRow = {
  subject: string;
  platformRoles: string[];
  tenantCount: number;
  tenantRoleOverrides: number;
};

export const PlatformPermissionUsersPage: React.FC = () => {
  const { subject, globalRoles, permittedTenantIds, tenantRoles } = useAppSession();
  const [activeUser, setActiveUser] = React.useState<PlatformUserRow | null>(null);
  const rows: PlatformUserRow[] = [
    {
      subject: subject ?? "当前用户",
      platformRoles: globalRoles,
      tenantCount: permittedTenantIds.length,
      tenantRoleOverrides: Object.keys(tenantRoles).length,
    },
  ];

  return (
    <Space direction="vertical" style={{ width: "100%" }} size="large">
      <div>
        <Title level={4} style={{ marginBottom: 4 }}>用户管理</Title>
        <Text type="secondary">只维护用户信息。当前后端尚未提供平台用户目录，先以当前登录用户权限快照承载页面结构。</Text>
      </div>

      <Card title="平台用户列表">
        <Table<PlatformUserRow>
          rowKey="subject"
          dataSource={rows}
          pagination={false}
          columns={[
            { title: "用户", dataIndex: "subject" },
            {
              title: "平台角色",
              dataIndex: "platformRoles",
              render: (roles: string[]) => <Space wrap>{roles.length ? roles.map((role) => <Tag key={role}>{role}</Tag>) : <Tag>无</Tag>}</Space>,
            },
            { title: "允许租户", dataIndex: "tenantCount", width: 120 },
            { title: "租户角色覆盖", dataIndex: "tenantRoleOverrides", width: 140 },
            { title: "操作", width: 120, render: (_, row) => <a onClick={() => setActiveUser(row)}>详情</a> },
          ]}
        />
      </Card>

      <Drawer title={activeUser?.subject} open={Boolean(activeUser)} width={640} onClose={() => setActiveUser(null)}>
        {activeUser && (
          <Descriptions bordered column={1} size="small">
            <Descriptions.Item label="用户">{activeUser.subject}</Descriptions.Item>
            <Descriptions.Item label="平台角色">
              <Space wrap>{activeUser.platformRoles.length ? activeUser.platformRoles.map((role) => <Tag key={role}>{role}</Tag>) : <Tag>无</Tag>}</Space>
            </Descriptions.Item>
            <Descriptions.Item label="允许租户">{activeUser.tenantCount}</Descriptions.Item>
            <Descriptions.Item label="租户角色覆盖">{activeUser.tenantRoleOverrides}</Descriptions.Item>
          </Descriptions>
        )}
      </Drawer>
    </Space>
  );
};
