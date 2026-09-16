# Install guide — NetPulse

Where release files appear and how end users install them.

[Build guide](BUILD.md) · [فارسی](../fa/INSTALL.md)

---

## Windows

1. Build with `scripts/build-windows.ps1` (or copy artifacts from CI).
2. Give users either:
   - `NetPulse_*-setup.exe` (NSIS) — recommended, embeds WebView2 bootstrapper  
   - or `NetPulse_*.msi`
3. Double-click → install → launch **NetPulse** from Start Menu.
4. Optional: run as Administrator for fuller per-process TCP stats.

**No** Node.js / Rust / Git required on the user PC.

---

## Linux

1. Build with `scripts/build-linux.sh`.
2. Prefer **AppImage**:
   ```bash
   chmod +x NetPulse_*.AppImage
   ./NetPulse_*.AppImage
   ```
3. Or install the `.deb` with your package manager:
   ```bash
   sudo dpkg -i NetPulse_*.deb
   ```

**No** Node.js / Rust required on the user machine for AppImage/deb usage.

---

## macOS

1. Build with `scripts/build-macos.sh` on a Mac.
2. Open the `.dmg`, drag **NetPulse** to Applications.
3. First launch: allow in System Settings if Gatekeeper blocks an unsigned build.
4. For complete per-app numbers, elevated privileges may be required (`nettop`).

**No** Node.js / Rust required for end users.

---

## Android

1. Build release APK/AAB with the Android scripts or Android Studio.
2. Install APK (sideload) or publish AAB to Play Store.
3. Open the app → grant **Usage access** when prompted (system setting, not a third-party download).

The APK is a normal self-contained Android package.

---

## After install — data

| Platform | Database |
|---|---|
| Windows | `%APPDATA%\com.saeedshamc.netpulse\netpulse.db` |
| Linux | `~/.local/share/com.saeedshamc.netpulse/netpulse.db` (typical) |
| macOS | `~/Library/Application Support/com.saeedshamc.netpulse/netpulse.db` (typical) |
| Android | app-private storage `netpulse.db` |

Uninstalling removes the app; wipe app data if you also want history gone.
