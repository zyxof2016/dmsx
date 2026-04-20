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
  InputNumber,
  Select,
  App,
  Spin,
  Alert,
  Empty,
} from "antd";
import {
  PlusOutlined,
  SyncOutlined,
  DownloadOutlined,
  EyeOutlined,
  ExclamationCircleFilled,
} from "@ant-design/icons";
import { Outlet, useNavigate } from "@tanstack/react-router";
import dayjs from "dayjs";
import {
  useCommands,
  useCreateCommand,
  useDevices,
  exportCsv,
} from "../api/hooks";
import type { Command, CreateCommandReq, ListParams } from "../api/types";
import { formatApiError } from "../api/errors";

const { Title } = Typography;
const { confirm } = Modal;

const statusColor: Record<string, string> = {
  queued: "default",
  delivered: "processing",
  acked: "cyan",
  running: "processing",
  succeeded: "success",
  failed: "error",
  expired: "warning",
  cancelled: "default",
};

const statusLabel: Record<string, string> = {
  queued: "排队中",
  delivered: "已下发",
  acked: "已确认",
  running: "执行中",
  succeeded: "成功",
  failed: "失败",
  expired: "已过期",
  cancelled: "已取消",
};

export const CommandsPage: React.FC = () => {
  const navigate = useNavigate();
  const { data: devicesData } = useDevices();
  const createMut = useCreateCommand();
  const { message } = App.useApp();
  const [form] = Form.useForm();
  const [open, setOpen] = useState(false);
  const [statusFilter, setStatusFilter] = useState<string>();
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(10);

  const params: ListParams = {
    limit: pageSize,
    offset: (page - 1) * pageSize,
    status: statusFilter || undefined,
  };

  const { data, isLoading, error, refetch } = useCommands(params);
  const items = data?.items ?? [];
  const total = data?.total ?? 0;

  const deviceOptions = (devicesData?.items ?? []).map((d) => ({
    value: d.id,
    label: d.hostname ?? d.id.slice(0, 8),
  }));

  const handleCreate = async (values: {
    target_device_id: string;
    payload_str: string;
    priority?: number;
    ttl_seconds?: number;
  }) => {
    const req: CreateCommandReq = {
      target_device_id: values.target_device_id,
      payload: JSON.parse(values.payload_str || "{}"),
      priority: values.priority,
      ttl_seconds: values.ttl_seconds,
    };

    const targetDevice = devicesData?.items?.find(
      (d) => d.id === req.target_device_id,
    );
    const deviceLabel = targetDevice?.hostname ?? req.target_device_id.slice(0, 8);

    confirm({
      title: "确认下发命令",
      icon: <ExclamationCircleFilled />,
      content: (
        <div>
          <p>
            目标设备：<strong>{deviceLabel}</strong>
          </p>
          <p>
            Payload：
            <code>{values.payload_str}</code>
          </p>
          <p style={{ color: "#faad14" }}>此操作将向设备发送远程指令，请确认。</p>
        </div>
      ),
      okText: "确认下发",
      cancelText: "取消",
      onOk: async () => {
        try {
          await createMut.mutateAsync(req);
          message.success("命令已下发");
          setOpen(false);
          form.resetFields();
        } catch (e: unknown) {
          message.error(formatApiError(e));
        }
      },
    });
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
        <Title level={4}>远程命令</Title>
        <Card>
          <Space style={{ marginBottom: 16 }} wrap>
            <Select
              value={statusFilter}
              style={{ width: 140 }}
              onChange={(v) => {
                setStatusFilter(v || undefined);
                setPage(1);
              }}
              allowClear
              placeholder="全部状态"
              options={Object.entries(statusLabel).map(([k, v]) => ({
                value: k,
                label: v,
              }))}
            />
            <Button
              type="primary"
              icon={<PlusOutlined />}
              onClick={() => setOpen(true)}
            >
              下发命令
            </Button>
            <Button icon={<SyncOutlined />} onClick={() => refetch()}>
              刷新
            </Button>
            <Button
              icon={<DownloadOutlined />}
              onClick={() =>
                exportCsv(
                  items as unknown as Record<string, unknown>[],
                  "commands.csv",
                )
              }
              disabled={items.length === 0}
            >
              导出 CSV
            </Button>
          </Space>
          <Table<Command>
            rowKey="id"
            dataSource={items}
            size="small"
            locale={{
              emptyText: <Empty description="暂无命令记录" />,
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
                title: "命令 ID",
                dataIndex: "id",
                width: 100,
                render: (id: string) => (
                  <Button
                    type="link"
                    size="small"
                    style={{ padding: 0 }}
                    onClick={() =>
                      navigate({
                        to: "/commands/$commandId",
                        params: { commandId: id },
                      })
                    }
                  >
                    {id.slice(0, 8)}…
                  </Button>
                ),
              },
              {
                title: "目标设备",
                dataIndex: "target_device_id",
                render: (id: string) => {
                  const dev = devicesData?.items?.find((d) => d.id === id);
                  return dev?.hostname ?? id.slice(0, 8) + "…";
                },
              },
              {
                title: "状态",
                dataIndex: "status",
                render: (s: string) => (
                  <Tag color={statusColor[s]}>{statusLabel[s] ?? s}</Tag>
                ),
              },
              { title: "优先级", dataIndex: "priority", width: 80 },
              {
                title: "TTL",
                dataIndex: "ttl_seconds",
                width: 80,
                render: (t: number) => `${t}s`,
              },
              {
                title: "创建时间",
                dataIndex: "created_at",
                render: (t: string) => dayjs(t).format("YYYY-MM-DD HH:mm"),
              },
              {
                title: "操作",
                key: "action",
                width: 80,
                render: (_: unknown, record: Command) => (
                  <Button
                    size="small"
                    type="link"
                    icon={<EyeOutlined />}
                    onClick={() =>
                      navigate({
                        to: "/commands/$commandId",
                        params: { commandId: record.id },
                      })
                    }
                  >
                    详情
                  </Button>
                ),
              },
            ]}
          />
        </Card>
      </Spin>

      <Modal
        title="下发命令"
        open={open}
        onCancel={() => setOpen(false)}
        onOk={() => form.submit()}
        confirmLoading={createMut.isPending}
      >
        <Form form={form} layout="vertical" onFinish={handleCreate}>
          <Form.Item
            name="target_device_id"
            label="目标设备"
            rules={[{ required: true, message: "请选择设备" }]}
          >
            <Select
              showSearch
              options={deviceOptions}
              filterOption={(input, opt) =>
                (opt?.label ?? "")
                  .toLowerCase()
                  .includes(input.toLowerCase())
              }
              placeholder="选择设备…"
            />
          </Form.Item>
          <Form.Item
            name="payload_str"
            label="Payload (JSON)"
            initialValue='{"action":"ping"}'
            rules={[
              {
                validator: (_, v) => {
                  try {
                    JSON.parse(v || "{}");
                    return Promise.resolve();
                  } catch {
                    return Promise.reject("无效 JSON");
                  }
                },
              },
            ]}
          >
            <Input.TextArea rows={3} />
          </Form.Item>
          <Space>
            <Form.Item name="priority" label="优先级" initialValue={0}>
              <InputNumber min={-10} max={10} />
            </Form.Item>
            <Form.Item name="ttl_seconds" label="TTL (秒)" initialValue={3600}>
              <InputNumber min={60} max={86400} />
            </Form.Item>
          </Space>
        </Form>
      </Modal>

      <Outlet />
    </>
  );
};
