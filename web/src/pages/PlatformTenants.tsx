import React from "react";
import { Alert, Card, Empty, Space, Table, Typography } from "antd";
import dayjs from "dayjs";
import { usePlatformTenants } from "../api/hooks";
import type { PlatformTenantSummary } from "../api/types";
import { formatApiError } from "../api/errors";

export const PlatformTenantsPage: React.FC = () => {
  const { data, isLoading, error } = usePlatformTenants();

  return (
    <Space direction="vertical" style={{ width: "100%" }}>
      <Typography.Title level={4}>平台租户目录</Typography.Title>
      {error && <Alert type="error" showIcon message="加载失败" description={formatApiError(error)} />}
      <Card>
        <Table<PlatformTenantSummary>
          loading={isLoading}
          rowKey={(row) => row.id}
          dataSource={data ?? []}
          locale={{ emptyText: <Empty description="暂无租户数据" /> }}
          pagination={false}
          columns={[
            { title: "租户名称", dataIndex: "name", key: "name" },
            { title: "租户 ID", dataIndex: "id", key: "id" },
            {
              title: "设备数",
              dataIndex: "device_count",
              key: "device_count",
            },
            {
              title: "策略数",
              dataIndex: "policy_count",
              key: "policy_count",
            },
            {
              title: "命令数",
              dataIndex: "command_count",
              key: "command_count",
            },
            {
              title: "创建时间",
              dataIndex: "created_at",
              key: "created_at",
              render: (value: string) => dayjs(value).format("YYYY-MM-DD HH:mm:ss"),
            },
          ]}
        />
      </Card>
    </Space>
  );
};
