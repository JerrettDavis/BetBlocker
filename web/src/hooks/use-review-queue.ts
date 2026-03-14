'use client';

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { blocklist } from '@/lib/api-client';
import type { BlocklistCategory } from '@/lib/api-types';

export interface ReviewQueueFilters {
  status?: string;
  source?: string;
  min_confidence?: number;
  domain?: string;
  sort?: 'confidence' | 'date';
  page?: number;
  per_page?: number;
}

export function useReviewQueueList(filters?: ReviewQueueFilters) {
  return useQuery({
    queryKey: ['review-queue', filters],
    queryFn: () =>
      blocklist.reviewQueue({
        min_confidence: filters?.min_confidence,
        sort: filters?.sort,
        page: filters?.page,
        per_page: filters?.per_page,
      }),
  });
}

export function useReviewQueueItem(domain: string) {
  return useQuery({
    queryKey: ['review-queue', 'item', domain],
    queryFn: () =>
      blocklist.reviewQueue({ page: 1, per_page: 100 }).then((res) => {
        const item = res.data.find((i) => i.domain === domain);
        if (!item) throw new Error('Item not found');
        return item;
      }),
    enabled: !!domain,
  });
}

export function useApproveReviewItem() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      domain,
      category,
      tags,
      notes,
    }: {
      domain: string;
      category: BlocklistCategory;
      tags?: string[];
      notes?: string;
    }) => blocklist.resolveReview(domain, { action: 'promote', category, tags, notes }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['review-queue'] });
      queryClient.invalidateQueries({ queryKey: ['blocklist'] });
    },
  });
}

export function useRejectReviewItem() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ domain, notes }: { domain: string; notes?: string }) =>
      blocklist.resolveReview(domain, { action: 'reject', notes }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['review-queue'] });
      queryClient.invalidateQueries({ queryKey: ['blocklist'] });
    },
  });
}

export function useBulkApproveReviewItems() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      domains,
      category,
    }: {
      domains: string[];
      category: BlocklistCategory;
    }) =>
      Promise.all(
        domains.map((domain) =>
          blocklist.resolveReview(domain, { action: 'promote', category }),
        ),
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['review-queue'] });
      queryClient.invalidateQueries({ queryKey: ['blocklist'] });
    },
  });
}

export function useBulkRejectReviewItems() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (domains: string[]) =>
      Promise.all(
        domains.map((domain) => blocklist.resolveReview(domain, { action: 'reject' })),
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['review-queue'] });
      queryClient.invalidateQueries({ queryKey: ['blocklist'] });
    },
  });
}
