'use client';

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { adminAppSignatures } from '@/lib/api-client';
import type { AppSignature } from '@/lib/api-types';
// AppSignature imported for use in useAppSignature return type

export interface AppSignatureFilters {
  search?: string;
  category?: string;
  platform?: string;
  status?: string;
  page?: number;
  per_page?: number;
}

export function useAppSignatures(filters?: AppSignatureFilters) {
  return useQuery({
    queryKey: ['app-signatures', filters],
    queryFn: () => adminAppSignatures.list(filters),
  });
}

export function useAppSignature(id: string) {
  return useQuery({
    queryKey: ['app-signatures', id],
    queryFn: () => adminAppSignatures.get(id),
    enabled: !!id,
  });
}

export function useCreateAppSignature() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: {
      name: string;
      package_names?: string[];
      executable_names?: string[];
      cert_hashes?: string[];
      display_name_patterns?: string[];
      platforms?: string[];
      category: string;
      status?: string;
      confidence?: number;
      source?: string;
      evidence_url?: string;
      tags?: string[];
    }) => adminAppSignatures.create(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['app-signatures'] });
    },
  });
}

export interface UpdateAppSignatureData {
  name?: string;
  package_names?: string[];
  executable_names?: string[];
  cert_hashes?: string[];
  display_name_patterns?: string[];
  platforms?: string[];
  category?: string;
  status?: string;
  confidence?: number;
  source?: string;
  evidence_url?: string;
  tags?: string[];
}

export function useUpdateAppSignature() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      id,
      data,
    }: {
      id: string;
      data: UpdateAppSignatureData;
    }) => adminAppSignatures.update(id, data),
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ['app-signatures', variables.id] });
      queryClient.invalidateQueries({ queryKey: ['app-signatures'] });
    },
  });
}

export function useDeleteAppSignature() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => adminAppSignatures.delete(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['app-signatures'] });
    },
  });
}
