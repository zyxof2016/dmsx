import React from "react";
import { Alert, App, Button, Card, Form, Input, Radio, Space, Typography } from "antd";
import { useNavigate, useRouterState } from "@tanstack/react-router";
import { useAppSession, type AuthMode } from "../appProviders";
import { getLastStoredTenantId } from "../api/client";
import { formatApiError } from "../api/errors";
import { useLogin, useSelectLoginScope } from "../api/hooks";
import type { LoginDecisionKind, LoginTenantOption } from "../api/types";

type LoginFormValues = {
  username: string;
  password: string;
};

type PendingLogin = {
  username: string;
  loginTransactionToken: string;
  displayName: string;
  kind: Extract<LoginDecisionKind, "choose_scope" | "choose_tenant">;
  tenantOptions: LoginTenantOption[];
  preferredTenantId?: string | null;
};

export const LoginPage: React.FC = () => {
  const { message } = App.useApp();
  const navigate = useNavigate();
  const redirect = useRouterState({
    select: (s) => new URLSearchParams(s.location.searchStr).get("redirect") || "/",
  });
  const {
    authMode,
    setAuthMode,
    setJwt,
    setTenantId,
    setAppMode,
    setDisplayName,
    setAvailableScopes,
  } = useAppSession();
  const [form] = Form.useForm<LoginFormValues>();
  const loginMut = useLogin();
  const selectMut = useSelectLoginScope();
  const [pendingLogin, setPendingLogin] = React.useState<PendingLogin | null>(null);
  const [scope, setScope] = React.useState<"platform" | "tenant">("tenant");
  const [selectedTenantId, setSelectedTenantId] = React.useState<string>();

  const finishLogin = React.useCallback(
    (resp: {
      token?: string;
      active_scope?: "platform" | "tenant";
      active_tenant_id?: string;
      display_name: string;
      available_scopes?: Array<"platform" | "tenant">;
    }) => {
      const fallbackPath = resp.active_scope === "platform" ? "/platform" : "/";
      const target = !redirect || redirect === "/" || redirect === "/login"
        ? fallbackPath
        : redirect;
      if (resp.token) setJwt(resp.token);
      if (resp.active_tenant_id) setTenantId(resp.active_tenant_id);
      if (resp.active_scope) setAppMode(resp.active_scope);
      setDisplayName(resp.display_name);
      setAvailableScopes(resp.available_scopes ?? ["tenant"]);
      navigate({ to: target as never, replace: true });
    },
    [navigate, redirect, setAppMode, setAvailableScopes, setDisplayName, setJwt, setTenantId],
  );

  const selectScopeAndEnter = React.useCallback(
    async (username: string, loginTransactionToken: string, nextScope: "platform" | "tenant", tenantId?: string) => {
      const resp = await selectMut.mutateAsync({
        username,
        login_transaction_token: loginTransactionToken,
        scope: nextScope,
        tenant_id: tenantId,
      });
      finishLogin(resp);
    },
    [finishLogin, selectMut],
  );

  const chooseInitialTenant = React.useCallback(
    (options: LoginTenantOption[], preferredTenantId?: string | null) => {
      const lastTenantId = getLastStoredTenantId();
      if (lastTenantId && options.some((option) => option.tenant_id === lastTenantId)) {
        return lastTenantId;
      }
      if (preferredTenantId && options.some((option) => option.tenant_id === preferredTenantId)) {
        return preferredTenantId;
      }
      return options[0]?.tenant_id;
    },
    [],
  );

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
      <Card style={{ width: "100%", maxWidth: 520 }}>
        <Space direction="vertical" size="large" style={{ width: "100%" }}>
          <div>
            <Typography.Title level={3} style={{ marginBottom: 8 }}>
              登录 DMSX 控制台
            </Typography.Title>
            <Typography.Text type="secondary">
              账号密码认证通过后，会根据账号的平台权限和租户权限自动进入，或提示你选择进入范围。
            </Typography.Text>
          </div>

          {authMode === "disabled" ? (
            <Alert
              type="warning"
              showIcon
              message="当前为 disabled 联调模式"
              description="账号密码登录仅在 jwt 模式下可用。请先把后端切到 jwt 模式。"
            />
          ) : null}

          {pendingLogin ? (
            <Space direction="vertical" size="large" style={{ width: "100%" }}>
              <Alert
                type="success"
                showIcon
                message={`认证通过：${pendingLogin.displayName}`}
                description="根据当前账号权限，继续选择要进入的平台或租户。"
              />

              {pendingLogin.kind === "choose_scope" && (
                <Form layout="vertical">
                  <Form.Item label="进入范围">
                    <Radio.Group
                      value={scope}
                      onChange={(event) => setScope(event.target.value)}
                      options={[
                        { value: "platform", label: "进入平台管理" },
                        { value: "tenant", label: "进入租户管理" },
                      ]}
                    />
                  </Form.Item>
                </Form>
              )}

              {(pendingLogin.kind === "choose_tenant" || scope === "tenant") && (
                <Form layout="vertical">
                  <Form.Item label="选择租户">
                    <Radio.Group
                      value={selectedTenantId}
                      onChange={(event) => setSelectedTenantId(event.target.value)}
                    >
                      <Space direction="vertical">
                        {pendingLogin.tenantOptions.map((option) => (
                          <Radio key={option.tenant_id} value={option.tenant_id}>
                            {option.tenant_id} ({option.roles.join(", ") || "无角色"})
                          </Radio>
                        ))}
                      </Space>
                    </Radio.Group>
                  </Form.Item>
                </Form>
              )}

              <Space>
                <Button
                  type="primary"
                  loading={selectMut.isPending}
                  onClick={async () => {
                    if (scope === "tenant" && !selectedTenantId) {
                      message.error("请选择要进入的租户");
                      return;
                    }
                    try {
                      await selectScopeAndEnter(
                        pendingLogin.username,
                        pendingLogin.loginTransactionToken,
                        scope,
                        scope === "tenant" ? selectedTenantId : selectedTenantId,
                      );
                    } catch (error) {
                      message.error(formatApiError(error));
                    }
                  }}
                >
                  进入控制台
                </Button>
                <Button
                  onClick={() => {
                    setPendingLogin(null);
                    setScope("tenant");
                    setSelectedTenantId(undefined);
                  }}
                >
                  返回登录
                </Button>
              </Space>
            </Space>
          ) : (
            <Form
              form={form}
              layout="vertical"
              initialValues={{ username: "", password: "" }}
              onFinish={async (values) => {
                if (authMode !== "jwt") {
                  message.error("请先切换到 jwt 模式");
                  return;
                }

                try {
                  const resp = await loginMut.mutateAsync(values);

                  if (resp.decision.kind === "platform_only") {
                    if (!resp.login_transaction_token) throw new Error("登录选择凭证缺失，请重新登录");
                    await selectScopeAndEnter(
                      resp.username,
                      resp.login_transaction_token,
                      "platform",
                      chooseInitialTenant(resp.decision.tenant_options, resp.decision.preferred_tenant_id),
                    );
                    return;
                  }

                  if (resp.decision.kind === "tenant_only") {
                    if (!resp.login_transaction_token) throw new Error("登录选择凭证缺失，请重新登录");
                    await selectScopeAndEnter(
                      resp.username,
                      resp.login_transaction_token,
                      "tenant",
                      chooseInitialTenant(resp.decision.tenant_options, resp.decision.preferred_tenant_id),
                    );
                    return;
                  }

                  if (!resp.login_transaction_token) throw new Error("登录选择凭证缺失，请重新登录");
                  setPendingLogin({
                    username: resp.username,
                    loginTransactionToken: resp.login_transaction_token,
                    displayName: resp.display_name,
                    kind: resp.decision.kind,
                    tenantOptions: resp.decision.tenant_options,
                    preferredTenantId: resp.decision.preferred_tenant_id,
                  });
                  setScope(resp.decision.kind === "choose_scope" ? "tenant" : "tenant");
                  setSelectedTenantId(
                    chooseInitialTenant(resp.decision.tenant_options, resp.decision.preferred_tenant_id),
                  );
                } catch (error) {
                  message.error(formatApiError(error));
                }
              }}
            >
              <Form.Item
                name="username"
                label="账号"
                rules={[{ required: true, message: "请输入账号" }]}
              >
                <Input placeholder="例如：platform / tenant / hybrid / multitenant" />
              </Form.Item>

              <Form.Item
                name="password"
                label="密码"
                rules={[{ required: true, message: "请输入密码" }]}
              >
                <Input.Password placeholder="请输入密码" />
              </Form.Item>

              <Space>
                <Button type="primary" htmlType="submit" loading={loginMut.isPending}>
                  登录
                </Button>
                <Button onClick={() => setAuthMode("jwt" as AuthMode)}>JWT 模式</Button>
              </Space>
            </Form>
          )}
        </Space>
      </Card>
    </div>
  );
};
