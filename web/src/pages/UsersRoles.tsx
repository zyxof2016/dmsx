import React from "react";
import { Alert, Button, Card, Empty, Space, Table, Typography, Spin, Tag } from "antd";
import { useAppI18n } from "../appProviders";
import { useRbacRoles } from "../api/hooks";
import { useResourceAccess } from "../authz";
import { ReadonlyBanner } from "../components/ReadonlyBanner";

type UserRow = {
  id: string;
  name: string;
  tenant: string;
  roles: string[];
};

export const UsersRolesPage: React.FC = () => {
  const { t } = useAppI18n();
  const { canWrite } = useResourceAccess("platformWrite");

  const { data: roles, isLoading: rolesLoading, error: rolesError, refetch } = useRbacRoles();

  const [rows] = React.useState<UserRow[]>([
    {
      id: "u-1",
      name: "demo-user",
      tenant: "00000000-0000-0000-0000-000000000001",
      roles: ["TenantAdmin"],
    },
  ]);

  return (
    <Space direction="vertical" style={{ width: "100%" }}>
      <Typography.Title level={4}>{t("page.usersRoles")}</Typography.Title>
      <ReadonlyBanner visible={!canWrite} resourceLabel="用户 / 角色管理" />

      <Alert
        type="info"
        showIcon
        message="后端已提供 RBAC 角色清单；用户/角色 CRUD 仍未实现"
      />

      <Card>
        <Space direction="vertical" style={{ width: "100%" }}>
          {rolesError && (
            <Alert
              type="error"
              showIcon
              message="加载角色清单失败"
              description={String(rolesError)}
              action={<Button onClick={() => refetch()}>重试</Button>}
            />
          )}

          <Typography.Text strong>内置 RBAC 角色</Typography.Text>
          <Spin spinning={rolesLoading}>
            <Space direction="vertical" style={{ width: "100%" }}>
              <Space wrap>
                {(roles ?? []).map((r) => (
                  <Tag key={r.name}>{r.name}</Tag>
                ))}
                {!rolesLoading && (roles?.length ?? 0) === 0 && (
                  <Typography.Text type="secondary">暂无数据</Typography.Text>
                )}
              </Space>
              {(roles?.length ?? 0) > 0 && (
                <Table
                  size="small"
                  pagination={false}
                  rowKey="name"
                  dataSource={roles}
                  columns={[
                    { title: "角色", dataIndex: "name", key: "name" },
                    { title: "范围", dataIndex: "scope", key: "scope" },
                    { title: "说明", dataIndex: "description", key: "description" },
                    {
                      title: "平台权限",
                      key: "platformAccess",
                      render: (_, row) => `${row.platform_read ? "读" : "-"}${row.platform_write ? "/写" : ""}`,
                    },
                    {
                      title: "租户权限",
                      key: "tenantAccess",
                      render: (_, row) => `${row.tenant_read ? "读" : "-"}${row.tenant_write ? "/写" : ""}`,
                    },
                  ]}
                />
              )}
            </Space>
          </Spin>

          <Typography.Text type="secondary">
            当前后端尚未提供用户/角色管理的 HTTP API；此页面先提供 UI 框架与权限展示格式。
          </Typography.Text>

          <Table<UserRow>
            dataSource={rows}
            rowKey={(r) => r.id}
            pagination={false}
            locale={{ emptyText: <Empty description="暂无用户数据" /> }}
            columns={[
              { title: "用户", dataIndex: "name", key: "name" },
              { title: "租户", dataIndex: "tenant", key: "tenant" },
              {
                title: "角色",
                dataIndex: "roles",
                key: "roles",
                render: (roles: string[]) => roles.join(", "),
              },
              {
                title: "操作",
                key: "ops",
                render: () => (
                  <Space>
                    <Button size="small" disabled>
                      编辑
                    </Button>
                    <Button size="small" disabled danger>
                      删除
                    </Button>
                  </Space>
                ),
              },
            ]}
          />
        </Space>
      </Card>
    </Space>
  );
};
