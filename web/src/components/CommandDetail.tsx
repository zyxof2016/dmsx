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
} from "antd";
import { CopyOutlined, CheckCircleOutlined, CloseCircleOutlined, LoadingOutlined } from "@ant-design/icons";
import { useNavigate, useParams } from "@tanstack/react-router";
import dayjs from "dayjs";
import { useCommand, useDevice, useCommandResult } from "../api/hooks";
import { formatApiError } from "../api/errors";

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
                <pre
                  style={{
                    margin: 0,
                    padding: 8,
                    background: "#f5f5f5",
                    borderRadius: 4,
                    fontSize: 12,
                    maxHeight: 300,
                    overflow: "auto",
                  }}
                >
                  {JSON.stringify(command.payload, null, 2)}
                </pre>
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
                  <div style={{ marginBottom: 8 }}>
                    <Text strong>stdout:</Text>
                    <pre
                      style={{
                        background: "#f6ffed",
                        padding: 8,
                        borderRadius: 4,
                        maxHeight: 200,
                        overflow: "auto",
                        fontSize: 12,
                        border: "1px solid #b7eb8f",
                      }}
                    >
                      {result.stdout || "(empty)"}
                    </pre>
                  </div>
                  <div>
                    <Text strong>stderr:</Text>
                    <pre
                      style={{
                        background: "#fff2f0",
                        padding: 8,
                        borderRadius: 4,
                        maxHeight: 200,
                        overflow: "auto",
                        fontSize: 12,
                        border: "1px solid #ffa39e",
                      }}
                    >
                      {result.stderr || "(empty)"}
                    </pre>
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
