'use client';

import { useEffect, useState } from 'react';
import { useSearchParams, useRouter } from 'next/navigation';
import { useAcceptInvitation } from '@/hooks/use-partners';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Skeleton } from '@/components/ui/skeleton';

export function AcceptInvitation() {
  const searchParams = useSearchParams();
  const router = useRouter();
  const token = searchParams.get('token') ?? '';
  const acceptInvitation = useAcceptInvitation();
  const [error, setError] = useState<string | null>(null);
  const [accepted, setAccepted] = useState(false);

  async function handleAccept() {
    setError(null);
    try {
      await acceptInvitation.mutateAsync(token);
      setAccepted(true);
    } catch (err: unknown) {
      const apiErr = err as { message?: string };
      setError(apiErr.message ?? 'Failed to accept invitation.');
    }
  }

  if (!token) {
    return (
      <Card className="max-w-md mx-auto">
        <CardContent className="pt-6">
          <Alert variant="destructive">
            <AlertDescription>Invalid invitation link.</AlertDescription>
          </Alert>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="max-w-md mx-auto">
      <CardHeader>
        <CardTitle>Partner Invitation</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        {accepted ? (
          <>
            <Alert>
              <AlertDescription>
                You have successfully accepted the partner invitation.
              </AlertDescription>
            </Alert>
            <Button onClick={() => router.push('/partner-dashboard')} className="w-full">
              Go to Partner Dashboard
            </Button>
          </>
        ) : (
          <>
            {error && (
              <Alert variant="destructive">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}
            <p className="text-sm text-muted-foreground">
              You have been invited to be an accountability partner. By accepting, you will be
              able to view reports and manage unenrollment requests.
            </p>
            <Button
              onClick={handleAccept}
              disabled={acceptInvitation.isPending}
              className="w-full"
            >
              {acceptInvitation.isPending ? 'Accepting...' : 'Accept Invitation'}
            </Button>
          </>
        )}
      </CardContent>
    </Card>
  );
}
