import React from "react";
import {
  App,
  Avatar,
  Badge,
  Button,
  Dropdown,
  FloatButton,
  Input,
  Modal,
  Select,
  Space,
  Tag,
  Typography,
} from "antd";
import { BellOutlined, RobotOutlined, UserOutlined } from "@ant-design/icons";
import { useLogout } from "../api/hooks";
import type { TenantOption } from "../appProviders";

type Props = {
  tenantId: string;
  tenantOptions: TenantOption[];
  jwt: string;
  userLabel: string;
  profileLabel: string;
  logoutLabel: string;
  aiTooltip: string;
  setTenantId: (tenantId: string) => void;
  setJwt: (jwt: string) => void;
  clearJwt: () => void;
  onLoggedOut: () => void;
  showTenantShortcut: boolean;
  onOpenAi: () => void;
};

export const AppDeferredTools: React.FC<Props> = ({
  tenantId,
  tenantOptions,
  jwt,
  userLabel,
  profileLabel,
  logoutLabel,
  aiTooltip,
  setTenantId,
  setJwt,
  clearJwt,
  onLoggedOut,
  showTenantShortcut,
  onOpenAi,
}) => {
  const { message } = App.useApp();
  const logoutMut = useLogout();
  const [jwtModalOpen, setJwtModalOpen] = React.useState(false);
  const [tenantModalOpen, setTenantModalOpen] = React.useState(false);
  const [jwtDraft, setJwtDraft] = React.useState("");
  const [tenantDraft, setTenantDraft] = React.useState("");

  const shortTenantId = `${tenantId.slice(0, 8)}...${tenantId.slice(-4)}`;

  const userMenu = {
    items: [
      { key: "profile", label: profileLabel },
      ...(showTenantShortcut
        ? [{ key: "set_tenant", label: "设置活动租户" }]
        : []),
      { key: "set_jwt", label: "设置 JWT（用于 jwt 模式联调）" },
      { key: "clear_jwt", label: "清除 JWT" },
      { key: "logout", label: logoutLabel },
    ],
    onClick: ({ key }: { key: string }) => {
      if (key === "set_tenant") {
        setTenantDraft(tenantId);
        setTenantModalOpen(true);
      } else if (key === "set_jwt") {
        setJwtDraft(jwt);
        setJwtModalOpen(true);
      } else if (key === "clear_jwt") {
        clearJwt();
        message.success("已清除 JWT");
      } else if (key === "logout") {
        void logoutMut
          .mutateAsync({ tenant_id: tenantId })
          .catch(() => undefined)
          .finally(() => {
            clearJwt();
            onLoggedOut();
            message.success("已退出登录");
          });
      }
    },
  };

  return (
    <>
      {showTenantShortcut && (
        <Tag
          className="dmsx-tenant-chip"
          color="blue"
          onClick={() => {
            setTenantDraft(tenantId);
            setTenantModalOpen(true);
          }}
        >
          租户 {shortTenantId}
        </Tag>
      )}
      <Badge count={0} size="small">
        <Button className="dmsx-tool-button" type="text" icon={<BellOutlined />} />
      </Badge>
      <Dropdown menu={userMenu}>
        <Space className="dmsx-user-trigger">
          <Avatar size={24} icon={<UserOutlined />} />
          <Typography.Text className="dmsx-user-name">{userLabel}</Typography.Text>
        </Space>
      </Dropdown>

      <FloatButton
        icon={<RobotOutlined />}
        type="primary"
        tooltip={aiTooltip}
        onClick={onOpenAi}
        style={{ insetInlineEnd: 32, insetBlockEnd: 32 }}
      />

      <Modal
        title="设置 JWT"
        open={jwtModalOpen}
        onCancel={() => setJwtModalOpen(false)}
        onOk={() => {
          const value = jwtDraft.trim();
          if (!value) {
            message.error("JWT 不能为空");
            return;
          }
          setJwt(value);
          setJwtModalOpen(false);
          message.success("JWT 已保存");
        }}
        okText="保存"
        cancelText="取消"
        destroyOnHidden
      >
        <Input.TextArea
          value={jwtDraft}
          onChange={(event) => setJwtDraft(event.target.value)}
          rows={6}
          placeholder="粘贴形如：xxxx.yyyy.zzzz 的 JWT（可带或不带 Bearer 前缀）"
        />
      </Modal>

      {showTenantShortcut && (
        <Modal
          title="设置活动租户"
          open={tenantModalOpen}
        onCancel={() => setTenantModalOpen(false)}
        onOk={() => {
          const value = tenantDraft.trim();
          if (!value) {
            message.error("请选择一个租户");
            return;
          }
          setTenantId(value);
            setTenantModalOpen(false);
            message.success("活动租户已更新");
          }}
          okText="保存"
          cancelText="取消"
          destroyOnHidden
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
            style={{ width: "100%" }}
          />
        </Modal>
      )}
    </>
  );
};
