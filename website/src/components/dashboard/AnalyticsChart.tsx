"use client";

import {
  ResponsiveContainer,
  AreaChart,
  Area,
  XAxis,
  YAxis,
  Tooltip,
  CartesianGrid,
} from "recharts";

interface DataPoint {
  date: string;
  count: number;
}

interface AnalyticsChartProps {
  data: DataPoint[];
}

export function AnalyticsChart({ data }: AnalyticsChartProps) {
  if (data.length === 0) {
    return (
      <div className="h-64 flex items-center justify-center text-muted-foreground">
        No download data yet.
      </div>
    );
  }

  const formatted = data.map((d) => ({
    ...d,
    label: new Date(d.date).toLocaleDateString("en-US", { month: "short", day: "numeric" }),
  }));

  return (
    <ResponsiveContainer width="100%" height={300}>
      <AreaChart data={formatted}>
        <defs>
          <linearGradient id="colorCount" x1="0" y1="0" x2="0" y2="1">
            <stop offset="5%" stopColor="#6d5bef" stopOpacity={0.3} />
            <stop offset="95%" stopColor="#6d5bef" stopOpacity={0} />
          </linearGradient>
        </defs>
        <CartesianGrid strokeDasharray="3 3" stroke="#27272a" />
        <XAxis
          dataKey="label"
          stroke="#71717a"
          fontSize={12}
          tickLine={false}
          axisLine={false}
        />
        <YAxis
          stroke="#71717a"
          fontSize={12}
          tickLine={false}
          axisLine={false}
          allowDecimals={false}
        />
        <Tooltip
          contentStyle={{
            backgroundColor: "#111113",
            border: "1px solid #27272a",
            borderRadius: "8px",
            fontSize: 13,
          }}
          labelStyle={{ color: "#fafafa" }}
          itemStyle={{ color: "#6d5bef" }}
        />
        <Area
          type="monotone"
          dataKey="count"
          stroke="#6d5bef"
          strokeWidth={2}
          fillOpacity={1}
          fill="url(#colorCount)"
          name="Downloads"
        />
      </AreaChart>
    </ResponsiveContainer>
  );
}
