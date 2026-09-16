#!/usr/bin/env bash
# Build NetPulse macOS app + DMG from this repository.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "==> NetPulse macOS release build"

command -v npm >/dev/null || { echo "npm not found (install Node.js 18+ on the build machine)"; exit 1; }
command -v cargo >/dev/null || { echo "cargo not found (install Rust on the build machine)"; exit 1; }
command -v xcodebuild >/dev/null || { echo "Xcode command line tools required on the build Mac"; exit 1; }

echo "==> npm install"
npm install --no-fund --no-audit

echo "==> tauri build (app + dmg)"
npm run desktop:build:macos

BUNDLE="$ROOT/src-tauri/target/release/bundle"
echo ""
echo "Build finished. Artifacts:"
find "$BUNDLE" -type f \( -name '*.dmg' -o -name '*.app' \) -print 2>/dev/null || true
find "$BUNDLE" -type d -name '*.app' -print 2>/dev/null || true
echo ""
echo "Distribute the .dmg installer. End users do not need Node.js or Rust."
echo "Per-app traffic via nettop may require running with elevated privileges."
