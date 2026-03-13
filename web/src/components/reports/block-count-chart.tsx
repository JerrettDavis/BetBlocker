'use client';

import type { EventSummary } from '@/lib/api-types';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  ResponsiveContainer,
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
} from 'recharts';

interface BlockCountChartProps {
  summary: EventSummary | undefined;
  isLoading: boolean;
}

export function BlockCountChart({ summary, isLoading }: BlockCountChartProps) {
  if (isLoading || !summary) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Block Activity</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="h-64 flex items-center justify-center text-muted-foreground">
            Loading chart...
          </div>
        </CardContent>
      </Card>
    );
  }

  const chartData = summary.timeseries.map((point) => ({
    date: new Date(point.period_start).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
    }),
    blocks: point.blocks,
    bypass: point.bypass_attempts,
    tamper: point.tamper_events,
  }));

  return (
    <Card>
      <CardHeader>
        <CardTitle>Block Activity</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="h-64">
          <ResponsiveContainer width="100%" height="100%">
            <AreaChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" className="stroke-muted" />
              <XAxis dataKey="date" className="text-xs" />
              <YAxis className="text-xs" />
              <Tooltip />
              <Area
                type="monotone"
                dataKey="blocks"
                stackId="1"
                stroke="hsl(var(--primary))"
                fill="hsl(var(--primary))"
                fillOpacity={0.3}
                name="Blocks"
              />
              <Area
                type="monotone"
                dataKey="bypass"
                stackId="1"
                stroke="hsl(30 100% 50%)"
                fill="hsl(30 100% 50%)"
                fillOpacity={0.3}
                name="Bypass Attempts"
              />
              <Area
                type="monotone"
                dataKey="tamper"
                stackId="1"
                stroke="hsl(0 100% 50%)"
                fill="hsl(0 100% 50%)"
                fillOpacity={0.3}
                name="Tamper Events"
              />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      </CardContent>
    </Card>
  );
}
