import React, { useState } from "react";
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
} from "../api/hooks";
import type { DeviceActionType, Command } from "../api/types";

const { TextArea } = Input;
const { Text } = Typography;

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
              <pre style={{ background: "#f5f5f5", padding: 8, borderRadius: 4, maxHeight: 200, overflow: "auto", fontSize: 12 }}>
                {result.stdout || "(empty)"}
              </pre>
            </div>
            <div style={{ marginTop: 8 }}>
              <Text strong>stderr:</Text>
              <pre style={{ background: "#fff2f0", padding: 8, borderRadius: 4, maxHeight: 200, overflow: "auto", fontSize: 12 }}>
                {result.stderr || "(empty)"}
              </pre>
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
}> = ({ deviceId, deviceHostname }) => {
  const sendAction = useDeviceAction();
  const { data: cmds, isLoading } = useDeviceCommands(deviceId, { limit: 20 });
  const { message, modal } = App.useApp();

  const [scriptOpen, setScriptOpen] = useState(false);
  const [script, setScript] = useState("");
  const [interpreter, setInterpreter] = useState("bash");
  const [timeout, setScriptTimeout] = useState(60);
  const [resultCmdId, setResultCmdId] = useState<string | null>(null);
  const wipeHostnameRef = React.useRef("");

  const doAction = (action: DeviceActionType, params: Record<string, unknown> = {}) => {
    sendAction.mutate(
      { deviceId, action: { action, params } },
      {
        onSuccess: () => message.success(`操作 ${action} 已下发`),
        onError: (e) => message.error(String(e)),
      },
    );
  };

  const confirmAction = (def: ActionDef) => {
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
                >
                  {a.label}
                </Button>
              </Tooltip>
            </Col>
          ))}
        </Row>
      </Card>

      <Card title="脚本执行" size="small" style={{ marginBottom: 16 }}>
        <Button icon={<CodeOutlined />} onClick={() => setScriptOpen(true)}>
          打开脚本编辑器
        </Button>
      </Card>

      <Card title="操作历史" size="small">
        <Table
          rowKey="id"
          dataSource={cmds?.items ?? []}
          columns={columns}
          loading={isLoading}
          size="small"
          pagination={{ pageSize: 10, total: cmds?.total ?? 0, size: "small" }}
        />
      </Card>

      <Modal
        title="执行脚本"
        open={scriptOpen}
        onCancel={() => setScriptOpen(false)}
        onOk={handleScriptRun}
        okText="执行"
        okButtonProps={{ icon: <PlayCircleOutlined /> }}
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

      {resultCmdId && (
        <ResultDrawer
          commandId={resultCmdId}
          onClose={() => setResultCmdId(null)}
        />
      )}
    </>
  );
};
