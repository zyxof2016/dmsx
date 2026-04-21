import React from "react";
import { Alert, Button, Card, Empty, Input, Space, Table, Typography } from "antd";
import { SearchOutlined } from "@ant-design/icons";
import dayjs from "dayjs";
import { useNavigate } from "@tanstack/react-router";
import { usePlatformTenantsList } from "../api/hooks";
import type { PlatformTenantSummary, ListParams } from "../api/types";
import { formatApiError } from "../api/errors";
import { setStoredTenantId } from "../api/client";
import { useResourceAccess } from "../authz";
import { ReadonlyBanner } from "../components/ReadonlyBanner";

export const PlatformTenantsPage: React.FC = () => {
  const navigate = useNavigate();
  const { canWrite } = useResourceAccess("platformWrite");
  const [search, setSearch] = React.useState("");
  const [page, setPage] = React.useState(1);
  const [pageSize, setPageSize] = React.useState(10);

  const params: ListParams = {
    limit: pageSize,
    offset: (page - 1) * pageSize,
    search: search || undefined,
  };

  const { data, isLoading, error, refetch } = usePlatformTenantsList(params);
  const items = data?.items ?? [];
  const total = data?.total ?? 0;

  const openTenant = (tenantId: string) => {
    setStoredTenantId(tenantId);
    navigate({ to: "/devices" });
  };

  return (
    <Space direction="vertical" style={{ width: "100%" }}>
      <Typography.Title level={4}>平台租户目录</Typography.Title>
      <ReadonlyBanner visible={!canWrite} resourceLabel="平台租户目录" />
      {error && <Alert type="error" showIcon message="加载失败" description={formatApiError(error)} />}
      <Card>
        <Space style={{ marginBottom: 16 }} wrap>
          <Input
            prefix={<SearchOutlined />}
            value={search}
            onChange={(e) => {
              setSearch(e.target.value);
              setPage(1);
            }}
            placeholder="搜索租户名称或 UUID"
            allowClear
            style={{ width: 280 }}
          />
          <Button onClick={() => refetch()}>刷新</Button>
        </Space>
        <Table<PlatformTenantSummary>
          loading={isLoading}
          rowKey={(row) => row.id}
          dataSource={items}
          locale={{ emptyText: <Empty description="暂无租户数据" /> }}
          pagination={{
            current: page,
            pageSize,
            total,
            showSizeChanger: true,
            showTotal: (value) => `共 ${value} 个租户`,
            onChange: (nextPage, nextPageSize) => {
              setPage(nextPage);
              setPageSize(nextPageSize);
            },
          }}
          columns={[
            {
              title: "租户名称",
              dataIndex: "name",
              key: "name",
              render: (value: string, row) => (
                <Button type="link" style={{ padding: 0 }} onClick={() => openTenant(row.id)}>
                  {value}
                </Button>
              ),
            },
            {
              title: "租户 ID",
              dataIndex: "id",
              key: "id",
              render: (value: string) => <Typography.Text code>{value}</Typography.Text>,
            },
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
            {
              title: "操作",
              key: "action",
              render: (_, row) => (
                <Button size="small" type="primary" ghost={!canWrite} onClick={() => openTenant(row.id)}>
                  切换并查看设备
                </Button>
              ),
            },
          ]}
        />
      </Card>
    </Space>
  );
};
