# راهنمای بیلد — NetPulse

از داخل همین ریپو، فایل **اجرایی / اینستالر خودکفا** برای هر پلتفرم بسازید.

> کاربر نهایی که بسته را نصب می‌کند به Node، Rust یا دانلود جداگانهٔ ران‌تایم نیاز ندارد.  
> این ابزارها فقط روی **ماشین بیلد** لازم‌اند.

همچنین: [راهنمای نصب](INSTALL.md) · [English](../en/BUILD.md)

---

## ۱. ابزار یک‌باره روی ماشین بیلد

| ابزار | ویندوز | لینوکس | مک | اندروید |
|---|---|---|---|---|
| Node.js 18+ | بله | بله | بله | — |
| Rust (stable) | بله | بله | بله | — |
| MSVC Build Tools | بله | — | — | — |
| وابستگی‌های Tauri لینوکس | — | بله | — | — |
| Xcode / CLT | — | — | بله | — |
| Android Studio / SDK / JDK 17 | — | — | — | بله |

نمونه پکیج لینوکس (اوبونتو/دبیان):

```bash
sudo apt update
sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

---

## ۲. اسکریپت‌های کمکی (پیشنهادی)

همه داخل [`scripts/`](../../scripts/) هستند.

### ویندوز → NSIS ‏(`.exe`) + MSI

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\build-windows.ps1
```

**خروجی معمول:**

- `src-tauri/target/release/bundle/nsis/NetPulse_*_x64-setup.exe`
- `src-tauri/target/release/bundle/msi/NetPulse_*_x64_en-US.msi`

نصب‌کنندهٔ NSIS بوت‌استرپ **WebView2 را داخل خودش دارد**؛ معمولاً کاربر نهایی نیازی به دانلود جداگانه ندارد.

### لینوکس → AppImage + deb

```bash
chmod +x scripts/build-linux.sh
./scripts/build-linux.sh
```

**خروجی معمول:**

- `src-tauri/target/release/bundle/appimage/NetPulse_*.AppImage` ← یک فایل پرتابل
- `src-tauri/target/release/bundle/deb/NetPulse_*.deb`

### مک → DMG + app

```bash
chmod +x scripts/build-macos.sh
./scripts/build-macos.sh
```

**خروجی معمول:**

- `src-tauri/target/release/bundle/dmg/NetPulse_*.dmg`
- `src-tauri/target/release/bundle/macos/NetPulse.app`

### اندروید → APK + AAB

یک‌بار پوشهٔ `android/` را در Android Studio باز کنید تا Gradle Wrapper ساخته شود، بعد:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\build-android.ps1
```

```bash
chmod +x scripts/build-android.sh
./scripts/build-android.sh
```

**خروجی معمول:**

- `android/app/build/outputs/apk/release/app-release.apk`
- `android/app/build/outputs/bundle/release/app-release.aab`

---

## ۳. میان‌بر npm (فقط همان OS فعلی)

```bash
npm install
npm run desktop:build
npm run desktop:build:windows
npm run desktop:build:linux
npm run desktop:build:macos
```

کراس‌کامپایل (مثلاً ساخت AppImage لینوکس روی ویندوز) با این اسکریپت‌ها پشتیبانی نمی‌شود — هر دسکتاپ را روی همان OS بیلد کنید.

---

## ۴. معنای «خودکفا» در این پروژه

| مخاطب | نیاز |
|---|---|
| **کاربر نهایی** | فقط فایل اینستالر / AppImage / APK خروجی همین پروژه |
| **توسعه‌دهنده / CI** | یک‌بار Node + Rust (+ SDKهای OS) برای اجرای اسکریپت‌ها |

اینستالر نهایی از کاربر نمی‌خواهد ریپو را کلون کند یا `npm install` بزند.

---

## ۵. امضا (اختیاری ولی توصیه‌شده برای انتشار عمومی)

- **ویندوز:** Authenticode روی NSIS/MSI  
- **مک:** Developer ID + Notarization  
- **اندروید:** keystore انتشار در Android Studio  

بیلد بدون امضا برای تست محلی کافی است؛ سیستم‌عامل ممکن است هشدار بدهد.
