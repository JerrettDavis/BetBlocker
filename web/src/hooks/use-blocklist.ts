'use client';

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { blocklist } from '@/lib/api-client';

export function useBlocklistEntries(params?: {
  search?: string;
  category?: string;
  source?: string;
  status?: string;
  page?: number;
  per_page?: number;
}) {
  return useQuery({
    queryKey: ['blocklist', 'entries', params],
    queryFn: () => blocklist.listEntries(params),
  });
}

export function useCreateBlocklistEntry() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: {
      domain?: string;
      pattern?: string;
      category: string;
      evidence_url?: string;
      tags?: string[];
      notes?: string;
    }) => blocklist.createEntry(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['blocklist'] });
    },
  });
}

export function useUpdateBlocklistEntry() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      id,
      data,
    }: {
      id: string;
      data: {
        category?: string;
        status?: string;
        tags?: string[];
        evidence_url?: string;
        notes?: string;
      };
    }) => blocklist.updateEntry(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['blocklist'] });
    },
  });
}

export function useDeleteBlocklistEntry() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => blocklist.deleteEntry(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['blocklist'] });
    },
  });
}

export function useReviewQueue(params?: {
  min_reports?: number;
  min_confidence?: number;
  sort?: string;
  page?: number;
  per_page?: number;
}) {
  return useQuery({
    queryKey: ['blocklist', 'review-queue', params],
    queryFn: () => blocklist.reviewQueue(params),
  });
}

export function useResolveReview() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      domain,
      data,
    }: {
      domain: string;
      data: { action: 'promote' | 'reject'; category?: string; tags?: string[]; notes?: string };
    }) => blocklist.resolveReview(domain, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['blocklist'] });
    },
  });
}
