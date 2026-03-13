'use client';

import { useReviewQueue, useResolveReview } from '@/hooks/use-blocklist';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { formatDate } from '@/lib/utils';
import { Check, X } from 'lucide-react';

export function ReviewQueueTable() {
  const { data, isLoading, error } = useReviewQueue();
  const resolveReview = useResolveReview();

  if (isLoading) {
    return (
      <div className="space-y-2">
        {Array.from({ length: 5 }).map((_, i) => (
          <Skeleton key={i} className="h-12" />
        ))}
      </div>
    );
  }

  if (error) {
    return <p className="text-destructive">Failed to load review queue.</p>;
  }

  const items = data?.data ?? [];

  if (items.length === 0) {
    return (
      <p className="text-sm text-muted-foreground text-center py-8">
        No items pending review.
      </p>
    );
  }

  return (
    <div className="rounded-md border">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Domain</TableHead>
            <TableHead>Reports</TableHead>
            <TableHead>Confidence</TableHead>
            <TableHead>First Reported</TableHead>
            <TableHead>Heuristics</TableHead>
            <TableHead className="w-[120px]">Actions</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {items.map((item) => (
            <TableRow key={item.domain}>
              <TableCell className="font-mono text-sm">{item.domain}</TableCell>
              <TableCell>{item.report_count}</TableCell>
              <TableCell>{(item.aggregated_confidence * 100).toFixed(0)}%</TableCell>
              <TableCell className="text-sm">{formatDate(item.first_reported_at)}</TableCell>
              <TableCell>
                <div className="flex flex-wrap gap-1">
                  {item.top_heuristic_matches.slice(0, 3).map((h) => (
                    <Badge key={h} variant="outline" className="text-xs">
                      {h}
                    </Badge>
                  ))}
                </div>
              </TableCell>
              <TableCell>
                <div className="flex gap-1">
                  <Button
                    size="sm"
                    variant="ghost"
                    className="text-green-600"
                    onClick={() =>
                      resolveReview.mutate({
                        domain: item.domain,
                        data: { action: 'promote' },
                      })
                    }
                    disabled={resolveReview.isPending}
                  >
                    <Check className="h-4 w-4" />
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    className="text-red-600"
                    onClick={() =>
                      resolveReview.mutate({
                        domain: item.domain,
                        data: { action: 'reject' },
                      })
                    }
                    disabled={resolveReview.isPending}
                  >
                    <X className="h-4 w-4" />
                  </Button>
                </div>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}
