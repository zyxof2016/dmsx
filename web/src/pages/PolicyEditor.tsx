import React from "react";
import {
  Alert,
  Button,
  Card,
  Input,
  Space,
  Typography,
  Tag,
  App,
} from "antd";
import type { CreatePolicyReq, Policy, PolicyEditorPublishReq } from "../api/types";
import { useAppI18n } from "../appProviders";
import { usePolicyEditorPublish } from "../api/hooks";

const { TextArea } = Input;

type PolicyDraft = Partial<Policy> & {
  name?: string;
  description?: string | null;
};

function safeStringify(obj: unknown): string {
  try {
    return JSON.stringify(obj, null, 2);
  } catch {
    return String(obj);
  }
}

async function copyToClipboard(text: string) {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    // Fallback for older browsers / restricted contexts
    const ta = document.createElement("textarea");
    ta.value = text;
    document.body.appendChild(ta);
    ta.select();
    document.execCommand("copy");
    document.body.removeChild(ta);
  }
}

export const PolicyEditorPage: React.FC = () => {
  const { t } = useAppI18n();
  const { message } = App.useApp();
  const publishMut = usePolicyEditorPublish();

  const [jsonText, setJsonText] = React.useState(() =>
    safeStringify({
      scope_kind: "tenant",
      scope_expr: "dmsx.is_platform_admin() OR dmsx.tenant_id = dmsx.current_tenant_id()",
      // name/description 为 CreatePolicyReq 预览用，可自行补充
      name: "example-policy",
      description: "前端 JSON 编辑与校验演示（待后端接入）",
    }),
  );

  const [error, setError] = React.useState<string | null>(null);
  const [draft, setDraft] = React.useState<PolicyDraft | null>(null);

  React.useEffect(() => {
    try {
      const parsed = JSON.parse(jsonText) as PolicyDraft;
      setDraft(parsed);
      setError(null);
    } catch (e) {
      setDraft(null);
      setError(String(e));
    }
  }, [jsonText]);

  const createPolicyReq: CreatePolicyReq | null = React.useMemo(() => {
    if (!draft) return null;
    const scope_kind = draft.scope_kind as Policy["scope_kind"] | undefined;
    const name = typeof draft.name === "string" ? draft.name : undefined;
    const description =
      typeof draft.description === "string" ? draft.description : undefined;

    if (!scope_kind || !name) return null;
    return { name, description, scope_kind };
  }, [draft]);

  const publishReq: PolicyEditorPublishReq | null = React.useMemo(() => {
    if (!draft) return null;
    const scope_kind = draft.scope_kind as Policy["scope_kind"] | undefined;
    const scope_expr =
      typeof (draft as any).scope_expr === "string" ? String((draft as any).scope_expr) : undefined;
    const name = typeof draft.name === "string" ? draft.name : undefined;
    const description =
      typeof draft.description === "string" ? draft.description : undefined;
    if (!scope_kind || !name || !scope_expr) return null;
    return { name, description, scope_kind, scope_expr };
  }, [draft]);

  const jsonQuality = React.useMemo(() => {
    if (!draft) return { ok: false, label: "invalid" as const };
    if (!draft.scope_kind || typeof draft.scope_kind !== "string") {
      return { ok: false, label: "missing scope_kind" as const };
    }
    if ("scope_expr" in draft && typeof draft.scope_expr !== "string" && draft.scope_expr !== null) {
      return { ok: false, label: "scope_expr type mismatch" as const };
    }
    return { ok: true, label: "ready" as const };
  }, [draft]);

  return (
    <Space direction="vertical" style={{ width: "100%" }}>
      <Typography.Title level={4}>{t("page.policyEditor")}</Typography.Title>

      <Alert type="info" showIcon message="已接入后端：保存将调用 /v1/tenants/{tid}/policies/editor" />

      <Card>
        <Space direction="vertical" style={{ width: "100%" }}>
          <Typography.Text strong>策略 JSON（编辑器最小版：Textarea + 校验）</Typography.Text>
          <TextArea
            value={jsonText}
            rows={12}
            onChange={(e) => setJsonText(e.target.value)}
            spellCheck={false}
            style={{ fontFamily: "monospace" }}
          />

          <div>
            <Tag color={jsonQuality.ok ? "green" : "red"}>{jsonQuality.label}</Tag>
          </div>

          {error && (
            <Alert type="error" showIcon message="JSON 解析失败" description={error} />
          )}

          <Typography.Text strong>生成的 CreatePolicyReq 预览（可复制）</Typography.Text>

          <TextArea
            value={createPolicyReq ? safeStringify(createPolicyReq) : "{}"}
            rows={6}
            readOnly
            style={{ fontFamily: "monospace" }}
          />

          <Space>
            <Button
              type="primary"
              disabled={!createPolicyReq}
              onClick={() => {
                if (!createPolicyReq) return;
                void copyToClipboard(safeStringify(createPolicyReq));
              }}
            >
              {t("buttons.copy")}
            </Button>
            <Button
              type="default"
              disabled={!publishReq}
              loading={publishMut.isPending}
              onClick={async () => {
                if (!publishReq) return;
                try {
                  const rev = await publishMut.mutateAsync(publishReq);
                  message.success(`已发布 revision v${rev.version}`);
                } catch (e) {
                  message.error(String(e));
                }
              }}
            >
              保存并发布
            </Button>
          </Space>
        </Space>
      </Card>
    </Space>
  );
};

