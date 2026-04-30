import React from "react";
import { Alert, App, Button, Card, Col, Empty, Form, Input, Row, Space, Table, Typography } from "antd";
import { PlusOutlined, SearchOutlined } from "@ant-design/icons";
import dayjs from "dayjs";
import { useNavigate } from "@tanstack/react-router";
import { useCreateTenant, usePlatformTenantsList } from "../api/hooks";
import type { PlatformTenantSummary, ListParams } from "../api/types";
import { formatApiError } from "../api/errors";
import { useResourceAccess } from "../authz";
import { ReadonlyBanner } from "../components/ReadonlyBanner";
import { useAppSession } from "../appProviders";
import { GuardedButton } from "../components/GuardedButton";

export const PlatformTenantsPage: React.FC = () => {
  const { message } = App.useApp();
  const navigate = useNavigate();
  const { canWrite } = useResourceAccess("platformWrite");
  const { setTenantId, setAppMode } = useAppSession();
  const [form] = Form.useForm<{ name: string }>();
  const [search, setSearch] = React.useState("");
  const [page, setPage] = React.useState(1);
  const [pageSize, setPageSize] = React.useState(10);
  const createTenant = useCreateTenant();

  const params: ListParams = {
    limit: pageSize,
    offset: (page - 1) * pageSize,
    search: search || undefined,
  };

  const { data, isLoading, error, refetch } = usePlatformTenantsList(params);
  const items = data?.items ?? [];
  const total = data?.total ?? 0;

  const openTenant = (tenantId: string) => {
    setTenantId(tenantId);
    setAppMode("tenant");
    navigate({ to: "/devices" });
  };

  const handleCreateTenant = async (values: { name: string }) => {
    try {
      await createTenant.mutateAsync(values);
      message.success("租户创建成功");
      form.resetFields();
      setPage(1);
      await refetch();
    } catch (error) {
      message.error(formatApiError(error));
    }
  };

  return (
    <Space direction="vertical" style={{ width: "100%" }} size="large">
      <div>
        <Typography.Title level={4}>租户管理</Typography.Title>
        <Typography.Text type="secondary">
          集中维护平台租户目录。创建租户、检索租户和切换到租户工作台都在这里完成。
        </Typography.Text>
      </div>
      <ReadonlyBanner visible={!canWrite} resourceLabel="平台租户目录" />
      {error && <Alert type="error" showIcon message="加载失败" description={formatApiError(error)} />}

      <Row gutter={[16, 16]}>
        <Col xs={24} lg={8}>
          <Card title="创建租户">
            <Form form={form} layout="vertical" onFinish={handleCreateTenant} disabled={!canWrite}>
              <Form.Item
                name="name"
                label="租户名称"
                rules={[
                  { required: true, message: "请输入租户名称" },
                  { max: 200, message: "最长 200 字符" },
                ]}
              >
                <Input placeholder="例如：华南大区 / partner-a / customer-prod" />
              </Form.Item>
              <GuardedButton
                type="primary"
                icon={<PlusOutlined />}
                htmlType="submit"
                loading={createTenant.isPending}
                allowed={canWrite}
              >
                创建租户
              </GuardedButton>
            </Form>
          </Card>
        </Col>
        <Col xs={24} lg={16}>
          <Card title="租户目录摘要">
            <Row gutter={16}>
              <Col span={8}>
                <Typography.Text type="secondary">租户总数</Typography.Text>
                <Typography.Title level={3} style={{ margin: 0 }}>
                  {total}
                </Typography.Title>
              </Col>
              <Col span={8}>
                <Typography.Text type="secondary">当前页租户</Typography.Text>
                <Typography.Title level={3} style={{ margin: 0 }}>
                  {items.length}
                </Typography.Title>
              </Col>
              <Col span={8}>
                <Typography.Text type="secondary">创建权限</Typography.Text>
                <Typography.Title level={3} style={{ margin: 0 }}>
                  {canWrite ? "可写" : "只读"}
                </Typography.Title>
              </Col>
            </Row>
          </Card>
        </Col>
      </Row>

      <Card title="租户目录">
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
