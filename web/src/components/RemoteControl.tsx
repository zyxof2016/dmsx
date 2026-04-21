import React, { useMemo, useState } from "react";
import {
  Card,
  Row,
  Col,
  Button,
  Select,
  InputNumber,
  Table,
  Tag,
  Modal,
  Input,
  Space,
  Alert,
  Spin,
  Tooltip,
  App,
  Typography,
  Form,
} from "antd";
import {
  ReloadOutlined,
  LockOutlined,
  PoweroffOutlined,
  DeleteOutlined,
  CodeOutlined,
  CloudUploadOutlined,
  FileSearchOutlined,
  ExclamationCircleFilled,
  PlayCircleOutlined,
} from "@ant-design/icons";
import dayjs from "dayjs";
import {
  useDeviceAction,
  useDeviceCommands,
  useCommandResult,
  useArtifacts,
} from "../api/hooks";
import type { Artifact, Device, DeviceActionType, Command } from "../api/types";
import { formatApiError } from "../api/errors";
import { useResourceAccess, WRITE_DISABLED_REASON } from "../authz";
import { GuardedButton } from "./GuardedButton";
import { ReadonlyBanner } from "./ReadonlyBanner";
import { TerminalBlock } from "./TerminalBlock";
import { buildArtifactLabel, chooseArtifactCommand, inferInstallerKind, normalizeDevicePlatform } from "../artifactMeta";

const { TextArea } = Input;
const { Text } = Typography;

type InstallUpdateForm = {
  artifactId?: string;
  downloadUrl?: string;
  sha256?: string;
  expectedVersion?: string;
  installerKind?: string;
  installCommand?: string;
  interpreter?: string;
  timeout?: number;
};

function defaultInterpreter(installerKind?: string): string | undefined {
  if (!installerKind) return undefined;
  if (["msi", "exe", "ps1"].includes(installerKind)) return "powershell";
  if (["sh", "deb", "rpm", "pkg", "apk"].includes(installerKind)) return "sh";
  return undefined;
}

function buildInstallParamsFromArtifact(artifact: Artifact, devicePlatform?: Device["platform"]): InstallUpdateForm {
  const platform = normalizeDevicePlatform(devicePlatform);
  const installCommand = chooseArtifactCommand(artifact, "upgrade", platform)
    ?? chooseArtifactCommand(artifact, "install", platform);
  const downloadUrl = typeof artifact.metadata?.download_url === "string"
    ? artifact.metadata.download_url
    : undefined;
  const installerKind = inferInstallerKind(artifact);
  return {
    artifactId: artifact.id,
    downloadUrl,
    sha256: artifact.sha256,
    expectedVersion: artifact.version,
    installerKind,
    installCommand,
    interpreter: defaultInterpreter(installerKind),
    timeout: 900,
  };
}

interface ActionDef {
  type: DeviceActionType;
  label: string;
  icon: React.ReactNode;
  danger?: boolean;
  description: string;
}

const ACTIONS: ActionDef[] = [
  { type: "reboot", label: "重启", icon: <ReloadOutlined />, description: "远程重启设备" },
  { type: "lock_screen", label: "锁屏", icon: <LockOutlined />, description: "锁定设备屏幕" },
  { type: "shutdown", label: "关机", icon: <PoweroffOutlined />, description: "远程关闭设备" },
  { type: "collect_logs", label: "收集日志", icon: <FileSearchOutlined />, description: "收集系统和Agent日志" },
  { type: "install_update", label: "安装更新", icon: <CloudUploadOutlined />, description: "推送并安装Agent更新" },
  { type: "wipe", label: "擦除数据", icon: <DeleteOutlined />, danger: true, description: "远程擦除设备数据（不可恢复）" },
];

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

const ResultDrawer: React.FC<{ commandId: string; onClose: () => void }> = ({
  commandId,
  onClose,
}) => {
  const { data: result, isLoading, error } = useCommandResult(commandId);
  return (
    <Modal title="执行结果" open onCancel={onClose} footer={null} width={600}>
      <Spin spinning={isLoading}>
        {error && <Alert type="warning" message="暂无执行结果" showIcon />}
        {result && (
          <>
            <Tag color={result.exit_code === 0 ? "success" : "error"}>
              exit_code: {result.exit_code ?? "N/A"}
            </Tag>
            <div style={{ marginTop: 12 }}>
              <Text strong>stdout:</Text>
              <TerminalBlock 
                code={result.stdout || "(empty)"}
                style={{ 
                  marginTop: 4, 
                  maxHeight: 200, 
                  background: result.stdout ? "#f6ffed" : undefined,
                  borderColor: result.stdout ? "#b7eb8f" : undefined,
                }}
              />
            </div>
            <div style={{ marginTop: 8 }}>
              <Text strong>stderr:</Text>
              <TerminalBlock 
                code={result.stderr || "(empty)"}
                style={{ 
                  marginTop: 4, 
                  maxHeight: 200, 
                  background: result.stderr ? "#fff2f0" : undefined,
                  borderColor: result.stderr ? "#ffa39e" : undefined,
                }}
              />
            </div>
            <Text type="secondary" style={{ fontSize: 12 }}>
              上报时间: {dayjs(result.reported_at).format("YYYY-MM-DD HH:mm:ss")}
            </Text>
          </>
        )}
      </Spin>
    </Modal>
  );
};

export const RemoteControlPanel: React.FC<{
  deviceId: string;
  deviceHostname?: string;
  devicePlatform?: Device["platform"];
  initialInstallUpdateArtifactId?: string;
  installUpdateTrigger?: number;
}> = ({ deviceId, deviceHostname, devicePlatform, initialInstallUpdateArtifactId, installUpdateTrigger }) => {
  const sendAction = useDeviceAction();
  const { message, modal } = App.useApp();
  const { canWrite } = useResourceAccess("commands");

  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(10);
  const [scriptOpen, setScriptOpen] = useState(false);
  const [installUpdateOpen, setInstallUpdateOpen] = useState(false);
  const [script, setScript] = useState("");
  const [interpreter, setInterpreter] = useState("bash");
  const [timeout, setScriptTimeout] = useState(60);
  const [resultCmdId, setResultCmdId] = useState<string | null>(null);
  const [installUpdateForm] = Form.useForm<InstallUpdateForm>();
  const wipeHostnameRef = React.useRef("");
  const { data: cmds, isLoading } = useDeviceCommands(deviceId, {
    limit: pageSize,
    offset: (page - 1) * pageSize,
  });
  const artifactsQuery = useArtifacts({ limit: 100, offset: 0 });
  const artifactOptions = useMemo(
    () => (artifactsQuery.data?.items ?? []).map((artifact) => ({ value: artifact.id, label: buildArtifactLabel(artifact) })),
    [artifactsQuery.data?.items],
  );
  React.useEffect(() => {
    const initialId = initialInstallUpdateArtifactId;
    if (!initialId || !artifactsQuery.data?.items?.length) return;
    const artifact = artifactsQuery.data.items.find((item) => item.id === initialId);
    if (!artifact) return;
    installUpdateForm.setFieldsValue(buildInstallParamsFromArtifact(artifact, devicePlatform));
    setInstallUpdateOpen(true);
  }, [artifactsQuery.data?.items, devicePlatform, initialInstallUpdateArtifactId, installUpdateForm, installUpdateTrigger]);

  const doAction = (action: DeviceActionType, params: Record<string, unknown> = {}) => {
    sendAction.mutate(
      { deviceId, action: { action, params } },
      {
        onSuccess: () => message.success(`操作 ${action} 已下发`),
        onError: (e) => message.error(formatApiError(e)),
      },
    );
  };

  const confirmAction = (def: ActionDef) => {
    if (!canWrite) return;
    if (def.type === "wipe") {
      wipeHostnameRef.current = "";
      const expected = deviceHostname ?? deviceId;
      modal.confirm({
        title: "高危操作确认",
        icon: <ExclamationCircleFilled />,
        content: (
          <div>
            <Alert
              type="error"
              message="此操作将擦除设备所有数据，不可恢复！"
              style={{ marginBottom: 12 }}
            />
            <Text>请输入设备主机名 <Text strong>{expected}</Text> 进行确认：</Text>
            <Input
              style={{ marginTop: 8 }}
              placeholder={expected}
              onChange={(e) => { wipeHostnameRef.current = e.target.value; }}
            />
          </div>
        ),
        okText: "确认擦除",
        okType: "danger",
        onOk: () => {
          if (wipeHostnameRef.current !== expected) {
            message.error("主机名不匹配，操作取消");
            return Promise.reject();
          }
          doAction("wipe", { confirm: true });
        },
      });
      return;
    }
    if (def.type === "run_script") {
      setScriptOpen(true);
      return;
    }
    if (def.type === "install_update") {
      setInstallUpdateOpen(true);
      return;
    }
    modal.confirm({
      title: `确认 ${def.label}?`,
      icon: <ExclamationCircleFilled />,
      content: `将对设备 ${deviceHostname ?? deviceId} 执行 ${def.description}`,
      onOk: () => doAction(def.type),
    });
  };

  const handleScriptRun = () => {
    if (!script.trim()) {
      message.warning("请输入脚本内容");
      return;
    }
    doAction("run_script", { interpreter, script, timeout });
    setScriptOpen(false);
    setScript("");
  };

  const handleInstallUpdate = async () => {
    try {
      const values = await installUpdateForm.validateFields();
      if (!values.downloadUrl?.trim()) {
        message.warning("请输入下载地址，或先从制品带入");
        return;
      }
      doAction("install_update", {
        artifact_id: values.artifactId,
        download_url: values.downloadUrl.trim(),
        sha256: values.sha256?.trim() || undefined,
        expected_version: values.expectedVersion?.trim() || undefined,
        installer_kind: values.installerKind?.trim() || undefined,
        install_command: values.installCommand?.trim() || undefined,
        interpreter: values.interpreter?.trim() || undefined,
        timeout: values.timeout ?? 900,
      });
      setInstallUpdateOpen(false);
      installUpdateForm.resetFields();
    } catch {
      return;
    }
  };

  const columns = [
    {
      title: "状态",
      dataIndex: "status",
      width: 100,
      render: (s: string) => (
        <Tag color={statusColor[s]}>{statusLabel[s] ?? s}</Tag>
      ),
    },
    {
      title: "操作",
      dataIndex: "payload",
      ellipsis: true,
      render: (p: Record<string, unknown>) => {
        const action = p?.action as string;
        return action ? <Tag>{action}</Tag> : <Text type="secondary">—</Text>;
      },
    },
    {
      title: "时间",
      dataIndex: "created_at",
      width: 170,
      render: (t: string) => dayjs(t).format("MM-DD HH:mm:ss"),
    },
    {
      title: "结果",
      dataIndex: "id",
      width: 80,
      render: (id: string, r: Command) => {
        const done = ["succeeded", "failed"].includes(r.status);
        return done ? (
          <Button size="small" type="link" onClick={() => setResultCmdId(id)}>
            查看
          </Button>
        ) : (
          <Text type="secondary">—</Text>
        );
      },
    },
  ];

  return (
    <>
      <ReadonlyBanner visible={!canWrite} resourceLabel="远控面板" />
      <Card title="快捷操作" size="small" style={{ marginBottom: 16 }}>
        <Row gutter={[12, 12]}>
          {ACTIONS.map((a) => (
            <Col key={a.type} xs={12} sm={8} md={6}>
              <Tooltip title={a.description}>
                <Button
                  block
                  icon={a.icon}
                  danger={a.danger}
                  onClick={() => confirmAction(a)}
                  loading={sendAction.isPending}
                  disabled={!canWrite}
                >
                  {a.label}
                </Button>
              </Tooltip>
            </Col>
          ))}
        </Row>
      </Card>

      <Card title="脚本执行" size="small" style={{ marginBottom: 16 }}>
        <Tooltip title={!canWrite ? WRITE_DISABLED_REASON : "编写并执行远端脚本"}>
          <GuardedButton icon={<CodeOutlined />} onClick={() => setScriptOpen(true)} allowed={canWrite}>
            打开脚本编辑器
          </GuardedButton>
        </Tooltip>
      </Card>

      <Card title="操作历史" size="small">
        <Table
          rowKey="id"
          dataSource={cmds?.items ?? []}
          columns={columns}
          loading={isLoading}
          size="small"
          pagination={{
            current: page,
            pageSize,
            total: cmds?.total ?? 0,
            size: "small",
            showSizeChanger: true,
            onChange: (nextPage, nextPageSize) => {
              setPage(nextPage);
              setPageSize(nextPageSize);
            },
          }}
        />
      </Card>

      <Modal
        title="执行脚本"
        open={scriptOpen}
        onCancel={() => setScriptOpen(false)}
        onOk={handleScriptRun}
        okText="执行"
        okButtonProps={{ icon: <PlayCircleOutlined />, disabled: !canWrite }}
        width={640}
      >
        <Space direction="vertical" style={{ width: "100%" }}>
          <Space>
            <Text>解释器：</Text>
            <Select
              value={interpreter}
              onChange={setInterpreter}
              style={{ width: 120 }}
              options={[
                { value: "bash", label: "Bash" },
                { value: "sh", label: "Sh" },
                { value: "powershell", label: "PowerShell" },
                { value: "python", label: "Python" },
              ]}
            />
            <Text>超时(秒)：</Text>
            <InputNumber
              min={10}
              max={600}
              value={timeout}
              onChange={(v) => setScriptTimeout(v ?? 60)}
            />
          </Space>
          <TextArea
            rows={12}
            value={script}
            onChange={(e) => setScript(e.target.value)}
            placeholder="输入要在远端设备上执行的脚本..."
            style={{ fontFamily: "monospace", fontSize: 12 }}
          />
        </Space>
      </Modal>

      <Modal
        title="安装更新"
        open={installUpdateOpen}
        onCancel={() => setInstallUpdateOpen(false)}
        onOk={handleInstallUpdate}
        okText="下发更新"
        okButtonProps={{ disabled: !canWrite, icon: <CloudUploadOutlined /> }}
        width={720}
      >
        <Space direction="vertical" style={{ width: "100%" }}>
          <Alert
            type="info"
            showIcon
            message="Agent 现在会下载更新包、按可选 SHA256 校验，并执行默认安装器或自定义安装命令。"
          />
          <Form form={installUpdateForm} layout="vertical" initialValues={{ timeout: 900 }}>
            <Form.Item label="从制品带入" name="artifactId">
              <Select
                allowClear
                showSearch
                placeholder="选择已上传制品，自动带入下载地址/校验值/安装命令"
                options={artifactOptions}
                loading={artifactsQuery.isLoading}
                filterOption={(input, option) => String(option?.label ?? "").toLowerCase().includes(input.toLowerCase())}
                  onChange={(artifactId) => {
                    const artifact = artifactsQuery.data?.items.find((item) => item.id === artifactId);
                    if (!artifact) return;
                  installUpdateForm.setFieldsValue(buildInstallParamsFromArtifact(artifact, devicePlatform));
                }}
              />
            </Form.Item>
            <Form.Item name="downloadUrl" label="下载地址" rules={[{ required: true, message: "请输入下载地址" }]}>
              <Input placeholder="https://downloads.example.com/dmsx-agent/update.sh" />
            </Form.Item>
            <Form.Item
              name="sha256"
              label="SHA256"
              rules={[
                {
                  pattern: /^$|^[0-9a-fA-F]{64}$/,
                  message: "必须为空或 64 位十六进制字符串",
                },
              ]}
            >
              <Input placeholder="可选，建议填写" />
            </Form.Item>
            <Form.Item name="expectedVersion" label="期望版本">
              <Input placeholder="例如 1.2.3；用于升级后版本确认" />
            </Form.Item>
            <Form.Item name="installerKind" label="安装器类型">
              <Select
                allowClear
                options={[
                  { value: "sh", label: "sh 脚本" },
                  { value: "ps1", label: "PowerShell 脚本" },
                  { value: "msi", label: "MSI" },
                  { value: "exe", label: "EXE" },
                  { value: "deb", label: "DEB" },
                  { value: "rpm", label: "RPM" },
                  { value: "pkg", label: "macOS PKG" },
                  { value: "apk", label: "Android APK" },
                ]}
              />
            </Form.Item>
            <Form.Item name="interpreter" label="自定义安装命令解释器">
              <Select
                allowClear
                options={[
                  { value: "sh", label: "Sh" },
                  { value: "bash", label: "Bash" },
                  { value: "powershell", label: "PowerShell" },
                ]}
              />
            </Form.Item>
            <Form.Item
              name="installCommand"
              label="自定义安装命令"
              tooltip="可选。可使用 {{file_path}}、{{download_url}}、{{sha256}} 占位符。留空则按安装器类型走 Agent 默认安装命令。"
            >
              <TextArea rows={4} placeholder="sh {{file_path}} --tenant example" />
            </Form.Item>
            <Form.Item name="timeout" label="超时(秒)">
              <InputNumber min={60} max={3600} style={{ width: 160 }} />
            </Form.Item>
          </Form>
        </Space>
      </Modal>

      {resultCmdId && (
        <ResultDrawer
          commandId={resultCmdId}
          onClose={() => setResultCmdId(null)}
        />
      )}
    </>
  );
};
