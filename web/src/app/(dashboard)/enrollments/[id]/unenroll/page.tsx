'use client';

import { useParams, useRouter } from 'next/navigation';
import { useEnrollment } from '@/hooks/use-enrollments';
import { UnenrollDialog } from '@/components/enrollments/unenroll-dialog';
import { UnenrollCountdown } from '@/components/enrollments/unenroll-countdown';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { capitalize } from '@/lib/utils';

export default function UnenrollPage() {
  const params = useParams();
  const router = useRouter();
  const enrollmentId = params.id as string;
  const { data, isLoading } = useEnrollment(enrollmentId);

  if (isLoading) {
    return <Skeleton className="h-48" />;
  }

  const enrollment = data?.data;
  if (!enrollment) {
    return <p className="text-destructive">Enrollment not found.</p>;
  }

  const hasRequest = !!enrollment.unenrollment_request;

  return (
    <div className="max-w-2xl mx-auto space-y-6">
      <h1 className="text-2xl font-bold">Unenrollment</h1>

      {hasRequest ? (
        <Card>
          <CardHeader>
            <CardTitle>Unenrollment In Progress</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <p className="text-sm text-muted-foreground">
              An unenrollment request is already active for this enrollment.
            </p>
            <p className="text-sm">
              Status: <strong>{capitalize(enrollment.status)}</strong>
            </p>
            {enrollment.unenrollment_request?.eligible_at && (
              <UnenrollCountdown
                eligibleAt={enrollment.unenrollment_request.eligible_at}
              />
            )}
            {enrollment.unenrollment_policy.type !== 'time_delayed' &&
              !enrollment.unenrollment_request?.approved_at && (
                <p className="text-sm text-muted-foreground">
                  Waiting for approval.
                </p>
              )}
          </CardContent>
        </Card>
      ) : (
        <Card>
          <CardHeader>
            <CardTitle>Request Unenrollment</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <p className="text-sm text-muted-foreground">
              {enrollment.tier === 'self'
                ? `This enrollment uses a time-delayed unenrollment policy. After requesting, there will be a ${enrollment.unenrollment_policy.cooldown_hours ?? 24}-hour cooldown period.`
                : 'This enrollment requires approval from your accountability partner before unenrollment can proceed.'}
            </p>
            <UnenrollDialog
              enrollmentId={enrollmentId}
              tier={enrollment.tier}
              onSuccess={() => router.refresh()}
            />
          </CardContent>
        </Card>
      )}
    </div>
  );
}
