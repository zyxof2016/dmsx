import React from "react";
import { Alert, Card, Descriptions, Space, Typography } from "antd";
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
      <Title level={4}>零接触安装指引</Title>
      <Alert
        type="info"
        showIcon
        message="将以下参数注入设备环境变量后启动 Agent，即可自动认领已预注册设备。"
      />
      <Card>
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
      </Card>
    </Space>
  );
};
