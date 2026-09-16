import {
  Area,
  AreaChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { formatBytes } from "../lib/types";

export interface ChartPoint {
  label: string;
  sent: number;
  received: number;
}

export function UsageChart({ data }: { data: ChartPoint[] }) {
  return (
    <div className="h-72 w-full">
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={data} margin={{ top: 8, right: 12, left: 0, bottom: 0 }}>
          <defs>
            <linearGradient id="sentFill" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="#3ecf8e" stopOpacity={0.45} />
              <stop offset="100%" stopColor="#3ecf8e" stopOpacity={0.02} />
            </linearGradient>
            <linearGradient id="recvFill" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="#6aa8ff" stopOpacity={0.4} />
              <stop offset="100%" stopColor="#6aa8ff" stopOpacity={0.02} />
            </linearGradient>
          </defs>
          <CartesianGrid stroke="#24352c" strokeDasharray="3 3" />
          <XAxis dataKey="label" stroke="#8fa399" tick={{ fill: "#8fa399", fontSize: 12 }} />
          <YAxis
            stroke="#8fa399"
            tick={{ fill: "#8fa399", fontSize: 12 }}
            tickFormatter={(v) => formatBytes(Number(v))}
          />
          <Tooltip
            contentStyle={{
              background: "#16221c",
              border: "1px solid #24352c",
              borderRadius: 8,
              color: "#e7f0ea",
            }}
            formatter={(value) => formatBytes(Number(value ?? 0))}
          />
          <Area
            type="monotone"
            dataKey="sent"
            name="Sent"
            stroke="#3ecf8e"
            fill="url(#sentFill)"
            strokeWidth={2}
          />
          <Area
            type="monotone"
            dataKey="received"
            name="Received"
            stroke="#6aa8ff"
            fill="url(#recvFill)"
            strokeWidth={2}
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}
