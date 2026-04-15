import React from "react";
import { Typography, Card, Table, Tag, Row, Col, Statistic, Alert } from "antd";
import { GlobalOutlined, LinkOutlined, WifiOutlined } from "@ant-design/icons";

const { Title } = Typography;

const mockSites = [
  {
    key: "1",
    name: "北京总部",
    type: "HQ",
    devices: 5200,
    bandwidth: "10 Gbps",
    status: "healthy",
  },
  {
    key: "2",
    name: "上海分部",
    type: "Branch",
    devices: 3100,
    bandwidth: "1 Gbps",
    status: "healthy",
  },
  {
    key: "3",
    name: "广州IDC",
    type: "DC",
    devices: 4000,
    bandwidth: "40 Gbps",
    status: "degraded",
  },
  {
    key: "4",
    name: "成都边缘",
    type: "Edge",
    devices: 547,
    bandwidth: "200 Mbps",
    status: "healthy",
  },
];

export const NetworkPage: React.FC = () => (
  <div>
    <Title level={4}>网络管控</Title>
    <Alert
      type="warning"
      message="演示数据"
      description="以下为静态演示数据，接入后端网络管控 API 后将显示真实站点和隧道信息。"
      showIcon
      closable
      style={{ marginBottom: 16 }}
    />
    <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
      <Col xs={12} lg={8}>
        <Card>
          <Statistic title="站点总数" value={4} prefix={<GlobalOutlined />} />
        </Card>
      </Col>
      <Col xs={12} lg={8}>
        <Card>
          <Statistic
            title="活跃隧道"
            value={12}
            prefix={<LinkOutlined />}
          />
        </Card>
      </Col>
      <Col xs={12} lg={8}>
        <Card>
          <Statistic
            title="总带宽"
            value="51.2 Gbps"
            prefix={<WifiOutlined />}
          />
        </Card>
      </Col>
    </Row>
    <Card>
      <Table
        dataSource={mockSites}
        columns={[
          { title: "站点", dataIndex: "name" },
          {
            title: "类型",
            dataIndex: "type",
            render: (t: string) => <Tag>{t}</Tag>,
          },
          { title: "设备数", dataIndex: "devices" },
          { title: "带宽", dataIndex: "bandwidth" },
          {
            title: "状态",
            dataIndex: "status",
            render: (s: string) => (
              <Tag color={s === "healthy" ? "green" : "orange"}>
                {s === "healthy" ? "正常" : "降级"}
              </Tag>
            ),
          },
        ]}
        size="small"
      />
    </Card>
  </div>
);
