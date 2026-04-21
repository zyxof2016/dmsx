import React from "react";
import { Alert, App, Button, Card, Empty, Form, Input, Modal, Segmented, Space, Table, Typography, Spin, Tag, Checkbox } from "antd";
import { SearchOutlined } from "@ant-design/icons";
import { useAppI18n, useAppSession } from "../appProviders";
import { useRbacRoles, useTenantRbacRoles, useTenantRoleBindings, useUpsertTenantRbacRoles, useUpsertTenantRoleBindings } from "../api/hooks";
import { KNOWN_TENANT_PERMISSION_NAMES, useResourceAccess } from "../authz";
import { ReadonlyBanner } from "../components/ReadonlyBanner";
import { TerminalBlock } from "../components/TerminalBlock";
import { formatApiError } from "../api/errors";
import type { RbacRole, TenantCustomRole, TenantRoleBinding } from "../api/types";

type RoleFormValues = TenantCustomRole;
type BindingFormValues = TenantRoleBinding;

export const UsersRolesPage: React.FC = () => {
  const { t } = useAppI18n();
  const { message } = App.useApp();
  const { tenantId, appMode } = useAppSession();
  const { canWrite } = useResourceAccess(appMode === "platform" ? "platformWrite" : "tenantRbac");
  const [scopeFilter, setScopeFilter] = React.useState<"all" | "platform" | "tenant">("all");
  const [search, setSearch] = React.useState("");
  const [modalOpen, setModalOpen] = React.useState(false);
  const [editingIndex, setEditingIndex] = React.useState<number | null>(null);
  const [bindingModalOpen, setBindingModalOpen] = React.useState(false);
  const [editingBindingIndex, setEditingBindingIndex] = React.useState<number | null>(null);
  const [form] = Form.useForm<RoleFormValues>();
  const [bindingForm] = Form.useForm<BindingFormValues>();

  const { data: roles, isLoading: rolesLoading, error: rolesError, refetch } = useRbacRoles();
  const tenantRolesQuery = useTenantRbacRoles();
  const tenantBindingsQuery = useTenantRoleBindings();
  const upsertTenantRoles = useUpsertTenantRbacRoles();
  const upsertTenantBindings = useUpsertTenantRoleBindings();
  const customRoles = tenantRolesQuery.data?.custom_roles ?? [];
  const roleBindings = tenantBindingsQuery.data?.bindings ?? [];

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

  const saveCustomRoles = async (nextRoles: TenantCustomRole[]) => {
    try {
      await upsertTenantRoles.mutateAsync({ custom_roles: nextRoles });
      message.success("租户自定义角色已保存");
    } catch (error) {
      message.error(formatApiError(error));
    }
  };

  const saveRoleBindings = async (nextBindings: TenantRoleBinding[]) => {
    try {
      await upsertTenantBindings.mutateAsync({ bindings: nextBindings });
      message.success("租户用户角色绑定已保存");
    } catch (error) {
      message.error(formatApiError(error));
    }
  };

  const openCreate = () => {
    setEditingIndex(null);
    form.setFieldsValue({ name: "", description: "", permissions: [] });
    setModalOpen(true);
  };

  const openEdit = (index: number) => {
    setEditingIndex(index);
    form.setFieldsValue(customRoles[index]);
    setModalOpen(true);
  };

  const openCreateBinding = () => {
    setEditingBindingIndex(null);
    bindingForm.setFieldsValue({ subject: "", display_name: "", roles: [] });
    setBindingModalOpen(true);
  };

  const openEditBinding = (index: number) => {
    setEditingBindingIndex(index);
    bindingForm.setFieldsValue(roleBindings[index]);
    setBindingModalOpen(true);
  };

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
                {appMode === "tenant" ? (
                  <Button type="primary" onClick={openCreate} disabled={!canWrite}>
                    新建租户角色
                  </Button>
                ) : null}
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
            平台模式用于查看所有内置角色；租户模式下可维护当前租户自己的自定义角色定义。
          </Typography.Text>

          <Card size="small" title={`当前租户自定义角色 (${tenantId})`}>
            {appMode !== "tenant" ? (
              <Alert type="info" showIcon message="切到租户模式后可维护当前租户的自定义角色。" />
            ) : customRoles.length === 0 ? (
              <Empty description="当前租户还没有自定义角色" />
            ) : (
              <Table<TenantCustomRole>
                dataSource={customRoles}
                rowKey="name"
                pagination={false}
                columns={[
                  { title: "角色名", dataIndex: "name", key: "name" },
                  { title: "说明", dataIndex: "description", key: "description" },
                  {
                    title: "权限",
                    dataIndex: "permissions",
                    key: "permissions",
                    render: (permissions: string[]) => (
                      <Space wrap>
                        {permissions.map((permission) => <Tag key={permission}>{permission}</Tag>)}
                      </Space>
                    ),
                  },
                  {
                    title: "操作",
                    key: "ops",
                    render: (_: unknown, __: TenantCustomRole, index: number) => (
                      <Space>
                        <Button size="small" disabled={!canWrite} onClick={() => openEdit(index)}>
                          编辑
                        </Button>
                        <Button
                          size="small"
                          danger
                          disabled={!canWrite}
                          onClick={() => {
                            void saveCustomRoles(customRoles.filter((_, itemIndex) => itemIndex !== index));
                          }}
                        >
                          删除
                        </Button>
                      </Space>
                    ),
                  },
                ]}
              />
            )}

            <div style={{ marginTop: 16 }}>
              <Typography.Text strong>原始配置 JSON</Typography.Text>
              <TerminalBlock code={JSON.stringify({ custom_roles: customRoles }, null, 2)} style={{ marginTop: 8 }} />
            </div>
          </Card>

          <Card
            size="small"
            title="租户用户-角色绑定"
            extra={
              appMode === "tenant" ? (
                <Button type="primary" onClick={openCreateBinding} disabled={!canWrite}>
                  新建绑定
                </Button>
              ) : null
            }
          >
            {appMode !== "tenant" ? (
              <Alert type="info" showIcon message="切到租户模式后可维护当前租户的用户-角色绑定。" />
            ) : roleBindings.length === 0 ? (
              <Empty description="当前租户还没有用户-角色绑定" />
            ) : (
              <Table<TenantRoleBinding>
                dataSource={roleBindings}
                rowKey="subject"
                pagination={false}
                columns={[
                  { title: "Subject", dataIndex: "subject", key: "subject" },
                  {
                    title: "显示名",
                    dataIndex: "display_name",
                    key: "display_name",
                    render: (value?: string | null) => value || "—",
                  },
                  {
                    title: "绑定角色",
                    dataIndex: "roles",
                    key: "roles",
                    render: (roles: string[]) => <Space wrap>{roles.map((role) => <Tag key={role}>{role}</Tag>)}</Space>,
                  },
                  {
                    title: "操作",
                    key: "ops",
                    render: (_: unknown, __: TenantRoleBinding, index: number) => (
                      <Space>
                        <Button size="small" disabled={!canWrite} onClick={() => openEditBinding(index)}>
                          编辑
                        </Button>
                        <Button
                          size="small"
                          danger
                          disabled={!canWrite}
                          onClick={() => {
                            void saveRoleBindings(roleBindings.filter((_, itemIndex) => itemIndex !== index));
                          }}
                        >
                          删除
                        </Button>
                      </Space>
                    ),
                  },
                ]}
              />
            )}
          </Card>
        </Space>
      </Card>

      <Modal
        title={editingIndex === null ? "新建租户角色" : "编辑租户角色"}
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onOk={async () => {
          const values = await form.validateFields();
          const nextRoles = [...customRoles];
          if (editingIndex === null) {
            nextRoles.push(values);
          } else {
            nextRoles[editingIndex] = values;
          }
          await saveCustomRoles(nextRoles);
          setModalOpen(false);
        }}
        okButtonProps={{ disabled: !canWrite, loading: upsertTenantRoles.isPending }}
      >
        <Form form={form} layout="vertical">
          <Form.Item name="name" label="角色名" rules={[{ required: true, message: "请输入角色名" }]}>
            <Input placeholder="例如 HelpdeskOperator" disabled={editingIndex !== null} />
          </Form.Item>
          <Form.Item name="description" label="说明" rules={[{ required: true, message: "请输入角色说明" }]}>
            <Input.TextArea rows={3} />
          </Form.Item>
          <Form.Item name="permissions" label="权限集合" rules={[{ required: true, message: "至少选择一项权限" }]}>
            <Checkbox.Group style={{ width: "100%" }}>
              <Space direction="vertical" style={{ width: "100%" }}>
                {KNOWN_TENANT_PERMISSION_NAMES.map((permission) => (
                  <Checkbox key={permission} value={permission}>
                    {permission}
                  </Checkbox>
                ))}
              </Space>
            </Checkbox.Group>
          </Form.Item>
        </Form>
      </Modal>

      <Modal
        title={editingBindingIndex === null ? "新建用户角色绑定" : "编辑用户角色绑定"}
        open={bindingModalOpen}
        onCancel={() => setBindingModalOpen(false)}
        onOk={async () => {
          const values = await bindingForm.validateFields();
          const nextBindings = [...roleBindings];
          if (editingBindingIndex === null) {
            nextBindings.push(values);
          } else {
            nextBindings[editingBindingIndex] = values;
          }
          await saveRoleBindings(nextBindings);
          setBindingModalOpen(false);
        }}
        okButtonProps={{ disabled: !canWrite, loading: upsertTenantBindings.isPending }}
      >
        <Form form={bindingForm} layout="vertical">
          <Form.Item name="subject" label="Subject" rules={[{ required: true, message: "请输入 subject" }]}>
            <Input placeholder="例如 alice@example.com 或 user-123" disabled={editingBindingIndex !== null} />
          </Form.Item>
          <Form.Item name="display_name" label="显示名">
            <Input placeholder="例如 Alice" />
          </Form.Item>
          <Form.Item name="roles" label="绑定角色" rules={[{ required: true, message: "至少选择一个角色" }]}>
            <Checkbox.Group style={{ width: "100%" }}>
              <Space direction="vertical" style={{ width: "100%" }}>
                {(roles ?? []).filter((role) => role.scope === "tenant").map((role) => (
                  <Checkbox key={role.name} value={role.name}>
                    {role.name}
                  </Checkbox>
                ))}
              </Space>
            </Checkbox.Group>
          </Form.Item>
        </Form>
      </Modal>
    </Space>
  );
};
