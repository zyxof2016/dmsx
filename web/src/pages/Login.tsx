import React from "react";
import { Alert, App, Button, Card, Form, Input, Segmented, Space, Typography } from "antd";
import { useNavigate, useRouterState } from "@tanstack/react-router";
import { useAppSession, type AuthMode } from "../appProviders";

type LoginFormValues = {
  jwt: string;
  tenantId: string;
};

export const LoginPage: React.FC = () => {
  const { message } = App.useApp();
  const navigate = useNavigate();
  const redirect = useRouterState({
    select: (s) => new URLSearchParams(s.location.searchStr).get("redirect") || "/",
  });
  const { authMode, setAuthMode, setJwt, setTenantId, jwtParseError } = useAppSession();
  const [form] = Form.useForm<LoginFormValues>();

  return (
    <div
      style={{
        minHeight: "100vh",
        display: "grid",
        placeItems: "center",
        padding: 24,
        background: "linear-gradient(180deg, #f5f8ff 0%, #eef3f8 100%)",
      }}
    >
      <Card style={{ width: "100%", maxWidth: 480 }}>
        <Space direction="vertical" size="large" style={{ width: "100%" }}>
          <div>
            <Typography.Title level={3} style={{ marginBottom: 8 }}>
              登录 DMSX 控制台
            </Typography.Title>
            <Typography.Text type="secondary">
              JWT 模式下未登录用户将被重定向到本页。开发联调可切到 disabled 模式直接进入。
            </Typography.Text>
          </div>

          <Segmented<AuthMode>
            block
            value={authMode}
            onChange={(value) => setAuthMode(value as AuthMode)}
            options={[
              { label: "JWT 登录", value: "jwt" },
              { label: "Disabled 联调", value: "disabled" },
            ]}
          />

          {authMode === "disabled" ? (
            <Alert
              type="info"
              showIcon
              message="当前为 disabled 联调模式"
              description="此模式不会强制要求 JWT，适合本地开发；生产应使用 JWT/OIDC。"
            />
          ) : null}

          <Form
            form={form}
            layout="vertical"
            initialValues={{ tenantId: "00000000-0000-0000-0000-000000000001", jwt: "" }}
            onFinish={(values) => {
              setTenantId(values.tenantId.trim());
              setJwt(values.jwt.trim());
              message.success("登录信息已保存");
              navigate({ to: redirect });
            }}
          >
            <Form.Item
              name="tenantId"
              label="默认租户 ID"
              rules={[{ required: true, message: "请输入租户 ID" }]}
            >
              <Input placeholder="00000000-0000-0000-0000-000000000001" />
            </Form.Item>

            <Form.Item
              name="jwt"
              label="JWT / Bearer Token"
              rules={authMode === "jwt" ? [{ required: true, message: "请输入 JWT" }] : []}
              validateStatus={jwtParseError ? "error" : undefined}
              help={jwtParseError ? "JWT 结构无法解析，请检查 token 格式。" : undefined}
            >
              <Input.TextArea rows={6} placeholder="粘贴 JWT，支持带 Bearer 前缀" />
            </Form.Item>

            <Space>
              <Button type="primary" htmlType="submit">
                进入控制台
              </Button>
              <Button
                onClick={() => {
                  if (authMode === "disabled") {
                    navigate({ to: redirect });
                  }
                }}
              >
                跳过并继续
              </Button>
            </Space>
          </Form>
        </Space>
      </Card>
    </div>
  );
};
