#!/usr/bin/env bash
# Build NetPulse Linux packages (AppImage + deb) from this repository.
# Output is self-contained for end users (especially AppImage).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "==> NetPulse Linux release build"

command -v npm >/dev/null || { echo "npm not found (install Node.js 18+ on the build machine)"; exit 1; }
command -v cargo >/dev/null || { echo "cargo not found (install Rust on the build machine)"; exit 1; }

echo "==> npm install"
npm install --no-fund --no-audit

echo "==> tauri build (appimage + deb)"
npm run desktop:build:linux

BUNDLE="$ROOT/src-tauri/target/release/bundle"
echo ""
echo "Build finished. Packages:"
find "$BUNDLE" -type f \( -name '*.AppImage' -o -name '*.deb' -o -name '*.rpm' \) -print 2>/dev/null || true
echo ""
echo "Prefer AppImage for a portable single-file executable with no system package install."
echo "End users do not need Node.js or Rust."
