# NetPulse

Local per-application and per-interface network traffic monitor for your own machine.

**Tagline:** Every byte, every app, every network — local and yours.

## What it does

- Live per-process traffic (sent / received / rate)
- Per-network-interface breakdown (Wi-Fi, Ethernet, VPN, hotspot, Bluetooth)
- Per-SSID tagging while connected to Wi-Fi
- Hourly / daily / monthly history stored in a local SQLite database
- CSV and JSON export

NetPulse only reads what the OS reports about **this machine**. It does not scan the LAN, discover other devices, or send usage data to any server.

## Requirements

- Windows 10/11 (current MVP target)
- [Rust](https://www.rust-lang.org/tools/install)
- [Node.js](https://nodejs.org/) 18+
- Microsoft C++ Build Tools (for Tauri on Windows)

Administrator rights improve completeness of per-process TCP statistics; the UI shows a notice when elevated access may help.

## Develop

```bash
npm install
npm run tauri dev
```

## Build

```bash
npm run tauri build
```

## Data location

SQLite database is stored under the OS app data directory for NetPulse (for example `%APPDATA%\com.saeedshamc.netpulse\netpulse.db` on Windows).

## License

See [LICENSE](LICENSE).
