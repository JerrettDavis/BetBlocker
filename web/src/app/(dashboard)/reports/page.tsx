'use client';

import { useEventSummary } from '@/hooks/use-events';
import { SummaryCards } from '@/components/reports/summary-cards';
import { BlockCountChart } from '@/components/reports/block-count-chart';
import { TamperAlertTimeline } from '@/components/reports/tamper-alert-timeline';

export default function ReportsPage() {
  const { data, isLoading } = useEventSummary({ period: 'day' });

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold">Reports</h1>
        <p className="text-sm text-muted-foreground">
          View blocking activity and security alerts across your devices.
        </p>
      </div>

      <SummaryCards summary={data?.data} isLoading={isLoading} />
      <BlockCountChart summary={data?.data} isLoading={isLoading} />
      <TamperAlertTimeline />
    </div>
  );
}
