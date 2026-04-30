import React from "react";
import { Alert, Button, Card, Descriptions, Drawer, Space, Table, Tag, Typography } from "antd";
import { useRbacRoles } from "../api/hooks";
import type { RbacRole } from "../api/types";
import { formatApiError } from "../api/errors";

const { Title, Text } = Typography;

export const PlatformPermissionRolesPage: React.FC = () => {
  const rolesQuery = useRbacRoles();
  const [activeRole, setActiveRole] = React.useState<RbacRole | null>(null);
  const platformRoles = (rolesQuery.data ?? []).filter((role) => role.platform_read || role.platform_write);

  return (
    <Space direction="vertical" style={{ width: "100%" }} size="large">
      <div>
        <Title level={4} style={{ marginBottom: 4 }}>角色管理</Title>
        <Text type="secondary">只维护平台角色信息。当前角色来自后端内置 RBAC 模板，页面以查看和审计为主。</Text>
      </div>

      {rolesQuery.error && <Alert type="error" showIcon message="加载角色失败" description={formatApiError(rolesQuery.error)} />}
      <Card title="平台角色列表">
        <Table<RbacRole>
          rowKey="name"
          loading={rolesQuery.isLoading}
          dataSource={platformRoles}
          pagination={false}
          columns={[
            { title: "角色", dataIndex: "name", width: 200 },
            { title: "说明", dataIndex: "description" },
            {
              title: "平台读写",
              width: 150,
              render: (_, row) => (
                <Space>
                  <Tag color={row.platform_read ? "green" : "default"}>读</Tag>
                  <Tag color={row.platform_write ? "red" : "default"}>写</Tag>
                </Space>
              ),
            },
            {
              title: "操作",
              width: 120,
              render: (_, row) => <Button size="small" onClick={() => setActiveRole(row)}>详情</Button>,
            },
          ]}
        />
      </Card>

      <Drawer title={activeRole?.name} open={Boolean(activeRole)} width={640} onClose={() => setActiveRole(null)}>
        {activeRole && (
          <Descriptions bordered column={1} size="small">
            <Descriptions.Item label="角色名">{activeRole.name}</Descriptions.Item>
            <Descriptions.Item label="说明">{activeRole.description}</Descriptions.Item>
            <Descriptions.Item label="作用域">{activeRole.scope}</Descriptions.Item>
            <Descriptions.Item label="平台权限">
              <Space>
                <Tag color={activeRole.platform_read ? "green" : "default"}>读</Tag>
                <Tag color={activeRole.platform_write ? "red" : "default"}>写</Tag>
              </Space>
            </Descriptions.Item>
            <Descriptions.Item label="权限点">
              <Space wrap>
                {activeRole.permissions.filter((item) => item.startsWith("platform.")).map((item) => <Tag key={item}>{item}</Tag>)}
              </Space>
            </Descriptions.Item>
          </Descriptions>
        )}
      </Drawer>
    </Space>
  );
};
