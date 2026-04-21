import React from "react";
import {
  Drawer,
  Descriptions,
  Tag,
  Spin,
  Alert,
  Typography,
  Button,
  Tooltip,
  Card,
  Divider,
  Space,
} from "antd";
import { CopyOutlined, CheckCircleOutlined, CloseCircleOutlined, LoadingOutlined } from "@ant-design/icons";
import { useNavigate, useParams } from "@tanstack/react-router";
import dayjs from "dayjs";
import { useCommand, useDevice, useCommandResult } from "../api/hooks";
import { formatApiError } from "../api/errors";
import { TerminalBlock } from "./TerminalBlock";

const { Text } = Typography;

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

export const CommandDetailDrawer: React.FC = () => {
  const { commandId } = useParams({ strict: false });
  const navigate = useNavigate();
  const { data: command, isLoading, error } = useCommand(commandId);
  const { data: targetDevice } = useDevice(command?.target_device_id);
  const expectedVersion = command?.payload?.params && typeof command.payload.params === "object"
    ? (command.payload.params as Record<string, unknown>).expected_version
    : undefined;
  const expectedVersionText = typeof expectedVersion === "string" ? expectedVersion : null;
  const isInstallUpdate = (command?.payload?.action as string | undefined) === "install_update";
  const versionConfirmed = Boolean(expectedVersionText && targetDevice?.agent_version === expectedVersionText);
  const versionMismatch = Boolean(expectedVersionText && targetDevice?.agent_version && targetDevice.agent_version !== expectedVersionText);

  const isTerminal = command && ["succeeded", "failed", "expired", "cancelled"].includes(command.status);
  const isRunning = command && ["queued", "delivered", "acked", "running"].includes(command.status);

  const { data: result, isLoading: resultLoading } = useCommandResult(
    isTerminal ? commandId : undefined,
  );

  return (
    <Drawer
      open
      width={640}
      title="命令详情"
      onClose={() => navigate({ to: "/commands" })}
      extra={
        <Tooltip title="复制命令 ID">
          <Button
            size="small"
            icon={<CopyOutlined />}
            onClick={() => navigator.clipboard.writeText(commandId ?? "")}
          />
        </Tooltip>
      }
    >
      {error && (
        <Alert
          type="error"
          message="加载失败"
          description={formatApiError(error)}
          showIcon
        />
      )}
      <Spin spinning={isLoading}>
        {command && (
          <>
            <Descriptions column={1} bordered size="small">
              <Descriptions.Item label="ID">
                <Text code copyable={{ text: command.id }}>
                  {command.id}
                </Text>
              </Descriptions.Item>
              <Descriptions.Item label="状态">
                <Tag color={statusColor[command.status]}>
                  {statusLabel[command.status] ?? command.status}
                </Tag>
              </Descriptions.Item>
              <Descriptions.Item label="目标设备">
                <Text>
                  {targetDevice?.hostname ?? command.target_device_id}
                </Text>
              </Descriptions.Item>
              {isInstallUpdate && expectedVersionText ? (
                <Descriptions.Item label="升级版本确认">
                  <Space wrap>
                    <Text>期望版本 {expectedVersionText}</Text>
                    <Tag color={versionConfirmed ? "success" : versionMismatch ? "error" : "processing"}>
                      {versionConfirmed ? "设备已确认新版本" : versionMismatch ? `当前为 ${targetDevice?.agent_version}` : "等待设备新心跳确认"}
                    </Tag>
                  </Space>
                </Descriptions.Item>
              ) : null}
              <Descriptions.Item label="目标设备 ID">
                <Text code copyable={{ text: command.target_device_id }}>
                  {command.target_device_id}
                </Text>
              </Descriptions.Item>
              <Descriptions.Item label="优先级">
                {command.priority}
              </Descriptions.Item>
              <Descriptions.Item label="TTL">
                {command.ttl_seconds} 秒
              </Descriptions.Item>
              {command.idempotency_key && (
                <Descriptions.Item label="幂等键">
                  <Text code>{command.idempotency_key}</Text>
                </Descriptions.Item>
              )}
              <Descriptions.Item label="Payload (JSON)">
                <TerminalBlock 
                  code={JSON.stringify(command.payload, null, 2)} 
                  style={{ maxHeight: 300 }}
                />
              </Descriptions.Item>
              <Descriptions.Item label="创建时间">
                {dayjs(command.created_at).format("YYYY-MM-DD HH:mm:ss")}
              </Descriptions.Item>
              <Descriptions.Item label="更新时间">
                {dayjs(command.updated_at).format("YYYY-MM-DD HH:mm:ss")}
              </Descriptions.Item>
            </Descriptions>

            <Divider />

            <Card
              title="执行结果"
              size="small"
              extra={
                isRunning ? (
                  <Tag icon={<LoadingOutlined />} color="processing">
                    执行中，等待结果…
                  </Tag>
                ) : null
              }
            >
              {isRunning && (
                <Alert
                  type="info"
                  message="命令正在执行中，结果将在完成后自动显示"
                  showIcon
                />
              )}
              {isTerminal && resultLoading && <Spin />}
              {isTerminal && !resultLoading && !result && (
                <Alert type="warning" message="暂无执行结果数据" showIcon />
              )}
              {result && (
                <>
                  <div style={{ marginBottom: 8 }}>
                    {result.exit_code === 0 ? (
                      <Tag icon={<CheckCircleOutlined />} color="success">
                        exit_code: 0
                      </Tag>
                    ) : (
                      <Tag icon={<CloseCircleOutlined />} color="error">
                        exit_code: {result.exit_code ?? "N/A"}
                      </Tag>
                    )}
                    <Text type="secondary" style={{ marginLeft: 8, fontSize: 12 }}>
                      上报: {dayjs(result.reported_at).format("YYYY-MM-DD HH:mm:ss")}
                    </Text>
                  </div>
                  {isInstallUpdate && expectedVersionText ? (
                    <Alert
                      style={{ marginBottom: 12 }}
                      type={versionConfirmed ? "success" : versionMismatch ? "warning" : "info"}
                      showIcon
                      message={versionConfirmed ? "设备已上报目标 Agent 版本" : versionMismatch ? "安装命令已结束，但设备当前版本与期望版本不一致" : "安装命令已结束，正在等待设备下一次心跳上报新版本"}
                    />
                  ) : null}
                  <div style={{ marginBottom: 8 }}>
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
                  <div>
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
                </>
              )}
            </Card>
          </>
        )}
      </Spin>
    </Drawer>
  );
};
