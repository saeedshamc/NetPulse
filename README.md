# NetPulse

Local per-application and per-interface network traffic monitor for your own machine.

**Tagline:** Every byte, every app, every network — local and yours.

## What it does

- Live per-process / per-app traffic (sent / received / rate)
- Per-network-interface breakdown (Wi-Fi, Ethernet, VPN, hotspot, mobile, Bluetooth)
- Per-SSID tagging while connected to Wi-Fi
- Hourly / daily / monthly history stored in a local SQLite database
- CSV and JSON export

NetPulse only reads what the OS reports about **this machine**. It does not scan the LAN, discover other devices, or send usage data to any server.

## Platforms

| Platform | App | Traffic source |
|---|---|---|
| Windows | Desktop (Tauri) | IP Helper TCP/UDP tables + ESTATS, `GetIfTable` |
| Linux | Desktop (Tauri) | `/proc/net/dev` + inet_diag (`SOCK_DIAG`) |
| macOS | Desktop (Tauri) | `nettop` + `getifaddrs` (elevated privileges recommended) |
| Android | Kotlin / Compose | `NetworkStatsManager` (Usage Access required) |
| iOS | Not in this tree yet | Planned separately (Packet Tunnel Provider) |

## Desktop requirements

- [Rust](https://www.rust-lang.org/tools/install)
- [Node.js](https://nodejs.org/) 18+
- Platform build tools (MSVC on Windows, Xcode on macOS, standard build-essential on Linux)

## Desktop develop / build

```bash
npm install
npm run tauri dev
npm run tauri build
```

## Android

See [android/README.md](android/README.md). Open the `android/` directory in Android Studio, grant **Usage access**, then run the app.

## Data location

- Desktop: OS app data directory for NetPulse (for example `%APPDATA%\com.saeedshamc.netpulse\netpulse.db` on Windows)
- Android: app-private `netpulse.db`

## License

See [LICENSE](LICENSE).
