# Roadmap / نقشه راه

**English** · [فارسی](#فارسی)

## Done (MVP+)

- Desktop monitors: Windows / Linux / macOS
- Android app with NetworkStatsManager
- Live apps + interfaces + SSID
- Local SQLite history + CSV/JSON export
- Self-contained installer scripts
- Bilingual docs + UI (EN/FA)
- Pause/resume monitoring, app search, history scope (apps vs interfaces)
- System tray, close-to-tray, start with OS
- Settings: poll interval, light/dark theme
- Data retention (auto-prune raw samples)
- Daily usage alerts (desktop notification)
- GitHub Actions CI (frontend + desktop bundles + Android)
- Android home-screen widget (today / month usage)

## Next improvements (suggested)

1. **Top talkers dashboard** — sparkline per app for the last hour
2. **ETW (Windows) / eBPF (Linux)** — higher accuracy than socket polling
3. **Signed releases** — code-signing certificates in CI
4. **iOS phase** — Packet Tunnel Provider (separate architecture)

---

## فارسی

### انجام‌شده
مانیتور دسکتاپ سه OS، اپ اندروید، تاریخچه محلی، خروجی، اینستالر، UI دوزبانه، مکث مانیتور، جستجو، اسکوپ تاریخچه، ترِی و استارت با OS، تنظیمات تم و poll، retention، هشدار مصرف، CI، ویجت اندروید.

### پیشنهاد بعدی
داشبورد Top talkers، دقت بالاتر با ETW/eBPF، امضای انتشار در CI، فاز iOS.
