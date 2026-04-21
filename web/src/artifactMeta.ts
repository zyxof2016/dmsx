import type { Artifact, Device } from "./api/types";

export type ArtifactPlatformKey = "linux" | "windows" | "android";

export type ArtifactMetaDraft = {
  platforms: ArtifactPlatformKey[];
  download_url?: string;
  installer_kind?: string;
  install_commands: Partial<Record<ArtifactPlatformKey, string>>;
  upgrade_commands: Partial<Record<ArtifactPlatformKey, string>>;
};

export function normalizeDevicePlatform(platform?: Device["platform"]): ArtifactPlatformKey {
  if (platform === "windows") return "windows";
  if (platform === "android") return "android";
  return "linux";
}

function asStringMap(value: unknown): Partial<Record<ArtifactPlatformKey, string>> {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  const entries = Object.entries(value)
    .filter((entry): entry is [string, string] => typeof entry[1] === "string")
    .filter(([key]) => ["linux", "windows", "android"].includes(key));
  return Object.fromEntries(entries) as Partial<Record<ArtifactPlatformKey, string>>;
}

function asPlatformList(value: unknown): ArtifactPlatformKey[] {
  if (!Array.isArray(value)) return [];
  return value.filter((item): item is ArtifactPlatformKey => item === "linux" || item === "windows" || item === "android");
}

export function parseArtifactMeta(metadata: Record<string, unknown> | undefined): ArtifactMetaDraft {
  return {
    platforms: asPlatformList(metadata?.platforms),
    download_url: typeof metadata?.download_url === "string" ? metadata.download_url : undefined,
    installer_kind: typeof metadata?.installer_kind === "string" ? metadata.installer_kind : undefined,
    install_commands: asStringMap(metadata?.install_commands),
    upgrade_commands: asStringMap(metadata?.upgrade_commands),
  };
}

export function buildArtifactMeta(draft: ArtifactMetaDraft): Record<string, unknown> | undefined {
  const metadata: Record<string, unknown> = {};
  if (draft.platforms.length > 0) metadata.platforms = draft.platforms;
  if (draft.download_url?.trim()) metadata.download_url = draft.download_url.trim();
  if (draft.installer_kind?.trim()) metadata.installer_kind = draft.installer_kind.trim();
  if (Object.keys(draft.install_commands).length > 0) metadata.install_commands = compactStringMap(draft.install_commands);
  if (Object.keys(draft.upgrade_commands).length > 0) metadata.upgrade_commands = compactStringMap(draft.upgrade_commands);
  return Object.keys(metadata).length > 0 ? metadata : undefined;
}

function compactStringMap(value: Partial<Record<ArtifactPlatformKey, string>>): Partial<Record<ArtifactPlatformKey, string>> {
  return Object.fromEntries(
    Object.entries(value).filter((entry): entry is [ArtifactPlatformKey, string] => typeof entry[1] === "string" && entry[1].trim().length > 0),
  ) as Partial<Record<ArtifactPlatformKey, string>>;
}

export function inferInstallerKind(artifact: Artifact): string | undefined {
  const meta = parseArtifactMeta(artifact.metadata);
  if (meta.installer_kind) return meta.installer_kind;
  const ext = artifact.object_key.split(".").pop()?.toLowerCase();
  return ext || undefined;
}

export function chooseArtifactCommand(
  artifact: Artifact,
  kind: "install" | "upgrade",
  platform: ArtifactPlatformKey,
): string | undefined {
  const meta = parseArtifactMeta(artifact.metadata);
  const source = kind === "install" ? meta.install_commands : meta.upgrade_commands;
  return source[platform] ?? source.windows ?? source.linux ?? source.android;
}

export function buildArtifactLabel(artifact: Artifact): string {
  return `${artifact.name} ${artifact.version} [${artifact.channel}]`;
}

export function artifactMatchesPlatform(artifact: Artifact, platform: ArtifactPlatformKey): boolean {
  const text = [artifact.name, artifact.version, artifact.channel, artifact.object_key, JSON.stringify(artifact.metadata ?? {})]
    .filter(Boolean)
    .join(" ")
    .toLowerCase();
  const platformTokens: Record<ArtifactPlatformKey, RegExp[]> = {
    linux: [/linux/i, /macos/i, /darwin/i],
    windows: [/windows/i, /win64/i, /win32/i, /msi/i, /exe/i],
    android: [/android/i, /apk/i, /adb/i],
  };
  return platformTokens[platform].some((matcher) => matcher.test(text));
}

export function selectRecommendedArtifact(
  artifacts: Artifact[],
  platform: ArtifactPlatformKey,
): Artifact | null {
  return [...artifacts]
    .map((artifact) => ({
      artifact,
      score: (artifactMatchesPlatform(artifact, platform) ? 100 : 0) + (/stable/i.test(artifact.channel) ? 20 : 0),
    }))
    .filter((item) => item.score > 0)
    .sort((a, b) => {
      if (b.score !== a.score) return b.score - a.score;
      return new Date(b.artifact.created_at).getTime() - new Date(a.artifact.created_at).getTime();
    })
    .map((item) => item.artifact)[0] ?? null;
}
