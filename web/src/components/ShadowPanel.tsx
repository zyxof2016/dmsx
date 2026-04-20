import React, { useState } from "react";
import {
  Card,
  Row,
  Col,
  Spin,
  Alert,
  Tag,
  Button,
  Modal,
  Input,
  Typography,
  Descriptions,
  Empty,
  App,
} from "antd";
import { EditOutlined, SyncOutlined } from "@ant-design/icons";
import dayjs from "dayjs";
import { useShadow, useUpdateShadowDesired } from "../api/hooks";
import { formatApiError } from "../api/errors";
import { useResourceAccess, WRITE_DISABLED_REASON } from "../authz";
import { ReadonlyBanner } from "./ReadonlyBanner";

const { Text } = Typography;
const { TextArea } = Input;

const JsonBlock: React.FC<{
  value: Record<string, unknown>;
  highlight?: Record<string, unknown>;
}> = ({ value, highlight }) => {
  const keys = Object.keys(value);
  if (!keys.length) return <Empty description="空" image={Empty.PRESENTED_IMAGE_SIMPLE} />;
  return (
    <pre
      style={{
        margin: 0,
        padding: 12,
        background: "#fafafa",
        borderRadius: 6,
        fontSize: 12,
        maxHeight: 400,
        overflow: "auto",
        lineHeight: 1.6,
      }}
    >
      {keys.map((k) => {
        const isDelta = highlight && k in highlight;
        return (
          <div key={k} style={isDelta ? { background: "#fff7e6", borderRadius: 2 } : undefined}>
            <Text strong={isDelta} type={isDelta ? "warning" : undefined}>
              "{k}"
            </Text>
            : {JSON.stringify(value[k], null, 2)}
          </div>
        );
      })}
    </pre>
  );
};

export const ShadowPanel: React.FC<{ deviceId: string }> = ({ deviceId }) => {
  const { data: shadow, isLoading, error, refetch } = useShadow(deviceId);
  const updateDesired = useUpdateShadowDesired();
  const { canWrite } = useResourceAccess("deviceShadow");
  const [editOpen, setEditOpen] = useState(false);
  const [editJson, setEditJson] = useState("");
  const [jsonError, setJsonError] = useState<string | null>(null);
  const { message } = App.useApp();

  const openEdit = () => {
    setEditJson(JSON.stringify(shadow?.desired ?? {}, null, 2));
    setJsonError(null);
    setEditOpen(true);
  };

  const handleSave = () => {
    try {
      const parsed = JSON.parse(editJson);
      if (typeof parsed !== "object" || Array.isArray(parsed)) {
        setJsonError("必须是 JSON 对象");
        return;
      }
      updateDesired.mutate(
        { deviceId, desired: { desired: parsed } },
        {
          onSuccess: () => {
            message.success("期望状态已更新");
            setEditOpen(false);
            refetch();
          },
          onError: (e) => message.error(formatApiError(e)),
        },
      );
    } catch {
      setJsonError("JSON 格式错误");
    }
  };

  if (error) return <Alert type="error" message="加载影子失败" description={formatApiError(error)} showIcon />;

  return (
    <Spin spinning={isLoading}>
      {shadow && (
        <>
          <ReadonlyBanner visible={!canWrite} resourceLabel="设备影子" />
          <Descriptions size="small" column={3} bordered style={{ marginBottom: 16 }}>
            <Descriptions.Item label="版本">{shadow.version}</Descriptions.Item>
            <Descriptions.Item label="上报时间">
              {shadow.reported_at ? dayjs(shadow.reported_at).format("YYYY-MM-DD HH:mm:ss") : "—"}
            </Descriptions.Item>
            <Descriptions.Item label="期望设定时间">
              {shadow.desired_at ? dayjs(shadow.desired_at).format("YYYY-MM-DD HH:mm:ss") : "—"}
            </Descriptions.Item>
          </Descriptions>

          {Object.keys(shadow.delta).length > 0 && (
            <Alert
              type="warning"
              message={`${Object.keys(shadow.delta).length} 项配置不一致`}
              style={{ marginBottom: 16 }}
              showIcon
            />
          )}

          <Row gutter={16}>
            <Col span={8}>
              <Card
                size="small"
                title={<><Tag color="green">Reported</Tag> 实际状态</>}
              >
                <JsonBlock value={shadow.reported} highlight={shadow.delta} />
              </Card>
            </Col>
            <Col span={8}>
              <Card
                size="small"
                title={
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                    <span><Tag color="blue">Desired</Tag> 期望状态</span>
                    <Button
                      size="small"
                      icon={<EditOutlined />}
                      onClick={openEdit}
                      disabled={!canWrite}
                      title={!canWrite ? WRITE_DISABLED_REASON : undefined}
                    >
                      编辑
                    </Button>
                  </div>
                }
              >
                <JsonBlock value={shadow.desired} />
              </Card>
            </Col>
            <Col span={8}>
              <Card
                size="small"
                title={<><Tag color="orange">Delta</Tag> 差异</>}
              >
                <JsonBlock value={shadow.delta} />
              </Card>
            </Col>
          </Row>

          <div style={{ marginTop: 12, textAlign: "right" }}>
            <Button icon={<SyncOutlined />} onClick={() => refetch()}>
              刷新
            </Button>
          </div>

          <Modal
            title="编辑期望状态 (JSON)"
            open={editOpen}
            onCancel={() => setEditOpen(false)}
            onOk={handleSave}
            confirmLoading={updateDesired.isPending}
            okButtonProps={{ disabled: !canWrite }}
            width={600}
          >
            <TextArea
              rows={16}
              value={editJson}
              onChange={(e) => {
                setEditJson(e.target.value);
                setJsonError(null);
              }}
              style={{ fontFamily: "monospace", fontSize: 12 }}
            />
            {jsonError && <Alert type="error" message={jsonError} style={{ marginTop: 8 }} />}
          </Modal>
        </>
      )}
    </Spin>
  );
};
