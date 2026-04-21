import React, { useState } from "react";
import {
  Table,
  Tag,
  Space,
  Button,
  Input,
  Typography,
  Card,
  Select,
  Modal,
  Form,
  App,
  Popconfirm,
  Spin,
  Alert,
  Empty,
} from "antd";
import {
  SearchOutlined,
  PlusOutlined,
  SyncOutlined,
  WindowsOutlined,
  AppleOutlined,
  LinuxOutlined,
  MobileOutlined,
  DeleteOutlined,
  DownloadOutlined,
  EyeOutlined,
} from "@ant-design/icons";
import { Outlet, useNavigate } from "@tanstack/react-router";
import dayjs from "dayjs";
import {
  useDevices,
  useCreateDevice,
  useBatchCreateDevices,
  useDeleteDevice,
  exportCsv,
} from "../api/hooks";
import type { BatchCreateDevicesResponse, Device, CreateDeviceReq, ListParams } from "../api/types";
import { formatApiError } from "../api/errors";
import { useResourceAccess } from "../authz";
import { GuardedButton } from "../components/GuardedButton";
import { ReadonlyBanner } from "../components/ReadonlyBanner";

const { Title } = Typography;

const platformIcon: Record<string, React.ReactNode> = {
  windows: <WindowsOutlined />,
  macos: <AppleOutlined />,
  linux: <LinuxOutlined />,
  ios: <MobileOutlined />,
  android: <MobileOutlined />,
};

const platformOptions = [
  { value: "windows", label: "Windows" },
  { value: "linux", label: "Linux" },
  { value: "macos", label: "macOS" },
  { value: "ios", label: "iOS" },
  { value: "android", label: "Android" },
  { value: "edge", label: "Edge/IoT" },
  { value: "other", label: "其他" },
];

export const DevicesPage: React.FC = () => {
  const navigate = useNavigate();
  const { message } = App.useApp();
  const [form] = Form.useForm<CreateDeviceReq>();
  const [open, setOpen] = useState(false);
  const [batchOpen, setBatchOpen] = useState(false);
  const [batchText, setBatchText] = useState("");
  const [batchResult, setBatchResult] = useState<BatchCreateDevicesResponse | null>(null);
  const [search, setSearch] = useState("");
  const [platformFilter, setPlatformFilter] = useState<string>();
  const [stateFilter, setStateFilter] = useState<string>();
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(10);
  const [selectedRowKeys, setSelectedRowKeys] = useState<React.Key[]>([]);
  const { canWrite } = useResourceAccess("devices");

  const params: ListParams = {
    limit: pageSize,
    offset: (page - 1) * pageSize,
    search: search || undefined,
    platform: platformFilter || undefined,
    online_state: stateFilter || undefined,
  };

  const { data, isLoading, error, refetch } = useDevices(params);
  const createMut = useCreateDevice();
  const batchCreateMut = useBatchCreateDevices();
  const deleteMut = useDeleteDevice();

  const items = data?.items ?? [];
  const total = data?.total ?? 0;

  const handleCreate = async (values: CreateDeviceReq) => {
    try {
      await createMut.mutateAsync(values);
      message.success("设备创建成功");
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
    message.success(`已删除 ${selectedRowKeys.length} 台设备`);
  };

  const handleBatchCreate = async () => {
    try {
      const items = batchText
        .split("\n")
        .map((line) => line.trim())
        .filter(Boolean)
        .map((line) => {
          const [registration_code, hostname, platform] = line.split(",").map((v) => v.trim());
          return {
            registration_code: registration_code || undefined,
            hostname: hostname || undefined,
            platform: (platform || "other") as CreateDeviceReq["platform"],
          };
        });
      const result = await batchCreateMut.mutateAsync({
        items,
        issue_enrollment_tokens: true,
        ttl_seconds: 1800,
      });
      setBatchResult(result);
      message.success(`已批量预注册 ${result.devices.length} 台设备`);
    } catch (e: unknown) {
      message.error(formatApiError(e));
    }
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
        <Title level={4}>设备管理</Title>
        <ReadonlyBanner visible={!canWrite} resourceLabel="设备管理" />
        <Card style={{ marginBottom: 16 }}>
          <Space wrap>
            <Input
              prefix={<SearchOutlined />}
              placeholder="搜索主机名或注册码"
              style={{ width: 220 }}
              value={search}
              onChange={(e) => {
                setSearch(e.target.value);
                setPage(1);
              }}
              allowClear
            />
            <Select
              value={platformFilter}
              style={{ width: 120 }}
              onChange={(v) => {
                setPlatformFilter(v || undefined);
                setPage(1);
              }}
              allowClear
              placeholder="全部平台"
              options={platformOptions}
            />
            <Select
              value={stateFilter}
              style={{ width: 120 }}
              onChange={(v) => {
                setStateFilter(v || undefined);
                setPage(1);
              }}
              allowClear
              placeholder="全部状态"
              options={[
                { value: "online", label: "在线" },
                { value: "offline", label: "离线" },
              ]}
            />
            <Button icon={<SyncOutlined />} onClick={() => refetch()}>
              刷新
            </Button>
            <GuardedButton
              type="primary"
              icon={<PlusOutlined />}
              onClick={() => setOpen(true)}
              allowed={canWrite}
            >
              注册设备
            </GuardedButton>
            <GuardedButton onClick={() => setBatchOpen(true)} allowed={canWrite}>
              批量预注册
            </GuardedButton>
            {canWrite && selectedRowKeys.length > 0 && (
              <>
                <Popconfirm
                  title={`确认删除 ${selectedRowKeys.length} 台设备？`}
                  onConfirm={handleBatchDelete}
                >
                  <Button danger icon={<DeleteOutlined />}>
                    批量删除 ({selectedRowKeys.length})
                  </Button>
                </Popconfirm>
              </>
            )}
            <Button
              icon={<DownloadOutlined />}
              onClick={() =>
                exportCsv(
                  items as unknown as Record<string, unknown>[],
                  "devices.csv",
                )
              }
              disabled={items.length === 0}
            >
              导出 CSV
            </Button>
          </Space>
        </Card>

        <Table<Device>
          rowKey="id"
          dataSource={items}
          size="small"
          rowSelection={{
            selectedRowKeys,
            onChange: setSelectedRowKeys,
          }}
          locale={{
            emptyText: (
              <Empty description="暂无设备，点击「注册设备」添加" />
            ),
          }}
          pagination={{
            current: page,
            pageSize,
            total,
            showSizeChanger: true,
            showTotal: (t) => `共 ${t} 台`,
            onChange: (p, ps) => {
              setPage(p);
              setPageSize(ps);
            },
          }}
          columns={[
            {
              title: "ID",
              dataIndex: "id",
              width: 100,
              render: (id: string) => (
                <Button
                  type="link"
                  size="small"
                  style={{ padding: 0 }}
                  onClick={() =>
                    navigate({
                      to: "/devices/$deviceId",
                      params: { deviceId: id },
                      search: { tab: "info" },
                    })
                  }
                >
                  {id.slice(0, 8)}…
                </Button>
              ),
            },
            {
              title: "主机名",
              dataIndex: "hostname",
              render: (h: string | null) => h ?? "—",
            },
            {
              title: "注册码",
              dataIndex: "registration_code",
              render: (value: string) => <Typography.Text code copyable>{value}</Typography.Text>,
            },
            {
              title: "平台",
              dataIndex: "platform",
              render: (p: string) => (
                <Space>
                  {platformIcon[p]}
                  {p}
                </Space>
              ),
            },
            {
              title: "系统版本",
              dataIndex: "os_version",
              render: (v: string | null) => v ?? "—",
            },
            {
              title: "状态",
              dataIndex: "online_state",
              render: (s: string) => (
                <Tag
                  color={
                    s === "online"
                      ? "green"
                      : s === "offline"
                        ? "red"
                        : "default"
                  }
                >
                  {s === "online" ? "在线" : s === "offline" ? "离线" : "未知"}
                </Tag>
              ),
            },
            {
              title: "注册状态",
              dataIndex: "enroll_status",
              render: (s: string) => (
                <Tag
                  color={
                    s === "active"
                      ? "green"
                      : s === "pending"
                        ? "gold"
                        : "red"
                  }
                >
                  {s}
                </Tag>
              ),
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
              render: (_: unknown, record: Device) => (
                <Space>
                  <Button
                    size="small"
                    type="link"
                    icon={<EyeOutlined />}
                    onClick={() =>
                      navigate({
                        to: "/devices/$deviceId",
                        params: { deviceId: record.id },
                        search: { tab: "info" },
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
      </Spin>

      <Modal
        title="注册新设备"
        open={open}
        onCancel={() => setOpen(false)}
        onOk={() => form.submit()}
        confirmLoading={createMut.isPending}
        okButtonProps={{ disabled: !canWrite }}
      >
        <Form form={form} layout="vertical" onFinish={handleCreate}>
          <Form.Item
            name="registration_code"
            label="注册码"
            rules={[{ max: 64, message: "最长 64 字符" }]}
            tooltip="建议填写设备贴纸、工单或安装包上可见的唯一注册码；留空则后端自动生成。"
          >
            <Input placeholder="例如 DEV-SH01-000123" />
          </Form.Item>
          <Form.Item
            name="hostname"
            label="主机名"
            rules={[
              { required: true, message: "请输入主机名" },
              { max: 253, message: "最长 253 字符" },
            ]}
          >
            <Input />
          </Form.Item>
          <Form.Item
            name="platform"
            label="平台"
            rules={[{ required: true, message: "请选择平台" }]}
          >
            <Select options={platformOptions} />
          </Form.Item>
          <Form.Item
            name="os_version"
            label="系统版本"
            rules={[{ max: 200, message: "最长 200 字符" }]}
          >
            <Input />
          </Form.Item>
          <Form.Item name="agent_version" label="Agent 版本">
            <Input />
          </Form.Item>
        </Form>
      </Modal>

      <Modal
        title="批量预注册设备"
        open={batchOpen}
        onCancel={() => setBatchOpen(false)}
        onOk={handleBatchCreate}
        confirmLoading={batchCreateMut.isPending}
        width={860}
        okButtonProps={{ disabled: !canWrite }}
      >
        <Space direction="vertical" style={{ width: "100%" }}>
          <Typography.Text type="secondary">
            每行一台设备，格式：`注册码,主机名,平台`。例如：`DEV-BJ-0001,BJ-KIOSK-01,windows`
          </Typography.Text>
          <Input.TextArea
            rows={10}
            value={batchText}
            onChange={(e) => setBatchText(e.target.value)}
            placeholder="DEV-BJ-0001,BJ-KIOSK-01,windows"
          />
          {batchResult ? (
            <>
              <Typography.Text strong>批量结果</Typography.Text>
              <Button
                icon={<DownloadOutlined />}
                onClick={() => {
                  const tokenMap = new Map(
                    batchResult.enrollment_tokens.map((token) => [token.device_id, token]),
                  );
                  exportCsv(
                    batchResult.devices.map((device) => ({
                      id: device.id,
                      registration_code: device.registration_code,
                      hostname: device.hostname,
                      platform: device.platform,
                      enrollment_token: tokenMap.get(device.id)?.token ?? "",
                      enrollment_command: `DMSX_API_URL=http://127.0.0.1:8080 DMSX_TENANT_ID=${device.tenant_id} DMSX_DEVICE_ENROLLMENT_TOKEN='${tokenMap.get(device.id)?.token ?? ""}' cargo run -p dmsx-agent`,
                    })),
                    "device-batch-enrollment.csv",
                  );
                }}
              >
                导出 token / 启动命令 CSV
              </Button>
            </>
          ) : null}
        </Space>
      </Modal>

      <Outlet />
    </>
  );
};
