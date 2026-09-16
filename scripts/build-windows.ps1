#Requires -Version 5.1
<#
.SYNOPSIS
  Build NetPulse Windows installers (NSIS + MSI) from this repository.

.DESCRIPTION
  Produces self-contained installers under src-tauri/target/release/bundle/.
  WebView2 bootstrapper is embedded so end users do not download runtime separately.
  Build machine still needs Node.js, Rust, and MSVC Build Tools (one-time for developers).
#>
$ErrorActionPreference = "Stop"
Set-Location (Split-Path -Parent $PSScriptRoot)

Write-Host "==> NetPulse Windows release build" -ForegroundColor Cyan

if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
  throw "npm not found. Install Node.js 18+ once on the build machine."
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
  throw "cargo not found. Install Rust once on the build machine."
}

Write-Host "==> npm install"
npm install --no-fund --no-audit
if ($LASTEXITCODE -ne 0) { throw "npm install failed" }

Write-Host "==> tauri build (nsis + msi)"
npm run desktop:build:windows
if ($LASTEXITCODE -ne 0) { throw "tauri build failed" }

$bundle = Join-Path $PWD "src-tauri\target\release\bundle"
Write-Host ""
Write-Host "Build finished. Installers:" -ForegroundColor Green
Get-ChildItem -Path $bundle -Recurse -Include *.exe,*.msi -ErrorAction SilentlyContinue |
  ForEach-Object { Write-Host "  $($_.FullName)" }

Write-Host ""
Write-Host "End users only need the .exe/.msi installer. No Node/Rust on target PCs."
Write-Host "WebView2 bootstrapper is embedded in the NSIS installer when missing on the PC."
