# NetPulse

**English** · [فارسی](README.fa.md)

Local traffic monitor for **this device only** — every app, every interface, every Wi-Fi network, with history kept on disk.

**Tagline:** Every byte, every app, every network — local and yours.

---

## What you get

| Feature | Description |
|---|---|
| Per-app / per-process | Bytes sent & received, live rates |
| Per-interface | Wi-Fi, Ethernet, VPN, hotspot, mobile, Bluetooth |
| Per-SSID | Usage tagged with the Wi-Fi network you were on |
| History | Hourly / daily / monthly aggregates in local SQLite |
| Export | CSV and JSON |
| Privacy | No cloud, no LAN scanning, no other devices |

---

## Platforms

| Platform | Product | How traffic is read | End-user package |
|---|---|---|---|
| **Windows** | Desktop (Tauri) | IP Helper + TCP ESTATS | `.exe` NSIS installer + `.msi` |
| **Linux** | Desktop (Tauri) | `/proc/net/dev` + inet_diag | `.AppImage` + `.deb` |
| **macOS** | Desktop (Tauri) | `nettop` + `getifaddrs` | `.dmg` + `.app` |
| **Android** | Kotlin / Compose | `NetworkStatsManager` | `.apk` / `.aab` |
| **iOS** | Later phase | Packet Tunnel Provider | — |

**Self-contained installers:** people who *install* NetPulse do **not** need Node.js, Rust, Android Studio, or extra sideloaded runtimes from the internet (Windows ships with an embedded WebView2 bootstrapper inside the installer). Build tools are only needed on the **developer/build machine**.

---

## Quick links

- [Build guide (English)](docs/en/BUILD.md) — produce installers for every platform  
- [Install guide (English)](docs/en/INSTALL.md) — where outputs are and how to install them  
- [Roadmap](docs/ROADMAP.md)  
- [راهنمای بیلد (فارسی)](docs/fa/BUILD.md)  
- [راهنمای نصب (فارسی)](docs/fa/INSTALL.md)  
- [Android notes](android/README.md)

---

## Develop (desktop)

```bash
npm install
npm run desktop:dev
```

## Build installers (helper scripts in `/scripts`)

| Platform | Script |
|---|---|
| Windows | `scripts/build-windows.ps1` |
| Linux | `scripts/build-linux.sh` |
| macOS | `scripts/build-macos.sh` |
| Android | `scripts/build-android.ps1` / `scripts/build-android.sh` |

Or via npm on the current OS:

```bash
npm run desktop:build
```

Artifacts land under `src-tauri/target/release/bundle/` (desktop) or `android/app/build/outputs/` (Android).

---

## Data stays local

- **Desktop:** OS app-data folder (e.g. `%APPDATA%\com.saeedshamc.netpulse\netpulse.db` on Windows)
- **Android:** app-private `netpulse.db`

---

## License

See [LICENSE](LICENSE).
