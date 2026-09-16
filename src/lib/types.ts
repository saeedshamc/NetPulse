export type Protocol = "tcp" | "udp" | "mixed";

export type InterfaceType =
  | "wifi"
  | "ethernet"
  | "hotspot"
  | "vpn"
  | "bluetooth"
  | "other";

export interface AppTraffic {
  pid: number;
  name: string;
  executable_path: string | null;
  bytes_sent: number;
  bytes_received: number;
  bytes_sent_rate: number;
  bytes_received_rate: number;
  protocol: Protocol;
}

export interface InterfaceTraffic {
  name: string;
  interface_type: InterfaceType;
  index: number;
  bytes_sent: number;
  bytes_received: number;
  bytes_sent_rate: number;
  bytes_received_rate: number;
  ssid: string | null;
}

export interface MonitorSnapshot {
  timestamp_ms: number;
  apps: AppTraffic[];
  interfaces: InterfaceTraffic[];
  current_ssid: string | null;
  privilege_note: string | null;
}

export interface HourlyAggregate {
  hour_start_ms: number;
  interface_name: string | null;
  application_name: string | null;
  network_ssid: string | null;
  bytes_sent: number;
  bytes_received: number;
}

export interface DailyAggregate {
  day_start_ms: number;
  interface_name: string | null;
  application_name: string | null;
  network_ssid: string | null;
  bytes_sent: number;
  bytes_received: number;
}

export function formatBytes(n: number): string {
  if (!Number.isFinite(n) || n <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let v = n;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i += 1;
  }
  return `${v.toFixed(v >= 10 || i === 0 ? 0 : 1)} ${units[i]}`;
}

export function formatRate(n: number): string {
  return `${formatBytes(n)}/s`;
}
