'use client';

import { useState } from 'react';
import { useRouter } from 'next/navigation';
import { useUpdateBlocklistEntry } from '@/hooks/use-blocklist';
import type { BlocklistEntry } from '@/lib/api-types';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Alert, AlertDescription } from '@/components/ui/alert';

interface EditEntryFormProps {
  entry: BlocklistEntry;
}

export function EditEntryForm({ entry }: EditEntryFormProps) {
  const router = useRouter();
  const updateEntry = useUpdateBlocklistEntry();

  const [category, setCategory] = useState(entry.category);
  const [status, setStatus] = useState(entry.status);
  const [evidenceUrl, setEvidenceUrl] = useState(entry.evidence_url ?? '');
  const [tags, setTags] = useState(entry.tags.join(', '));
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    try {
      await updateEntry.mutateAsync({
        id: entry.id,
        data: {
          category,
          status,
          evidence_url: evidenceUrl || undefined,
          tags: tags ? tags.split(',').map((t) => t.trim()) : undefined,
        },
      });
      router.push('/admin/blocklist');
    } catch (err: unknown) {
      const apiErr = err as { message?: string };
      setError(apiErr.message ?? 'Failed to update entry.');
    }
  }

  return (
    <Card className="max-w-lg">
      <CardHeader>
        <CardTitle>Edit Blocklist Entry</CardTitle>
        <p className="text-sm font-mono text-muted-foreground">
          {entry.domain ?? entry.pattern}
        </p>
      </CardHeader>
      <CardContent>
        <form onSubmit={handleSubmit} className="space-y-4">
          {error && (
            <Alert variant="destructive">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          <div className="space-y-2">
            <Label>Category</Label>
            <Select value={category} onValueChange={(v) => v && setCategory(v)}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="online_casino">Online Casino</SelectItem>
                <SelectItem value="sports_betting">Sports Betting</SelectItem>
                <SelectItem value="poker">Poker</SelectItem>
                <SelectItem value="lottery">Lottery</SelectItem>
                <SelectItem value="bingo">Bingo</SelectItem>
                <SelectItem value="fantasy_sports">Fantasy Sports</SelectItem>
                <SelectItem value="crypto_gambling">Crypto Gambling</SelectItem>
                <SelectItem value="affiliate">Affiliate</SelectItem>
                <SelectItem value="payment_processor">Payment Processor</SelectItem>
                <SelectItem value="other">Other</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <Label>Status</Label>
            <Select value={status} onValueChange={(v) => v && setStatus(v)}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="active">Active</SelectItem>
                <SelectItem value="inactive">Inactive</SelectItem>
                <SelectItem value="pending_review">Pending Review</SelectItem>
                <SelectItem value="rejected">Rejected</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <Label htmlFor="evidence">Evidence URL</Label>
            <Input
              id="evidence"
              type="url"
              value={evidenceUrl}
              onChange={(e) => setEvidenceUrl(e.target.value)}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="tags">Tags (comma-separated)</Label>
            <Input id="tags" value={tags} onChange={(e) => setTags(e.target.value)} />
          </div>

          <div className="flex gap-2">
            <Button type="submit" className="flex-1" disabled={updateEntry.isPending}>
              {updateEntry.isPending ? 'Saving...' : 'Save Changes'}
            </Button>
            <Button
              type="button"
              variant="outline"
              onClick={() => router.push('/admin/blocklist')}
            >
              Cancel
            </Button>
          </div>
        </form>
      </CardContent>
    </Card>
  );
}
