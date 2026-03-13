import type { Metadata } from 'next';
import { ApprovalQueue } from '@/components/partner-dashboard/approval-queue';

export const metadata: Metadata = {
  title: 'Pending Approvals - BetBlocker',
};

export default function ApprovalsPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold">Pending Approvals</h1>
        <p className="text-sm text-muted-foreground">
          Review and respond to unenrollment requests from your partners.
        </p>
      </div>
      <ApprovalQueue />
    </div>
  );
}
