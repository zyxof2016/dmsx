import React from "react";
import { Button, Card, Col, Progress, Row, Space, Statistic, Tag, Typography } from "antd";
import {
  AuditOutlined,
  ClusterOutlined,
  DashboardOutlined,
  DatabaseOutlined,
  SafetyOutlined,
  SettingOutlined,
} from "@ant-design/icons";
import { useNavigate } from "@tanstack/react-router";
import { usePlatformHealth, usePlatformQuotas, useRbacRoles } from "../api/hooks";
import { useAppSession } from "../appProviders";

const { Title, Text } = Typography;

type PlatformModule = {
  title: string;
  description: string;
  path: string;
  icon: React.ReactNode;
  tag: string;
};

const MODULES: PlatformModule[] = [
  {
    title: "权限管理",
    description: "集中维护平台角色、平台权限策略、登录后进入平台或租户的选择规则。",
    path: "/platform/permissions",
    icon: <SafetyOutlined />,
    tag: "RBAC",
  },
  {
    title: "租户管理",
    description: "查看跨租户目录、创建租户，并快速切换到某个租户继续排查资源。",
    path: "/platform/tenants",
    icon: <ClusterOutlined />,
    tag: "Tenant",
  },
  {
    title: "配额治理",
    description: "维护租户、设备、命令、制品的平台级容量上限和使用率。",
    path: "/platform/quotas",
    icon: <DatabaseOutlined />,
    tag: "Quota",
  },
  {
    title: "全局审计",
    description: "按动作和资源类型检索跨租户审计日志。",
    path: "/platform/audit",
    icon: <AuditOutlined />,
    tag: "Audit",
  },
  {
    title: "运行健康",
    description: "查看平台组件、LiveKit、Redis、Command Bus 和核心资源计数。",
    path: "/platform/health",
    icon: <DashboardOutlined />,
    tag: "Health",
  },
  {
    title: "系统设置",
    description: "维护全局系统设置，并查看当前 JWT / RBAC 调试信息。",
    path: "/settings",
    icon: <SettingOutlined />,
    tag: "Config",
  },
];

export const PlatformOverviewPage: React.FC = () => {
  const navigate = useNavigate();
  const { subject, globalRoles, permittedTenantIds, tenantRoles, canUsePlatformMode } = useAppSession();
  const healthQuery = usePlatformHealth();
  const quotasQuery = usePlatformQuotas();
  const rolesQuery = useRbacRoles();
  const quotas = quotasQuery.data?.items ?? [];
  const maxQuotaUsage = quotas.reduce((max, item) => {
    if (item.limit <= 0) return max;
    return Math.max(max, Math.round((item.used / item.limit) * 100));
  }, 0);

  return (
    <Space direction="vertical" style={{ width: "100%" }} size="large">
      <div>
        <Title level={4} style={{ marginBottom: 4 }}>
          平台总览
        </Title>
        <Text type="secondary">
          平台模式按职责拆分为独立模块。这里作为全局入口，只展示关键摘要和模块导航。
        </Text>
      </div>

      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic title="租户总数" value={healthQuery.data?.tenant_count ?? permittedTenantIds.length} />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic title="设备总数" value={healthQuery.data?.device_count ?? 0} />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic title="平台角色" value={rolesQuery.data?.filter((role) => role.scope === "platform").length ?? 0} />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Space direction="vertical" style={{ width: "100%" }}>
              <Statistic title="最高配额使用率" value={maxQuotaUsage} suffix="%" />
              <Progress percent={maxQuotaUsage} size="small" status={maxQuotaUsage >= 90 ? "exception" : "normal"} />
            </Space>
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]}>
        {MODULES.map((item) => (
          <Col key={item.path} xs={24} md={12} xl={8}>
            <Card
              hoverable
              onClick={() => navigate({ to: item.path })}
              style={{ height: "100%" }}
              styles={{ body: { height: "100%" } }}
            >
              <Space direction="vertical" style={{ width: "100%", height: "100%" }} size="middle">
                <Space align="start" style={{ justifyContent: "space-between", width: "100%" }}>
                  <Space>
                    <span style={{ color: "#2563eb", fontSize: 22 }}>{item.icon}</span>
                    <Title level={5} style={{ margin: 0 }}>
                      {item.title}
                    </Title>
                  </Space>
                  <Tag>{item.tag}</Tag>
                </Space>
                <Text type="secondary">{item.description}</Text>
                <div style={{ flex: 1 }} />
                <Button type="link" style={{ padding: 0 }}>
                  进入模块
                </Button>
              </Space>
            </Card>
          </Col>
        ))}
      </Row>

      <Card title="当前平台会话">
        <Space direction="vertical" style={{ width: "100%" }} size="middle">
          <Space wrap>
            <Tag color={canUsePlatformMode ? "green" : "default"}>
              {canUsePlatformMode ? "平台模式已授权" : "平台模式未授权"}
            </Tag>
            {globalRoles.length ? globalRoles.map((role) => <Tag key={role}>{role}</Tag>) : <Tag>无全局角色</Tag>}
          </Space>
          <Text type="secondary">Subject: {subject ?? "未提供"}</Text>
          <Text type="secondary">允许租户数: {permittedTenantIds.length}</Text>
          <Text type="secondary">存在租户角色覆盖: {Object.keys(tenantRoles).length}</Text>
        </Space>
      </Card>
    </Space>
  );
};
