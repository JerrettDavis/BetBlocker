import type { Metadata } from 'next';
import { BlocklistTable } from '@/components/blocklist/blocklist-table';
import { Button } from '@/components/ui/button';
import Link from 'next/link';
import { Plus, ClipboardList } from 'lucide-react';

export const metadata: Metadata = {
  title: 'Blocklist Admin - BetBlocker',
};

export default function BlocklistPage() {
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Blocklist Administration</h1>
          <p className="text-sm text-muted-foreground">
            Manage gambling domain blocklist entries.
          </p>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" asChild>
            <Link href="/admin/blocklist/review">
              <ClipboardList className="mr-2 h-4 w-4" />
              Review Queue
            </Link>
          </Button>
          <Button asChild>
            <Link href="/admin/blocklist/add">
              <Plus className="mr-2 h-4 w-4" />
              Add Entry
            </Link>
          </Button>
        </div>
      </div>
      <BlocklistTable />
    </div>
  );
}
