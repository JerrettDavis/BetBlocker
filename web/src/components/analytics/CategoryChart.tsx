'use client';

import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Cell,
} from 'recharts';

interface DataPoint {
  event_type: string;
  event_count: number;
}

interface CategoryChartProps {
  data: DataPoint[] | undefined;
  isLoading?: boolean;
}

const COLORS = [
  '#3b82f6',
  '#f97316',
  '#ef4444',
  '#22c55e',
  '#a855f7',
  '#eab308',
  '#06b6d4',
];

/** Aggregates rows by event_type for use as category distribution. */
function aggregateByCategory(rows: DataPoint[]) {
  const map = new Map<string, number>();
  for (const row of rows) {
    map.set(row.event_type, (map.get(row.event_type) ?? 0) + row.event_count);
  }
  return Array.from(map.entries())
    .map(([name, value]) => ({ name, value }))
    .sort((a, b) => b.value - a.value);
}

export function CategoryChart({ data, isLoading }: CategoryChartProps) {
  if (isLoading) {
    return (
      <div className="flex h-48 items-center justify-center rounded-lg border border-dashed text-sm text-muted-foreground">
        Loading chart...
      </div>
    );
  }

  if (!data || data.length === 0) {
    return (
      <div className="flex h-48 items-center justify-center rounded-lg border border-dashed text-sm text-muted-foreground">
        No category data available.
      </div>
    );
  }

  const chartData = aggregateByCategory(data);

  return (
    <ResponsiveContainer width="100%" height={240}>
      <BarChart data={chartData} margin={{ top: 5, right: 20, left: 0, bottom: 5 }}>
        <CartesianGrid strokeDasharray="3 3" className="stroke-muted" />
        <XAxis dataKey="name" tick={{ fontSize: 10 }} />
        <YAxis tick={{ fontSize: 11 }} />
        <Tooltip />
        <Bar dataKey="value" name="Events" radius={[4, 4, 0, 0]}>
          {chartData.map((_, i) => (
            <Cell key={i} fill={COLORS[i % COLORS.length]} />
          ))}
        </Bar>
      </BarChart>
    </ResponsiveContainer>
  );
}
