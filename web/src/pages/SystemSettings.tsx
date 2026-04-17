import React from "react";
import { Alert, Button, Card, Form, Switch, Typography, Space, Spin, message } from "antd";
import { useAppI18n } from "../appProviders";
import { useSystemSetting, useUpsertSystemSetting } from "../api/hooks";
import { ApiError } from "../api/client";

const METRICS_BEARER_ENABLED_KEY = "metrics.bearer.enabled";

export const SystemSettingsPage: React.FC = () => {
  const { t } = useAppI18n();

  const {
    data,
    error,
    isLoading,
    refetch,
  } = useSystemSetting(METRICS_BEARER_ENABLED_KEY);

  const upsertMut = useUpsertSystemSetting(METRICS_BEARER_ENABLED_KEY);

  const [metricsBearerEnabled, setMetricsBearerEnabled] = React.useState(false);

  React.useEffect(() => {
    const v = data?.value as Record<string, unknown> | undefined;
    const enabled = v?.enabled;
    if (typeof enabled === "boolean") {
      setMetricsBearerEnabled(enabled);
    }
  }, [data]);

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
          description={String(error)}
          action={<Button onClick={() => refetch()}>重试</Button>}
        />
      </Space>
    );
  }

  return (
    <Space direction="vertical" style={{ width: "100%" }}>
      <Typography.Title level={4}>{t("page.systemSettings")}</Typography.Title>

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
                    message.error(String(e));
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

