import React from "react";
import { Button, Card, Col, Progress, Row, Space, Statistic, Tag, Typography } from "antd";
import { AppstoreOutlined, SafetyOutlined, SettingOutlined, TeamOutlined } from "@ant-design/icons";
import { useNavigate } from "@tanstack/react-router";
import { useAppSession } from "../appProviders";
import { usePlatformRbacPolicy, useRbacRoles } from "../api/hooks";

const { Title, Text } = Typography;

const PERMISSION_MODULES = [
  {
    title: "角色管理",
    description: "维护平台角色对象。当前展示内置 PlatformAdmin / PlatformViewer 及 platform.* 权限点。",
    path: "/platform/permissions/roles",
    icon: <SafetyOutlined />,
    tag: "Role",
  },
  {
    title: "用户管理",
    description: "维护平台用户对象。当前先展示登录用户的平台角色、租户授权与租户角色覆盖。",
    path: "/platform/permissions/users",
    icon: <TeamOutlined />,
    tag: "User",
  },
  {
    title: "菜单管理",
    description: "维护平台菜单对象。当前展示菜单、路由、权限点与允许角色的关系。",
    path: "/platform/permissions/menus",
    icon: <AppstoreOutlined />,
    tag: "Menu",
  },
  {
    title: "权限策略",
    description: "维护平台进入策略，包括平台角色启用、是否强制选择范围和默认进入范围。",
    path: "/platform/permissions/policy",
    icon: <SettingOutlined />,
    tag: "Policy",
  },
];

export const PlatformPermissionsPage: React.FC = () => {
  const navigate = useNavigate();
  const { globalRoles, permittedTenantIds, tenantRoles } = useAppSession();
  const rolesQuery = useRbacRoles();
  const policyQuery = usePlatformRbacPolicy();
  const platformRoles = (rolesQuery.data ?? []).filter((role) => role.platform_read || role.platform_write);
  const defaultScope = policyQuery.data?.value?.default_scope === "tenant" ? "租户优先" : "平台优先";
  const coverage = permittedTenantIds.length > 0
    ? Math.min(100, Math.round((Object.keys(tenantRoles).length / permittedTenantIds.length) * 100))
    : 0;

  return (
    <Space direction="vertical" style={{ width: "100%" }} size="large">
      <div>
        <Title level={4} style={{ marginBottom: 4 }}>
          权限总览
        </Title>
        <Text type="secondary">
          权限中心按对象拆分：角色只管角色，用户只管用户，菜单只管菜单，策略只管全局进入规则。
        </Text>
      </div>

      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic title="平台角色" value={platformRoles.length} prefix={<SafetyOutlined />} />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic title="当前平台角色" value={globalRoles.length} prefix={<TeamOutlined />} />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic title="允许租户" value={permittedTenantIds.length} prefix={<AppstoreOutlined />} />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic title="默认进入" value={defaultScope} prefix={<SettingOutlined />} />
          </Card>
        </Col>
      </Row>

      <Card title="租户角色覆盖">
        <Space direction="vertical" style={{ width: "100%" }}>
          <Progress percent={coverage} />
          <Text type="secondary">
            当前 JWT 允许 {permittedTenantIds.length} 个租户，其中 {Object.keys(tenantRoles).length} 个租户声明了 tenant_roles 覆盖。
          </Text>
        </Space>
      </Card>

      <Row gutter={[16, 16]}>
        {PERMISSION_MODULES.map((item) => (
          <Col key={item.path} xs={24} md={12} xl={6}>
            <Card hoverable onClick={() => navigate({ to: item.path })} style={{ height: "100%" }}>
              <Space direction="vertical" size="middle" style={{ width: "100%" }}>
                <Space align="start" style={{ justifyContent: "space-between", width: "100%" }}>
                  <span style={{ color: "#2563eb", fontSize: 22 }}>{item.icon}</span>
                  <Tag>{item.tag}</Tag>
                </Space>
                <Title level={5} style={{ margin: 0 }}>
                  {item.title}
                </Title>
                <Text type="secondary">{item.description}</Text>
                <Button type="link" style={{ padding: 0 }}>
                  进入维护
                </Button>
              </Space>
            </Card>
          </Col>
        ))}
      </Row>
    </Space>
  );
};
