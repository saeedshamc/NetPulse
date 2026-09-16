# NetPulse Android

Standalone Kotlin + Jetpack Compose app using `NetworkStatsManager` for this device only.

## Features

- Per-app usage (requires Usage Access)
- Per-interface totals (Wi-Fi / mobile)
- Current SSID tagging for Wi-Fi
- Hourly / daily history views
- CSV / JSON export
- Local SQLite with the same conceptual schema as the desktop app

## Requirements

- Android Studio Ladybug+ / AGP 8.7
- Android 8.0 (API 26)+
- User must manually grant **Usage access** in system settings (`PACKAGE_USAGE_STATS`)

## Build

```bash
cd android
./gradlew :app:assembleDebug
```

On Windows:

```bat
cd android
gradlew.bat :app:assembleDebug
```

Open the `android/` folder in Android Studio for the simplest workflow.
