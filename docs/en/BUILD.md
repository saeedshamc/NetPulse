# Build guide — NetPulse

Produce **self-contained** executables / installers from this repository.

> End users of the built packages do **not** install Node, Rust, or sideload extra runtimes.  
> Those tools are only required on the **machine that builds** the release.

Also see: [Install guide](INSTALL.md) · [فارسی](../fa/BUILD.md)

---

## 1. One-time tools on the build machine

| Tool | Windows | Linux | macOS | Android |
|---|---|---|---|---|
| Node.js 18+ | yes | yes | yes | — |
| Rust (stable) | yes | yes | yes | — |
| MSVC Build Tools | yes | — | — | — |
| `build-essential`, `webkit2gtk`, `libssl`, etc. | — | yes (Tauri Linux deps) | — | — |
| Xcode / CLT | — | — | yes | — |
| Android Studio / SDK / JDK 17 | — | — | — | yes |

Tauri Linux system packages (Ubuntu/Debian example):

```bash
sudo apt update
sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

---

## 2. Helper scripts (preferred)

All scripts live under [`scripts/`](../../scripts/) inside this repo.

### Windows → NSIS (`.exe`) + MSI

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\build-windows.ps1
```

**Outputs (typical):**

- `src-tauri/target/release/bundle/nsis/NetPulse_*_x64-setup.exe`
- `src-tauri/target/release/bundle/msi/NetPulse_*_x64_en-US.msi`

The NSIS installer **embeds the WebView2 bootstrapper**, so target PCs do not need a separate WebView2 download step for first install in most cases.

### Linux → AppImage + deb

```bash
chmod +x scripts/build-linux.sh
./scripts/build-linux.sh
```

**Outputs (typical):**

- `src-tauri/target/release/bundle/appimage/NetPulse_*.AppImage` ← portable single file
- `src-tauri/target/release/bundle/deb/NetPulse_*.deb`

### macOS → DMG + app

```bash
chmod +x scripts/build-macos.sh
./scripts/build-macos.sh
```

**Outputs (typical):**

- `src-tauri/target/release/bundle/dmg/NetPulse_*.dmg`
- `src-tauri/target/release/bundle/macos/NetPulse.app`

### Android → APK + AAB

First open `android/` once in Android Studio so the Gradle Wrapper (`gradlew` / `gradlew.bat`) is generated, then:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\build-android.ps1
```

```bash
chmod +x scripts/build-android.sh
./scripts/build-android.sh
```

**Outputs (typical):**

- `android/app/build/outputs/apk/release/app-release.apk`
- `android/app/build/outputs/bundle/release/app-release.aab`

---

## 3. npm shortcuts (desktop, current OS only)

```bash
npm install
npm run desktop:build              # all default bundles for this OS
npm run desktop:build:windows      # nsis + msi
npm run desktop:build:linux        # appimage + deb
npm run desktop:build:macos        # dmg + app
```

Cross-compiling (e.g. building a Linux AppImage on Windows) is **not** supported by these scripts — build each desktop target on that OS (or CI runners for that OS).

---

## 4. What “self-contained” means here

| Audience | Needs |
|---|---|
| **End user** | Only the installer / AppImage / APK from this project’s build output |
| **Developer / CI** | Node + Rust (+ OS SDKs) once, to run the scripts above |

Nothing in the shipping installer expects users to clone this repo or install npm packages on their PC.

---

## 5. Signing (optional but recommended for distribution)

- **Windows:** Authenticode sign the NSIS/MSI after build  
- **macOS:** Apple Developer ID + notarization for Gatekeeper  
- **Android:** configure a release keystore in Android Studio (`signingConfigs`) before Play Store upload  

Unsigned builds still run for local testing; OS may show warnings.
