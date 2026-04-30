import type { Artifact } from "./api/types";

export function readArtifactDownloadUrl(artifact: Artifact | null): string | null {
  const raw = artifact?.metadata?.download_url;
  return typeof raw === "string" && raw.trim() ? raw.trim() : null;
}

function psString(value: string): string {
  return `'${value.replaceAll("'", "''")}'`;
}

export interface WindowsInstallerScriptParams {
  apiUrl: string;
  tenantId: string;
  enrollmentToken: string;
  agentDownloadUrl?: string | null;
  serviceName?: string;
  displayName?: string;
}

export function buildWindowsOneClickInstallerScript({
  apiUrl,
  tenantId,
  enrollmentToken,
  agentDownloadUrl,
  serviceName = "DMSXAgent",
  displayName = "DMSX Agent",
}: WindowsInstallerScriptParams): string {
  const agentUrl = agentDownloadUrl?.trim() ?? "";

  return [
    "param()",
    "",
    "$ErrorActionPreference = \"Stop\"",
    "",
    `$ApiUrl = ${psString(apiUrl)}`,
    `$TenantId = ${psString(tenantId)}`,
    `$EnrollmentToken = ${psString(enrollmentToken)}`,
    `$AgentExeUrl = ${psString(agentUrl)}`,
    `$ServiceName = ${psString(serviceName)}`,
    `$DisplayName = ${psString(displayName)}`,
    "$InstallDir = Join-Path $env:ProgramFiles \"DMSX\\Agent\"",
    "$DataDir = Join-Path $env:ProgramData \"DMSX\\Agent\"",
    "",
    "function Test-IsAdmin {",
    "  $identity = [Security.Principal.WindowsIdentity]::GetCurrent()",
    "  $principal = [Security.Principal.WindowsPrincipal] $identity",
    "  return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)",
    "}",
    "",
    "if (-not (Test-IsAdmin)) {",
    "  if (-not $PSCommandPath) { throw \"Please save this script and run it from an elevated PowerShell session.\" }",
    "  Start-Process -FilePath \"powershell.exe\" -Verb RunAs -ArgumentList @(\"-ExecutionPolicy\", \"Bypass\", \"-File\", \"`\"$PSCommandPath`\"\")",
    "  exit",
    "}",
    "",
    "New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null",
    "New-Item -ItemType Directory -Force -Path $DataDir | Out-Null",
    "",
    "$targetExe = Join-Path $InstallDir \"dmsx-agent.exe\"",
    "$localExe = Join-Path $PSScriptRoot \"dmsx-agent.exe\"",
    "",
    "if ($AgentExeUrl) {",
    "  Write-Host \"Downloading DMSX Agent...\"",
    "  Invoke-WebRequest -Uri $AgentExeUrl -OutFile $targetExe",
    "} elseif (Test-Path $localExe) {",
    "  Copy-Item -Force $localExe $targetExe",
    "} else {",
    "  throw \"No Agent download URL was embedded and dmsx-agent.exe was not found next to this script.\"",
    "}",
    "",
    "$configPath = Join-Path $DataDir \"agent.json\"",
    "$config = [ordered]@{",
    "  api_base = $ApiUrl.TrimEnd('/')",
    "  tenant_id = $TenantId",
    "  enrollment_token = $EnrollmentToken",
    "  heartbeat_secs = 30",
    "  poll_secs = 10",
    "  command_execution_timeout_secs = 300",
    "}",
    "$config | ConvertTo-Json -Depth 4 | Set-Content -Encoding UTF8 -Path $configPath",
    "",
    "$existing = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue",
    "if ($existing) {",
    "  Stop-Service -Name $ServiceName -ErrorAction SilentlyContinue",
    "  sc.exe delete $ServiceName | Out-Null",
    "  Start-Sleep -Seconds 2",
    "}",
    "",
    "$binaryPath = '\"{0}\" --windows-service --config \"{1}\"' -f $targetExe, $configPath",
    "New-Service -Name $ServiceName -BinaryPathName $binaryPath -DisplayName $DisplayName -Description \"DMSX Windows device agent\" -StartupType Automatic | Out-Null",
    "Start-Service -Name $ServiceName",
    "",
    "Write-Host \"DMSX Agent installed and started.\"",
    "Write-Host \"Service: $ServiceName\"",
    "Write-Host \"Binary:  $targetExe\"",
    "Write-Host \"Config:  $configPath\"",
    "",
  ].join("\r\n");
}

export function downloadTextFile(filename: string, content: string) {
  const blob = new Blob([content], { type: "text/plain;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}
