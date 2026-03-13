'use client';

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { partners } from '@/lib/api-client';
import type { PartnerPermissions } from '@/lib/api-types';

export function usePartners(params?: {
  status?: string;
  role?: string;
  page?: number;
  per_page?: number;
}) {
  return useQuery({
    queryKey: ['partners', params],
    queryFn: () => partners.list(params),
  });
}

export function useInvitePartner() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: {
      email: string;
      role: string;
      permissions?: Partial<PartnerPermissions>;
      message?: string;
    }) => partners.invite(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['partners'] });
    },
  });
}

export function useAcceptInvitation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (token: string) => partners.accept(token),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['partners'] });
    },
  });
}

export function useRemovePartner() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => partners.remove(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['partners'] });
    },
  });
}
