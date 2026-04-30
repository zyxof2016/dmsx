param(
  [Parameter(Mandatory=$true)] [string] $ApiUrl,
  [Parameter(Mandatory=$true)] [string] $TenantId,
  [Parameter(Mandatory=$true)] [string] $EnrollmentToken,
  [string] $AgentExe = "$PSScriptRoot\..\target\release\dmsx-agent.exe",
  [string] $OutputPath = "$PSScriptRoot\..\target\packages\DMSX-Agent-Install.zip",
  [string] $ServiceName = "DMSXAgent",
  [string] $DisplayName = "DMSX Agent"
)

$ErrorActionPreference = "Stop"

function ConvertTo-PowerShellSingleQuotedString {
  param([string] $Value)
  return "'" + $Value.Replace("'", "''") + "'"
}

if (-not (Test-Path $AgentExe)) {
  throw "Agent executable not found: $AgentExe. Build it first with: cargo build -p dmsx-agent --release"
}

$installerSource = Join-Path $PSScriptRoot "install-windows-agent.ps1"
if (-not (Test-Path $installerSource)) {
  throw "Installer script not found: $installerSource"
}

$outputFullPath = [System.IO.Path]::GetFullPath((Join-Path (Get-Location) $OutputPath))
$outputDir = Split-Path -Parent $outputFullPath
$stagingRoot = Join-Path $PSScriptRoot "..\target\package-staging"
$stagingDir = Join-Path $stagingRoot ("DMSX-Agent-Install-" + [Guid]::NewGuid().ToString("N"))

New-Item -ItemType Directory -Force -Path $outputDir | Out-Null
New-Item -ItemType Directory -Force -Path $stagingDir | Out-Null

try {
  Copy-Item -Force $AgentExe (Join-Path $stagingDir "dmsx-agent.exe")
  Copy-Item -Force $installerSource (Join-Path $stagingDir "install-windows-agent.ps1")

  $entrypoint = @(
    '$ErrorActionPreference = "Stop"',
    '$scriptDir = Split-Path -Parent $PSCommandPath',
    '$installer = Join-Path $scriptDir "install-windows-agent.ps1"',
    '$agentExe = Join-Path $scriptDir "dmsx-agent.exe"',
    '& $installer `',
    ('  -ApiUrl ' + (ConvertTo-PowerShellSingleQuotedString $ApiUrl) + ' `'),
    ('  -TenantId ' + (ConvertTo-PowerShellSingleQuotedString $TenantId) + ' `'),
    ('  -EnrollmentToken ' + (ConvertTo-PowerShellSingleQuotedString $EnrollmentToken) + ' `'),
    '  -AgentExe $agentExe `',
    ('  -ServiceName ' + (ConvertTo-PowerShellSingleQuotedString $ServiceName) + ' `'),
    ('  -DisplayName ' + (ConvertTo-PowerShellSingleQuotedString $DisplayName))
  ) -join "`r`n"

  Set-Content -Encoding UTF8 -Path (Join-Path $stagingDir "Install-DMSX-Agent.ps1") -Value $entrypoint

  if (Test-Path $outputFullPath) {
    Remove-Item -Force $outputFullPath
  }
  Compress-Archive -Path (Join-Path $stagingDir "*") -DestinationPath $outputFullPath

  Write-Host "Windows Agent install package created:"
  Write-Host $outputFullPath
  Write-Host "Send this zip to the user. They only need to extract it and run Install-DMSX-Agent.ps1."
}
finally {
  if (Test-Path $stagingDir) {
    Remove-Item -Recurse -Force $stagingDir
  }
}
