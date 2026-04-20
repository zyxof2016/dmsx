import React, { useState } from "react";
import {
  Typography,
  Table,
  Tag,
  Button,
  Space,
  Card,
  Modal,
  Form,
  Input,
  Select,
  App,
  Popconfirm,
  Spin,
  Alert,
  Empty,
} from "antd";
import {
  PlusOutlined,
  DeleteOutlined,
  SyncOutlined,
  DownloadOutlined,
  EyeOutlined,
} from "@ant-design/icons";
import { Outlet, useNavigate } from "@tanstack/react-router";
import dayjs from "dayjs";
import {
  usePolicies,
  useCreatePolicy,
  useDeletePolicy,
  exportCsv,
} from "../api/hooks";
import type { Policy, CreatePolicyReq, ListParams } from "../api/types";
import { formatApiError } from "../api/errors";
import { useResourceAccess } from "../authz";
import { GuardedButton } from "../components/GuardedButton";
import { ReadonlyBanner } from "../components/ReadonlyBanner";

const { Title } = Typography;

const scopeOptions = [
  { value: "tenant", label: "租户" },
  { value: "org", label: "组织" },
  { value: "site", label: "站点" },
  { value: "group", label: "分组" },
  { value: "label", label: "标签" },
];

export const PoliciesPage: React.FC = () => {
  const navigate = useNavigate();
  const { message } = App.useApp();
  const [form] = Form.useForm<CreatePolicyReq>();
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(10);
  const [selectedRowKeys, setSelectedRowKeys] = useState<React.Key[]>([]);
  const { canWrite } = useResourceAccess("policies");

  const params: ListParams = {
    limit: pageSize,
    offset: (page - 1) * pageSize,
    search: search || undefined,
  };

  const { data, isLoading, error, refetch } = usePolicies(params);
  const createMut = useCreatePolicy();
  const deleteMut = useDeletePolicy();

  const items = data?.items ?? [];
  const total = data?.total ?? 0;

  const handleCreate = async (values: CreatePolicyReq) => {
    try {
      await createMut.mutateAsync(values);
      message.success("策略创建成功");
      setOpen(false);
      form.resetFields();
    } catch (e: unknown) {
      message.error(formatApiError(e));
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteMut.mutateAsync(id);
      message.success("已删除");
    } catch (e: unknown) {
      message.error(formatApiError(e));
    }
  };

  const handleBatchDelete = async () => {
    for (const id of selectedRowKeys) {
      await deleteMut.mutateAsync(String(id));
    }
    setSelectedRowKeys([]);
    message.success(`已删除 ${selectedRowKeys.length} 条策略`);
  };

  if (error) {
    return (
        <Alert
          type="error"
          message="加载失败"
          description={formatApiError(error)}
          showIcon
        />
      );
  }

  return (
    <>
      <Spin spinning={isLoading}>
        <Title level={4}>策略中心</Title>
        <ReadonlyBanner visible={!canWrite} resourceLabel="策略中心" />
        <Card>
          <Space style={{ marginBottom: 16 }} wrap>
            <Input.Search
              placeholder="搜索策略名称"
              style={{ width: 220 }}
              value={search}
              onChange={(e) => {
                setSearch(e.target.value);
                setPage(1);
              }}
              allowClear
            />
            <GuardedButton
              type="primary"
              icon={<PlusOutlined />}
              onClick={() => setOpen(true)}
              allowed={canWrite}
            >
              创建策略
            </GuardedButton>
            <Button icon={<SyncOutlined />} onClick={() => refetch()}>
              刷新
            </Button>
            {canWrite && selectedRowKeys.length > 0 && (
              <Popconfirm
                title={`确认删除 ${selectedRowKeys.length} 条策略？`}
                onConfirm={handleBatchDelete}
              >
                <Button danger icon={<DeleteOutlined />}>
                  批量删除 ({selectedRowKeys.length})
                </Button>
              </Popconfirm>
            )}
            <Button
              icon={<DownloadOutlined />}
              onClick={() =>
                exportCsv(
                  items as unknown as Record<string, unknown>[],
                  "policies.csv",
                )
              }
              disabled={items.length === 0}
            >
              导出 CSV
            </Button>
          </Space>
          <Table<Policy>
            rowKey="id"
            dataSource={items}
            size="small"
            rowSelection={{
              selectedRowKeys,
              onChange: setSelectedRowKeys,
            }}
            locale={{
              emptyText: (
                <Empty description="暂无策略，点击「创建策略」添加" />
              ),
            }}
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
                title: "策略名称",
                dataIndex: "name",
                render: (name: string, record: Policy) => (
                  <Button
                    type="link"
                    size="small"
                    style={{ padding: 0 }}
                    onClick={() =>
                      navigate({
                        to: "/policies/$policyId",
                        params: { policyId: record.id },
                      })
                    }
                  >
                    {name}
                  </Button>
                ),
              },
              {
                title: "描述",
                dataIndex: "description",
                render: (d: string | null) => d ?? "—",
              },
              {
                title: "作用域",
                dataIndex: "scope_kind",
                render: (s: string) => <Tag>{s}</Tag>,
              },
              {
                title: "创建时间",
                dataIndex: "created_at",
                render: (t: string) => dayjs(t).format("YYYY-MM-DD HH:mm"),
              },
              {
                title: "操作",
                key: "action",
                width: 140,
                render: (_: unknown, record: Policy) => (
                  <Space>
                    <Button
                      size="small"
                      type="link"
                      icon={<EyeOutlined />}
                      onClick={() =>
                        navigate({
                          to: "/policies/$policyId",
                          params: { policyId: record.id },
                        })
                      }
                    >
                      详情
                    </Button>
                    {canWrite && (
                      <Popconfirm
                        title="确认删除？"
                        onConfirm={() => handleDelete(record.id)}
                      >
                        <Button
                          size="small"
                          type="link"
                          danger
                          icon={<DeleteOutlined />}
                        >
                          删除
                        </Button>
                      </Popconfirm>
                    )}
                  </Space>
                ),
              },
            ]}
          />
        </Card>
      </Spin>

      <Modal
        title="创建策略"
        open={open}
        onCancel={() => setOpen(false)}
        onOk={() => form.submit()}
        confirmLoading={createMut.isPending}
        okButtonProps={{ disabled: !canWrite }}
      >
        <Form form={form} layout="vertical" onFinish={handleCreate}>
          <Form.Item
            name="name"
            label="策略名称"
            rules={[
              { required: true, message: "请输入名称" },
              { max: 200, message: "最长 200 字符" },
            ]}
          >
            <Input />
          </Form.Item>
          <Form.Item name="description" label="描述">
            <Input.TextArea rows={2} />
          </Form.Item>
          <Form.Item
            name="scope_kind"
            label="作用域"
            rules={[{ required: true, message: "请选择作用域" }]}
          >
            <Select options={scopeOptions} />
          </Form.Item>
        </Form>
      </Modal>

      <Outlet />
    </>
  );
};
