param(
  [string] $ApiUrl = $env:DMSX_API_URL,
  [string] $TenantId = $env:DMSX_TENANT_ID,
  [string] $EnrollmentToken = $env:DMSX_DEVICE_ENROLLMENT_TOKEN,
  [string] $RegistrationCode = $env:DMSX_DEVICE_REGISTRATION_CODE,
  [int] $HeartbeatSecs = 10,
  [int] $PollSecs = 2,
  [string] $Cargo = "cargo"
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$envFile = Join-Path $repoRoot ".env"

if (Test-Path $envFile) {
  Get-Content $envFile | ForEach-Object {
    if ($_ -match '^\s*([^#][^=]+)=(.*)$') {
      $name = $matches[1].Trim()
      $value = $matches[2].Trim().Trim('"').Trim("'")
      if (-not [Environment]::GetEnvironmentVariable($name, "Process")) {
        Set-Item -Path "Env:$name" -Value $value
      }
    }
  }
}

if ([string]::IsNullOrWhiteSpace($ApiUrl)) {
  $ApiUrl = if ($env:DMSX_API_URL) { $env:DMSX_API_URL } else { "http://127.0.0.1:8080" }
}

if ([string]::IsNullOrWhiteSpace($TenantId)) {
  $TenantId = if ($env:DMSX_TENANT_ID) { $env:DMSX_TENANT_ID } else { "00000000-0000-0000-0000-000000000001" }
}

if ([string]::IsNullOrWhiteSpace($EnrollmentToken)) {
  $EnrollmentToken = $env:DMSX_DEVICE_ENROLLMENT_TOKEN
}

if ([string]::IsNullOrWhiteSpace($RegistrationCode)) {
  $RegistrationCode = $env:DMSX_DEVICE_REGISTRATION_CODE
}

if ([string]::IsNullOrWhiteSpace($EnrollmentToken) -and [string]::IsNullOrWhiteSpace($RegistrationCode)) {
  throw "Set DMSX_DEVICE_ENROLLMENT_TOKEN or DMSX_DEVICE_REGISTRATION_CODE, or pass -EnrollmentToken/-RegistrationCode."
}

$env:DMSX_API_URL = $ApiUrl.TrimEnd("/")
$env:DMSX_TENANT_ID = $TenantId
$env:DMSX_HEARTBEAT_SECS = "$HeartbeatSecs"
$env:DMSX_POLL_SECS = "$PollSecs"
$env:RUSTFLAGS = "-C target-feature=+crt-static"

if (-not [string]::IsNullOrWhiteSpace($EnrollmentToken)) {
  $env:DMSX_DEVICE_ENROLLMENT_TOKEN = $EnrollmentToken
}

if (-not [string]::IsNullOrWhiteSpace($RegistrationCode)) {
  $env:DMSX_DEVICE_REGISTRATION_CODE = $RegistrationCode
}

Write-Host "Starting DMSX Agent for Windows dev..."
Write-Host "API:    $($env:DMSX_API_URL)"
Write-Host "Tenant: $($env:DMSX_TENANT_ID)"
Write-Host "CRT:    static via RUSTFLAGS=$($env:RUSTFLAGS)"

Push-Location $repoRoot
try {
  & $Cargo run -p dmsx-agent
}
finally {
  Pop-Location
}
