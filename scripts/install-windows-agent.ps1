param(
  [Parameter(Mandatory=$true)] [string] $ApiUrl,
  [Parameter(Mandatory=$true)] [string] $TenantId,
  [Parameter(Mandatory=$true)] [string] $EnrollmentToken,
  [string] $AgentExe = "$PSScriptRoot\..\target\release\dmsx-agent.exe",
  [string] $AgentExeUrl = "",
  [string] $InstallDir = "$env:ProgramFiles\DMSX\Agent",
  [string] $DataDir = "$env:ProgramData\DMSX\Agent",
  [string] $ServiceName = "DMSXAgent",
  [string] $DisplayName = "DMSX Agent"
)

$ErrorActionPreference = "Stop"

if (-not ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
  if (-not $PSCommandPath) {
    throw "Please run this script from an elevated PowerShell session."
  }

  $args = @(
    "-ExecutionPolicy", "Bypass",
    "-File", "`"$PSCommandPath`"",
    "-ApiUrl", "`"$ApiUrl`"",
    "-TenantId", "`"$TenantId`"",
    "-EnrollmentToken", "`"$EnrollmentToken`"",
    "-AgentExe", "`"$AgentExe`"",
    "-AgentExeUrl", "`"$AgentExeUrl`"",
    "-InstallDir", "`"$InstallDir`"",
    "-DataDir", "`"$DataDir`"",
    "-ServiceName", "`"$ServiceName`"",
    "-DisplayName", "`"$DisplayName`""
  )
  Start-Process -FilePath "powershell.exe" -Verb RunAs -ArgumentList $args
  exit
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $DataDir | Out-Null

$targetExe = Join-Path $InstallDir "dmsx-agent.exe"
$configPath = Join-Path $DataDir "agent.json"

if ($AgentExeUrl.Trim()) {
  Invoke-WebRequest -Uri $AgentExeUrl.Trim() -OutFile $targetExe
} elseif (Test-Path $AgentExe) {
  Copy-Item -Force $AgentExe $targetExe
} else {
  throw "Agent executable not found: $AgentExe. Pass -AgentExeUrl to download it during installation."
}

$config = [ordered]@{
  api_base = $ApiUrl.TrimEnd('/')
  tenant_id = $TenantId
  enrollment_token = $EnrollmentToken
  heartbeat_secs = 30
  poll_secs = 10
  command_execution_timeout_secs = 300
}
$config | ConvertTo-Json -Depth 4 | Set-Content -Encoding UTF8 -Path $configPath

$existing = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($existing) {
  Stop-Service -Name $ServiceName -ErrorAction SilentlyContinue
  sc.exe delete $ServiceName | Out-Null
  Start-Sleep -Seconds 2
}

$binaryPath = '"{0}" --windows-service --config "{1}"' -f $targetExe, $configPath
New-Service -Name $ServiceName -BinaryPathName $binaryPath -DisplayName $DisplayName -Description "DMSX Windows device agent" -StartupType Automatic | Out-Null
Start-Service -Name $ServiceName

Write-Host "DMSX Agent installed and started."
Write-Host "Service: $ServiceName"
Write-Host "Binary:  $targetExe"
Write-Host "Config:  $configPath"
