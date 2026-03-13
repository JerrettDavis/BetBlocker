import type { Metadata } from 'next';
import { AddEntryForm } from '@/components/blocklist/add-entry-form';

export const metadata: Metadata = {
  title: 'Add Blocklist Entry - BetBlocker',
};

export default function AddBlocklistEntryPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold">Add Blocklist Entry</h1>
        <p className="text-sm text-muted-foreground">Add a new domain to the blocklist.</p>
      </div>
      <AddEntryForm />
    </div>
  );
}
