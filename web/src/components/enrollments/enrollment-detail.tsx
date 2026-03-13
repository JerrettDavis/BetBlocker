'use client';

import { useEnrollment } from '@/hooks/use-enrollments';
import { EnrollmentConfig } from './enrollment-config';
import { UnenrollCountdown } from './unenroll-countdown';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { formatDateTime, capitalize } from '@/lib/utils';
import Link from 'next/link';

interface EnrollmentDetailProps {
  enrollmentId: string;
}

export function EnrollmentDetail({ enrollmentId }: EnrollmentDetailProps) {
  const { data, isLoading } = useEnrollment(enrollmentId);

  if (isLoading) {
    return (
      <div className="space-y-4">
        <Skeleton className="h-32" />
        <Skeleton className="h-48" />
      </div>
    );
  }

  const enrollment = data?.data;
  if (!enrollment) {
    return <p className="text-destructive">Enrollment not found.</p>;
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <div>
            <CardTitle>Enrollment</CardTitle>
            <p className="text-sm text-muted-foreground mt-1">
              <code className="text-xs bg-muted px-1 py-0.5 rounded">{enrollment.id}</code>
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Badge variant="outline">{capitalize(enrollment.tier)} tier</Badge>
            <Badge
              variant={
                enrollment.status === 'active'
                  ? 'default'
                  : enrollment.status.startsWith('unenroll')
                    ? 'destructive'
                    : 'secondary'
              }
            >
              {capitalize(enrollment.status)}
            </Badge>
          </div>
        </CardHeader>
        <CardContent>
          <dl className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
            <div>
              <dt className="text-muted-foreground">Created</dt>
              <dd className="font-medium">{formatDateTime(enrollment.created_at)}</dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Last Updated</dt>
              <dd className="font-medium">{formatDateTime(enrollment.updated_at)}</dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Expires</dt>
              <dd className="font-medium">
                {enrollment.expires_at ? formatDateTime(enrollment.expires_at) : 'Never'}
              </dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Unenrollment Policy</dt>
              <dd className="font-medium">{capitalize(enrollment.unenrollment_policy.type)}</dd>
            </div>
          </dl>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Configuration</CardTitle>
        </CardHeader>
        <CardContent>
          <EnrollmentConfig
            protection={enrollment.protection_config}
            reporting={enrollment.reporting_config}
          />
        </CardContent>
      </Card>

      {enrollment.unenrollment_request && (
        <Card className="border-destructive">
          <CardHeader>
            <CardTitle className="text-destructive">Unenrollment Request</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <dl className="grid grid-cols-2 gap-4 text-sm">
              <div>
                <dt className="text-muted-foreground">Requested At</dt>
                <dd className="font-medium">
                  {formatDateTime(enrollment.unenrollment_request.requested_at)}
                </dd>
              </div>
              {enrollment.unenrollment_request.reason && (
                <div>
                  <dt className="text-muted-foreground">Reason</dt>
                  <dd className="font-medium">{enrollment.unenrollment_request.reason}</dd>
                </div>
              )}
            </dl>
            {enrollment.unenrollment_request.eligible_at && (
              <UnenrollCountdown
                eligibleAt={enrollment.unenrollment_request.eligible_at}
              />
            )}
            {enrollment.unenrollment_policy.type !== 'time_delayed' &&
              !enrollment.unenrollment_request.approved_at && (
                <p className="text-sm text-muted-foreground">
                  Waiting for approval from the{' '}
                  {enrollment.unenrollment_policy.type === 'partner_approval'
                    ? 'accountability partner'
                    : 'authority representative'}
                  .
                </p>
              )}
          </CardContent>
        </Card>
      )}

      {enrollment.status === 'active' && !enrollment.unenrollment_request && (
        <div className="flex justify-end">
          <Button variant="destructive" asChild>
            <Link href={`/enrollments/${enrollment.id}/unenroll`}>Request Unenrollment</Link>
          </Button>
        </div>
      )}
    </div>
  );
}
