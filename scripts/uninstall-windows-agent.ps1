param(
  [string] $ServiceName = "DMSXAgent",
  [string] $InstallDir = "$env:ProgramFiles\DMSX\Agent",
  [string] $DataDir = "$env:ProgramData\DMSX\Agent",
  [switch] $KeepConfig
)

$ErrorActionPreference = "Stop"

if (-not ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
  throw "Please run this script from an elevated PowerShell session."
}

$existing = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($existing) {
  Stop-Service -Name $ServiceName -ErrorAction SilentlyContinue
  sc.exe delete $ServiceName | Out-Null
  Start-Sleep -Seconds 2
}

if (Test-Path $InstallDir) {
  Remove-Item -Recurse -Force $InstallDir
}
if (-not $KeepConfig -and (Test-Path $DataDir)) {
  Remove-Item -Recurse -Force $DataDir
}

Write-Host "DMSX Agent uninstalled."
