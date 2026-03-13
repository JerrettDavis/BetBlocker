import type { Metadata } from 'next';
import { ReviewQueueTable } from '@/components/blocklist/review-queue-table';

export const metadata: Metadata = {
  title: 'Review Queue - BetBlocker',
};

export default function ReviewQueuePage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold">Review Queue</h1>
        <p className="text-sm text-muted-foreground">
          Review automatically detected gambling domains and decide whether to add them to the
          blocklist.
        </p>
      </div>
      <ReviewQueueTable />
    </div>
  );
}
