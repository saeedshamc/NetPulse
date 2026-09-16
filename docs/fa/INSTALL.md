# راهنمای نصب — NetPulse

خروجی بیلد کجاست و کاربر نهایی چطور نصب می‌کند.

[راهنمای بیلد](BUILD.md) · [English](../en/INSTALL.md)

---

## ویندوز

1. با `scripts/build-windows.ps1` بیلد بگیرید (یا از CI کپی کنید).
2. به کاربر بدهید:
   - `NetPulse_*-setup.exe` (NSIS) — پیشنهادی؛ WebView2 داخلش است  
   - یا `NetPulse_*.msi`
3. دابل‌کلیک → نصب → اجرا از منوی Start با نام **NetPulse**.
4. اختیاری: اجرا با Administrator برای آمار کامل‌تر per-process.

روی سیستم کاربر **Node / Rust / Git لازم نیست**.

---

## لینوکس

1. با `scripts/build-linux.sh` بیلد بگیرید.
2. ترجیحاً **AppImage**:
   ```bash
   chmod +x NetPulse_*.AppImage
   ./NetPulse_*.AppImage
   ```
3. یا `.deb`:
   ```bash
   sudo dpkg -i NetPulse_*.deb
   ```

برای استفاده از خروجی، کاربر به Node/Rust نیاز ندارد.

---

## مک

1. روی مک با `scripts/build-macos.sh` بیلد بگیرید.
2. `.dmg` را باز کنید و NetPulse را به Applications بکشید.
3. اگر Gatekeeper جلوی اجرای بیلد بدون امضا را گرفت، از System Settings اجازه بدهید.
4. برای اعداد کامل per-app ممکن است دسترسی بالا (`nettop`) لازم باشد.

کاربر نهایی به Node/Rust نیاز ندارد.

---

## اندروید

1. APK/AAB را با اسکریپت اندروید یا Android Studio بسازید.
2. APK را سایدلود کنید یا AAB را در Play Store منتشر کنید.
3. داخل اپ، **Usage access** را از تنظیمات سیستم بدهید (مجوز سیستم‌عامل است، نه نرم‌افزار جانبی).

APK یک بستهٔ استاندارد و خودکفای اندروید است.

---

## بعد از نصب — محل داده

| پلتفرم | دیتابیس |
|---|---|
| ویندوز | `%APPDATA%\com.saeedshamc.netpulse\netpulse.db` |
| لینوکس | معمولاً `~/.local/share/com.saeedshamc.netpulse/netpulse.db` |
| مک | معمولاً `~/Library/Application Support/com.saeedshamc.netpulse/netpulse.db` |
| اندروید | `netpulse.db` خصوصی اپ |

حذف برنامه، اپ را برمی‌دارد؛ برای پاک شدن تاریخچه، دادهٔ اپ را هم پاک کنید.
