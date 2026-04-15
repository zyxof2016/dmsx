import React, { useState } from "react";
import {
  Typography,
  Card,
  Tabs,
  Row,
  Col,
  Tag,
  List,
  Button,
  Input,
  Space,
  Timeline,
  Alert,
  Statistic,
  Progress,
  Avatar,
  Tooltip,
} from "antd";
import {
  RobotOutlined,
  ThunderboltOutlined,
  ToolOutlined,
  SendOutlined,
  BulbOutlined,
  UserOutlined,
  ExperimentOutlined,
} from "@ant-design/icons";

const { Title, Text, Paragraph } = Typography;
const { TextArea } = Input;

const DEMO_ALERT = (
  <Alert
    type="warning"
    message="演示数据"
    description="以下为静态演示数据。接入 AI/LLM 后端后将展示真实的异常检测、策略推荐和预测分析结果。"
    showIcon
    closable
    style={{ marginBottom: 16 }}
  />
);

// ---------------------------------------------------------------------------
// AI 异常检测面板
// ---------------------------------------------------------------------------

const AnomalyPanel: React.FC = () => {
  const anomalies = [
    {
      id: 1,
      level: "critical",
      device: "node-gz-7",
      summary: "心跳中断超过 15 分钟，历史无类似模式",
      time: "3 分钟前",
      actions: ["远程诊断", "告警升级"],
    },
    {
      id: 2,
      level: "critical",
      device: "node-bj-12",
      summary: "CPU 持续 100% 超过 30 分钟，疑似挖矿行为",
      time: "8 分钟前",
      actions: ["隔离设备", "取证分析"],
    },
    {
      id: 3,
      level: "warning",
      device: "node-sh-5",
      summary: "Agent 上报版本回退，可能遭到篡改",
      time: "22 分钟前",
      actions: ["重新安装Agent", "安全扫描"],
    },
    {
      id: 4,
      level: "warning",
      device: "edge-cd-*（3台）",
      summary: "命令失败率突增至 42%，网络质量下降",
      time: "35 分钟前",
      actions: ["网络诊断", "切换备路"],
    },
    {
      id: 5,
      level: "info",
      device: "全局",
      summary:
        "批量心跳延迟从 1.2s 上升到 2.8s（P95），建议关注网关负载",
      time: "1 小时前",
      actions: ["查看监控"],
    },
  ];

  const levelColor: Record<string, string> = {
    critical: "red",
    warning: "orange",
    info: "blue",
  };
  const levelLabel: Record<string, string> = {
    critical: "严重",
    warning: "警告",
    info: "信息",
  };

  return (
    <div>
      {DEMO_ALERT}
      <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
        <Col span={6}>
          <Card>
            <Statistic
              title="异常设备"
              value={5}
              valueStyle={{ color: "#f5222d" }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="严重"
              value={2}
              valueStyle={{ color: "#f5222d" }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="警告"
              value={2}
              valueStyle={{ color: "#faad14" }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="检测覆盖"
              value={99.8}
              suffix="%"
              valueStyle={{ color: "#52c41a" }}
            />
          </Card>
        </Col>
      </Row>

      <List
        itemLayout="vertical"
        dataSource={anomalies}
        renderItem={(item) => (
          <List.Item
            actions={item.actions.map((a) => (
              <Button key={a} size="small" type="primary" ghost>
                {a}
              </Button>
            ))}
          >
            <List.Item.Meta
              avatar={
                <Tag color={levelColor[item.level]}>
                  {levelLabel[item.level]}
                </Tag>
              }
              title={
                <span>
                  {item.device} — {item.summary}
                </span>
              }
              description={
                <Text type="secondary">{item.time} · AI 自动检测</Text>
              }
            />
          </List.Item>
        )}
      />
    </div>
  );
};

// ---------------------------------------------------------------------------
// AI 策略推荐面板
// ---------------------------------------------------------------------------

const RecommendationPanel: React.FC = () => {
  const recommendations = [
    {
      name: "启用 Windows BitLocker 全盘加密",
      confidence: 0.94,
      rationale:
        "42 台 Windows 设备未开启磁盘加密，违反 CIS 基线 CIS-WIN-001。启用后可消除 8.6% 的高危发现。",
      risk: "加密过程需 2-4 小时，建议分批灰度。",
      impact: 42,
    },
    {
      name: "禁用 SSH root 直登",
      confidence: 0.98,
      rationale:
        "7 台 Linux 服务器开放 root SSH，属严重合规风险（CIS-LIN-003）。",
      risk: "需确认运维团队已配置 sudoer，否则可能锁死管理通道。",
      impact: 7,
    },
    {
      name: "Agent 自动升级至 1.2.3",
      confidence: 0.87,
      rationale:
        "312 台设备运行旧版 Agent，缺少心跳压缩与安全修复。",
      risk: "升级重启 Agent 约 5 秒中断。灰度 10% → 50% → 100%。",
      impact: 312,
    },
  ];

  return (
    <div>
      {DEMO_ALERT}
      <Alert
        type="info"
        showIcon
        icon={<BulbOutlined />}
        message="AI 策略推荐基于当前设备画像、合规发现、历史命令成功率综合分析"
        style={{ marginBottom: 16 }}
      />
      <List
        itemLayout="vertical"
        dataSource={recommendations}
        renderItem={(item) => (
          <Card style={{ marginBottom: 12 }}>
            <Row gutter={16} align="middle">
              <Col flex="auto">
                <Title level={5} style={{ margin: 0 }}>
                  {item.name}
                </Title>
                <Paragraph type="secondary" style={{ margin: "8px 0" }}>
                  {item.rationale}
                </Paragraph>
                <Space>
                  <Tag color="orange">风险：{item.risk}</Tag>
                  <Tag>影响设备：{item.impact}</Tag>
                </Space>
              </Col>
              <Col>
                <div style={{ textAlign: "center" }}>
                  <Progress
                    type="circle"
                    percent={Math.round(item.confidence * 100)}
                    size={64}
                  />
                  <div style={{ marginTop: 4, fontSize: 12 }}>置信度</div>
                </div>
              </Col>
              <Col>
                <Space direction="vertical">
                  <Button type="primary">采纳并创建策略</Button>
                  <Button>查看详情</Button>
                </Space>
              </Col>
            </Row>
          </Card>
        )}
      />
    </div>
  );
};

// ---------------------------------------------------------------------------
// AI 智能助手（对话式）
// ---------------------------------------------------------------------------

interface ChatMsg {
  role: "user" | "assistant";
  content: string;
  actions?: string[];
}

const ChatPanel: React.FC = () => {
  const [messages, setMessages] = useState<ChatMsg[]>([
    {
      role: "assistant",
      content:
        "你好！我是 DMSX AI 助手。你可以用自然语言与我交互，例如：\n\n" +
        "• 「查看北京总部所有离线设备」\n" +
        "• 「给生产环境组下发安全补丁」\n" +
        "• 「分析最近一周的异常趋势」\n" +
        "• 「为什么 node-gz-7 心跳中断了？」\n\n" +
        "请问有什么可以帮你的？",
    },
  ]);
  const [input, setInput] = useState("");

  const handleSend = () => {
    if (!input.trim()) return;
    const userMsg: ChatMsg = { role: "user", content: input };
    const reply: ChatMsg = {
      role: "assistant",
      content: `收到指令「${input}」。正在分析...\n\n（当前为演示模式。接入 LLM 后端后将执行真实操作并返回结果。）`,
      actions: ["执行命令", "创建策略", "生成报表"],
    };
    setMessages((prev) => [...prev, userMsg, reply]);
    setInput("");
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: 500 }}>
      {DEMO_ALERT}
      <div style={{ flex: 1, overflow: "auto", padding: "8px 0" }}>
        {messages.map((msg, i) => (
          <div
            key={i}
            style={{
              display: "flex",
              justifyContent:
                msg.role === "user" ? "flex-end" : "flex-start",
              marginBottom: 12,
            }}
          >
            {msg.role === "assistant" && (
              <Avatar
                icon={<RobotOutlined />}
                style={{
                  backgroundColor: "#1677ff",
                  marginRight: 8,
                  flexShrink: 0,
                }}
              />
            )}
            <div
              style={{
                maxWidth: "70%",
                padding: "10px 14px",
                borderRadius: 8,
                background:
                  msg.role === "user" ? "#1677ff" : "#f5f5f5",
                color: msg.role === "user" ? "#fff" : "#333",
                whiteSpace: "pre-wrap",
              }}
            >
              {msg.content}
              {msg.actions && (
                <div style={{ marginTop: 8 }}>
                  <Space wrap>
                    {msg.actions.map((a) => (
                      <Button
                        key={a}
                        size="small"
                        ghost={msg.role === "assistant"}
                      >
                        {a}
                      </Button>
                    ))}
                  </Space>
                </div>
              )}
            </div>
            {msg.role === "user" && (
              <Avatar
                icon={<UserOutlined />}
                style={{ marginLeft: 8, flexShrink: 0 }}
              />
            )}
          </div>
        ))}
      </div>
      <div
        style={{
          display: "flex",
          gap: 8,
          paddingTop: 8,
          borderTop: "1px solid #f0f0f0",
        }}
      >
        <TextArea
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="输入自然语言指令…（例如：查看所有离线设备）"
          autoSize={{ minRows: 1, maxRows: 3 }}
          onPressEnter={(e) => {
            if (!e.shiftKey) {
              e.preventDefault();
              handleSend();
            }
          }}
          style={{ flex: 1 }}
        />
        <Button type="primary" icon={<SendOutlined />} onClick={handleSend}>
          发送
        </Button>
      </div>
    </div>
  );
};

// ---------------------------------------------------------------------------
// 预测性维护面板
// ---------------------------------------------------------------------------

const PredictionPanel: React.FC = () => {
  const predictions = [
    {
      device: "node-bj-3",
      issue: "磁盘空间将在 12 天内耗尽",
      probability: 0.91,
      risk: "high",
      eta: "12 天",
    },
    {
      device: "node-sh-8",
      issue: "内存使用趋势异常，预计 21 天后 OOM",
      probability: 0.76,
      risk: "medium",
      eta: "21 天",
    },
    {
      device: "node-gz-2",
      issue: "Agent 证书 30 天后过期",
      probability: 1.0,
      risk: "high",
      eta: "30 天",
    },
    {
      device: "edge-cd-1",
      issue: "网络丢包率上升趋势，可能 7 天内影响业务",
      probability: 0.68,
      risk: "medium",
      eta: "7 天",
    },
    {
      device: "node-bj-15",
      issue: "SSD 健康度下降，预计 60 天后需更换",
      probability: 0.82,
      risk: "low",
      eta: "60 天",
    },
  ];

  const riskColor: Record<string, string> = {
    high: "red",
    medium: "orange",
    low: "green",
  };

  return (
    <div>
      {DEMO_ALERT}
      <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
        <Col span={8}>
          <Card>
            <Statistic
              title="高风险设备"
              value={2}
              valueStyle={{ color: "#f5222d" }}
            />
          </Card>
        </Col>
        <Col span={8}>
          <Card>
            <Statistic
              title="中风险设备"
              value={2}
              valueStyle={{ color: "#faad14" }}
            />
          </Card>
        </Col>
        <Col span={8}>
          <Card>
            <Statistic
              title="预测准确率"
              value={89}
              suffix="%"
              valueStyle={{ color: "#52c41a" }}
            />
          </Card>
        </Col>
      </Row>

      <Timeline
        items={predictions.map((p) => ({
          color: riskColor[p.risk],
          children: (
            <Card size="small" style={{ marginBottom: 0 }}>
              <Row justify="space-between" align="middle">
                <Col>
                  <Text strong>{p.device}</Text>
                  <br />
                  <Text>{p.issue}</Text>
                  <br />
                  <Space style={{ marginTop: 4 }}>
                    <Tag color={riskColor[p.risk]}>{p.risk}</Tag>
                    <Text type="secondary">
                      概率 {(p.probability * 100).toFixed(0)}%
                    </Text>
                    <Text type="secondary">预计 {p.eta}</Text>
                  </Space>
                </Col>
                <Col>
                  <Tooltip title="AI 一键处置：生成修复命令或策略">
                    <Button type="primary" ghost icon={<ToolOutlined />}>
                      智能处置
                    </Button>
                  </Tooltip>
                </Col>
              </Row>
            </Card>
          ),
        }))}
      />
    </div>
  );
};

// ---------------------------------------------------------------------------
// AI 中心主页
// ---------------------------------------------------------------------------

export const AiCenterPage: React.FC = () => (
  <div>
    <Title level={4}>
      <RobotOutlined style={{ marginRight: 8 }} />
      AI 智慧管控中心
    </Title>
    <Paragraph type="secondary">
      融合异常检测、策略推荐、自然语言交互与预测性维护，辅助运维人员高效决策。
    </Paragraph>

    <Tabs
      items={[
        {
          key: "anomaly",
          label: (
            <span>
              <ThunderboltOutlined /> 异常检测
            </span>
          ),
          children: <AnomalyPanel />,
        },
        {
          key: "recommend",
          label: (
            <span>
              <BulbOutlined /> 策略推荐
            </span>
          ),
          children: <RecommendationPanel />,
        },
        {
          key: "chat",
          label: (
            <span>
              <RobotOutlined /> 智能助手
            </span>
          ),
          children: <ChatPanel />,
        },
        {
          key: "predict",
          label: (
            <span>
              <ExperimentOutlined /> 预测维护
            </span>
          ),
          children: <PredictionPanel />,
        },
      ]}
    />
  </div>
);
