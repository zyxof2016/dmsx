import React from "react";
import { Alert, App, Button, Card, Empty, Form, InputNumber, Progress, Space, Table, Typography } from "antd";
import {
  usePlatformQuotaSettings,
  usePlatformQuotas,
  useUpsertPlatformQuotaSettings,
} from "../api/hooks";
import type { PlatformQuota, PlatformQuotaSettings } from "../api/types";
import { formatApiError } from "../api/errors";
import { ApiError } from "../api/client";
import { ReadonlyBanner } from "../components/ReadonlyBanner";
import { useResourceAccess } from "../authz";

const QUOTA_LABELS: Record<string, { title: string; description: string }> = {
  tenants: { title: "租户数", description: "平台最多可创建的租户数量。" },
  devices: { title: "设备数", description: "全平台最多可纳管的设备数量。" },
  commands: { title: "命令数", description: "全平台命令记录容量上限。" },
  artifacts: { title: "制品数", description: "全平台可登记的 Agent / 应用制品数量。" },
};

function quotaMeta(key: string) {
  return QUOTA_LABELS[key] ?? { title: key, description: "" };
}

function settingsValue(value: Record<string, unknown> | undefined): PlatformQuotaSettings {
  const readNumber = (key: keyof PlatformQuotaSettings) => {
    const raw = value?.[key];
    return typeof raw === "number" && Number.isFinite(raw) ? raw : undefined;
  };
  return {
    tenants: readNumber("tenants"),
    devices: readNumber("devices"),
    commands: readNumber("commands"),
    artifacts: readNumber("artifacts"),
  };
}

function quotaValues(items: PlatformQuota[]): PlatformQuotaSettings {
  return Object.fromEntries(items.map((item) => [item.key, item.limit])) as PlatformQuotaSettings;
}

function isNotFound(error: unknown): boolean {
  return error instanceof ApiError && error.status === 404;
}

export const PlatformQuotasPage: React.FC = () => {
  const { message } = App.useApp();
  const [form] = Form.useForm<PlatformQuotaSettings>();
  const { canWrite } = useResourceAccess("platformWrite");
  const quotasQuery = usePlatformQuotas();
  const settingsQuery = usePlatformQuotaSettings();
  const saveMut = useUpsertPlatformQuotaSettings();
  const items = quotasQuery.data?.items ?? [];
  const settingsError = settingsQuery.error && !isNotFound(settingsQuery.error) ? settingsQuery.error : null;

  React.useEffect(() => {
    if (!items.length) return;
    const currentLimits = quotaValues(items);
    if (settingsQuery.data?.value) {
      form.setFieldsValue({ ...currentLimits, ...settingsValue(settingsQuery.data.value) });
    } else {
      form.setFieldsValue(currentLimits);
    }
  }, [form, items, settingsQuery.data?.value]);

  const handleSave = async (values: PlatformQuotaSettings) => {
    const payload = Object.fromEntries(
      Object.entries(values)
        .filter(([, value]) => typeof value === "number" && Number.isFinite(value))
        .map(([key, value]) => [key, Number(value)]),
    ) as PlatformQuotaSettings;
    try {
      await saveMut.mutateAsync(payload);
      message.success("平台配额已保存");
    } catch (error) {
      message.error(formatApiError(error));
    }
  };

  return (
    <Space direction="vertical" style={{ width: "100%" }} size="large">
      <div>
        <Typography.Title level={4}>平台配额</Typography.Title>
        <Typography.Text type="secondary">
          维护平台级容量上限。保存后会写入全局配置 `platform.quotas`，并立即影响配额展示。
        </Typography.Text>
      </div>

      <ReadonlyBanner visible={!canWrite} resourceLabel="平台配额" />
      {(quotasQuery.error || settingsError) && (
        <Alert
          type="error"
          showIcon
          message="加载失败"
          description={formatApiError(quotasQuery.error ?? settingsError)}
        />
      )}

      <Card title="配额维护">
        <Form form={form} layout="vertical" onFinish={handleSave} disabled={!canWrite}>
          <Table<PlatformQuota>
            loading={quotasQuery.isLoading || settingsQuery.isLoading}
            rowKey={(row) => row.key}
            dataSource={items}
            locale={{ emptyText: <Empty description="暂无配额数据" /> }}
            pagination={false}
            columns={[
              {
                title: "配额项",
                dataIndex: "key",
                key: "key",
                render: (value: string) => {
                  const meta = quotaMeta(value);
                  return (
                    <Space direction="vertical" size={0}>
                      <Typography.Text strong>{meta.title}</Typography.Text>
                      {meta.description ? (
                        <Typography.Text type="secondary">{meta.description}</Typography.Text>
                      ) : null}
                    </Space>
                  );
                },
              },
              { title: "已用", dataIndex: "used", key: "used", width: 110 },
              {
                title: "上限",
                dataIndex: "limit",
                key: "limit",
                width: 220,
                render: (_value, row) => (
                  <Form.Item
                    name={row.key as keyof PlatformQuotaSettings}
                    style={{ marginBottom: 0 }}
                    rules={[{ required: true, message: "请输入上限" }]}
                  >
                    <InputNumber min={1} precision={0} style={{ width: "100%" }} addonAfter={row.unit} />
                  </Form.Item>
                ),
              },
              {
                title: "使用率",
                key: "usage",
                width: 220,
                render: (_, row) => (
                  <Progress
                    percent={row.limit > 0 ? Math.min(100, Math.round((row.used / row.limit) * 100)) : 0}
                    size="small"
                    status={row.limit > 0 && row.used >= row.limit ? "exception" : "normal"}
                  />
                ),
              },
            ]}
          />
          <Space style={{ marginTop: 16 }}>
            <Button type="primary" htmlType="submit" loading={saveMut.isPending} disabled={!canWrite}>
              保存配额
            </Button>
            <Button
              onClick={() => {
                form.setFieldsValue(quotaValues(items));
              }}
            >
              使用当前值
            </Button>
          </Space>
        </Form>
      </Card>
    </Space>
  );
};
