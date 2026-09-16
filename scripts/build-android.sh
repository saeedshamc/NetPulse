#!/usr/bin/env bash
# Build NetPulse Android release APK + AAB from this repository.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/android"

echo "==> NetPulse Android release build"

if [[ ! -f ./gradlew ]]; then
  echo "gradlew not found."
  echo "Open android/ once in Android Studio to generate the Gradle Wrapper."
  exit 1
fi

chmod +x ./gradlew
./gradlew :app:assembleRelease :app:bundleRelease --no-daemon

echo ""
echo "Build finished:"
find app/build/outputs -type f \( -name '*.apk' -o -name '*.aab' \) -print 2>/dev/null || true
echo ""
echo "APK is self-contained. Usage Access is an OS setting, not an external dependency."
