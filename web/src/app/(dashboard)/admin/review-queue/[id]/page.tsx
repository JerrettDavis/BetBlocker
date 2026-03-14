'use client';

import { use } from 'react';
import { useReviewQueueItem, useApproveReviewItem, useRejectReviewItem } from '@/hooks/use-review-queue';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { capitalize, formatDate } from '@/lib/utils';
import { ArrowLeft, Check, X } from 'lucide-react';
import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { useState } from 'react';
import type { BlocklistCategory } from '@/lib/api-types';

const CATEGORIES: BlocklistCategory[] = [
  'online_casino',
  'sports_betting',
  'poker',
  'lottery',
  'bingo',
  'fantasy_sports',
  'crypto_gambling',
  'affiliate',
  'payment_processor',
  'other',
];

interface PageProps {
  params: Promise<{ id: string }>;
}

export default function ReviewQueueDetailPage({ params }: PageProps) {
  const { id } = use(params);
  const domain = decodeURIComponent(id);
  const router = useRouter();

  const { data: item, isLoading, error } = useReviewQueueItem(domain);
  const approveItem = useApproveReviewItem();
  const rejectItem = useRejectReviewItem();

  const [selectedCategory, setSelectedCategory] = useState<BlocklistCategory>('online_casino');

  function handleApprove() {
    approveItem.mutate(
      { domain, category: selectedCategory },
      { onSuccess: () => router.push('/admin/review-queue') },
    );
  }

  function handleReject() {
    rejectItem.mutate(
      { domain },
      { onSuccess: () => router.push('/admin/review-queue') },
    );
  }

  if (isLoading) {
    return (
      <div className="space-y-4">
        <Skeleton className="h-8 w-64" />
        <Skeleton className="h-48" />
        <Skeleton className="h-48" />
      </div>
    );
  }

  if (error || !item) {
    return (
      <div className="space-y-4">
        <Button variant="ghost" asChild>
          <Link href="/admin/review-queue">
            <ArrowLeft className="h-4 w-4 mr-2" />
            Back to Review Queue
          </Link>
        </Button>
        <p className="text-destructive">Item not found or failed to load.</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Breadcrumbs */}
      <div className="flex items-center gap-2 text-sm text-muted-foreground">
        <Link href="/admin/review-queue" className="hover:text-foreground">
          Review Queue
        </Link>
        <span>/</span>
        <span className="text-foreground font-medium">{item.domain}</span>
      </div>

      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold font-mono">{item.domain}</h1>
          <p className="text-sm text-muted-foreground mt-1">Discovery candidate detail</p>
        </div>
        <div className="flex items-center gap-3">
          <Select
            value={selectedCategory}
            onValueChange={(v) => setSelectedCategory(v as BlocklistCategory)}
          >
            <SelectTrigger className="w-[180px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {CATEGORIES.map((cat) => (
                <SelectItem key={cat} value={cat}>
                  {capitalize(cat)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Button
            className="bg-green-600 hover:bg-green-700 text-white"
            onClick={handleApprove}
            disabled={approveItem.isPending}
          >
            <Check className="h-4 w-4 mr-2" />
            Approve
          </Button>
          <Button
            variant="destructive"
            onClick={handleReject}
            disabled={rejectItem.isPending}
          >
            <X className="h-4 w-4 mr-2" />
            Reject
          </Button>
        </div>
      </div>

      {/* Summary card */}
      <Card>
        <CardHeader>
          <CardTitle>Summary</CardTitle>
        </CardHeader>
        <CardContent>
          <dl className="grid grid-cols-2 gap-4 sm:grid-cols-4">
            <div>
              <dt className="text-xs text-muted-foreground uppercase tracking-wide">Reports</dt>
              <dd className="text-2xl font-bold mt-1">{item.report_count}</dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground uppercase tracking-wide">Confidence</dt>
              <dd className="text-2xl font-bold mt-1">
                {(item.aggregated_confidence * 100).toFixed(0)}%
              </dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground uppercase tracking-wide">First Seen</dt>
              <dd className="text-sm font-medium mt-1">{formatDate(item.first_reported_at)}</dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground uppercase tracking-wide">Last Seen</dt>
              <dd className="text-sm font-medium mt-1">{formatDate(item.last_reported_at)}</dd>
            </div>
          </dl>
        </CardContent>
      </Card>

      {/* Classification evidence */}
      <Card>
        <CardHeader>
          <CardTitle>Classification Evidence</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <div>
            <p className="text-sm font-medium mb-2">Heuristic Matches</p>
            <div className="flex flex-wrap gap-2">
              {item.top_heuristic_matches.length === 0 ? (
                <p className="text-sm text-muted-foreground">No heuristic matches.</p>
              ) : (
                item.top_heuristic_matches.map((h) => (
                  <Badge key={h} variant="outline">
                    {h}
                  </Badge>
                ))
              )}
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Source metadata */}
      <Card>
        <CardHeader>
          <CardTitle>Source Metadata</CardTitle>
        </CardHeader>
        <CardContent>
          {Object.keys(item.sample_context).length === 0 ? (
            <p className="text-sm text-muted-foreground">No source metadata available.</p>
          ) : (
            <pre className="text-xs bg-muted rounded-md p-4 overflow-auto max-h-64">
              {JSON.stringify(item.sample_context, null, 2)}
            </pre>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
