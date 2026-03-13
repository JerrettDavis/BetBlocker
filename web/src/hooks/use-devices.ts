'use client';

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { devices } from '@/lib/api-client';

export function useDevices(params?: {
  status?: string;
  platform?: string;
  page?: number;
  per_page?: number;
}) {
  return useQuery({
    queryKey: ['devices', params],
    queryFn: () => devices.list(params),
  });
}

export function useDevice(id: string) {
  return useQuery({
    queryKey: ['devices', id],
    queryFn: () => devices.get(id),
    enabled: !!id,
  });
}

export function useDeleteDevice() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, reason }: { id: string; reason?: string }) => devices.delete(id, reason),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['devices'] });
    },
  });
}
