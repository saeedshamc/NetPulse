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

## Next improvements (suggested)

1. **Tray + start with OS** — run minimized, keep collecting history
2. **Alerts** — notify when an app exceeds X GB / day on metered Wi-Fi
3. **Top talkers dashboard** — sparkline per app for the last hour
4. **Data retention policy** — auto-prune raw records older than N days
5. **ETW (Windows) / eBPF (Linux)** — higher accuracy than socket polling
6. **Signed releases + CI** — GitHub Actions build installers per OS
7. **Android widgets** — today / this month usage on home screen
8. **iOS phase** — Packet Tunnel Provider (separate architecture)

---

## فارسی

### انجام‌شده
مانیتور دسکتاپ سه OS، اپ اندروید، تاریخچه محلی، خروجی، اینستالر، UI دوزبانه، مکث مانیتور، جستجو، اسکوپ تاریخچه.

### پیشنهاد بعدی
ترِی و استارت با سیستم‌عامل، هشدار مصرف، داشبورد Top talkers، پاکسازی خودکار داده قدیمی، دقت بالاتر با ETW/eBPF، CI امضا شده، ویجت اندروید، فاز iOS.
