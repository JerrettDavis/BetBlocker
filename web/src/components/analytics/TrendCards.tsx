'use client';

import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Flame, Clock, TrendingUp } from 'lucide-react';

interface TrendEntry {
  id: number;
  device_id: number;
  metric_name: string;
  metric_value: Record<string, unknown>;
  computed_at: string;
  period_start: string;
  period_end: string;
}

interface TrendCardsProps {
  trends: TrendEntry[] | undefined;
  isLoading?: boolean;
}

function findTrend(trends: TrendEntry[], name: string): TrendEntry | undefined {
  return trends.find((t) => t.metric_name === name);
}

function formatMetricValue(value: Record<string, unknown>): string {
  if (typeof value === 'object' && value !== null) {
    // Handle common shapes: { value: number }, { count: number }, { days: number }
    const v = value['value'] ?? value['count'] ?? value['days'] ?? value['hours'];
    if (v !== undefined) return String(v);
    return JSON.stringify(value);
  }
  return String(value);
}

export function TrendCards({ trends, isLoading }: TrendCardsProps) {
  if (isLoading) {
    return (
      <div className="grid gap-4 md:grid-cols-3">
        {Array.from({ length: 3 }).map((_, i) => (
          <Card key={i}>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium text-muted-foreground">
                Loading...
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">--</div>
            </CardContent>
          </Card>
        ))}
      </div>
    );
  }

  const items = trends ?? [];

  const streak = findTrend(items, 'clean_streak_days');
  const peakHour = findTrend(items, 'peak_hour');
  const weeklyTrend = findTrend(items, 'weekly_block_trend');

  return (
    <div className="grid gap-4 md:grid-cols-3">
      <Card>
        <CardHeader className="flex flex-row items-center justify-between pb-2">
          <CardTitle className="text-sm font-medium">Clean Streak</CardTitle>
          <Flame className="h-4 w-4 text-orange-500" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold">
            {streak ? `${formatMetricValue(streak.metric_value)} days` : '--'}
          </div>
          <p className="text-xs text-muted-foreground mt-1">Days without bypass attempts</p>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex flex-row items-center justify-between pb-2">
          <CardTitle className="text-sm font-medium">Peak Activity Hour</CardTitle>
          <Clock className="h-4 w-4 text-blue-500" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold">
            {peakHour ? `${formatMetricValue(peakHour.metric_value)}:00` : '--'}
          </div>
          <p className="text-xs text-muted-foreground mt-1">Hour with most block events</p>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex flex-row items-center justify-between pb-2">
          <CardTitle className="text-sm font-medium">Weekly Trend</CardTitle>
          <TrendingUp className="h-4 w-4 text-green-500" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold">
            {weeklyTrend ? formatMetricValue(weeklyTrend.metric_value) : '--'}
          </div>
          <p className="text-xs text-muted-foreground mt-1">
            {weeklyTrend
              ? `as of ${new Date(weeklyTrend.computed_at).toLocaleDateString()}`
              : 'No trend data'}
          </p>
        </CardContent>
      </Card>

      {/* Render any additional unknown trends as generic cards */}
      {items
        .filter(
          (t) =>
            t.metric_name !== 'clean_streak_days' &&
            t.metric_name !== 'peak_hour' &&
            t.metric_name !== 'weekly_block_trend',
        )
        .slice(0, 3)
        .map((t) => (
          <Card key={t.id}>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium capitalize">
                {t.metric_name.replace(/_/g, ' ')}
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="text-xl font-bold">{formatMetricValue(t.metric_value)}</div>
              <p className="text-xs text-muted-foreground mt-1">
                {new Date(t.computed_at).toLocaleDateString()}
              </p>
            </CardContent>
          </Card>
        ))}
    </div>
  );
}
