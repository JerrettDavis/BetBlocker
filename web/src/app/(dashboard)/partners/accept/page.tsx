import type { Metadata } from 'next';
import { Suspense } from 'react';
import { AcceptInvitation } from '@/components/partners/accept-invitation';
import { Skeleton } from '@/components/ui/skeleton';

export const metadata: Metadata = {
  title: 'Accept Invitation - BetBlocker',
};

export default function AcceptPartnerPage() {
  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold">Accept Partner Invitation</h1>
      <Suspense fallback={<Skeleton className="h-48 max-w-md mx-auto" />}>
        <AcceptInvitation />
      </Suspense>
    </div>
  );
}
