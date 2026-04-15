import React from "react";
import {
  Drawer,
  Descriptions,
  Tag,
  Spin,
  Alert,
  Typography,
  Button,
  Tooltip,
} from "antd";
import { CopyOutlined } from "@ant-design/icons";
import { useNavigate, useParams } from "@tanstack/react-router";
import dayjs from "dayjs";
import { usePolicy } from "../api/hooks";

const { Text } = Typography;

const scopeLabel: Record<string, string> = {
  tenant: "租户",
  org: "组织",
  site: "站点",
  group: "分组",
  label: "标签",
};

export const PolicyDetailDrawer: React.FC = () => {
  const { policyId } = useParams({ strict: false });
  const navigate = useNavigate();
  const { data: policy, isLoading, error } = usePolicy(policyId);

  return (
    <Drawer
      open
      width={560}
      title="策略详情"
      onClose={() => navigate({ to: "/policies" })}
      extra={
        <Tooltip title="复制策略 ID">
          <Button
            size="small"
            icon={<CopyOutlined />}
            onClick={() => navigator.clipboard.writeText(policyId ?? "")}
          />
        </Tooltip>
      }
    >
      {error && (
        <Alert
          type="error"
          message="加载失败"
          description={String(error)}
          showIcon
        />
      )}
      <Spin spinning={isLoading}>
        {policy && (
          <Descriptions column={1} bordered size="small">
            <Descriptions.Item label="ID">
              <Text code copyable={{ text: policy.id }}>
                {policy.id}
              </Text>
            </Descriptions.Item>
            <Descriptions.Item label="策略名称">
              {policy.name}
            </Descriptions.Item>
            <Descriptions.Item label="描述">
              {policy.description ?? "—"}
            </Descriptions.Item>
            <Descriptions.Item label="作用域">
              <Tag>{scopeLabel[policy.scope_kind] ?? policy.scope_kind}</Tag>
            </Descriptions.Item>
            {policy.scope_org_id && (
              <Descriptions.Item label="作用域组织 ID">
                {policy.scope_org_id}
              </Descriptions.Item>
            )}
            {policy.scope_site_id && (
              <Descriptions.Item label="作用域站点 ID">
                {policy.scope_site_id}
              </Descriptions.Item>
            )}
            {policy.scope_group_id && (
              <Descriptions.Item label="作用域分组 ID">
                {policy.scope_group_id}
              </Descriptions.Item>
            )}
            {policy.scope_expr && (
              <Descriptions.Item label="作用域表达式">
                <Text code>{policy.scope_expr}</Text>
              </Descriptions.Item>
            )}
            <Descriptions.Item label="创建时间">
              {dayjs(policy.created_at).format("YYYY-MM-DD HH:mm:ss")}
            </Descriptions.Item>
            <Descriptions.Item label="更新时间">
              {dayjs(policy.updated_at).format("YYYY-MM-DD HH:mm:ss")}
            </Descriptions.Item>
          </Descriptions>
        )}
      </Spin>
    </Drawer>
  );
};
