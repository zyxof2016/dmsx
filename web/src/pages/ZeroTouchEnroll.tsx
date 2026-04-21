import React from "react";
import { Alert, Button, Card, Descriptions, Divider, Space, Steps, Typography } from "antd";
import { useRouterState } from "@tanstack/react-router";

const { Title, Text } = Typography;

export const ZeroTouchEnrollPage: React.FC = () => {
  const search = useRouterState({ select: (s) => s.location.searchStr });
  const params = React.useMemo(() => new URLSearchParams(search), [search]);

  const apiUrl = params.get("api_url") ?? "";
  const tenantId = params.get("tenant_id") ?? "";
  const enrollmentToken = params.get("enrollment_token") ?? "";
  const mode = params.get("mode") ?? "manual";

  return (
    <Space direction="vertical" style={{ width: "100%" }}>
      <Title level={3}>DMSX 零接触安装向导</Title>
      <Alert
        type="info"
        showIcon
        message="将以下参数注入设备环境变量后启动 Agent，即可自动认领已预注册设备。适用于扫码安装、MDM 下发、工厂预置和远程实施。"
      />
      <Card>
        <Steps
          current={1}
          items={[
            { title: "平台预注册" },
            { title: "下发 Enrollment 参数" },
            { title: "启动 Agent 完成认领" },
          ]}
        />
      </Card>
      <Card>
        <Typography.Title level={5}>安装参数</Typography.Title>
        <Descriptions bordered column={1} size="small">
          <Descriptions.Item label="模式">{mode}</Descriptions.Item>
          <Descriptions.Item label="API URL">
            <Text code copyable={{ text: apiUrl }}>{apiUrl || "—"}</Text>
          </Descriptions.Item>
          <Descriptions.Item label="Tenant ID">
            <Text code copyable={{ text: tenantId }}>{tenantId || "—"}</Text>
          </Descriptions.Item>
          <Descriptions.Item label="Enrollment Token">
            <Text code copyable={{ text: enrollmentToken }}>{enrollmentToken || "—"}</Text>
          </Descriptions.Item>
          <Descriptions.Item label="Agent 启动命令">
            <Text
              code
              copyable={{
                text: `DMSX_API_URL=${apiUrl} DMSX_TENANT_ID=${tenantId} DMSX_DEVICE_ENROLLMENT_TOKEN='${enrollmentToken}' cargo run -p dmsx-agent`,
              }}
            >
              {`DMSX_API_URL=${apiUrl} DMSX_TENANT_ID=${tenantId} DMSX_DEVICE_ENROLLMENT_TOKEN='${enrollmentToken}' cargo run -p dmsx-agent`}
            </Text>
          </Descriptions.Item>
        </Descriptions>
        <Divider />
        <Typography.Title level={5}>推荐安装方式</Typography.Title>
        <Space direction="vertical">
          <Typography.Text>1. 扫码/打开本页，复制启动命令。</Typography.Text>
          <Typography.Text>2. Linux/macOS 直接导出 shell 脚本执行。</Typography.Text>
          <Typography.Text>3. Windows 使用批量导出的 `.bat` 命令。</Typography.Text>
          <Typography.Text>4. Android 通过 ADB 或 MDM 注入环境变量后启动 Agent。</Typography.Text>
        </Space>
        <Divider />
        <Button
          type="primary"
          onClick={async () => {
            await navigator.clipboard.writeText(
              `DMSX_API_URL=${apiUrl} DMSX_TENANT_ID=${tenantId} DMSX_DEVICE_ENROLLMENT_TOKEN='${enrollmentToken}' cargo run -p dmsx-agent`,
            );
          }}
        >
          复制启动命令
        </Button>
      </Card>
    </Space>
  );
};
