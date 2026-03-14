'use client';

import { useQuery } from '@tanstack/react-query';
import { analyticsApi } from '@/lib/api-client';

export function useAnalyticsTimeseries(params: {
  device_id: number;
  period?: 'hourly' | 'daily';
  from: string;
  to: string;
}) {
  return useQuery({
    queryKey: ['analytics', 'timeseries', params],
    queryFn: () => analyticsApi.timeseries(params),
    enabled: !!params.device_id && !!params.from && !!params.to,
  });
}

export function useAnalyticsTrends(params: { device_id: number; metrics?: string }) {
  return useQuery({
    queryKey: ['analytics', 'trends', params],
    queryFn: () => analyticsApi.trends(params),
    enabled: !!params.device_id,
  });
}

export function useAnalyticsSummary(params: {
  device_id: number;
  from: string;
  to: string;
}) {
  return useQuery({
    queryKey: ['analytics', 'summary', params],
    queryFn: () => analyticsApi.summary(params),
    enabled: !!params.device_id && !!params.from && !!params.to,
  });
}

export function useAnalyticsHeatmap(params: {
  device_id: number;
  from: string;
  to: string;
}) {
  return useQuery({
    queryKey: ['analytics', 'heatmap', params],
    queryFn: () => analyticsApi.heatmap(params),
    enabled: !!params.device_id && !!params.from && !!params.to,
  });
}
