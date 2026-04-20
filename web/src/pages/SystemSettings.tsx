import React from "react";
import {
  Alert,
  Button,
  Card,
  Form,
  Input,
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

const METRICS_BEARER_ENABLED_KEY = "metrics.bearer.enabled";

function maskJwt(jwt: string): string {
  if (jwt.length <= 20) return jwt;
  return `${jwt.slice(0, 12)}...${jwt.slice(-6)}`;
}

function isValidUuid(value: string): boolean {
  return /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(
    value,
  );
}

export const SystemSettingsPage: React.FC = () => {
  const { t } = useAppI18n();
  const { tenantId, setTenantId, jwt, setJwt, clearJwt } = useAppSession();

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
              label="活动租户 ID"
              validateStatus={tenantDraft && !isValidUuid(tenantDraft.trim()) ? "error" : ""}
              help={
                tenantDraft && !isValidUuid(tenantDraft.trim())
                  ? "租户 ID 必须是合法 UUID"
                  : "所有 /v1/tenants/{tid}/... 请求都会使用这里的租户 ID"
              }
            >
              <Input
                value={tenantDraft}
                onChange={(e) => setTenantDraft(e.target.value)}
                placeholder="例如 00000000-0000-0000-0000-000000000001"
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
                  if (!isValidUuid(nextTenantId)) {
                    message.error("租户 ID 必须是合法 UUID");
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
