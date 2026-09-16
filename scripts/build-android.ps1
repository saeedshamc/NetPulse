#Requires -Version 5.1
<#
.SYNOPSIS
  Build NetPulse Android release APK (and optional AAB) from this repository.
#>
$ErrorActionPreference = "Stop"
$androidRoot = Join-Path (Split-Path -Parent $PSScriptRoot) "android"
Set-Location $androidRoot

Write-Host "==> NetPulse Android release build" -ForegroundColor Cyan

$gradlew = Join-Path $androidRoot "gradlew.bat"
if (-not (Test-Path $gradlew)) {
  Write-Host "gradlew.bat not found."
  Write-Host "Open the android/ folder once in Android Studio to generate the Gradle Wrapper,"
  Write-Host "or install Android Studio and use Build > Generate Signed Bundle / APK."
  throw "Missing Gradle Wrapper"
}

Write-Host "==> assembleRelease"
& $gradlew :app:assembleRelease --no-daemon
if ($LASTEXITCODE -ne 0) { throw "assembleRelease failed" }

Write-Host "==> bundleRelease (Play Store AAB)"
& $gradlew :app:bundleRelease --no-daemon
if ($LASTEXITCODE -ne 0) { throw "bundleRelease failed" }

$apkDir = Join-Path $androidRoot "app\build\outputs\apk\release"
$aabDir = Join-Path $androidRoot "app\build\outputs\bundle\release"
Write-Host ""
Write-Host "Build finished:" -ForegroundColor Green
Get-ChildItem -Path $apkDir, $aabDir -Recurse -Include *.apk,*.aab -ErrorAction SilentlyContinue |
  ForEach-Object { Write-Host "  $($_.FullName)" }
Write-Host ""
Write-Host "APK installs directly. No extra runtime packages required on the phone."
Write-Host "User must still grant Usage Access in Android Settings (OS permission, not an external app)."
