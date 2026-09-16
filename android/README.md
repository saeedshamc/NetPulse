# NetPulse Android

**English** · [فارسی](#فارسی)

Standalone Kotlin + Jetpack Compose app using `NetworkStatsManager` (this device only).

## Features

- Per-app usage (requires **Usage access**)
- Per-interface totals (Wi-Fi / mobile) + current SSID
- Hourly / daily history, CSV / JSON export
- Local SQLite matching the desktop data model

## Build release packages

Preferred (from repo root, after Gradle Wrapper exists):

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\build-android.ps1
```

```bash
chmod +x scripts/build-android.sh && ./scripts/build-android.sh
```

Or open `android/` in Android Studio → **Build > Generate Signed Bundle / APK**.

Outputs:

- `app/build/outputs/apk/release/*.apk` — direct install
- `app/build/outputs/bundle/release/*.aab` — Play Store

The APK/AAB is self-contained. Users do not install Node/Rust. They only grant the system **Usage access** permission.

---

## فارسی

اپ اندروید جدا با Compose و `NetworkStatsManager` — فقط ترافیک همین گوشی.

### ساخت پکیج انتشار

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\build-android.ps1
```

یا Android Studio → Generate Signed Bundle / APK.

خروجی APK برای نصب مستقیم و AAB برای پلی‌استور است؛ خودکفاست و کاربر به Node/Rust نیاز ندارد. فقط مجوز **Usage access** سیستم را بدهد.
