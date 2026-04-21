import React from "react";
import { Alert, Button, Card, Empty, Input, Space, Spin, Table, Typography } from "antd";
import dayjs from "dayjs";
import { usePlatformAuditLogs } from "../api/hooks";
import type { AuditLogListParams, AuditLog } from "../api/types";
import { formatApiError } from "../api/errors";

export const PlatformAuditLogsPage: React.FC = () => {
  const [action, setAction] = React.useState("");
  const [resourceType, setResourceType] = React.useState("");
  const [page, setPage] = React.useState(1);
  const [pageSize, setPageSize] = React.useState(10);

  const params: AuditLogListParams = {
    limit: pageSize,
    offset: (page - 1) * pageSize,
    action: action || undefined,
    resource_type: resourceType || undefined,
  };

  const { data, isLoading, error, refetch } = usePlatformAuditLogs(params);
  const items = data?.items ?? [];
  const total = data?.total ?? 0;

  return (
    <Space direction="vertical" style={{ width: "100%" }}>
      <Typography.Title level={4}>平台全局审计</Typography.Title>
      {error && (
        <Alert
          type="error"
          showIcon
          message="加载失败"
          description={formatApiError(error)}
          action={<Button onClick={() => refetch()}>重试</Button>}
        />
      )}
      <Card>
        <Spin spinning={isLoading}>
          <Space style={{ marginBottom: 16 }} wrap>
            <Input value={action} onChange={(e) => { setAction(e.target.value); setPage(1); }} placeholder="action" allowClear style={{ width: 220 }} />
            <Input value={resourceType} onChange={(e) => { setResourceType(e.target.value); setPage(1); }} placeholder="resource_type" allowClear style={{ width: 220 }} />
            <Button onClick={() => refetch()}>刷新</Button>
          </Space>
          <Table<AuditLog>
            rowKey={(row) => row.id}
            dataSource={items}
            locale={{ emptyText: <Empty description="暂无平台审计日志" /> }}
            pagination={{
              current: page,
              pageSize,
              total,
              showSizeChanger: true,
              showTotal: (value) => `共 ${value} 条`,
              onChange: (nextPage, nextPageSize) => {
                setPage(nextPage);
                setPageSize(nextPageSize);
              },
            }}
            columns={[
              { title: "时间", dataIndex: "created_at", key: "created_at", render: (value: string) => dayjs(value).format("YYYY-MM-DD HH:mm:ss") },
              { title: "动作", dataIndex: "action", key: "action" },
              { title: "资源类型", dataIndex: "resource_type", key: "resource_type" },
              { title: "资源 ID", dataIndex: "resource_id", key: "resource_id" },
              { title: "payload", dataIndex: "payload", key: "payload", render: (payload: Record<string, unknown>) => <Typography.Text code>{JSON.stringify(payload)}</Typography.Text> },
            ]}
          />
        </Spin>
      </Card>
    </Space>
  );
};
