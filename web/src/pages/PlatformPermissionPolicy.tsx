import React from "react";
import { Alert, App, Button, Card, Drawer, Form, Input, Select, Space, Spin, Switch, Table, Tag, Typography } from "antd";
import { usePlatformRbacPolicy, useUpsertPlatformRbacPolicy } from "../api/hooks";
import type { PlatformRbacPolicy } from "../api/types";
import { GuardedButton } from "../components/GuardedButton";
import { ReadonlyBanner } from "../components/ReadonlyBanner";
import { formatApiError } from "../api/errors";
import { useResourceAccess } from "../authz";

const { Title, Text } = Typography;

function policyValue(value: Record<string, unknown> | undefined): PlatformRbacPolicy {
  return {
    platform_admin_enabled: value?.platform_admin_enabled !== false,
    platform_viewer_enabled: value?.platform_viewer_enabled !== false,
    require_scope_selection: value?.require_scope_selection !== false,
    default_scope: value?.default_scope === "tenant" ? "tenant" : "platform",
    notes: typeof value?.notes === "string" ? value.notes : "",
  };
}

export const PlatformPermissionPolicyPage: React.FC = () => {
  const { message } = App.useApp();
  const { canWrite } = useResourceAccess("platformWrite");
  const [drawerOpen, setDrawerOpen] = React.useState(false);
  const [form] = Form.useForm<PlatformRbacPolicy>();
  const policyQuery = usePlatformRbacPolicy();
  const savePolicy = useUpsertPlatformRbacPolicy();
  const policy = policyValue(policyQuery.data?.value);
  const rows = [{ key: "platform.rbac.policy", ...policy }];

  React.useEffect(() => {
    form.setFieldsValue(policy);
  }, [form, policyQuery.data?.value]);

  const handleSave = async (values: PlatformRbacPolicy) => {
    try {
      await savePolicy.mutateAsync(values);
      message.success("权限策略已保存");
      setDrawerOpen(false);
    } catch (error) {
      message.error(formatApiError(error));
    }
  };

  return (
    <Space direction="vertical" style={{ width: "100%" }} size="large">
      <div>
        <Title level={4} style={{ marginBottom: 4 }}>权限策略</Title>
        <Text type="secondary">只维护平台权限策略。策略保存到全局设置 platform.rbac.policy。</Text>
      </div>

      <ReadonlyBanner visible={!canWrite} resourceLabel="权限策略" />
      {policyQuery.error && (
        <Alert type="warning" showIcon message="权限策略尚未初始化" description="保存后会创建全局配置 platform.rbac.policy。" />
      )}

      <Card title="策略列表">
        <Table
          rowKey="key"
          loading={policyQuery.isLoading}
          dataSource={rows}
          pagination={false}
          columns={[
            { title: "策略", dataIndex: "key" },
            {
              title: "PlatformAdmin",
              dataIndex: "platform_admin_enabled",
              render: (value: boolean) => <Tag color={value ? "green" : "default"}>{value ? "启用" : "停用"}</Tag>,
            },
            {
              title: "PlatformViewer",
              dataIndex: "platform_viewer_enabled",
              render: (value: boolean) => <Tag color={value ? "green" : "default"}>{value ? "启用" : "停用"}</Tag>,
            },
            { title: "默认范围", dataIndex: "default_scope", render: (value: string) => (value === "tenant" ? "租户管理" : "平台管理") },
            {
              title: "操作",
              width: 120,
              render: () => <Button size="small" type="primary" disabled={!canWrite} onClick={() => setDrawerOpen(true)}>维护</Button>,
            },
          ]}
        />
      </Card>

      <Drawer title="维护权限策略" open={drawerOpen} width={560} onClose={() => setDrawerOpen(false)}>
        <Spin spinning={policyQuery.isLoading}>
          <Form form={form} layout="vertical" onFinish={handleSave} disabled={!canWrite}>
            <Form.Item name="platform_admin_enabled" label="启用 PlatformAdmin" valuePropName="checked">
              <Switch />
            </Form.Item>
            <Form.Item name="platform_viewer_enabled" label="启用 PlatformViewer" valuePropName="checked">
              <Switch />
            </Form.Item>
            <Form.Item name="require_scope_selection" label="登录后要求选择平台 / 租户" valuePropName="checked">
              <Switch />
            </Form.Item>
            <Form.Item name="default_scope" label="默认进入范围">
              <Select
                options={[
                  { value: "platform", label: "平台管理" },
                  { value: "tenant", label: "租户管理" },
                ]}
              />
            </Form.Item>
            <Form.Item name="notes" label="策略备注">
              <Input.TextArea rows={4} placeholder="记录平台权限审批、运维策略或接入约定" />
            </Form.Item>
            <GuardedButton type="primary" htmlType="submit" loading={savePolicy.isPending} allowed={canWrite}>
              保存策略
            </GuardedButton>
          </Form>
        </Spin>
      </Drawer>
    </Space>
  );
};
