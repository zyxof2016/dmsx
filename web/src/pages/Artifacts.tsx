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
  Checkbox,
  Collapse,
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
import { buildArtifactMeta, parseArtifactMeta } from "../artifactMeta";

const { Title } = Typography;

type ArtifactFormValues = CreateArtifactReq & {
  platforms?: Array<"linux" | "windows" | "android">;
  download_url?: string;
  installer_kind?: string;
  install_linux?: string;
  install_windows?: string;
  install_android?: string;
  upgrade_linux?: string;
  upgrade_windows?: string;
  upgrade_android?: string;
};

export const ArtifactsPage: React.FC = () => {
  const { message } = App.useApp();
  const [form] = Form.useForm<ArtifactFormValues>();
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

  const handleCreate = async (values: ArtifactFormValues) => {
    try {
      await createMut.mutateAsync({
        name: values.name,
        version: values.version,
        sha256: values.sha256,
        channel: values.channel,
        object_key: values.object_key,
        metadata: buildArtifactMeta({
          platforms: values.platforms ?? [],
          download_url: values.download_url,
          installer_kind: values.installer_kind,
          install_commands: {
            linux: values.install_linux,
            windows: values.install_windows,
            android: values.install_android,
          },
          upgrade_commands: {
            linux: values.upgrade_linux,
            windows: values.upgrade_windows,
            android: values.upgrade_android,
          },
        }),
      });
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
            {
              title: "升级元数据",
              dataIndex: "metadata",
              render: (metadata: Record<string, unknown>) => {
                const parsed = parseArtifactMeta(metadata);
                const tags = [
                  parsed.download_url ? "下载地址" : null,
                  parsed.installer_kind ? parsed.installer_kind : null,
                  Object.keys(parsed.install_commands).length ? "首装模板" : null,
                  Object.keys(parsed.upgrade_commands).length ? "升级模板" : null,
                ].filter(Boolean);
                return tags.length ? <Space wrap>{tags.map((tag) => <Tag key={String(tag)}>{tag}</Tag>)}</Space> : <Typography.Text type="secondary">—</Typography.Text>;
              },
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
          <Form form={form} layout="vertical" onFinish={handleCreate} initialValues={{ channel: "stable", platforms: [] }}>
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
          <Form.Item name="channel" label="渠道">
            <Input />
          </Form.Item>
          <Form.Item
            name="object_key"
            label="对象存储 Key"
            rules={[{ required: true }]}
          >
            <Input />
          </Form.Item>
          <Collapse
            items={[
              {
                key: "metadata",
                label: "安装 / 升级元数据",
                children: (
                  <Space direction="vertical" style={{ width: "100%" }}>
                    <Form.Item name="platforms" label="适用平台">
                      <Checkbox.Group
                        options={[
                          { label: "Linux/macOS", value: "linux" },
                          { label: "Windows", value: "windows" },
                          { label: "Android", value: "android" },
                        ]}
                      />
                    </Form.Item>
                    <Form.Item name="download_url" label="下载地址">
                      <Input placeholder="https://downloads.example.com/dmsx-agent/linux/update.sh" />
                    </Form.Item>
                    <Form.Item name="installer_kind" label="安装器类型">
                      <Input placeholder="例如 sh / ps1 / msi / exe / deb / rpm / pkg / apk" />
                    </Form.Item>
                    <Typography.Text strong>首装命令模板</Typography.Text>
                    <Form.Item name="install_linux" label="Linux/macOS">
                      <Input.TextArea rows={2} placeholder="curl -fsSL {{download_url}} | sh" />
                    </Form.Item>
                    <Form.Item name="install_windows" label="Windows">
                      <Input.TextArea rows={2} placeholder="powershell -ExecutionPolicy Bypass -File .\\install-agent.ps1" />
                    </Form.Item>
                    <Form.Item name="install_android" label="Android">
                      <Input.TextArea rows={2} placeholder="adb shell sh /data/local/tmp/install-agent.sh" />
                    </Form.Item>
                    <Typography.Text strong>升级命令模板</Typography.Text>
                    <Form.Item name="upgrade_linux" label="Linux/macOS">
                      <Input.TextArea rows={2} placeholder="sh {{file_path}} --upgrade" />
                    </Form.Item>
                    <Form.Item name="upgrade_windows" label="Windows">
                      <Input.TextArea rows={2} placeholder="powershell -ExecutionPolicy Bypass -File {{file_path}}" />
                    </Form.Item>
                    <Form.Item name="upgrade_android" label="Android">
                      <Input.TextArea rows={2} placeholder="pm install -r {{file_path}}" />
                    </Form.Item>
                  </Space>
                ),
              },
            ]}
          />
        </Form>
      </Modal>
    </Spin>
  );
};
