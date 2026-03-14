'use client';

import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from 'recharts';

interface DataPoint {
  timestamp: string;
  event_type: string;
  event_count: number;
}

interface TimeseriesChartProps {
  data: DataPoint[] | undefined;
  isLoading?: boolean;
}

/** Groups timeseries rows by timestamp, pivoting event_type into separate series. */
function transformData(rows: DataPoint[]) {
  const map = new Map<string, Record<string, number>>();

  for (const row of rows) {
    const label = row.timestamp.slice(0, 10); // YYYY-MM-DD
    const existing = map.get(label) ?? { date: label as unknown as number };
    existing[row.event_type] = (existing[row.event_type] ?? 0) + row.event_count;
    map.set(label, existing);
  }

  return Array.from(map.values());
}

const SERIES_COLORS: Record<string, string> = {
  block: '#3b82f6',
  bypass_attempt: '#f97316',
  tamper_detected: '#ef4444',
  tamper_self_healed: '#22c55e',
  vpn_detected: '#a855f7',
};

const DEFAULT_COLOR = '#94a3b8';

export function TimeseriesChart({ data, isLoading }: TimeseriesChartProps) {
  if (isLoading) {
    return (
      <div className="flex h-64 items-center justify-center rounded-lg border border-dashed text-sm text-muted-foreground">
        Loading chart...
      </div>
    );
  }

  if (!data || data.length === 0) {
    return (
      <div className="flex h-64 items-center justify-center rounded-lg border border-dashed text-sm text-muted-foreground">
        No timeseries data available.
      </div>
    );
  }

  const transformed = transformData(data);
  const eventTypes = Array.from(new Set(data.map((d) => d.event_type)));

  return (
    <ResponsiveContainer width="100%" height={300}>
      <LineChart data={transformed} margin={{ top: 5, right: 20, left: 0, bottom: 5 }}>
        <CartesianGrid strokeDasharray="3 3" className="stroke-muted" />
        <XAxis dataKey="date" tick={{ fontSize: 11 }} />
        <YAxis tick={{ fontSize: 11 }} />
        <Tooltip />
        <Legend />
        {eventTypes.map((type) => (
          <Line
            key={type}
            type="monotone"
            dataKey={type}
            stroke={SERIES_COLORS[type] ?? DEFAULT_COLOR}
            strokeWidth={2}
            dot={false}
          />
        ))}
      </LineChart>
    </ResponsiveContainer>
  );
}
