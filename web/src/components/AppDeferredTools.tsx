import React from "react";
import {
  App,
  Avatar,
  Badge,
  Dropdown,
  FloatButton,
  Input,
  Modal,
  Space,
  Tag,
} from "antd";
import { BellOutlined, RobotOutlined, UserOutlined } from "@ant-design/icons";

type Props = {
  tenantId: string;
  jwt: string;
  userLabel: string;
  profileLabel: string;
  logoutLabel: string;
  aiTooltip: string;
  setTenantId: (tenantId: string) => void;
  setJwt: (jwt: string) => void;
  clearJwt: () => void;
  onOpenAi: () => void;
};

const isValidUuid = (value: string) =>
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(value);

export const AppDeferredTools: React.FC<Props> = ({
  tenantId,
  jwt,
  userLabel,
  profileLabel,
  logoutLabel,
  aiTooltip,
  setTenantId,
  setJwt,
  clearJwt,
  onOpenAi,
}) => {
  const { message } = App.useApp();
  const [jwtModalOpen, setJwtModalOpen] = React.useState(false);
  const [tenantModalOpen, setTenantModalOpen] = React.useState(false);
  const [jwtDraft, setJwtDraft] = React.useState("");
  const [tenantDraft, setTenantDraft] = React.useState("");

  const shortTenantId = `${tenantId.slice(0, 8)}...${tenantId.slice(-4)}`;

  const userMenu = {
    items: [
      { key: "profile", label: profileLabel },
      { key: "set_tenant", label: "设置活动租户" },
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
      }
    },
  };

  return (
    <>
      <Tag
        color="blue"
        style={{ cursor: "pointer", marginInlineEnd: 0 }}
        onClick={() => {
          setTenantDraft(tenantId);
          setTenantModalOpen(true);
        }}
      >
        租户 {shortTenantId}
      </Tag>
      <Badge count={0} size="small">
        <BellOutlined style={{ fontSize: 18, cursor: "pointer" }} />
      </Badge>
      <Dropdown menu={userMenu}>
        <Space style={{ cursor: "pointer" }}>
          <Avatar size="small" icon={<UserOutlined />} />
          {userLabel}
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
        destroyOnClose
      >
        <Input.TextArea
          value={jwtDraft}
          onChange={(event) => setJwtDraft(event.target.value)}
          rows={6}
          placeholder="粘贴形如：xxxx.yyyy.zzzz 的 JWT（可带或不带 Bearer 前缀）"
        />
      </Modal>

      <Modal
        title="设置活动租户"
        open={tenantModalOpen}
        onCancel={() => setTenantModalOpen(false)}
        onOk={() => {
          const value = tenantDraft.trim();
          if (!isValidUuid(value)) {
            message.error("租户 ID 必须是合法 UUID");
            return;
          }
          setTenantId(value);
          setTenantModalOpen(false);
          message.success("活动租户已更新");
        }}
        okText="保存"
        cancelText="取消"
        destroyOnClose
      >
        <Input
          value={tenantDraft}
          onChange={(event) => setTenantDraft(event.target.value)}
          placeholder="输入租户 UUID，例如 00000000-0000-0000-0000-000000000001"
        />
      </Modal>
    </>
  );
};
