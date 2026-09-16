# NetPulse

**فارسی** · [English](README.md)

مانیتور ترافیک **فقط همین دستگاه** — هر اپ، هر اینترفیس، هر شبکه وای‌فای، با تاریخچهٔ محلی روی دیسک.

**شعار:** هر بایت، هر اپ، هر شبکه — محلی و مال خودت.

---

## چه چیزی می‌دهد؟

| قابلیت | توضیح |
|---|---|
| per-app / per-process | بایت ارسالی و دریافتی + نرخ لحظه‌ای |
| per-interface | وای‌فای، اترنت، VPN، هات‌اسپات، موبایل، بلوتوث |
| per-SSID | ترافیک بر اساس شبکه وای‌فایی که وصل بوده‌اید |
| تاریخچه | تجمیع ساعتی / روزانه / ماهانه در SQLite محلی |
| خروجی | CSV و JSON |
| حریم خصوصی | بدون کلود، بدون اسکن LAN، بدون دستگاه‌های دیگر |

---

## پلتفرم‌ها

| پلتفرم | محصول | منبع داده | بستهٔ نهایی برای کاربر |
|---|---|---|---|
| **ویندوز** | دسکتاپ (Tauri) | IP Helper + TCP ESTATS | نصب‌کنندهٔ NSIS ‏(`.exe`) و `.msi` |
| **لینوکس** | دسکتاپ (Tauri) | `/proc/net/dev` + inet_diag | `.AppImage` و `.deb` |
| **مک** | دسکتاپ (Tauri) | `nettop` + `getifaddrs` | `.dmg` و `.app` |
| **اندروید** | Kotlin / Compose | `NetworkStatsManager` | `.apk` / `.aab` |
| **iOS** | فاز بعد | Packet Tunnel Provider | — |

**اینستالر خودکفا:** کسی که NetPulse را *نصب* می‌کند به Node.js، Rust، اندروید استودیو یا دانلود جداگانهٔ ران‌تایم از اینترنت نیاز ندارد (در ویندوز بوت‌استرپ WebView2 داخل خود نصب‌کننده است). ابزار بیلد فقط روی **ماشین سازنده** لازم است.

---

## لینک‌های سریع

- [راهنمای بیلد (فارسی)](docs/fa/BUILD.md) — ساخت اینستالر برای همهٔ پلتفرم‌ها  
- [راهنمای نصب (فارسی)](docs/fa/INSTALL.md) — خروجی کجاست و چطور نصب می‌شود  
- [Build guide (English)](docs/en/BUILD.md)  
- [Install guide (English)](docs/en/INSTALL.md)  
- [یادداشت اندروید](android/README.md)

---

## توسعه (دسکتاپ)

```bash
npm install
npm run desktop:dev
```

## ساخت اینستالر (اسکریپت‌های کمکی در `/scripts`)

| پلتفرم | اسکریپت |
|---|---|
| ویندوز | `scripts/build-windows.ps1` |
| لینوکس | `scripts/build-linux.sh` |
| مک | `scripts/build-macos.sh` |
| اندروید | `scripts/build-android.ps1` / `scripts/build-android.sh` |

یا روی همان OS:

```bash
npm run desktop:build
```

خروجی دسکتاپ: `src-tauri/target/release/bundle/`  
خروجی اندروید: `android/app/build/outputs/`

---

## داده محلی می‌ماند

- **دسکتاپ:** پوشهٔ app-data سیستم (مثلاً `%APPDATA%\com.saeedshamc.netpulse\netpulse.db` در ویندوز)
- **اندروید:** `netpulse.db` خصوصی اپ

---

## مجوز

[LICENSE](LICENSE)
