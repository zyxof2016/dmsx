param(
  [Parameter(Mandatory=$true)] [string] $ApiUrl,
  [Parameter(Mandatory=$true)] [string] $TenantId,
  [Parameter(Mandatory=$true)] [string] $EnrollmentToken,
  [string] $ProjectDir = "$PSScriptRoot\..\android-agent",
  [string] $OutputPath = "$PSScriptRoot\..\target\packages\DMSX-Agent-Android.apk",
  [ValidateSet("Debug", "Release")] [string] $Variant = "Debug"
)

$ErrorActionPreference = "Stop"

$projectFullPath = [System.IO.Path]::GetFullPath((Join-Path (Get-Location) $ProjectDir))
$outputFullPath = [System.IO.Path]::GetFullPath((Join-Path (Get-Location) $OutputPath))
$outputDir = Split-Path -Parent $outputFullPath
$gradlew = Join-Path $projectFullPath "gradlew.bat"
$gradleCommand = if (Test-Path $gradlew) { $gradlew } else { "gradle" }
$task = "assemble$Variant"
$apkName = if ($Variant -eq "Release") { "app-release.apk" } else { "app-debug.apk" }
$apkPath = Join-Path $projectFullPath "app\build\outputs\apk\$($Variant.ToLowerInvariant())\$apkName"

New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

Push-Location $projectFullPath
try {
  & $gradleCommand `
    $task `
    "-PdmsxApiUrl=$ApiUrl" `
    "-PdmsxTenantId=$TenantId" `
    "-PdmsxEnrollmentToken=$EnrollmentToken" `
    "-PdmsxStartOnBoot=true"
  if ($LASTEXITCODE -ne 0) {
    throw "Gradle task failed: $task"
  }
}
finally {
  Pop-Location
}

if (-not (Test-Path $apkPath)) {
  throw "APK was not produced: $apkPath"
}

Copy-Item -Force $apkPath $outputFullPath

Write-Host "Android Agent APK created:"
Write-Host $outputFullPath
Write-Host "Install and open this APK on the Android device. It will auto-load enrollment config and start registration."
