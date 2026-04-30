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
  Upload,
  Checkbox,
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
  UploadOutlined,
  CopyOutlined,
} from "@ant-design/icons";
import { Outlet, useNavigate } from "@tanstack/react-router";
import dayjs from "dayjs";
import {
  useDevices,
  useCreateDevice,
  useBatchCreateDevices,
  useDeviceEnrollmentBatch,
  useDeviceEnrollmentBatches,
  useDeleteDevice,
  exportCsv,
} from "../api/hooks";
import type {
  BatchCreateDevicesResponse,
  Device,
  CreateDeviceReq,
  DeviceEnrollmentBatchResponse,
  ListParams,
} from "../api/types";
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

type BatchValidationError = {
  line: number;
  reason: string;
  raw: string;
};

const BATCH_RESULT_KEY = "dmsx.devices.batch_result";

type StoredBatchResult = {
  batchId: string;
};

function parseBatchCsv(text: string) {
  const lines = text.replace(/\r\n/g, "\n").split("\n").filter((line) => line.trim());
  if (lines.length === 0) return { items: [], errors: [] as BatchValidationError[] };

  const firstColumns = lines[0].split(",").map((part) => part.trim().toLowerCase());
  const headerAliases: Record<string, keyof CreateDeviceReq> = {
    registration_code: "registration_code",
    code: "registration_code",
    hostname: "hostname",
    host: "hostname",
    platform: "platform",
    os_version: "os_version",
    agent_version: "agent_version",
  };
  const hasHeader = firstColumns.some((value) => value in headerAliases);
  const headerMap = hasHeader
    ? Object.fromEntries(
        firstColumns
          .map((value, index) => [index, headerAliases[value] ?? null])
          .filter((entry) => entry[1]),
      )
    : null;

  const rows = hasHeader ? lines.slice(1) : lines;
  const errors: BatchValidationError[] = [];
  const items = rows.map((raw, index) => {
    const line = raw.split(",").map((part) => part.trim());
    const mapped = headerMap
      ? Object.fromEntries(line.map((value, col) => [headerMap[col], value]))
      : {
          registration_code: line[0],
          hostname: line[1],
          platform: line[2],
          os_version: line[3],
          agent_version: line[4],
        };
    const hostname = mapped.hostname?.trim();
    const platform = mapped.platform?.trim();
    if (!hostname) {
      errors.push({ line: index + 1 + (hasHeader ? 1 : 0), reason: "缺少主机名", raw });
    }
    if (platform && !platformOptions.some((option) => option.value === platform)) {
      errors.push({
        line: index + 1 + (hasHeader ? 1 : 0),
        reason: `不支持的平台 ${platform}`,
        raw,
      });
    }
    return {
      registration_code: mapped.registration_code?.trim() || undefined,
      hostname: hostname || undefined,
      platform: (platform || "other") as CreateDeviceReq["platform"],
      os_version: mapped.os_version?.trim() || undefined,
      agent_version: mapped.agent_version?.trim() || undefined,
    };
  });

  return { items, errors };
}

function downloadTextFile(filename: string, content: string) {
  const blob = new Blob([content], { type: "text/plain;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.click();
  URL.revokeObjectURL(url);
}

export const DevicesPage: React.FC = () => {
  const navigate = useNavigate();
  const { message } = App.useApp();
  const [form] = Form.useForm<CreateDeviceReq>();
  const [open, setOpen] = useState(false);
  const [batchOpen, setBatchOpen] = useState(false);
  const [batchText, setBatchText] = useState("");
  const [storedBatch, setStoredBatch] = useState<StoredBatchResult | null>(() => {
    const raw = window.localStorage.getItem(BATCH_RESULT_KEY);
    if (!raw) return null;
    try {
      return JSON.parse(raw) as StoredBatchResult;
    } catch {
      return null;
    }
  });
  const { data: storedBatchResult } = useDeviceEnrollmentBatch(storedBatch?.batchId);
  const { data: batchHistory } = useDeviceEnrollmentBatches({ limit: 10, offset: 0 });
  const [batchResult, setBatchResult] = useState<BatchCreateDevicesResponse | DeviceEnrollmentBatchResponse | null>(null);
  const [batchIssueTokens, setBatchIssueTokens] = useState(true);
  const [batchErrors, setBatchErrors] = useState<BatchValidationError[]>([]);
  const [agentApiUrl, setAgentApiUrl] = useState(
    () => window.localStorage.getItem("dmsx.agent_api_url") || "http://127.0.0.1:8080",
  );
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
      const parsed = parseBatchCsv(batchText);
      setBatchErrors(parsed.errors);
      if (parsed.errors.length > 0) {
        message.error(`批量数据存在 ${parsed.errors.length} 处问题，请先修正`);
        return;
      }
      const result = await batchCreateMut.mutateAsync({
        items: parsed.items,
        issue_enrollment_tokens: batchIssueTokens,
        ttl_seconds: 1800,
      });
      setBatchResult(result);
      const stored = { batchId: result.batch_id };
      setStoredBatch(stored);
      window.localStorage.setItem(BATCH_RESULT_KEY, JSON.stringify(stored));
      message.success(`已批量预注册 ${result.devices.length} 台设备`);
    } catch (e: unknown) {
      message.error(formatApiError(e));
    }
  };

  React.useEffect(() => {
    if (storedBatchResult) {
      setBatchResult(storedBatchResult);
    }
  }, [storedBatchResult]);

  const buildEnrollmentUri = (tenantId: string, token: string) => {
    const params = new URLSearchParams({
      api_url: agentApiUrl,
      tenant_id: tenantId,
      enrollment_token: token,
      mode: "zero-touch",
    });
    return `dmsx://enroll?${params.toString()}`;
  };

  const buildAgentCommand = (tenantId: string, token: string) =>
    `DMSX_API_URL=${agentApiUrl} DMSX_TENANT_ID=${tenantId} DMSX_DEVICE_ENROLLMENT_TOKEN='${token}' cargo run -p dmsx-agent`;

  const buildZeroTouchScript = () => {
    if (!batchResult) return "";
    const tokenMap = new Map(
      batchResult.enrollment_tokens.map((token) => [token.device_id, token]),
    );
    return batchResult.devices
      .map((device) => {
        const token = tokenMap.get(device.id)?.token ?? "";
        return [
          `# ${device.hostname ?? device.registration_code}`,
          `export DMSX_API_URL=${agentApiUrl}`,
          `export DMSX_TENANT_ID=${device.tenant_id}`,
          `export DMSX_DEVICE_ENROLLMENT_TOKEN='${token}'`,
          `cargo run -p dmsx-agent`,
          "",
        ].join("\n");
      })
      .join("\n");
  };

  const buildWindowsScript = () => {
    if (!batchResult) return "";
    const tokenMap = new Map(
      batchResult.enrollment_tokens.map((token) => [token.device_id, token]),
    );
    return batchResult.devices
      .map((device) => {
        const token = tokenMap.get(device.id)?.token ?? "";
        return [
          `REM ${device.hostname ?? device.registration_code}`,
          `set DMSX_API_URL=${agentApiUrl}`,
          `set DMSX_TENANT_ID=${device.tenant_id}`,
          `set DMSX_DEVICE_ENROLLMENT_TOKEN=${token}`,
          `cargo run -p dmsx-agent`,
          "",
        ].join("\n");
      })
      .join("\n");
  };

  const buildAndroidScript = () => {
    if (!batchResult) return "";
    const tokenMap = new Map(
      batchResult.enrollment_tokens.map((token) => [token.device_id, token]),
    );
    return batchResult.devices
      .map((device) => {
        const token = tokenMap.get(device.id)?.token ?? "";
        return [
          `# ${device.hostname ?? device.registration_code}`,
          `.\\scripts\\package-android-agent.ps1 -ApiUrl "${agentApiUrl}" -TenantId "${device.tenant_id}" -EnrollmentToken "${token}" -OutputPath ".\\target\\packages\\DMSX-Agent-Android-${device.registration_code}.apk"`,
          `adb install -r ".\\target\\packages\\DMSX-Agent-Android-${device.registration_code}.apk"`,
          "",
        ].join("\n");
      })
      .join("\n");
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
          <Button
            icon={<DownloadOutlined />}
            onClick={() =>
              downloadTextFile(
                "device-batch-template.csv",
                "registration_code,hostname,platform\nDEV-BJ-0001,BJ-KIOSK-01,windows\nDEV-BJ-0002,BJ-KIOSK-02,linux\n",
              )
            }
          >
            下载 CSV 模板
          </Button>
          <Input
            value={agentApiUrl}
            onChange={(e) => {
              const next = e.target.value;
              setAgentApiUrl(next);
              window.localStorage.setItem("dmsx.agent_api_url", next);
            }}
            placeholder="Agent API URL，例如 https://api.example.com"
          />
          <Upload
            beforeUpload={async (file) => {
              const text = await file.text();
              setBatchText(text.replace(/\r\n/g, "\n"));
              return false;
            }}
            showUploadList={false}
            accept=".csv,.txt"
          >
            <Button icon={<UploadOutlined />}>上传 CSV / TXT</Button>
          </Upload>
          <Checkbox checked={batchIssueTokens} onChange={(e) => setBatchIssueTokens(e.target.checked)}>
            同时签发 enrollment token（推荐零接触注册）
          </Checkbox>
          <Input.TextArea
            rows={10}
            value={batchText}
            onChange={(e) => setBatchText(e.target.value)}
            placeholder="DEV-BJ-0001,BJ-KIOSK-01,windows"
          />
          {batchErrors.length > 0 ? (
            <Alert
              type="error"
              showIcon
              message="批量数据校验失败"
              description={
                <Space direction="vertical">
                  {batchErrors.map((item) => (
                    <Typography.Text key={`${item.line}-${item.reason}`}>
                      第 {item.line} 行：{item.reason}，原始内容：`{item.raw || "(空行)"}`
                    </Typography.Text>
                  ))}
                </Space>
              }
            />
          ) : null}
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
                      enrollment_uri: tokenMap.get(device.id)
                        ? buildEnrollmentUri(device.tenant_id, tokenMap.get(device.id)?.token ?? "")
                        : "",
                      enrollment_command: tokenMap.get(device.id)
                        ? buildAgentCommand(device.tenant_id, tokenMap.get(device.id)?.token ?? "")
                        : "",
                    })),
                    "device-batch-enrollment.csv",
                  );
                }}
              >
                导出 token / 启动命令 CSV
              </Button>
              <Button
                onClick={() => {
                  const first = batchResult.enrollment_tokens[0];
                  const firstDevice = batchResult.devices.find((device) => device.id === first?.device_id);
                  if (!first || !firstDevice) return;
                  const url = new URL("/zero-touch-enroll", window.location.origin);
                  url.search = new URLSearchParams({
                    api_url: agentApiUrl,
                    tenant_id: firstDevice.tenant_id,
                    enrollment_token: first.token,
                    mode: "zero-touch",
                  }).toString();
                  window.open(url.toString(), "_blank", "noopener,noreferrer");
                }}
                disabled={batchResult.enrollment_tokens.length === 0}
              >
                打开零接触安装页
              </Button>
              <Button
                icon={<CopyOutlined />}
                onClick={async () => {
                  await navigator.clipboard.writeText(buildZeroTouchScript());
                  message.success("Linux/macOS 启动脚本已复制");
                }}
              >
                复制 Linux/macOS 脚本
              </Button>
              <Button
                icon={<CopyOutlined />}
                onClick={async () => {
                  await navigator.clipboard.writeText(buildWindowsScript());
                  message.success("Windows 启动脚本已复制");
                }}
              >
                复制 Windows 脚本
              </Button>
              <Button
                icon={<CopyOutlined />}
                onClick={async () => {
                  await navigator.clipboard.writeText(buildAndroidScript());
                  message.success("Android ADB 脚本已复制");
                }}
              >
                复制 Android ADB 脚本
              </Button>
              <Button
                onClick={() => {
                  setBatchResult(null);
                  window.localStorage.removeItem(BATCH_RESULT_KEY);
                  message.success("已清空批量结果缓存");
                }}
              >
                清空批量结果缓存
              </Button>
            </>
          ) : null}
          {batchHistory?.items?.length ? (
            <Card size="small" title="最近批次历史">
              <Space direction="vertical" style={{ width: "100%" }}>
                {batchHistory.items.map((batch) => (
                  <Space key={batch.batch_id} wrap>
                    <Typography.Text code>{batch.batch_id}</Typography.Text>
                    <Typography.Text>{new Date(batch.created_at).toLocaleString()}</Typography.Text>
                    <Typography.Text>{batch.devices.length} 台设备</Typography.Text>
                    <Button
                      size="small"
                      onClick={() => {
                        setStoredBatch({ batchId: batch.batch_id });
                        setBatchResult(batch);
                        window.localStorage.setItem(
                          BATCH_RESULT_KEY,
                          JSON.stringify({ batchId: batch.batch_id }),
                        );
                      }}
                    >
                      回填本批次
                    </Button>
                  </Space>
                ))}
              </Space>
            </Card>
          ) : null}
        </Space>
      </Modal>

      <Outlet />
    </>
  );
};
