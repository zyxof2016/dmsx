import React, { useRef } from "react";
import {
  Row,
  Col,
  Card,
  Statistic,
  Tag,
  List,
  Progress,
  Typography,
  Spin,
  Alert,
} from "antd";
import {
  DesktopOutlined,
  CheckCircleOutlined,
  WarningOutlined,
  ClockCircleOutlined,
  RobotOutlined,
  SafetyOutlined,
} from "@ant-design/icons";
import {
  PieChart,
  Pie,
  Cell,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip as RTooltip,
  LineChart,
  Line,
  ResponsiveContainer,
  Legend,
} from "recharts";
import { useStats, useFindings } from "../api/hooks";
import type { DashboardStats } from "../api/types";

const { Title, Text } = Typography;

const PIE_COLORS = [
  "#1677ff",
  "#52c41a",
  "#faad14",
  "#f5222d",
  "#722ed1",
  "#eb2f96",
  "#13c2c2",
];

const SEVERITY_COLORS: Record<string, string> = {
  critical: "#f5222d",
  high: "#fa541c",
  medium: "#faad14",
  low: "#52c41a",
  info: "#1677ff",
};

const CMD_COLORS: Record<string, string> = {
  queued: "#d9d9d9",
  delivered: "#1677ff",
  running: "#1677ff",
  succeeded: "#52c41a",
  failed: "#f5222d",
  expired: "#faad14",
  cancelled: "#bfbfbf",
  acked: "#13c2c2",
};

export const DashboardPage: React.FC = () => {
  const { data: stats, isLoading, error } = useStats();
  const { data: findingsData } = useFindings();

  const historyRef = useRef<{ time: string; online: number; offline: number }[]>([]);
  if (stats) {
    const now = new Date().toLocaleTimeString("zh-CN", {
      hour: "2-digit",
      minute: "2-digit",
    });
    const last = historyRef.current[historyRef.current.length - 1];
    if (!last || last.time !== now) {
      historyRef.current = [
        ...historyRef.current.slice(-19),
        {
          time: now,
          online: stats.device_online,
          offline: stats.device_total - stats.device_online,
        },
      ];
    }
  }

  if (error) {
    return (
      <Alert
        type="error"
        message="加载失败"
        description={String(error)}
        showIcon
      />
    );
  }

  const total = stats?.device_total ?? 0;
  const online = stats?.device_online ?? 0;
  const onlinePct = total > 0 ? ((online / total) * 100).toFixed(1) : "0";
  const findingOpen = stats?.finding_open ?? 0;
  const complianceRate = computeComplianceRate(stats, findingsData?.items);

  return (
    <Spin spinning={isLoading}>
      <Title level={4}>态势总览</Title>

      {/* KPI cards */}
      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic
              title="在线设备"
              value={online}
              prefix={<DesktopOutlined />}
              suffix={<Tag color="green">{onlinePct}%</Tag>}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic
              title="策略合规率"
              value={Number(complianceRate)}
              precision={1}
              prefix={<CheckCircleOutlined />}
              suffix="%"
              valueStyle={{ color: "#52c41a" }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic
              title="待处理发现"
              value={findingOpen}
              prefix={<WarningOutlined />}
              valueStyle={{ color: "#faad14" }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic
              title="执行中命令"
              value={stats?.command_pending ?? 0}
              prefix={<ClockCircleOutlined />}
            />
          </Card>
        </Col>
      </Row>

      {/* Charts row */}
      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        <Col xs={24} lg={8}>
          <Card title="设备平台分布" size="small">
            <ResponsiveContainer width="100%" height={220}>
              <PieChart>
                <Pie
                  data={stats?.platforms ?? []}
                  dataKey="count"
                  nameKey="label"
                  cx="50%"
                  cy="50%"
                  outerRadius={80}
                  label={({ label, count }) => `${label} (${count})`}
                >
                  {(stats?.platforms ?? []).map((_, i) => (
                    <Cell
                      key={i}
                      fill={PIE_COLORS[i % PIE_COLORS.length]}
                    />
                  ))}
                </Pie>
                <RTooltip />
              </PieChart>
            </ResponsiveContainer>
          </Card>
        </Col>

        <Col xs={24} lg={8}>
          <Card title="命令状态分布" size="small">
            <ResponsiveContainer width="100%" height={220}>
              <BarChart data={stats?.command_statuses ?? []}>
                <CartesianGrid strokeDasharray="3 3" />
                <XAxis dataKey="label" tick={{ fontSize: 11 }} />
                <YAxis allowDecimals={false} />
                <RTooltip />
                <Bar dataKey="count" name="数量">
                  {(stats?.command_statuses ?? []).map((s, i) => (
                    <Cell
                      key={i}
                      fill={CMD_COLORS[s.label] ?? "#8884d8"}
                    />
                  ))}
                </Bar>
              </BarChart>
            </ResponsiveContainer>
          </Card>
        </Col>

        <Col xs={24} lg={8}>
          <Card title="合规发现严重度" size="small">
            <ResponsiveContainer width="100%" height={220}>
              <BarChart
                data={stats?.finding_severities ?? []}
                layout="vertical"
              >
                <CartesianGrid strokeDasharray="3 3" />
                <XAxis type="number" allowDecimals={false} />
                <YAxis
                  dataKey="label"
                  type="category"
                  tick={{ fontSize: 12 }}
                  width={60}
                />
                <RTooltip />
                <Bar dataKey="count" name="数量">
                  {(stats?.finding_severities ?? []).map((s, i) => (
                    <Cell
                      key={i}
                      fill={SEVERITY_COLORS[s.label] ?? "#8884d8"}
                    />
                  ))}
                </Bar>
              </BarChart>
            </ResponsiveContainer>
          </Card>
        </Col>
      </Row>

      {/* Online trend + insights */}
      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        <Col xs={24} lg={12}>
          <Card title="在线设备趋势" size="small">
            <ResponsiveContainer width="100%" height={200}>
              <LineChart data={historyRef.current}>
                <CartesianGrid strokeDasharray="3 3" />
                <XAxis dataKey="time" tick={{ fontSize: 11 }} />
                <YAxis allowDecimals={false} />
                <RTooltip />
                <Legend />
                <Line
                  type="monotone"
                  dataKey="online"
                  stroke="#52c41a"
                  name="在线"
                  dot={false}
                />
                <Line
                  type="monotone"
                  dataKey="offline"
                  stroke="#f5222d"
                  name="离线"
                  dot={false}
                />
              </LineChart>
            </ResponsiveContainer>
          </Card>
        </Col>

        <Col xs={24} lg={6}>
          <Card
            title={
              <span>
                <RobotOutlined style={{ marginRight: 8 }} />
                AI 洞察
              </span>
            }
            size="small"
          >
            <List
              size="small"
              dataSource={buildInsights(stats)}
              renderItem={(item) => (
                <List.Item>
                  <Tag
                    color={
                      item.level === "warning"
                        ? "orange"
                        : item.level === "success"
                          ? "green"
                          : "blue"
                    }
                  >
                    {item.level === "warning"
                      ? "告警"
                      : item.level === "success"
                        ? "正常"
                        : "信息"}
                  </Tag>
                  <Text>{item.text}</Text>
                </List.Item>
              )}
            />
          </Card>
        </Col>

        <Col xs={24} lg={6}>
          <Card
            title={
              <span>
                <SafetyOutlined style={{ marginRight: 8 }} />
                安全态势
              </span>
            }
            size="small"
          >
            <div style={{ textAlign: "center", marginBottom: 8 }}>
              <Progress
                type="dashboard"
                percent={Number(complianceRate)}
                format={(p) => `${p}%`}
                size={100}
              />
              <div>合规率</div>
            </div>
          </Card>
        </Col>
      </Row>
    </Spin>
  );
};

function computeComplianceRate(
  stats: DashboardStats | undefined,
  findings: { status: string; device_id: string }[] | undefined,
): string {
  const total = stats?.device_total ?? 0;
  if (total === 0) return "100.0";
  const openFindings = (findings ?? []).filter((f) => f.status === "open");
  const affectedDevices = new Set(openFindings.map((f) => f.device_id));
  return (((total - affectedDevices.size) / total) * 100).toFixed(1);
}

function buildInsights(
  stats: DashboardStats | undefined,
): { text: string; level: string }[] {
  if (!stats) return [];
  const items: { text: string; level: string }[] = [];
  items.push({
    text: `共 ${stats.device_total} 台设备，${stats.device_online} 台在线`,
    level: "info",
  });
  items.push({
    text: `${stats.policy_count} 条策略生效中`,
    level: "success",
  });
  if (stats.finding_open > 0) {
    items.push({
      text: `${stats.finding_open} 条合规发现待处理`,
      level: "warning",
    });
  }
  if (stats.command_pending > 0) {
    items.push({
      text: `${stats.command_pending} 条命令执行中`,
      level: "info",
    });
  }
  return items;
}
