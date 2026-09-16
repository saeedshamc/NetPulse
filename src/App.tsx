import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { UsageChart, type ChartPoint } from "./components/UsageChart";
import {
  formatBytes,
  formatRate,
  type DailyAggregate,
  type HourlyAggregate,
  type MonitorSnapshot,
} from "./lib/types";

type Tab = "apps" | "interfaces" | "history" | "export";
type HistoryGrain = "hourly" | "daily" | "monthly";

export default function App() {
  const [tab, setTab] = useState<Tab>("apps");
  const [snapshot, setSnapshot] = useState<MonitorSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [ssids, setSsids] = useState<string[]>([]);
  const [interfaces, setInterfaces] = useState<string[]>([]);
  const [filterSsid, setFilterSsid] = useState("");
  const [filterIface, setFilterIface] = useState("");
  const [grain, setGrain] = useState<HistoryGrain>("daily");
  const [chartData, setChartData] = useState<ChartPoint[]>([]);
  const [exportText, setExportText] = useState("");
  const [exportFormat, setExportFormat] = useState<"csv" | "json">("csv");

  useEffect(() => {
    let unlistenUpdate: (() => void) | undefined;
    let unlistenError: (() => void) | undefined;

    (async () => {
      try {
        const latest = await invoke<MonitorSnapshot | null>("get_latest_snapshot");
        if (latest) setSnapshot(latest);
      } catch {
        // first poll may not be ready
      }

      unlistenUpdate = await listen<MonitorSnapshot>("traffic://update", (event) => {
        setSnapshot(event.payload);
        setError(null);
      });
      unlistenError = await listen<string>("traffic://error", (event) => {
        setError(event.payload);
      });
    })();

    return () => {
      unlistenUpdate?.();
      unlistenError?.();
    };
  }, []);

  useEffect(() => {
    if (tab !== "history" && tab !== "export") return;
    (async () => {
      try {
        setSsids(await invoke<string[]>("list_ssids"));
        setInterfaces(await invoke<string[]>("list_interfaces"));
      } catch (e) {
        setError(String(e));
      }
    })();
  }, [tab]);

  useEffect(() => {
    if (tab !== "history") return;
    const now = Date.now();
    const from =
      grain === "hourly"
        ? now - 48 * 3600_000
        : grain === "daily"
          ? now - 30 * 86_400_000
          : now - 365 * 86_400_000;

    (async () => {
      try {
        if (grain === "hourly") {
          const rows = await invoke<HourlyAggregate[]>("get_hourly_history", {
            fromMs: from,
            toMs: now,
            ssid: filterSsid || null,
            interfaceName: filterIface || null,
          });
          setChartData(aggregateChart(rows.map((r) => ({
            t: r.hour_start_ms,
            sent: r.bytes_sent,
            received: r.bytes_received,
          })), "hour"));
        } else {
          const rows = await invoke<DailyAggregate[]>("get_daily_history", {
            fromMs: from,
            toMs: now,
            ssid: filterSsid || null,
            interfaceName: filterIface || null,
          });
          const points = rows.map((r) => ({
            t: r.day_start_ms,
            sent: r.bytes_sent,
            received: r.bytes_received,
          }));
          setChartData(
            grain === "monthly" ? aggregateChart(points, "month") : aggregateChart(points, "day"),
          );
        }
      } catch (e) {
        setError(String(e));
      }
    })();
  }, [tab, grain, filterSsid, filterIface, snapshot?.timestamp_ms]);

  const apps = snapshot?.apps ?? [];
  const ifaces = snapshot?.interfaces ?? [];

  const totals = useMemo(() => {
    const sent = apps.reduce((s, a) => s + a.bytes_sent_rate, 0);
    const recv = apps.reduce((s, a) => s + a.bytes_received_rate, 0);
    return { sent, recv };
  }, [apps]);

  async function runExport() {
    const now = Date.now();
    const from = now - 90 * 86_400_000;
    try {
      const text = await invoke<string>("export_usage", {
        format: exportFormat,
        fromMs: from,
        toMs: now,
      });
      setExportText(text);
    } catch (e) {
      setError(String(e));
    }
  }

  function downloadExport() {
    if (!exportText) return;
    const blob = new Blob([exportText], {
      type: exportFormat === "json" ? "application/json" : "text/csv",
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `netpulse-export.${exportFormat}`;
    a.click();
    URL.revokeObjectURL(url);
  }

  return (
    <div className="flex h-screen flex-col">
      <header className="flex items-end justify-between border-b border-[var(--line)] px-6 pb-4 pt-5">
        <div>
          <p className="text-xs tracking-[0.2em] text-[var(--muted)] uppercase">Local traffic</p>
          <h1
            className="mt-1 text-3xl font-semibold tracking-tight text-[var(--accent)]"
            style={{ fontFamily: "var(--font-display)" }}
          >
            NetPulse
          </h1>
          <p className="mt-1 max-w-xl text-sm text-[var(--muted)]">
            Every byte, every app, every network — local and yours.
          </p>
        </div>
        <div className="text-right text-sm text-[var(--muted)]">
          <div>
            Live:{" "}
            <span className="font-mono text-[var(--text)]">
              ↑ {formatRate(totals.sent)} · ↓ {formatRate(totals.recv)}
            </span>
          </div>
          <div className="mt-1">
            SSID:{" "}
            <span className="text-[var(--text)]">
              {snapshot?.current_ssid ?? "—"}
            </span>
          </div>
        </div>
      </header>

      {snapshot?.privilege_note && (
        <div className="border-b border-[var(--line)] bg-[#2a2414] px-6 py-2 text-sm text-[var(--warn)]">
          {snapshot.privilege_note}
        </div>
      )}
      {error && (
        <div className="border-b border-[var(--line)] bg-[#2a1414] px-6 py-2 text-sm text-red-300">
          {error}
        </div>
      )}

      <nav className="flex gap-1 border-b border-[var(--line)] px-4 pt-3">
        {(
          [
            ["apps", "Apps"],
            ["interfaces", "Interfaces"],
            ["history", "History"],
            ["export", "Export"],
          ] as const
        ).map(([id, label]) => (
          <button
            key={id}
            onClick={() => setTab(id)}
            className={`rounded-t-md px-4 py-2 text-sm transition ${
              tab === id
                ? "bg-[var(--panel)] text-[var(--accent)]"
                : "text-[var(--muted)] hover:text-[var(--text)]"
            }`}
          >
            {label}
          </button>
        ))}
      </nav>

      <main className="min-h-0 flex-1 overflow-auto bg-[var(--panel)]/60 p-4">
        {tab === "apps" && (
          <table className="w-full border-collapse text-left text-sm">
            <thead className="sticky top-0 bg-[var(--panel)] text-[var(--muted)]">
              <tr>
                <th className="px-3 py-2 font-medium">Application</th>
                <th className="px-3 py-2 font-medium">PID</th>
                <th className="px-3 py-2 font-medium">Sent (session)</th>
                <th className="px-3 py-2 font-medium">Received (session)</th>
                <th className="px-3 py-2 font-medium">↑ rate</th>
                <th className="px-3 py-2 font-medium">↓ rate</th>
              </tr>
            </thead>
            <tbody>
              {apps.length === 0 && (
                <tr>
                  <td colSpan={6} className="px-3 py-8 text-center text-[var(--muted)]">
                    Waiting for traffic samples…
                  </td>
                </tr>
              )}
              {apps.map((app) => (
                <tr key={app.pid} className="border-t border-[var(--line)]/70 hover:bg-white/5">
                  <td className="px-3 py-2">
                    <div className="font-medium">{app.name}</div>
                    {app.executable_path && (
                      <div className="max-w-md truncate font-mono text-xs text-[var(--muted)]">
                        {app.executable_path}
                      </div>
                    )}
                  </td>
                  <td className="px-3 py-2 font-mono text-[var(--muted)]">{app.pid}</td>
                  <td className="px-3 py-2 font-mono">{formatBytes(app.bytes_sent)}</td>
                  <td className="px-3 py-2 font-mono">{formatBytes(app.bytes_received)}</td>
                  <td className="px-3 py-2 font-mono text-[var(--accent)]">
                    {formatRate(app.bytes_sent_rate)}
                  </td>
                  <td className="px-3 py-2 font-mono text-sky-300">
                    {formatRate(app.bytes_received_rate)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}

        {tab === "interfaces" && (
          <table className="w-full border-collapse text-left text-sm">
            <thead className="sticky top-0 bg-[var(--panel)] text-[var(--muted)]">
              <tr>
                <th className="px-3 py-2 font-medium">Interface</th>
                <th className="px-3 py-2 font-medium">Type</th>
                <th className="px-3 py-2 font-medium">SSID</th>
                <th className="px-3 py-2 font-medium">Sent (lifetime)</th>
                <th className="px-3 py-2 font-medium">Received (lifetime)</th>
                <th className="px-3 py-2 font-medium">↑ rate</th>
                <th className="px-3 py-2 font-medium">↓ rate</th>
              </tr>
            </thead>
            <tbody>
              {ifaces.map((iface) => (
                <tr
                  key={iface.index}
                  className="border-t border-[var(--line)]/70 hover:bg-white/5"
                >
                  <td className="px-3 py-2 font-medium">{iface.name}</td>
                  <td className="px-3 py-2 capitalize text-[var(--muted)]">
                    {iface.interface_type}
                  </td>
                  <td className="px-3 py-2">{iface.ssid ?? "—"}</td>
                  <td className="px-3 py-2 font-mono">{formatBytes(iface.bytes_sent)}</td>
                  <td className="px-3 py-2 font-mono">{formatBytes(iface.bytes_received)}</td>
                  <td className="px-3 py-2 font-mono text-[var(--accent)]">
                    {formatRate(iface.bytes_sent_rate)}
                  </td>
                  <td className="px-3 py-2 font-mono text-sky-300">
                    {formatRate(iface.bytes_received_rate)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}

        {tab === "history" && (
          <div className="space-y-4">
            <div className="flex flex-wrap items-center gap-3">
              <select
                value={grain}
                onChange={(e) => setGrain(e.target.value as HistoryGrain)}
                className="rounded border border-[var(--line)] bg-[var(--bg)] px-3 py-2 text-sm"
              >
                <option value="hourly">Hourly</option>
                <option value="daily">Daily</option>
                <option value="monthly">Monthly</option>
              </select>
              <select
                value={filterSsid}
                onChange={(e) => setFilterSsid(e.target.value)}
                className="rounded border border-[var(--line)] bg-[var(--bg)] px-3 py-2 text-sm"
              >
                <option value="">All SSIDs</option>
                {ssids.map((s) => (
                  <option key={s} value={s}>
                    {s}
                  </option>
                ))}
              </select>
              <select
                value={filterIface}
                onChange={(e) => setFilterIface(e.target.value)}
                className="rounded border border-[var(--line)] bg-[var(--bg)] px-3 py-2 text-sm"
              >
                <option value="">All interfaces</option>
                {interfaces.map((s) => (
                  <option key={s} value={s}>
                    {s}
                  </option>
                ))}
              </select>
            </div>
            <UsageChart data={chartData} />
            {chartData.length === 0 && (
              <p className="text-sm text-[var(--muted)]">
                No history yet. Keep NetPulse running to accumulate usage.
              </p>
            )}
          </div>
        )}

        {tab === "export" && (
          <div className="mx-auto max-w-3xl space-y-4">
            <p className="text-sm text-[var(--muted)]">
              Export the last 90 days of hourly aggregates. Data never leaves this machine unless
              you save the file.
            </p>
            <div className="flex flex-wrap gap-3">
              <select
                value={exportFormat}
                onChange={(e) => setExportFormat(e.target.value as "csv" | "json")}
                className="rounded border border-[var(--line)] bg-[var(--bg)] px-3 py-2 text-sm"
              >
                <option value="csv">CSV</option>
                <option value="json">JSON</option>
              </select>
              <button
                onClick={runExport}
                className="rounded bg-[var(--accent-dim)] px-4 py-2 text-sm font-medium text-white hover:bg-[var(--accent)]"
              >
                Generate
              </button>
              <button
                onClick={downloadExport}
                disabled={!exportText}
                className="rounded border border-[var(--line)] px-4 py-2 text-sm disabled:opacity-40"
              >
                Download
              </button>
            </div>
            <textarea
              readOnly
              value={exportText}
              placeholder="Export preview appears here"
              className="h-80 w-full rounded border border-[var(--line)] bg-[var(--bg)] p-3 font-mono text-xs"
            />
          </div>
        )}
      </main>
    </div>
  );
}

function aggregateChart(
  rows: { t: number; sent: number; received: number }[],
  mode: "hour" | "day" | "month",
): ChartPoint[] {
  const map = new Map<string, { sent: number; received: number; t: number }>();
  for (const row of rows) {
    const d = new Date(row.t);
    const key =
      mode === "hour"
        ? `${d.getMonth() + 1}/${d.getDate()} ${String(d.getHours()).padStart(2, "0")}:00`
        : mode === "day"
          ? `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`
          : `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}`;
    const cur = map.get(key) ?? { sent: 0, received: 0, t: row.t };
    cur.sent += row.sent;
    cur.received += row.received;
    map.set(key, cur);
  }
  return [...map.entries()]
    .sort((a, b) => a[1].t - b[1].t)
    .map(([label, v]) => ({ label, sent: v.sent, received: v.received }));
}
