'use client';

import { useQuery } from '@tanstack/react-query';
import { events } from '@/lib/api-client';

export function useEvents(params?: {
  device_id?: string;
  enrollment_id?: string;
  type?: string;
  category?: string;
  severity?: string;
  from?: string;
  to?: string;
  page?: number;
  per_page?: number;
}) {
  return useQuery({
    queryKey: ['events', params],
    queryFn: () => events.list(params),
  });
}

export function useEventSummary(params?: {
  enrollment_id?: string;
  device_id?: string;
  period?: 'hour' | 'day' | 'week' | 'month';
  from?: string;
  to?: string;
}) {
  return useQuery({
    queryKey: ['events', 'summary', params],
    queryFn: () => events.summary(params),
  });
}
