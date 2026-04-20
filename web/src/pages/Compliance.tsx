import React, { useState } from "react";
import {
  Typography,
  Table,
  Tag,
  Card,
  Row,
  Col,
  Statistic,
  Progress,
  Spin,
  Alert,
  Button,
  Space,
  Select,
  Input,
  Empty,
} from "antd";
import { SyncOutlined, DownloadOutlined } from "@ant-design/icons";
import dayjs from "dayjs";
import { useFindings, useStats, exportCsv } from "../api/hooks";
import type { ComplianceFinding, ListParams } from "../api/types";
import { formatApiError } from "../api/errors";

const { Title } = Typography;

const severityColor: Record<string, string> = {
  critical: "red",
  high: "orange",
  medium: "gold",
  low: "green",
  info: "blue",
};

const statusOptions = [
  { value: "open", label: "待修复" },
  { value: "accepted", label: "已接受" },
  { value: "fixed", label: "已修复" },
  { value: "false_positive", label: "误报" },
];

const severityOptions = [
  { value: "critical", label: "严重" },
  { value: "high", label: "高" },
  { value: "medium", label: "中" },
  { value: "low", label: "低" },
  { value: "info", label: "信息" },
];

export const CompliancePage: React.FC = () => {
  const { data: stats } = useStats();
  const [severityFilter, setSeverityFilter] = useState<string>();
  const [statusFilter, setStatusFilter] = useState<string>();
  const [search, setSearch] = useState("");
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(10);

  const params: ListParams = {
    limit: pageSize,
    offset: (page - 1) * pageSize,
    search: search || undefined,
    severity: severityFilter || undefined,
    status: statusFilter || undefined,
  };

  const { data, isLoading, error, refetch } = useFindings(params);

  const items = data?.items ?? [];
  const total = data?.total ?? 0;
  const deviceTotal = stats?.device_total ?? 1;
  const findingOpen = stats?.finding_open ?? 0;

  const openDevices = new Set(
    items.filter((i) => i.status === "open").map((i) => i.device_id),
  );
  const complianceRate =
    deviceTotal > 0
      ? (((deviceTotal - openDevices.size) / deviceTotal) * 100).toFixed(1)
      : "100.0";

  if (error) {
    return (
      <Alert
        type="error"
        message="加载失败"
        description={formatApiError(error)}
        showIcon
      />
    );
  }

  return (
    <Spin spinning={isLoading}>
      <Title level={4}>安全合规</Title>
      <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
        <Col xs={12} lg={6}>
          <Card>
            <Statistic
              title="合规率"
              value={Number(complianceRate)}
              suffix="%"
              valueStyle={{ color: "#52c41a" }}
            />
          </Card>
        </Col>
        <Col xs={12} lg={6}>
          <Card>
            <Statistic
              title="未修复发现"
              value={findingOpen}
              valueStyle={{ color: "#faad14" }}
            />
          </Card>
        </Col>
        <Col xs={12} lg={6}>
          <Card>
            <div style={{ textAlign: "center" }}>
              <Progress
                type="circle"
                percent={Number(complianceRate)}
                size={80}
              />
              <div style={{ marginTop: 4, fontSize: 12 }}>总体合规</div>
            </div>
          </Card>
        </Col>
        <Col xs={12} lg={6}>
          <Card>
            <Statistic title="发现总数" value={total} />
          </Card>
        </Col>
      </Row>
      <Card
        extra={
          <Space>
            <Button icon={<SyncOutlined />} onClick={() => refetch()}>
              刷新
            </Button>
            <Button
              icon={<DownloadOutlined />}
              onClick={() =>
                exportCsv(
                  items as unknown as Record<string, unknown>[],
                  "findings.csv",
                )
              }
              disabled={items.length === 0}
            >
              导出 CSV
            </Button>
          </Space>
        }
      >
        <Space style={{ marginBottom: 12 }} wrap>
          <Input.Search
            placeholder="搜索规则 / 标题"
            style={{ width: 220 }}
            value={search}
            onChange={(e) => {
              setSearch(e.target.value);
              setPage(1);
            }}
            allowClear
          />
          <Select
            value={severityFilter}
            style={{ width: 120 }}
            onChange={(v) => {
              setSeverityFilter(v || undefined);
              setPage(1);
            }}
            allowClear
            placeholder="严重度"
            options={severityOptions}
          />
          <Select
            value={statusFilter}
            style={{ width: 120 }}
            onChange={(v) => {
              setStatusFilter(v || undefined);
              setPage(1);
            }}
            allowClear
            placeholder="状态"
            options={statusOptions}
          />
        </Space>
        <Table<ComplianceFinding>
          rowKey="id"
          dataSource={items}
          size="small"
          locale={{
            emptyText: <Empty description="暂无合规发现" />,
          }}
          pagination={{
            current: page,
            pageSize,
            total,
            showSizeChanger: true,
            showTotal: (t) => `共 ${t} 条`,
            onChange: (p, ps) => {
              setPage(p);
              setPageSize(ps);
            },
          }}
          columns={[
            { title: "规则", dataIndex: "rule_id", width: 140 },
            { title: "描述", dataIndex: "title" },
            {
              title: "严重级",
              dataIndex: "severity",
              render: (s: string) => (
                <Tag color={severityColor[s]}>{s}</Tag>
              ),
            },
            {
              title: "状态",
              dataIndex: "status",
              render: (s: string) => (
                <Tag color={s === "open" ? "red" : "blue"}>{s}</Tag>
              ),
            },
            {
              title: "设备",
              dataIndex: "device_id",
              render: (id: string) => id.slice(0, 8) + "…",
            },
            {
              title: "检测时间",
              dataIndex: "detected_at",
              render: (t: string) => dayjs(t).format("YYYY-MM-DD HH:mm"),
            },
          ]}
        />
      </Card>
    </Spin>
  );
};
