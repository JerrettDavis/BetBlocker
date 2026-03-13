import type { Metadata } from 'next';
import { SupervisedDevices } from '@/components/partner-dashboard/supervised-devices';
import { ApprovalQueue } from '@/components/partner-dashboard/approval-queue';
import { Button } from '@/components/ui/button';
import Link from 'next/link';

export const metadata: Metadata = {
  title: 'Partner Dashboard - BetBlocker',
};

export default function PartnerDashboardPage() {
  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-2xl font-bold">Partner Dashboard</h1>
        <p className="text-sm text-muted-foreground">
          Monitor supervised devices and manage unenrollment requests.
        </p>
      </div>

      <div>
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold">Pending Approvals</h2>
          <Button variant="outline" size="sm" asChild>
            <Link href="/partner-dashboard/approvals">View All</Link>
          </Button>
        </div>
        <ApprovalQueue />
      </div>

      <div>
        <h2 className="text-lg font-semibold mb-4">Supervised Devices</h2>
        <SupervisedDevices />
      </div>
    </div>
  );
}
