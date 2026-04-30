import React from "react";
import { Link } from "@tanstack/react-router";
import { Alert, Button, Card, Descriptions, Divider, List, Segmented, Space, Spin, Steps, Tag, Typography } from "antd";
import { useRouterState } from "@tanstack/react-router";
import dayjs from "dayjs";
import { useArtifacts } from "../api/hooks";
import type { Artifact } from "../api/types";
import { TerminalBlock } from "../components/TerminalBlock";
import { artifactMatchesPlatform, chooseArtifactCommand, inferInstallerKind, selectRecommendedArtifact } from "../artifactMeta";
import {
  buildWindowsOneClickInstallerScript,
  downloadTextFile,
  readArtifactDownloadUrl,
} from "../enrollmentInstall";

const { Title, Text } = Typography;

type Platform = "linux" | "windows" | "android";

function summarizeArtifact(artifact: Artifact): string {
  const metadataEntries = Object.entries(artifact.metadata ?? {}).slice(0, 2);
  if (!metadataEntries.length) return artifact.object_key;
  return metadataEntries.map(([key, value]) => `${key}: ${String(value)}`).join(" | ");
}

function readMetadataString(value: unknown): string | null {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

function buildDefaultUpgradeCommand(downloadUrl: string, installerKind: string | null, platform: Platform): string | null {
  if (platform === "windows") {
    const target = "$env:TEMP\\dmsx-agent-update";
    if (installerKind === "msi") {
      return `powershell -Command \"Invoke-WebRequest -Uri '${downloadUrl}' -OutFile '${target}.msi'; Start-Process msiexec.exe -Wait -ArgumentList @('/i','${target}.msi','/qn','/norestart')\"`;
    }
    if (installerKind === "exe") {
      return `powershell -Command \"Invoke-WebRequest -Uri '${downloadUrl}' -OutFile '${target}.exe'; Start-Process -FilePath '${target}.exe' -Wait -ArgumentList @('/quiet','/norestart')\"`;
    }
    if (installerKind === "ps1") {
      return `powershell -Command \"Invoke-WebRequest -Uri '${downloadUrl}' -OutFile '${target}.ps1'; powershell -ExecutionPolicy Bypass -File '${target}.ps1'\"`;
    }
    return `powershell -Command \"Invoke-WebRequest -Uri '${downloadUrl}' -OutFile '${target}.bin'\"`;
  }

  if (platform === "android") {
    const target = "/data/local/tmp/dmsx-agent-update";
    if (installerKind === "apk") {
      return `curl -fsSL '${downloadUrl}' -o '${target}.apk' && pm install -r '${target}.apk'`;
    }
    if (installerKind === "sh") {
      return `curl -fsSL '${downloadUrl}' -o '${target}.sh' && sh '${target}.sh'`;
    }
    return `curl -fsSL '${downloadUrl}' -o '${target}.bin'`;
  }

  const target = "/tmp/dmsx-agent-update";
  if (installerKind === "sh") {
    return `curl -fsSL '${downloadUrl}' -o '${target}.sh' && sh '${target}.sh'`;
  }
  if (installerKind === "deb") {
    return `curl -fsSL '${downloadUrl}' -o '${target}.deb' && sudo dpkg -i '${target}.deb'`;
  }
  if (installerKind === "rpm") {
    return `curl -fsSL '${downloadUrl}' -o '${target}.rpm' && sudo rpm -Uvh '${target}.rpm'`;
  }
  if (installerKind === "pkg") {
    return `curl -fsSL '${downloadUrl}' -o '${target}.pkg' && sudo installer -pkg '${target}.pkg' -target /`;
  }
  return `curl -fsSL '${downloadUrl}' -o '${target}.bin'`;
}

function defaultInstallCommand(platform: Platform, apiUrl: string, tenantId: string, enrollmentToken: string): string {
  return platform === "windows"
    ? `powershell -ExecutionPolicy Bypass -File .\\Install-DMSX-Agent.ps1`
    : platform === "android"
      ? "adb install -r DMSX-Agent-Android.apk && adb shell monkey -p com.dmsx.agent 1"
      : `DMSX_API_URL=${apiUrl} DMSX_TENANT_ID=${tenantId} DMSX_DEVICE_ENROLLMENT_TOKEN='${enrollmentToken}' cargo run -p dmsx-agent`;
}

function androidApkPackageCommand(apiUrl: string, tenantId: string, enrollmentToken: string): string {
  return [
    ".\\scripts\\package-android-agent.ps1 `",
    `  -ApiUrl "${apiUrl}" \``,
    `  -TenantId "${tenantId}" \``,
    `  -EnrollmentToken "${enrollmentToken}" \``,
    '  -OutputPath ".\\target\\packages\\DMSX-Agent-Android.apk"',
  ].join("\n");
}

function resolveInstallCommand(
  artifact: Artifact | null,
  platform: Platform,
  apiUrl: string,
  tenantId: string,
  enrollmentToken: string,
): string {
  const command = artifact ? chooseArtifactCommand(artifact, "install", platform) : undefined;
  if (!command) return defaultInstallCommand(platform, apiUrl, tenantId, enrollmentToken);
  return command
    .replaceAll("{{api_url}}", apiUrl)
    .replaceAll("{{tenant_id}}", tenantId)
    .replaceAll("{{enrollment_token}}", enrollmentToken)
    .replaceAll("{{object_key}}", artifact?.object_key ?? "")
    .replaceAll("{{version}}", artifact?.version ?? "")
    .replaceAll("{{channel}}", artifact?.channel ?? "");
}

function resolveUpgradeCommand(artifact: Artifact | null, platform: Platform): string | null {
  if (!artifact) return null;
  const explicit = chooseArtifactCommand(artifact, "upgrade", platform);
  if (explicit) {
    return explicit
      .replaceAll("{{object_key}}", artifact.object_key)
      .replaceAll("{{version}}", artifact.version)
      .replaceAll("{{channel}}", artifact.channel);
  }
  const downloadUrl = readMetadataString(artifact.metadata?.download_url);
  if (!downloadUrl) return null;
  return buildDefaultUpgradeCommand(downloadUrl, inferInstallerKind(artifact) ?? null, platform);
}

export const ZeroTouchEnrollPage: React.FC = () => {
  const search = useRouterState({ select: (s) => s.location.searchStr });
  const params = React.useMemo(() => new URLSearchParams(search), [search]);

  const apiUrl = params.get("api_url") ?? "";
  const tenantId = params.get("tenant_id") ?? "";
  const enrollmentToken = params.get("enrollment_token") ?? "";
  const mode = params.get("mode") ?? "manual";
  const initialPlatform = React.useMemo<Platform>(() => {
    const raw = params.get("platform");
    if (raw === "windows" || raw === "android") return raw;
    return "linux";
  }, [params]);
  const [platform, setPlatform] = React.useState<Platform>(initialPlatform);
  React.useEffect(() => {
    setPlatform(initialPlatform);
  }, [initialPlatform]);
  const artifactsQuery = useArtifacts(
    {
      limit: 50,
    },
    {
      tenantId,
      enabled: Boolean(tenantId),
    },
  );

  const recommendedArtifacts = React.useMemo(() => {
    const items = artifactsQuery.data?.items ?? [];
    const primary = selectRecommendedArtifact(items, platform);
    const fallback = items
      .filter((artifact) => artifact.id !== primary?.id)
      .filter((artifact) => artifactMatchesPlatform(artifact, platform))
      .sort((a, b) => dayjs(b.created_at).valueOf() - dayjs(a.created_at).valueOf())
      .slice(0, 2);
    return primary ? [primary, ...fallback] : fallback;
  }, [artifactsQuery.data?.items, platform]);
  const primaryArtifact = recommendedArtifacts[0] ?? null;
  const installCommand = resolveInstallCommand(primaryArtifact, platform, apiUrl, tenantId, enrollmentToken);
  const upgradeCommand = resolveUpgradeCommand(primaryArtifact, platform);
  const androidPackageCommand = androidApkPackageCommand(apiUrl, tenantId, enrollmentToken);
  const agentDownloadUrl = readArtifactDownloadUrl(primaryArtifact);
  const windowsInstallerScript = React.useMemo(
    () =>
      buildWindowsOneClickInstallerScript({
        apiUrl,
        tenantId,
        enrollmentToken,
        agentDownloadUrl,
      }),
    [agentDownloadUrl, apiUrl, enrollmentToken, tenantId],
  );

  return (
    <Space direction="vertical" style={{ width: "100%" }}>
      <Title level={3}>DMSX 零接触安装向导</Title>
      <Alert
        type="info"
        showIcon
        message="将以下参数注入设备环境变量后启动 Agent，即可自动认领已预注册设备。适用于扫码安装、MDM 下发、工厂预置和远程实施。"
      />
      <Card>
        <Steps
          current={1}
          items={[
            { title: "平台预注册" },
            { title: "下发 Enrollment 参数" },
            { title: "启动 Agent 完成认领" },
          ]}
        />
      </Card>
      <Card>
        <Typography.Title level={5}>安装参数</Typography.Title>
        <Segmented
          value={platform}
          onChange={(value) => setPlatform(value as "linux" | "windows" | "android")}
          options={[
            { label: "Linux/macOS", value: "linux" },
            { label: "Windows", value: "windows" },
            { label: "Android/ADB", value: "android" },
          ]}
          style={{ marginBottom: 16 }}
        />
        <Descriptions bordered column={1} size="small">
          <Descriptions.Item label="模式">{mode}</Descriptions.Item>
          <Descriptions.Item label="API URL">
            <Text code copyable={{ text: apiUrl }}>{apiUrl || "—"}</Text>
          </Descriptions.Item>
          <Descriptions.Item label="Tenant ID">
            <Text code copyable={{ text: tenantId }}>{tenantId || "—"}</Text>
          </Descriptions.Item>
          <Descriptions.Item label="Enrollment Token">
            <Text code copyable={{ text: enrollmentToken }}>{enrollmentToken || "—"}</Text>
          </Descriptions.Item>
          <Descriptions.Item label="推荐首装命令">
            <TerminalBlock code={installCommand} />
          </Descriptions.Item>
          {platform === "android" ? (
            <Descriptions.Item label="生成专属 APK">
              <Space direction="vertical" style={{ width: "100%" }}>
                <TerminalBlock code={androidPackageCommand} />
                <Text type="secondary">
                  生成的 APK 会内置 API URL、Tenant ID 和 enrollment token。用户安装后首次打开 App，会自动启动前台服务并完成设备认领。
                </Text>
              </Space>
            </Descriptions.Item>
          ) : null}
          {platform === "windows" ? (
            <Descriptions.Item label="Windows 一键安装脚本">
              <Space direction="vertical" style={{ width: "100%" }}>
                <Button
                  type="primary"
                  disabled={!apiUrl || !tenantId || !enrollmentToken}
                  onClick={() => {
                    downloadTextFile("Install-DMSX-Agent.ps1", windowsInstallerScript);
                  }}
                >
                  下载安装脚本
                </Button>
                <Text type={agentDownloadUrl ? "secondary" : "warning"}>
                  {agentDownloadUrl
                    ? "脚本会自动下载 Agent、写入 enrollment 配置并注册 Windows 服务。"
                    : "当前没有可下载的 Agent 制品；请将 dmsx-agent.exe 与脚本放在同一目录后运行。"}
                </Text>
              </Space>
            </Descriptions.Item>
          ) : null}
          {primaryArtifact ? (
            <Descriptions.Item label="推荐 Agent 制品">
              <Space wrap>
                <Text strong>{primaryArtifact.name}</Text>
                <Tag color="green">{primaryArtifact.channel}</Tag>
                <Text type="secondary">{primaryArtifact.version}</Text>
              </Space>
            </Descriptions.Item>
          ) : null}
        </Descriptions>
        <Divider />
        <Typography.Title level={5}>OTA / 制品建议</Typography.Title>
        <Space wrap style={{ marginBottom: 16 }}>
          <Tag color="blue">首装完成后，优先切到 stable 渠道做后续 OTA 升级</Tag>
          <Tag color="purple">Android 设备建议结合 ADB/MDM 预置二进制，再走 Enrollment Token 首次认领</Tag>
        </Space>
        {!tenantId ? (
          <Alert type="warning" showIcon message="缺少 tenant_id，无法加载该租户可用制品。" />
        ) : artifactsQuery.isLoading ? (
          <Spin size="small" />
        ) : artifactsQuery.error ? (
          <Alert type="warning" showIcon message="制品建议加载失败" description="请确认当前链接中的 tenant_id 有权限访问制品列表。" />
        ) : recommendedArtifacts.length > 0 ? (
          <List
            size="small"
            dataSource={recommendedArtifacts}
            renderItem={(artifact) => (
              <List.Item>
                <Space direction="vertical" size={2} style={{ width: "100%" }}>
                  <Space wrap>
                    <Text strong>{artifact.name}</Text>
                    <Tag color="green">{artifact.channel}</Tag>
                    <Text type="secondary">{artifact.version}</Text>
                  </Space>
                  <Text type="secondary">{summarizeArtifact(artifact)}</Text>
                  <Space wrap>
                    <Text code copyable={{ text: artifact.object_key }}>{artifact.object_key}</Text>
                    <Text type="secondary">上传于 {dayjs(artifact.created_at).format("YYYY-MM-DD HH:mm")}</Text>
                  </Space>
                  {artifact === primaryArtifact && upgradeCommand ? (
                    <TerminalBlock code={upgradeCommand} style={{ marginTop: 8 }} />
                  ) : null}
                </Space>
              </List.Item>
            )}
          />
        ) : (
          <Alert
            type="info"
            showIcon
            message="当前租户还没有匹配该平台的推荐制品"
            description={
              <Space wrap>
                <Text>可先去应用分发页上传对应平台的 Agent 安装包，并在名称、渠道或对象 Key 中标明平台。</Text>
                <Link to="/artifacts">前往应用分发</Link>
              </Space>
            }
          />
        )}
        <Divider />
        <Typography.Title level={5}>推荐安装方式</Typography.Title>
        <Space direction="vertical">
          <Typography.Text>1. 直接复制“推荐首装命令”，优先使用制品里内嵌的安装脚本模板。</Typography.Text>
          <Typography.Text>2. 若没有制品模板，页面会回退为标准 Enrollment 环境变量启动命令。</Typography.Text>
          <Typography.Text>3. 首装完成后，优先对当前平台使用 stable 渠道制品的升级命令。</Typography.Text>
          <Typography.Text>4. 设备侧 `install_update` 已支持最小下载、校验和安装执行链路；零接触页同时保留可直接复制的本地升级命令。</Typography.Text>
        </Space>
        <Divider />
        <Space wrap>
          <Button
            type="primary"
            onClick={async () => {
              await navigator.clipboard.writeText(installCommand);
            }}
          >
            复制首装命令
          </Button>
          <Button
            onClick={() => {
              downloadTextFile("Install-DMSX-Agent.ps1", windowsInstallerScript);
            }}
            disabled={platform !== "windows" || !apiUrl || !tenantId || !enrollmentToken}
          >
            下载 Windows 脚本
          </Button>
          <Button
            onClick={async () => {
              if (!upgradeCommand) return;
              await navigator.clipboard.writeText(upgradeCommand);
            }}
            disabled={!upgradeCommand}
          >
            复制升级命令
          </Button>
        </Space>
      </Card>
    </Space>
  );
};
