'use client';

import { usePartners, useRemovePartner } from '@/hooks/use-partners';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { capitalize, formatDate } from '@/lib/utils';
import { Users, Trash2 } from 'lucide-react';

export function PartnerList() {
  const { data, isLoading, error } = usePartners();
  const removePartner = useRemovePartner();

  if (isLoading) {
    return (
      <div className="space-y-4">
        {Array.from({ length: 2 }).map((_, i) => (
          <Skeleton key={i} className="h-24" />
        ))}
      </div>
    );
  }

  if (error) {
    return <p className="text-destructive">Failed to load partners.</p>;
  }

  const partners = data?.data ?? [];

  if (partners.length === 0) {
    return (
      <div className="text-center py-12">
        <Users className="mx-auto h-12 w-12 text-muted-foreground" />
        <h3 className="mt-4 text-lg font-semibold">No partners yet</h3>
        <p className="mt-2 text-sm text-muted-foreground">
          Invite a trusted person to be your accountability partner.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {partners.map((partner) => (
        <Card key={partner.id}>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-base">
              Partner:{' '}
              <code className="text-xs bg-muted px-1 py-0.5 rounded">
                {partner.partner_account_id}
              </code>
            </CardTitle>
            <div className="flex items-center gap-2">
              <Badge variant="outline">{capitalize(partner.role)}</Badge>
              <Badge
                variant={
                  partner.status === 'active'
                    ? 'default'
                    : partner.status === 'pending'
                      ? 'secondary'
                      : 'destructive'
                }
              >
                {capitalize(partner.status)}
              </Badge>
            </div>
          </CardHeader>
          <CardContent>
            <div className="flex items-center justify-between">
              <div className="text-sm text-muted-foreground">
                <p>Invited: {formatDate(partner.invited_at)}</p>
                {partner.accepted_at && <p>Accepted: {formatDate(partner.accepted_at)}</p>}
                <p className="mt-1">
                  Permissions: {partner.permissions.view_reports && 'View Reports '}
                  {partner.permissions.approve_unenrollment && 'Approve Unenrollment '}
                  {partner.permissions.modify_enrollment && 'Modify Enrollment'}
                </p>
              </div>
              {partner.status !== 'revoked' && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => removePartner.mutate(partner.id)}
                  disabled={removePartner.isPending}
                >
                  <Trash2 className="h-4 w-4 text-destructive" />
                </Button>
              )}
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
