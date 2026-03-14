'use client';

import { useParams } from 'next/navigation';
import { useOrganization, useOrgMembers, useOrgDevices, useOrgTokens } from '@/hooks/use-organizations';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { capitalize, formatDate } from '@/lib/utils';
import { Users, Monitor, Key, Building2 } from 'lucide-react';

export default function OrgDetailPage() {
  const params = useParams();
  const orgId = params.id as string;

  const { data: orgData, isLoading: orgLoading, error: orgError } = useOrganization(orgId);
  const { data: membersData } = useOrgMembers(orgId);
  const { data: devicesData } = useOrgDevices(orgId);
  const { data: tokensData } = useOrgTokens(orgId);

  const org = orgData?.data;
  const memberCount = membersData?.pagination?.total ?? membersData?.data?.length ?? 0;
  const deviceCount = devicesData?.pagination?.total ?? devicesData?.data?.length ?? 0;
  const tokenCount = tokensData?.pagination?.total ?? tokensData?.data?.length ?? 0;

  if (orgLoading) {
    return (
      <div className="space-y-4">
        <Skeleton className="h-32" />
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} className="h-24" />
          ))}
        </div>
      </div>
    );
  }

  if (orgError || !org) {
    return <p className="text-destructive">Failed to load organization.</p>;
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Building2 className="h-5 w-5" />
            Organization Details
          </CardTitle>
        </CardHeader>
        <CardContent>
          <dl className="grid grid-cols-1 sm:grid-cols-2 gap-4 text-sm">
            <div>
              <dt className="text-muted-foreground">Name</dt>
              <dd className="font-medium">{org.name}</dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Type</dt>
              <dd>
                <Badge variant="outline">{capitalize(org.org_type)}</Badge>
              </dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Created</dt>
              <dd>{formatDate(org.created_at)}</dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Last Updated</dt>
              <dd>{formatDate(org.updated_at)}</dd>
            </div>
          </dl>
        </CardContent>
      </Card>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Members</CardTitle>
            <Users className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{memberCount}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Devices</CardTitle>
            <Monitor className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{deviceCount}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Enrollment Tokens</CardTitle>
            <Key className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{tokenCount}</div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
