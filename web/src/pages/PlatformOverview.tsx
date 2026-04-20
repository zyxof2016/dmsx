import React from "react";
import { Alert, Card, Col, Form, Input, List, Row, Space, Spin, Statistic, Tag, Typography, message } from "antd";
import {
  ClusterOutlined,
  PlusOutlined,
  SafetyOutlined,
  SettingOutlined,
  TeamOutlined,
} from "@ant-design/icons";
import { useAppSession } from "../appProviders";
import { useCreateTenant, useLivekitConfig, useRbacRoles, useSystemSetting } from "../api/hooks";
import { GuardedButton } from "../components/GuardedButton";
import { formatApiError } from "../api/errors";
import { ReadonlyBanner } from "../components/ReadonlyBanner";
import { useResourceAccess } from "../authz";

const { Title, Text } = Typography;
const RECENT_TENANTS_KEY = "dmsx.platform.recent_tenants";

type RecentTenant = {
  id: string;
  name: string;
  createdAt: string;
};

function getRecentTenants(): RecentTenant[] {
  try {
    const raw = localStorage.getItem(RECENT_TENANTS_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as RecentTenant[];
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function saveRecentTenants(items: RecentTenant[]) {
  localStorage.setItem(RECENT_TENANTS_KEY, JSON.stringify(items.slice(0, 8)));
}

export const PlatformOverviewPage: React.FC = () => {
  const {
    subject,
    primaryTenantId,
    permittedTenantIds,
    globalRoles,
    tenantRoles,
    canUsePlatformMode,
  } = useAppSession();
  const { canWrite } = useResourceAccess("globalConfig");
  const [tenantForm] = Form.useForm<{ name: string }>();
  const { data: roles, isLoading: rolesLoading, error: rolesError } = useRbacRoles();
  const {
    data: livekit,
    isLoading: livekitLoading,
    error: livekitError,
  } = useLivekitConfig();
  const {
    data: metricsBearer,
    isLoading: metricsLoading,
    error: metricsError,
  } = useSystemSetting("metrics.bearer.enabled");
  const createTenant = useCreateTenant();
  const [recentTenants, setRecentTenants] = React.useState<RecentTenant[]>(() =>
    getRecentTenants(),
  );

  const handleCreateTenant = async (values: { name: string }) => {
    try {
      const tenant = await createTenant.mutateAsync(values);
      const next = [
        { id: tenant.id, name: tenant.name, createdAt: new Date().toISOString() },
        ...recentTenants.filter((item) => item.id !== tenant.id),
      ];
      setRecentTenants(next);
      saveRecentTenants(next);
      message.success("租户创建成功");
      tenantForm.resetFields();
    } catch (error) {
      message.error(formatApiError(error));
    }
  };

  return (
    <Space direction="vertical" style={{ width: "100%" }} size="large">
      <div>
        <Title level={4} style={{ marginBottom: 4 }}>
          平台总览
        </Title>
        <Text type="secondary">
          面向平台运营方的全局入口。这里优先展示跨租户配置、角色模型和令牌覆盖范围，而不是单租户资源明细。
        </Text>
      </div>

      <Alert
        type="info"
        showIcon
        message="平台模式已具备独立首页"
        description="当前版本先承载平台级会话摘要、授权覆盖面与后续能力挂点。后续如果接入租户目录、配额、全局审计和平台健康页，可以继续往这里扩展。"
      />

      <ReadonlyBanner visible={!canWrite} resourceLabel="平台配置与租户目录" />

      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic
              title="允许租户数"
              value={permittedTenantIds.length}
              prefix={<ClusterOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic
              title="平台角色模板"
              value={roles?.length ?? globalRoles.length}
              prefix={<SafetyOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic
              title="租户角色覆盖"
              value={Object.keys(tenantRoles).length}
              prefix={<TeamOutlined />}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic
              title="平台模式"
              value={canUsePlatformMode ? "已启用" : "未授权"}
              prefix={<SettingOutlined />}
            />
          </Card>
        </Col>
      </Row>

      <Card title="当前会话摘要">
        <Space direction="vertical" style={{ width: "100%" }} size="middle">
          <div>
            <Text type="secondary">Subject</Text>
            <div>{subject ?? "未提供"}</div>
          </div>
          <div>
            <Text type="secondary">主租户</Text>
            <div>{primaryTenantId ?? "未声明"}</div>
          </div>
          <div>
            <Text type="secondary">全局角色</Text>
            <div style={{ marginTop: 8 }}>
              <Space wrap>
                {globalRoles.length ? globalRoles.map((role) => <Tag key={role}>{role}</Tag>) : <Tag>无</Tag>}
              </Space>
            </div>
          </div>
          <div>
            <Text type="secondary">允许访问的租户</Text>
            <div style={{ marginTop: 8 }}>
              <Space wrap>
                {permittedTenantIds.map((tenantId) => (
                  <Tag key={tenantId}>{tenantId}</Tag>
                ))}
              </Space>
            </div>
          </div>
        </Space>
      </Card>

      <Row gutter={[16, 16]}>
        <Col xs={24} lg={8}>
          <Card title="租户目录（当前能力）" size="small">
            <Form form={tenantForm} layout="vertical" onFinish={handleCreateTenant}>
              <Form.Item
                name="name"
                label="新租户名称"
                rules={[
                  { required: true, message: "请输入租户名称" },
                  { max: 200, message: "最长 200 字符" },
                ]}
              >
                <Input placeholder="例如：华南大区 / partner-a / customer-prod" />
              </Form.Item>
              <GuardedButton
                type="primary"
                icon={<PlusOutlined />}
                htmlType="submit"
                loading={createTenant.isPending}
                allowed={canWrite}
              >
                创建租户
              </GuardedButton>
            </Form>
            <List
              style={{ marginTop: 16 }}
              size="small"
              dataSource={[
                `主租户: ${primaryTenantId ?? "未声明"}`,
                `允许租户数: ${permittedTenantIds.length}`,
                "当前后端仅开放创建租户，租户列表接口仍未提供",
              ]}
              renderItem={(item) => <List.Item>{item}</List.Item>}
            />
            {createTenant.error && (
              <Alert
                style={{ marginTop: 12 }}
                type="error"
                showIcon
                message="创建租户失败"
                description={formatApiError(createTenant.error)}
              />
            )}
            {createTenant.data && (
              <Alert
                style={{ marginTop: 12 }}
                type="success"
                showIcon
                message="已创建租户"
                description={`${createTenant.data.name} (${createTenant.data.id})`}
              />
            )}
            {recentTenants.length > 0 && (
              <List
                style={{ marginTop: 16 }}
                size="small"
                header="最近创建（本浏览器会话记录）"
                dataSource={recentTenants}
                renderItem={(item) => (
                  <List.Item>
                    <Space direction="vertical" size={0}>
                      <Text strong>{item.name}</Text>
                      <Text type="secondary" style={{ fontSize: 12 }}>
                        {item.id} · {new Date(item.createdAt).toLocaleString()}
                      </Text>
                    </Space>
                  </List.Item>
                )}
              />
            )}
          </Card>
        </Col>
        <Col xs={24} lg={8}>
          <Card title="平台配置（真实接口）" size="small">
            <Spin spinning={rolesLoading || livekitLoading || metricsLoading}>
              <List
                size="small"
                dataSource={[
                  `RBAC 角色模板: ${roles?.map((role) => role.name).join(", ") || "暂无"}`,
                  `LiveKit: ${livekit?.enabled ? `已启用 (${livekit.url})` : "未启用"}`,
                  `Metrics Bearer: ${String((metricsBearer?.value?.enabled as boolean | undefined) ?? false)}`,
                ]}
                renderItem={(item) => <List.Item>{item}</List.Item>}
              />
            </Spin>
            {(rolesError || livekitError || metricsError) && (
              <Alert
                style={{ marginTop: 12 }}
                type="warning"
                showIcon
                message="部分平台配置读取失败"
                description={formatApiError(rolesError ?? livekitError ?? metricsError)}
              />
            )}
          </Card>
        </Col>
        <Col xs={24} lg={8}>
          <Card title="容量与配额（规划入口）" size="small">
            <List
              size="small"
              dataSource={[
                "设备数配额",
                "制品存储额度",
                "命令并发 / 会话并发",
              ]}
              renderItem={(item) => <List.Item>{item}</List.Item>}
            />
          </Card>
        </Col>
      </Row>
    </Space>
  );
};
