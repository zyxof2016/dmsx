import React from "react";
import { Alert, Button, Card, Empty, Input, Segmented, Space, Table, Typography, Spin, Tag } from "antd";
import { SearchOutlined } from "@ant-design/icons";
import { useAppI18n } from "../appProviders";
import { useRbacRoles } from "../api/hooks";
import { useResourceAccess } from "../authz";
import { ReadonlyBanner } from "../components/ReadonlyBanner";
import type { RbacRole } from "../api/types";

type UserRow = {
  id: string;
  name: string;
  tenant: string;
  roles: string[];
};

export const UsersRolesPage: React.FC = () => {
  const { t } = useAppI18n();
  const { canWrite } = useResourceAccess("platformWrite");
  const [scopeFilter, setScopeFilter] = React.useState<"all" | "platform" | "tenant">("all");
  const [search, setSearch] = React.useState("");

  const { data: roles, isLoading: rolesLoading, error: rolesError, refetch } = useRbacRoles();

  const [rows] = React.useState<UserRow[]>([
    {
      id: "u-1",
      name: "demo-user",
      tenant: "00000000-0000-0000-0000-000000000001",
      roles: ["TenantAdmin"],
    },
  ]);

  const filteredRoles = React.useMemo(() => {
    const keyword = search.trim().toLowerCase();
    return (roles ?? []).filter((role) => {
      if (scopeFilter !== "all" && role.scope !== scopeFilter) return false;
      if (!keyword) return true;
      return [role.name, role.scope, role.description].some((value) =>
        value.toLowerCase().includes(keyword),
      );
    });
  }, [roles, scopeFilter, search]);

  const groupedRoles = React.useMemo(() => {
    return {
      platform: filteredRoles.filter((role) => role.scope === "platform"),
      tenant: filteredRoles.filter((role) => role.scope === "tenant"),
    };
  }, [filteredRoles]);

  const roleColumns = [
    { title: "角色", dataIndex: "name", key: "name" },
    { title: "说明", dataIndex: "description", key: "description" },
    {
      title: "平台权限",
      key: "platformAccess",
      render: (_: unknown, row: RbacRole) => `${row.platform_read ? "读" : "-"}${row.platform_write ? "/写" : ""}`,
    },
    {
      title: "租户权限",
      key: "tenantAccess",
      render: (_: unknown, row: RbacRole) => `${row.tenant_read ? "读" : "-"}${row.tenant_write ? "/写" : ""}`,
    },
  ];

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
                <Input
                  prefix={<SearchOutlined />}
                  placeholder="搜索角色名或说明"
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  allowClear
                  style={{ width: 260 }}
                />
                <Segmented
                  value={scopeFilter}
                  onChange={(value) => setScopeFilter(value as "all" | "platform" | "tenant")}
                  options={[
                    { label: "全部", value: "all" },
                    { label: "平台", value: "platform" },
                    { label: "租户", value: "tenant" },
                  ]}
                />
              </Space>
              <Space wrap>
                {filteredRoles.map((r) => (
                  <Tag key={r.name}>{r.name}</Tag>
                ))}
                {!rolesLoading && filteredRoles.length === 0 && (
                  <Typography.Text type="secondary">暂无数据</Typography.Text>
                )}
              </Space>
              {(filteredRoles.length ?? 0) > 0 && (
                <Space direction="vertical" style={{ width: "100%" }} size="large">
                  {(scopeFilter === "all" || scopeFilter === "platform") && groupedRoles.platform.length > 0 && (
                    <Card size="small" title="平台角色">
                      <Table size="small" pagination={false} rowKey="name" dataSource={groupedRoles.platform} columns={roleColumns} />
                    </Card>
                  )}
                  {(scopeFilter === "all" || scopeFilter === "tenant") && groupedRoles.tenant.length > 0 && (
                    <Card size="small" title="租户角色">
                      <Table size="small" pagination={false} rowKey="name" dataSource={groupedRoles.tenant} columns={roleColumns} />
                    </Card>
                  )}
                </Space>
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
