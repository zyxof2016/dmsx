import React, { useState } from "react";
import {
  Typography,
  Table,
  Tag,
  Button,
  Space,
  Card,
  Modal,
  Form,
  Input,
  App,
  Spin,
  Alert,
  Empty,
} from "antd";
import {
  UploadOutlined,
  SyncOutlined,
  DownloadOutlined,
} from "@ant-design/icons";
import dayjs from "dayjs";
import { useArtifacts, useCreateArtifact, exportCsv } from "../api/hooks";
import type { Artifact, CreateArtifactReq, ListParams } from "../api/types";
import { formatApiError } from "../api/errors";
import { useResourceAccess } from "../authz";
import { GuardedButton } from "../components/GuardedButton";
import { ReadonlyBanner } from "../components/ReadonlyBanner";

const { Title } = Typography;

export const ArtifactsPage: React.FC = () => {
  const { message } = App.useApp();
  const [form] = Form.useForm<CreateArtifactReq>();
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(10);
  const { canWrite } = useResourceAccess("artifacts");

  const params: ListParams = {
    limit: pageSize,
    offset: (page - 1) * pageSize,
    search: search || undefined,
  };

  const { data, isLoading, error, refetch } = useArtifacts(params);
  const createMut = useCreateArtifact();

  const items = data?.items ?? [];
  const total = data?.total ?? 0;

  const handleCreate = async (values: CreateArtifactReq) => {
    try {
      await createMut.mutateAsync(values);
      message.success("制品创建成功");
      setOpen(false);
      form.resetFields();
    } catch (e: unknown) {
      message.error(formatApiError(e));
    }
  };

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
      <Title level={4}>应用分发</Title>
      <ReadonlyBanner visible={!canWrite} resourceLabel="应用分发" />
      <Card>
        <Space style={{ marginBottom: 16 }} wrap>
          <Input.Search
            placeholder="搜索制品名称"
            style={{ width: 220 }}
            value={search}
            onChange={(e) => {
              setSearch(e.target.value);
              setPage(1);
            }}
            allowClear
          />
          <GuardedButton
            type="primary"
            icon={<UploadOutlined />}
            onClick={() => setOpen(true)}
            allowed={canWrite}
          >
            上传制品
          </GuardedButton>
          <Button icon={<SyncOutlined />} onClick={() => refetch()}>
            刷新
          </Button>
          <Button
            icon={<DownloadOutlined />}
            onClick={() =>
              exportCsv(
                items as unknown as Record<string, unknown>[],
                "artifacts.csv",
              )
            }
            disabled={items.length === 0}
          >
            导出 CSV
          </Button>
        </Space>
        <Table<Artifact>
          rowKey="id"
          dataSource={items}
          size="small"
          locale={{
            emptyText: (
              <Empty description="暂无制品，点击「上传制品」添加" />
            ),
          }}
          pagination={{
            current: page,
            pageSize,
            total,
            showSizeChanger: true,
            showTotal: (t) => `共 ${t} 个`,
            onChange: (p, ps) => {
              setPage(p);
              setPageSize(ps);
            },
          }}
          columns={[
            { title: "名称", dataIndex: "name" },
            { title: "版本", dataIndex: "version" },
            {
              title: "渠道",
              dataIndex: "channel",
              render: (c: string) => (
                <Tag color={c === "stable" ? "green" : "orange"}>{c}</Tag>
              ),
            },
            {
              title: "SHA256",
              dataIndex: "sha256",
              render: (s: string) => s.slice(0, 16) + "…",
            },
            {
              title: "创建时间",
              dataIndex: "created_at",
              render: (t: string) => dayjs(t).format("YYYY-MM-DD HH:mm"),
            },
          ]}
        />
      </Card>

      <Modal
        title="上传制品"
        open={open}
        onCancel={() => setOpen(false)}
        onOk={() => form.submit()}
        confirmLoading={createMut.isPending}
        okButtonProps={{ disabled: !canWrite }}
      >
        <Form form={form} layout="vertical" onFinish={handleCreate}>
          <Form.Item name="name" label="名称" rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item name="version" label="版本" rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item
            name="sha256"
            label="SHA256"
            rules={[
              { required: true },
              {
                pattern: /^[0-9a-fA-F]{64}$/,
                message: "必须为 64 位十六进制字符串",
              },
            ]}
          >
            <Input />
          </Form.Item>
          <Form.Item name="channel" label="渠道" initialValue="stable">
            <Input />
          </Form.Item>
          <Form.Item
            name="object_key"
            label="对象存储 Key"
            rules={[{ required: true }]}
          >
            <Input />
          </Form.Item>
        </Form>
      </Modal>
    </Spin>
  );
};
