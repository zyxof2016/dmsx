import React from "react";
import { Alert, Card, Empty, Progress, Space, Table, Typography } from "antd";
import { usePlatformQuotas } from "../api/hooks";
import type { PlatformQuota } from "../api/types";
import { formatApiError } from "../api/errors";

const QUOTA_LABELS: Record<string, { title: string; description: string }> = {
  tenants: { title: "租户数", description: "当前平台下已创建的租户总数。" },
  devices: { title: "设备数", description: "全平台纳管设备总数。" },
  commands: { title: "命令数", description: "全平台累计命令总数。" },
  artifacts: { title: "制品数", description: "全平台已登记制品总数。" },
};

function quotaMeta(key: string) {
  return QUOTA_LABELS[key] ?? { title: key, description: "" };
}

export const PlatformQuotasPage: React.FC = () => {
  const { data, isLoading, error } = usePlatformQuotas();
  const items = data?.items ?? [];

  return (
    <Space direction="vertical" style={{ width: "100%" }}>
      <Typography.Title level={4}>平台配额</Typography.Title>
      {error && <Alert type="error" showIcon message="加载失败" description={formatApiError(error)} />}
      <Card>
        <Table<PlatformQuota>
          loading={isLoading}
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
            { title: "已用", dataIndex: "used", key: "used" },
            { title: "总量", dataIndex: "limit", key: "limit" },
            { title: "单位", dataIndex: "unit", key: "unit" },
            {
              title: "使用率",
              key: "usage",
              render: (_, row) => (
                <Progress
                  percent={row.limit > 0 ? Math.min(100, Math.round((row.used / row.limit) * 100)) : 0}
                  size="small"
                />
              ),
            },
          ]}
        />
      </Card>
    </Space>
  );
};
