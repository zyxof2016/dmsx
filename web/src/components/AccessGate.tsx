import React from "react";
import { Alert, Button, Result, Space, Tag, Typography } from "antd";

type Props = {
  title: string;
  description: string;
  roles: string[];
  modeLabel: string;
  onGoDefault: () => void;
  onSwitchMode?: () => void;
  switchModeLabel?: string;
};

export const AccessGate: React.FC<Props> = ({
  title,
  description,
  roles,
  modeLabel,
  onGoDefault,
  onSwitchMode,
  switchModeLabel,
}) => {
  return (
    <Result
      status="403"
      title={title}
      subTitle={description}
      extra={
        <Space wrap>
          <Button type="primary" onClick={onGoDefault}>
            返回当前模式首页
          </Button>
          {onSwitchMode && switchModeLabel && (
            <Button onClick={onSwitchMode}>{switchModeLabel}</Button>
          )}
        </Space>
      }
    >
      <Space direction="vertical" size="middle" style={{ width: "100%" }}>
        <Alert
          type="info"
          showIcon
          message="访问受限"
          description={`当前页面不属于${modeLabel}可访问范围，或当前 JWT/角色不足。后端仍会继续做最终权限校验。`}
        />
        <div>
          <Typography.Text type="secondary">当前有效角色：</Typography.Text>
          <div style={{ marginTop: 8 }}>
            <Space wrap>
              {roles.length ? roles.map((role) => <Tag key={role}>{role}</Tag>) : <Tag>无角色</Tag>}
            </Space>
          </div>
        </div>
      </Space>
    </Result>
  );
};
