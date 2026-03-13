'use client';

import { useState } from 'react';
import { useRouter } from 'next/navigation';
import { useCreateBlocklistEntry } from '@/hooks/use-blocklist';
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
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Alert, AlertDescription } from '@/components/ui/alert';

export function AddEntryForm() {
  const router = useRouter();
  const createEntry = useCreateBlocklistEntry();

  const [domain, setDomain] = useState('');
  const [pattern, setPattern] = useState('');
  const [category, setCategory] = useState('online_casino');
  const [evidenceUrl, setEvidenceUrl] = useState('');
  const [tags, setTags] = useState('');
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    try {
      await createEntry.mutateAsync({
        domain: domain || undefined,
        pattern: pattern || undefined,
        category,
        evidence_url: evidenceUrl || undefined,
        tags: tags ? tags.split(',').map((t) => t.trim()) : undefined,
      });
      router.push('/admin/blocklist');
    } catch (err: unknown) {
      const apiErr = err as { message?: string };
      setError(apiErr.message ?? 'Failed to create entry.');
    }
  }

  return (
    <Card className="max-w-lg">
      <CardHeader>
        <CardTitle>Add Blocklist Entry</CardTitle>
        <CardDescription>Add a new domain or pattern to the blocklist.</CardDescription>
      </CardHeader>
      <CardContent>
        <form onSubmit={handleSubmit} className="space-y-4">
          {error && (
            <Alert variant="destructive">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          <div className="space-y-2">
            <Label htmlFor="domain">Domain (exact match)</Label>
            <Input
              id="domain"
              value={domain}
              onChange={(e) => setDomain(e.target.value)}
              placeholder="example-casino.com"
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="pattern">Pattern (wildcard)</Label>
            <Input
              id="pattern"
              value={pattern}
              onChange={(e) => setPattern(e.target.value)}
              placeholder="*.casino-*.com"
            />
            <p className="text-xs text-muted-foreground">
              Use either domain or pattern, not both.
            </p>
          </div>

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
            <Label htmlFor="evidence">Evidence URL (optional)</Label>
            <Input
              id="evidence"
              type="url"
              value={evidenceUrl}
              onChange={(e) => setEvidenceUrl(e.target.value)}
              placeholder="https://..."
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="tags">Tags (comma-separated)</Label>
            <Input
              id="tags"
              value={tags}
              onChange={(e) => setTags(e.target.value)}
              placeholder="casino, slots, uk-licensed"
            />
          </div>

          <Button type="submit" className="w-full" disabled={createEntry.isPending}>
            {createEntry.isPending ? 'Creating...' : 'Add Entry'}
          </Button>
        </form>
      </CardContent>
    </Card>
  );
}
