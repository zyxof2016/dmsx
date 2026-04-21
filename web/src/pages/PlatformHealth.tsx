import React from "react";
import { Alert, Card, Col, Row, Space, Statistic, Tag, Typography } from "antd";
import {
  DashboardOutlined,
  DesktopOutlined,
  ClusterOutlined,
  SafetyOutlined,
  AppstoreOutlined,
  AuditOutlined,
} from "@ant-design/icons";
import { useLivekitConfig, usePlatformHealth } from "../api/hooks";
import { formatApiError } from "../api/errors";

function statusTag(enabled: boolean, enabledLabel = "已启用", disabledLabel = "未启用") {
  return <Tag color={enabled ? "green" : "default"}>{enabled ? enabledLabel : disabledLabel}</Tag>;
}

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
        <Col xs={24} sm={12} lg={6}>
          <Card loading={isLoading}>
            <Statistic title="策略数" value={health?.policy_count ?? 0} prefix={<SafetyOutlined />} />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card loading={isLoading}>
            <Statistic title="制品数" value={health?.artifact_count ?? 0} prefix={<AppstoreOutlined />} />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card loading={isLoading}>
            <Statistic title="审计数" value={health?.audit_log_count ?? 0} prefix={<AuditOutlined />} />
          </Card>
        </Col>
      </Row>
      <Card title="平台组件">
        <Space direction="vertical">
          <Typography.Text>API: {health?.status === "ok" ? "健康" : "待检查"}</Typography.Text>
          <Typography.Text>
            LiveKit: {statusTag(Boolean(health?.livekit_enabled && livekit?.enabled))} {livekit?.enabled ? livekit.url : null}
          </Typography.Text>
          <Typography.Text>Redis: {statusTag(Boolean(health?.redis_enabled))}</Typography.Text>
          <Typography.Text>Command Bus: {statusTag(Boolean(health?.command_bus_enabled))}</Typography.Text>
        </Space>
      </Card>
    </Space>
  );
};
