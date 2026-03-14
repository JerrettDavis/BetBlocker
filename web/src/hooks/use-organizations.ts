'use client';

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { organizations } from '@/lib/api-client';
import type { OrgMemberRole } from '@/lib/api-types';

export function useOrganizations(params?: { page?: number; per_page?: number }) {
  return useQuery({
    queryKey: ['organizations', params],
    queryFn: () => organizations.list(params),
  });
}

export function useOrganization(id: string) {
  return useQuery({
    queryKey: ['organizations', id],
    queryFn: () => organizations.get(id),
    enabled: !!id,
  });
}

export function useCreateOrganization() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: { name: string; org_type: string }) => organizations.create(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['organizations'] });
    },
  });
}

export function useUpdateOrganization() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      id,
      data,
    }: {
      id: string;
      data: {
        name?: string;
        org_type?: string;
        default_protection_config?: Record<string, unknown>;
        default_reporting_config?: Record<string, unknown>;
        default_unenrollment_policy?: Record<string, unknown>;
      };
    }) => organizations.update(id, data),
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ['organizations', variables.id] });
      queryClient.invalidateQueries({ queryKey: ['organizations'] });
    },
  });
}

export function useDeleteOrganization() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => organizations.delete(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['organizations'] });
    },
  });
}

// Members
export function useOrgMembers(orgId: string, params?: { page?: number; per_page?: number }) {
  return useQuery({
    queryKey: ['organizations', orgId, 'members', params],
    queryFn: () => organizations.listMembers(orgId, params),
    enabled: !!orgId,
  });
}

export function useInviteOrgMember() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ orgId, data }: { orgId: string; data: { email: string; role: OrgMemberRole } }) =>
      organizations.inviteMember(orgId, data),
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ['organizations', variables.orgId, 'members'] });
    },
  });
}

export function useUpdateMemberRole() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      orgId,
      memberId,
      role,
    }: {
      orgId: string;
      memberId: string;
      role: OrgMemberRole;
    }) => organizations.updateMemberRole(orgId, memberId, { role }),
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ['organizations', variables.orgId, 'members'] });
    },
  });
}

export function useRemoveOrgMember() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ orgId, memberId }: { orgId: string; memberId: string }) =>
      organizations.removeMember(orgId, memberId),
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ['organizations', variables.orgId, 'members'] });
    },
  });
}

// Devices
export function useOrgDevices(orgId: string, params?: { page?: number; per_page?: number }) {
  return useQuery({
    queryKey: ['organizations', orgId, 'devices', params],
    queryFn: () => organizations.listDevices(orgId, params),
    enabled: !!orgId,
  });
}

export function useAssignOrgDevice() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ orgId, device_id }: { orgId: string; device_id: number }) =>
      organizations.assignDevice(orgId, { device_id }),
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ['organizations', variables.orgId, 'devices'] });
    },
  });
}

export function useUnassignOrgDevice() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ orgId, deviceId }: { orgId: string; deviceId: number }) =>
      organizations.unassignDevice(orgId, deviceId),
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ['organizations', variables.orgId, 'devices'] });
    },
  });
}

// Tokens
export function useOrgTokens(orgId: string, params?: { page?: number; per_page?: number }) {
  return useQuery({
    queryKey: ['organizations', orgId, 'tokens', params],
    queryFn: () => organizations.listTokens(orgId, params),
    enabled: !!orgId,
  });
}

export function useCreateOrgToken() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      orgId,
      data,
    }: {
      orgId: string;
      data: {
        label?: string;
        protection_config: Record<string, unknown>;
        reporting_config: Record<string, unknown>;
        unenrollment_policy: Record<string, unknown>;
        max_uses?: number;
        expires_at?: string;
      };
    }) => organizations.createToken(orgId, data),
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ['organizations', variables.orgId, 'tokens'] });
    },
  });
}

export function useRevokeOrgToken() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ orgId, tokenId }: { orgId: string; tokenId: number }) =>
      organizations.revokeToken(orgId, tokenId),
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ['organizations', variables.orgId, 'tokens'] });
    },
  });
}
