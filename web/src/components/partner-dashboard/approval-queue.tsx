'use client';

import { useEnrollments, useApproveUnenroll } from '@/hooks/use-enrollments';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { formatDateTime } from '@/lib/utils';

export function ApprovalQueue() {
  const { data, isLoading, error } = useEnrollments({ status: 'unenroll_requested' });
  const approveUnenroll = useApproveUnenroll();

  if (isLoading) {
    return (
      <div className="space-y-4">
        {Array.from({ length: 2 }).map((_, i) => (
          <Skeleton key={i} className="h-32" />
        ))}
      </div>
    );
  }

  if (error) {
    return <p className="text-destructive">Failed to load approval queue.</p>;
  }

  const enrollments = data?.data ?? [];

  if (enrollments.length === 0) {
    return (
      <p className="text-sm text-muted-foreground text-center py-8">
        No pending unenrollment requests.
      </p>
    );
  }

  return (
    <div className="space-y-4">
      {enrollments.map((enrollment) => (
        <Card key={enrollment.id}>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-base">
              Device: {enrollment.device_id}
            </CardTitle>
            <Badge variant="destructive">Unenrollment Requested</Badge>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {enrollment.unenrollment_request && (
                <div className="text-sm text-muted-foreground">
                  <p>
                    Requested: {formatDateTime(enrollment.unenrollment_request.requested_at)}
                  </p>
                  {enrollment.unenrollment_request.reason && (
                    <p>Reason: {enrollment.unenrollment_request.reason}</p>
                  )}
                </div>
              )}
              <div className="flex gap-2">
                <Button
                  size="sm"
                  onClick={() =>
                    approveUnenroll.mutate({
                      id: enrollment.id,
                      data: { approved: true },
                    })
                  }
                  disabled={approveUnenroll.isPending}
                >
                  Approve
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() =>
                    approveUnenroll.mutate({
                      id: enrollment.id,
                      data: { approved: false, note: 'Denied by partner' },
                    })
                  }
                  disabled={approveUnenroll.isPending}
                >
                  Deny
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
