'use client';

import { useState, useCallback } from 'react';
import { analyticsApi } from '@/lib/api-client';
import {
  useAnalyticsTimeseries,
  useAnalyticsTrends,
  useAnalyticsSummary,
  useAnalyticsHeatmap,
} from '@/hooks/use-analytics';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { TimeseriesChart } from '@/components/analytics/TimeseriesChart';
import { ActivityHeatmap } from '@/components/analytics/ActivityHeatmap';
import { TrendCards } from '@/components/analytics/TrendCards';
import { CategoryChart } from '@/components/analytics/CategoryChart';
import { Shield, Ban, AlertTriangle, Activity, Download } from 'lucide-react';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function toIso(date: Date): string {
  return date.toISOString();
}

function defaultDateRange() {
  const to = new Date();
  const from = new Date();
  from.setDate(from.getDate() - 30);
  return {
    from: from.toISOString().slice(0, 10),
    to: to.toISOString().slice(0, 10),
  };
}

function downloadBlob(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function AnalyticsDashboardPage() {
  const defaults = defaultDateRange();
  const [deviceIdInput, setDeviceIdInput] = useState('');
  const [fromDate, setFromDate] = useState(defaults.from);
  const [toDate, setToDate] = useState(defaults.to);
  const [deviceId, setDeviceId] = useState<number | null>(null);
  const [exportError, setExportError] = useState<string | null>(null);
  const [exporting, setExporting] = useState<'csv' | 'pdf' | null>(null);

  const from = toIso(new Date(fromDate));
  const to = toIso(new Date(toDate));

  const enabled = deviceId !== null && !!from && !!to;
  const queryParams = enabled ? { device_id: deviceId, from, to } : null;

  const { data: summaryData, isLoading: summaryLoading } = useAnalyticsSummary(
    queryParams ?? { device_id: 0, from, to },
  );
  const { data: timeseriesData, isLoading: timeseriesLoading } = useAnalyticsTimeseries(
    queryParams ? { ...queryParams, period: 'daily' } : { device_id: 0, period: 'daily', from, to },
  );
  const { data: trendsData, isLoading: trendsLoading } = useAnalyticsTrends(
    queryParams ?? { device_id: 0 },
  );
  const { data: heatmapData, isLoading: heatmapLoading } = useAnalyticsHeatmap(
    queryParams ?? { device_id: 0, from, to },
  );

  const summary = summaryData?.data;
  const timeseriesRows = timeseriesData?.data?.data ?? [];
  const trends = trendsData?.data?.trends ?? [];
  const heatmap = heatmapData?.data?.heatmap ?? [];

  const handleApply = useCallback(() => {
    const parsed = parseInt(deviceIdInput, 10);
    if (!isNaN(parsed) && parsed > 0) {
      setDeviceId(parsed);
    }
  }, [deviceIdInput]);

  const handleExportCsv = useCallback(async () => {
    if (!queryParams) return;
    setExportError(null);
    setExporting('csv');
    try {
      const blob = await analyticsApi.exportCsv(queryParams);
      downloadBlob(
        blob,
        `analytics_device${queryParams.device_id}_${fromDate}_${toDate}.csv`,
      );
    } catch (e) {
      setExportError(e instanceof Error ? e.message : 'Export failed');
    } finally {
      setExporting(null);
    }
  }, [queryParams, fromDate, toDate]);

  const handleExportPdf = useCallback(async () => {
    if (!queryParams) return;
    setExportError(null);
    setExporting('pdf');
    try {
      const blob = await analyticsApi.exportPdf(queryParams);
      downloadBlob(
        blob,
        `analytics_device${queryParams.device_id}_${fromDate}_${toDate}.pdf`,
      );
    } catch (e) {
      setExportError(e instanceof Error ? e.message : 'Export failed');
    } finally {
      setExporting(null);
    }
  }, [queryParams, fromDate, toDate]);

  return (
    <div className="space-y-6">
      {/* Page header */}
      <div className="flex items-start justify-between flex-wrap gap-4">
        <div>
          <h1 className="text-2xl font-bold">Analytics Dashboard</h1>
          <p className="text-sm text-muted-foreground">
            Deep-dive into block activity, trends and heatmaps for a specific device.
          </p>
        </div>

        {/* Export buttons */}
        {enabled && (
          <div className="flex items-center gap-2">
            <button
              onClick={handleExportCsv}
              disabled={exporting !== null}
              className="inline-flex items-center gap-1.5 rounded-md border border-input bg-background px-3 py-2 text-sm font-medium shadow-sm hover:bg-accent disabled:opacity-50"
            >
              <Download className="h-4 w-4" />
              {exporting === 'csv' ? 'Exporting...' : 'Export CSV'}
            </button>
            <button
              onClick={handleExportPdf}
              disabled={exporting !== null}
              className="inline-flex items-center gap-1.5 rounded-md border border-input bg-background px-3 py-2 text-sm font-medium shadow-sm hover:bg-accent disabled:opacity-50"
            >
              <Download className="h-4 w-4" />
              {exporting === 'pdf' ? 'Exporting...' : 'Export PDF'}
            </button>
          </div>
        )}
      </div>

      {/* Filters */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-sm">Filters</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex flex-wrap gap-4 items-end">
            <div className="flex flex-col gap-1">
              <label className="text-xs text-muted-foreground" htmlFor="device-id">
                Device ID
              </label>
              <input
                id="device-id"
                type="number"
                min={1}
                value={deviceIdInput}
                onChange={(e) => setDeviceIdInput(e.target.value)}
                placeholder="e.g. 42"
                className="rounded-md border border-input bg-background px-3 py-1.5 text-sm w-32 focus:outline-none focus:ring-1 focus:ring-ring"
              />
            </div>

            <div className="flex flex-col gap-1">
              <label className="text-xs text-muted-foreground" htmlFor="from-date">
                From
              </label>
              <input
                id="from-date"
                type="date"
                value={fromDate}
                onChange={(e) => setFromDate(e.target.value)}
                className="rounded-md border border-input bg-background px-3 py-1.5 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
              />
            </div>

            <div className="flex flex-col gap-1">
              <label className="text-xs text-muted-foreground" htmlFor="to-date">
                To
              </label>
              <input
                id="to-date"
                type="date"
                value={toDate}
                onChange={(e) => setToDate(e.target.value)}
                className="rounded-md border border-input bg-background px-3 py-1.5 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
              />
            </div>

            <button
              onClick={handleApply}
              className="rounded-md bg-primary px-4 py-1.5 text-sm font-medium text-primary-foreground hover:bg-primary/90"
            >
              Apply
            </button>
          </div>

          {exportError && (
            <p className="mt-2 text-sm text-destructive">{exportError}</p>
          )}
        </CardContent>
      </Card>

      {/* Prompt if no device selected */}
      {!enabled && (
        <div className="flex h-40 items-center justify-center rounded-lg border border-dashed text-sm text-muted-foreground">
          Enter a device ID and date range above to load analytics.
        </div>
      )}

      {enabled && (
        <>
          {/* Summary stats */}
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
            <StatCard
              label="Total Events"
              value={summary?.total_events}
              icon={<Activity className="h-4 w-4 text-muted-foreground" />}
              loading={summaryLoading}
            />
            <StatCard
              label="Total Blocks"
              value={summary?.total_blocks}
              icon={<Shield className="h-4 w-4 text-primary" />}
              loading={summaryLoading}
            />
            <StatCard
              label="Bypass Attempts"
              value={summary?.total_bypass_attempts}
              icon={<Ban className="h-4 w-4 text-orange-500" />}
              loading={summaryLoading}
              valueClass="text-orange-500"
            />
            <StatCard
              label="Tamper Events"
              value={summary?.total_tamper_events}
              icon={<AlertTriangle className="h-4 w-4 text-red-500" />}
              loading={summaryLoading}
              valueClass="text-red-500"
            />
          </div>

          {/* Trend cards */}
          <section>
            <h2 className="text-base font-semibold mb-3">Trends</h2>
            <TrendCards trends={trends} isLoading={trendsLoading} />
          </section>

          {/* Timeseries chart */}
          <Card>
            <CardHeader>
              <CardTitle className="text-sm">Daily Block Activity</CardTitle>
            </CardHeader>
            <CardContent>
              <TimeseriesChart data={timeseriesRows} isLoading={timeseriesLoading} />
            </CardContent>
          </Card>

          {/* Category distribution */}
          <Card>
            <CardHeader>
              <CardTitle className="text-sm">Event Type Distribution</CardTitle>
            </CardHeader>
            <CardContent>
              <CategoryChart data={timeseriesRows} isLoading={timeseriesLoading} />
            </CardContent>
          </Card>

          {/* Activity heatmap */}
          <Card>
            <CardHeader>
              <CardTitle className="text-sm">Activity Heatmap (Hour of Day × Day of Week)</CardTitle>
            </CardHeader>
            <CardContent>
              <ActivityHeatmap data={heatmap} isLoading={heatmapLoading} />
            </CardContent>
          </Card>
        </>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// StatCard helper
// ---------------------------------------------------------------------------

interface StatCardProps {
  label: string;
  value: number | undefined;
  icon: React.ReactNode;
  loading: boolean;
  valueClass?: string;
}

function StatCard({ label, value, icon, loading, valueClass }: StatCardProps) {
  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between pb-2">
        <CardTitle className="text-sm font-medium">{label}</CardTitle>
        {icon}
      </CardHeader>
      <CardContent>
        <div className={`text-2xl font-bold ${valueClass ?? ''}`}>
          {loading || value === undefined ? '--' : value.toLocaleString()}
        </div>
      </CardContent>
    </Card>
  );
}
