'use client';

import { useParams } from 'next/navigation';
import { useQuery } from '@tanstack/react-query';
import { blocklist } from '@/lib/api-client';
import { EditEntryForm } from '@/components/blocklist/edit-entry-form';
import { Skeleton } from '@/components/ui/skeleton';

export default function EditBlocklistEntryPage() {
  const params = useParams();
  const id = params.id as string;

  // We need to fetch the individual entry - use the list endpoint with a search
  // In a real app, there'd be a GET /admin/blocklist/entries/:id endpoint
  const { data, isLoading } = useQuery({
    queryKey: ['blocklist', 'entries', id],
    queryFn: () => blocklist.listEntries({ search: id }),
  });

  if (isLoading) {
    return <Skeleton className="h-96" />;
  }

  const entry = data?.data?.[0];
  if (!entry) {
    return <p className="text-destructive">Entry not found.</p>;
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold">Edit Blocklist Entry</h1>
        <p className="text-sm text-muted-foreground">Update entry details and status.</p>
      </div>
      <EditEntryForm entry={entry} />
    </div>
  );
}
