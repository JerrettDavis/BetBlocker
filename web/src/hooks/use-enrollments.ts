'use client';

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { enrollments } from '@/lib/api-client';
import type { ProtectionConfig, ReportingConfig, UnenrollmentPolicy } from '@/lib/api-types';

export function useEnrollments(params?: {
  status?: string;
  tier?: string;
  device_id?: string;
  page?: number;
  per_page?: number;
}) {
  return useQuery({
    queryKey: ['enrollments', params],
    queryFn: () => enrollments.list(params),
  });
}

export function useEnrollment(id: string) {
  return useQuery({
    queryKey: ['enrollments', id],
    queryFn: () => enrollments.get(id),
    enabled: !!id,
  });
}

export function useCreateEnrollment() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: {
      device_id: string;
      tier: string;
      protection_config?: Partial<ProtectionConfig>;
      reporting_config?: Partial<ReportingConfig>;
      unenrollment_policy?: Partial<UnenrollmentPolicy>;
      expires_at?: string | null;
    }) => enrollments.create(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['enrollments'] });
      queryClient.invalidateQueries({ queryKey: ['devices'] });
    },
  });
}

export function useUpdateEnrollment() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      id,
      data,
    }: {
      id: string;
      data: {
        protection_config?: Partial<ProtectionConfig>;
        reporting_config?: Partial<ReportingConfig>;
        unenrollment_policy?: Partial<UnenrollmentPolicy>;
        expires_at?: string | null;
      };
    }) => enrollments.update(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['enrollments'] });
    },
  });
}

export function useRequestUnenroll() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, reason }: { id: string; reason?: string }) =>
      enrollments.requestUnenroll(id, reason),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['enrollments'] });
      queryClient.invalidateQueries({ queryKey: ['devices'] });
    },
  });
}

export function useApproveUnenroll() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      id,
      data,
    }: {
      id: string;
      data: { approved: boolean; note?: string };
    }) => enrollments.approveUnenroll(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['enrollments'] });
      queryClient.invalidateQueries({ queryKey: ['devices'] });
    },
  });
}
