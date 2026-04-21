import React from "react";
import { Alert, Card, Col, Row, Space, Statistic, Typography } from "antd";
import { DashboardOutlined, DesktopOutlined, ClusterOutlined } from "@ant-design/icons";
import { useLivekitConfig, usePlatformHealth } from "../api/hooks";
import { formatApiError } from "../api/errors";

export const PlatformHealthPage: React.FC = () => {
  const { data: health, isLoading, error } = usePlatformHealth();
  const { data: livekit } = useLivekitConfig();

  return (
    <Space direction="vertical" style={{ width: "100%" }}>
      <Typography.Title level={4}>平台健康</Typography.Title>
      {error && <Alert type="error" showIcon message="加载失败" description={formatApiError(error)} />}
      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} lg={6}>
          <Card loading={isLoading}>
            <Statistic title="状态" value={health?.status ?? "unknown"} prefix={<DashboardOutlined />} />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card loading={isLoading}>
            <Statistic title="租户数" value={health?.tenant_count ?? 0} prefix={<ClusterOutlined />} />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card loading={isLoading}>
            <Statistic title="设备数" value={health?.device_count ?? 0} prefix={<DesktopOutlined />} />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card loading={isLoading}>
            <Statistic title="命令数" value={health?.command_count ?? 0} />
          </Card>
        </Col>
      </Row>
      <Card title="平台组件">
        <Space direction="vertical">
          <Typography.Text>API: {health?.status === "ok" ? "健康" : "待检查"}</Typography.Text>
          <Typography.Text>LiveKit: {livekit?.enabled ? `已启用 (${livekit.url})` : "未启用"}</Typography.Text>
        </Space>
      </Card>
    </Space>
  );
};
