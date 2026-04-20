import React from "react";
import { Alert, Button, Card, Empty, Input, Space, Table, Typography, Spin } from "antd";
import { useAppI18n } from "../appProviders";
import dayjs from "dayjs";
import { useAuditLogs } from "../api/hooks";
import type { AuditLog, AuditLogListParams } from "../api/types";
import { formatApiError } from "../api/errors";

export const AuditLogsPage: React.FC = () => {
  const { t } = useAppI18n();

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

  const { data, isLoading, error, refetch } = useAuditLogs(params);

  const items = data?.items ?? [];
  const total = data?.total ?? 0;

  return (
    <Space direction="vertical" style={{ width: "100%" }}>
      <Typography.Title level={4}>{t("page.auditLogs")}</Typography.Title>

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
            <Input
              placeholder="action（如 create/update/delete/publish）"
              style={{ width: 260 }}
              value={action}
              onChange={(e) => {
                setAction(e.target.value);
                setPage(1);
              }}
              allowClear
            />
            <Input
              placeholder="resource_type（如 policy/device/policy_revision）"
              style={{ width: 260 }}
              value={resourceType}
              onChange={(e) => {
                setResourceType(e.target.value);
                setPage(1);
              }}
              allowClear
            />
            <Button onClick={() => refetch()}>刷新</Button>
          </Space>

          <Table<AuditLog>
            dataSource={items}
            rowKey={(r) => r.id}
            size="small"
            locale={{ emptyText: <Empty description="暂无审计日志" /> }}
            pagination={{
              current: page,
              pageSize,
              total,
              showSizeChanger: true,
              showTotal: (t) => `共 ${t} 条`,
              onChange: (p, ps) => {
                setPage(p);
                setPageSize(ps);
              },
            }}
            columns={[
              {
                title: "时间",
                dataIndex: "created_at",
                key: "created_at",
                width: 170,
                render: (v: string) => dayjs(v).format("YYYY-MM-DD HH:mm:ss"),
              },
              {
                title: "操作者",
                dataIndex: "actor_user_id",
                key: "actor_user_id",
                width: 220,
                render: (v: string | null) => v ?? "—",
              },
              { title: "动作", dataIndex: "action", key: "action", width: 120 },
              {
                title: "资源类型",
                dataIndex: "resource_type",
                key: "resource_type",
                width: 160,
              },
              {
                title: "资源 ID",
                dataIndex: "resource_id",
                key: "resource_id",
                width: 220,
                render: (v: string) => (v.length > 20 ? `${v.slice(0, 20)}…` : v),
              },
              {
                title: "payload",
                dataIndex: "payload",
                key: "payload",
                render: (p: Record<string, unknown>) => (
                  <Typography.Text code>{JSON.stringify(p)}</Typography.Text>
                ),
              },
            ]}
          />
        </Spin>
      </Card>
    </Space>
  );
};
