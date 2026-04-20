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
  Tabs,
} from "antd";
import { CopyOutlined } from "@ant-design/icons";
import { useNavigate, useParams, useSearch } from "@tanstack/react-router";
import dayjs from "dayjs";
import { useDevice } from "../api/hooks";
import { ShadowPanel } from "./ShadowPanel";
import { RemoteControlPanel } from "./RemoteControl";
import { formatApiError } from "../api/errors";

const RemoteDesktopPanel = React.lazy(async () => {
  const mod = await import("./RemoteDesktop");
  return { default: mod.RemoteDesktopPanel };
});

const { Text } = Typography;

const platformLabel: Record<string, string> = {
  windows: "Windows",
  linux: "Linux",
  macos: "macOS",
  ios: "iOS",
  android: "Android",
  edge: "Edge/IoT",
  other: "其他",
};

const stateTag = (s: string) => {
  const map: Record<string, { color: string; label: string }> = {
    online: { color: "green", label: "在线" },
    offline: { color: "red", label: "离线" },
    unknown: { color: "default", label: "未知" },
  };
  const { color, label } = map[s] ?? { color: "default", label: s };
  return <Tag color={color}>{label}</Tag>;
};

const enrollTag = (s: string) => {
  const map: Record<string, string> = {
    active: "green",
    pending: "gold",
    revoked: "red",
    blocked: "red",
  };
  return <Tag color={map[s] ?? "default"}>{s}</Tag>;
};

export const DeviceDetailDrawer: React.FC = () => {
  const { deviceId } = useParams({ strict: false });
  const search = useSearch({ from: "/devices/$deviceId" });
  const navigate = useNavigate();
  const { data: device, isLoading, error } = useDevice(deviceId);
  const activeTab = search.tab ?? "info";

  return (
    <Drawer
      open
      width={800}
      title={`设备详情${device?.hostname ? ` — ${device.hostname}` : ""}`}
      onClose={() => navigate({ to: "/devices" })}
      extra={
        <Tooltip title="复制设备 ID">
          <Button
            size="small"
            icon={<CopyOutlined />}
            onClick={() => navigator.clipboard.writeText(deviceId ?? "")}
          />
        </Tooltip>
      }
    >
      {error && (
        <Alert
          type="error"
          message="加载失败"
          description={formatApiError(error)}
          showIcon
        />
      )}
      <Spin spinning={isLoading}>
        {device && (
          <Tabs
            activeKey={activeTab}
            onChange={(tab) => {
              navigate({
                to: "/devices/$deviceId",
                params: { deviceId: device.id },
                search: { tab },
                replace: true,
              });
            }}
            items={[
              {
                key: "info",
                label: "基本信息",
                children: (
                  <Descriptions column={1} bordered size="small">
                    <Descriptions.Item label="ID">
                      <Text code copyable={{ text: device.id }}>
                        {device.id}
                      </Text>
                    </Descriptions.Item>
                    <Descriptions.Item label="主机名">
                      {device.hostname ?? "—"}
                    </Descriptions.Item>
                    <Descriptions.Item label="平台">
                      {platformLabel[device.platform] ?? device.platform}
                    </Descriptions.Item>
                    <Descriptions.Item label="系统版本">
                      {device.os_version ?? "—"}
                    </Descriptions.Item>
                    <Descriptions.Item label="Agent 版本">
                      {device.agent_version ?? "—"}
                    </Descriptions.Item>
                    <Descriptions.Item label="在线状态">
                      {stateTag(device.online_state)}
                    </Descriptions.Item>
                    <Descriptions.Item label="注册状态">
                      {enrollTag(device.enroll_status)}
                    </Descriptions.Item>
                    <Descriptions.Item label="最后心跳">
                      {device.last_seen_at
                        ? dayjs(device.last_seen_at).format("YYYY-MM-DD HH:mm:ss")
                        : "—"}
                    </Descriptions.Item>
                    <Descriptions.Item label="站点 ID">
                      {device.site_id ?? "—"}
                    </Descriptions.Item>
                    <Descriptions.Item label="分组 ID">
                      {device.primary_group_id ?? "—"}
                    </Descriptions.Item>
                    <Descriptions.Item label="标签 (labels)">
                      <Text code>{JSON.stringify(device.labels, null, 2)}</Text>
                    </Descriptions.Item>
                    <Descriptions.Item label="能力 (capabilities)">
                      <Text code>{JSON.stringify(device.capabilities, null, 2)}</Text>
                    </Descriptions.Item>
                    <Descriptions.Item label="创建时间">
                      {dayjs(device.created_at).format("YYYY-MM-DD HH:mm:ss")}
                    </Descriptions.Item>
                    <Descriptions.Item label="更新时间">
                      {dayjs(device.updated_at).format("YYYY-MM-DD HH:mm:ss")}
                    </Descriptions.Item>
                  </Descriptions>
                ),
              },
              {
                key: "shadow",
                label: "设备影子",
                children: <ShadowPanel deviceId={device.id} />,
              },
              {
                key: "remote",
                label: "远控面板",
                children: (
                  <RemoteControlPanel
                    deviceId={device.id}
                    deviceHostname={device.hostname ?? undefined}
                  />
                ),
              },
              {
                key: "desktop",
                label: "远程桌面",
                children: (
                  activeTab === "desktop" ? (
                    <React.Suspense fallback={<Spin />}>
                      <RemoteDesktopPanel
                        deviceId={device.id}
                        deviceHostname={device.hostname ?? undefined}
                        devicePlatform={device.platform}
                        deviceOnlineState={device.online_state}
                      />
                    </React.Suspense>
                  ) : null
                ),
              },
            ]}
          />
        )}
      </Spin>
    </Drawer>
  );
};
