import React from "react";
import {
  Alert,
  Button,
  Card,
  Descriptions,
  Form,
  Input,
  Select,
  Space,
  Spin,
  Switch,
  Tag,
  Typography,
  message,
} from "antd";
import { useAppI18n, useAppSession } from "../appProviders";
import { useSystemSetting, useUpsertSystemSetting } from "../api/hooks";
import { ApiError, DEFAULT_TENANT_ID } from "../api/client";
import { formatApiError } from "../api/errors";
import { useResourceAccess } from "../authz";
import { ReadonlyBanner } from "../components/ReadonlyBanner";

const METRICS_BEARER_ENABLED_KEY = "metrics.bearer.enabled";

function maskJwt(jwt: string): string {
  if (jwt.length <= 20) return jwt;
  return `${jwt.slice(0, 12)}...${jwt.slice(-6)}`;
}

export const SystemSettingsPage: React.FC = () => {
  const { t } = useAppI18n();
  const { canWrite } = useResourceAccess("platformWrite");
  const {
    tenantId,
    setTenantId,
    jwt,
    setJwt,
    clearJwt,
    subject,
    primaryTenantId,
    permittedTenantIds,
    tenantOptions,
    globalRoles,
    tenantRoles,
    effectiveRoles,
    platformRoles,
    hasJwt,
    jwtParseError,
  } = useAppSession();

  const {
    data,
    error,
    isLoading,
    refetch,
  } = useSystemSetting(METRICS_BEARER_ENABLED_KEY);

  const upsertMut = useUpsertSystemSetting(METRICS_BEARER_ENABLED_KEY);

  const [metricsBearerEnabled, setMetricsBearerEnabled] = React.useState(false);
  const [tenantDraft, setTenantDraft] = React.useState(tenantId);
  const [jwtDraft, setJwtDraft] = React.useState(jwt);

  React.useEffect(() => {
    const v = data?.value as Record<string, unknown> | undefined;
    const enabled = v?.enabled;
    if (typeof enabled === "boolean") {
      setMetricsBearerEnabled(enabled);
    }
  }, [data]);

  React.useEffect(() => {
    setTenantDraft(tenantId);
  }, [tenantId]);

  React.useEffect(() => {
    setJwtDraft(jwt);
  }, [jwt]);

  const isNotFound = (err: unknown): boolean => {
    const e = err as Partial<ApiError>;
    return typeof e?.status === "number" && e.status === 404;
  };

  if (error && !isNotFound(error)) {
    return (
      <Space direction="vertical" style={{ width: "100%" }}>
        <Typography.Title level={4}>{t("page.systemSettings")}</Typography.Title>
        <Alert
          type="error"
          showIcon
          message="加载系统设置失败"
          description={formatApiError(error)}
          action={<Button onClick={() => refetch()}>重试</Button>}
        />
      </Space>
    );
  }

  return (
    <Space direction="vertical" style={{ width: "100%" }}>
      <Typography.Title level={4}>{t("page.systemSettings")}</Typography.Title>
      <ReadonlyBanner visible={!canWrite} resourceLabel="系统设置" />

      <Card title="前端会话设置">
        <Space direction="vertical" style={{ width: "100%" }} size="middle">
          <Alert
            type="info"
            showIcon
            message="此卡片仅影响当前浏览器"
            description="用于本地联调 jwt / disabled 两种模式，不会写入后端 system_settings。"
          />

          <Space wrap>
            <Tag color="blue">活动租户：{tenantId}</Tag>
            <Tag color={jwt ? "green" : "default"}>
              JWT：{jwt ? "已设置" : "未设置"}
            </Tag>
            {tenantId === DEFAULT_TENANT_ID && <Tag>默认种子租户</Tag>}
          </Space>

          <Form layout="vertical">
            <Form.Item
              label="活动租户"
              help={
                "仅展示当前 JWT 许可的租户，以及本浏览器最近创建过的租户。所有 /v1/tenants/{tid}/... 请求都会使用这里选择的租户。"
              }
            >
                <Select
                  value={tenantDraft}
                  onChange={(value) => setTenantDraft(value)}
                  options={tenantOptions.map((option) => ({
                    value: option.id,
                    label: (
                      <Space direction="vertical" size={0}>
                        <Typography.Text strong>
                          {option.name ?? `${option.id.slice(0, 8)}…${option.id.slice(-4)}`}
                        </Typography.Text>
                        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                          {option.id}
                        </Typography.Text>
                        <Space wrap size={4}>
                          {option.id === tenantId && <Tag color="blue">当前</Tag>}
                          <Tag>{option.source === "jwt" ? "JWT 授权" : "最近创建"}</Tag>
                          {option.effectiveRoles.map((role) => (
                            <Tag key={`${option.id}-${role}`}>{role}</Tag>
                          ))}
                        </Space>
                      </Space>
                    ),
                  }))}
                  placeholder="选择当前用户可访问的租户"
                />
            </Form.Item>

            <Form.Item
              label="JWT"
              help={jwt ? `当前已保存：${maskJwt(jwt)}` : "为空时前端不会发送 Authorization 头"}
            >
              <Input.TextArea
                value={jwtDraft}
                onChange={(e) => setJwtDraft(e.target.value)}
                rows={5}
                placeholder="粘贴 JWT，可带或不带 Bearer 前缀"
              />
            </Form.Item>

            <Space wrap>
              <Button
                type="primary"
                onClick={() => {
                  const nextTenantId = tenantDraft.trim();
                  if (!nextTenantId) {
                    message.error("请选择一个租户");
                    return;
                  }

                  setTenantId(nextTenantId);

                  const nextJwt = jwtDraft.trim();
                  if (nextJwt) {
                    setJwt(nextJwt);
                  } else {
                    clearJwt();
                  }

                  message.success("前端会话设置已保存");
                }}
              >
                保存前端会话
              </Button>

              <Button
                onClick={() => {
                  setTenantDraft(tenantId);
                  setJwtDraft(jwt);
                }}
              >
                撤销会话改动
              </Button>

              <Button
                danger
                onClick={() => {
                  clearJwt();
                  setJwtDraft("");
                  message.success("JWT 已清除");
                }}
              >
                清除 JWT
              </Button>
            </Space>
          </Form>
        </Space>
      </Card>

      <Card title="当前 JWT 解析结果">
        <Space direction="vertical" style={{ width: "100%" }} size="middle">
          {!hasJwt ? (
            <Alert
              type="info"
              showIcon
              message="当前未设置 JWT"
              description="disabled 模式下这很常见。若切到 jwt 模式，可在上方粘贴令牌后，这里会展示主租户、允许租户和角色覆盖信息。"
            />
          ) : jwtParseError ? (
            <Alert
              type="warning"
              showIcon
              message="JWT 已设置，但前端未能解析"
              description="这通常表示令牌不是标准 JWT 三段格式，或 payload 不是合法 JSON。前端会停止用它做导航收敛，最终权限仍以后端校验为准。"
            />
          ) : (
            <>
              <Descriptions bordered size="small" column={1}>
                <Descriptions.Item label="Subject">
                  {subject ?? "未声明"}
                </Descriptions.Item>
                <Descriptions.Item label="主租户">
                  {primaryTenantId ?? "未声明"}
                </Descriptions.Item>
                <Descriptions.Item label="当前活动租户有效角色">
                  <Space wrap>
                    {effectiveRoles.length ? effectiveRoles.map((role) => <Tag key={role}>{role}</Tag>) : <Tag>无角色</Tag>}
                  </Space>
                </Descriptions.Item>
                <Descriptions.Item label="令牌级 roles">
                  <Space wrap>
                    {globalRoles.length ? globalRoles.map((role) => <Tag key={role}>{role}</Tag>) : <Tag>无</Tag>}
                  </Space>
                </Descriptions.Item>
                <Descriptions.Item label="平台模式角色">
                  <Space wrap>
                    {platformRoles.length ? platformRoles.map((role) => <Tag key={role}>{role}</Tag>) : <Tag>无</Tag>}
                  </Space>
                </Descriptions.Item>
                <Descriptions.Item label="允许访问租户">
                  <Space wrap>
                    {permittedTenantIds.map((id) => (
                      <Tag key={id} color={id === tenantId ? "blue" : "default"}>
                        {id}
                      </Tag>
                    ))}
                  </Space>
                </Descriptions.Item>
              </Descriptions>

              <div>
                <Typography.Text strong>tenant_roles 覆盖</Typography.Text>
                <div style={{ marginTop: 12 }}>
                  {Object.keys(tenantRoles).length === 0 ? (
                    <Alert
                      type="info"
                      showIcon
                      message="当前 JWT 未声明 tenant_roles"
                      description="这意味着前端和后端都会回退到令牌级 roles。"
                    />
                  ) : (
                    <Descriptions bordered size="small" column={1}>
                      {Object.entries(tenantRoles).map(([id, roles]) => (
                        <Descriptions.Item key={id} label={id}>
                          <Space wrap>
                            {roles.length ? roles.map((role) => <Tag key={`${id}-${role}`}>{role}</Tag>) : <Tag>空数组（显式无角色）</Tag>}
                          </Space>
                        </Descriptions.Item>
                      ))}
                    </Descriptions>
                  )}
                </div>
              </div>
            </>
          )}
        </Space>
      </Card>

      <Card>
        <Spin spinning={isLoading}>
          <Form layout="vertical">
            <Form.Item label="Metrics：/metrics 是否启用 Bearer（存储到 system_settings）">
              <Switch
                checked={metricsBearerEnabled}
                onChange={(checked) => setMetricsBearerEnabled(checked)}
              />
            </Form.Item>

            <Form.Item label="说明">
              <div style={{ color: "rgba(0,0,0,0.65)" }}>
                当前页面以 `system_settings` 的键 `metrics.bearer.enabled` 作为存储介质。是否真正影响 `/metrics` 鉴权逻辑取决于后端运行时实现（以服务端实际行为为准）。
              </div>
            </Form.Item>

            <Space>
              <Button
                type="primary"
                loading={upsertMut.isPending}
                disabled={!canWrite}
                onClick={async () => {
                  try {
                    await upsertMut.mutateAsync({
                      value: { enabled: metricsBearerEnabled },
                    });
                    message.success("系统设置已保存");
                  } catch (e) {
                    message.error(formatApiError(e));
                  }
                }}
              >
                保存
              </Button>
              <Button
                onClick={() => {
                  const v = data?.value as Record<string, unknown> | undefined;
                  const enabled = v?.enabled;
                  setMetricsBearerEnabled(typeof enabled === "boolean" ? enabled : false);
                }}
                disabled={!data}
              >
                撤销
              </Button>
            </Space>
          </Form>
        </Spin>
      </Card>
    </Space>
  );
};
