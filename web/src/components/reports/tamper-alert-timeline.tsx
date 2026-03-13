'use client';

import { useEvents } from '@/hooks/use-events';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { formatDistanceToNow, capitalize } from '@/lib/utils';
import { AlertTriangle } from 'lucide-react';

export function TamperAlertTimeline() {
  const { data, isLoading } = useEvents({
    category: 'tamper',
    per_page: 10,
  });

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <AlertTriangle className="h-4 w-4 text-red-600" />
          Tamper Alerts
        </CardTitle>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="space-y-2">
            {Array.from({ length: 3 }).map((_, i) => (
              <Skeleton key={i} className="h-12" />
            ))}
          </div>
        ) : (data?.data ?? []).length === 0 ? (
          <p className="text-sm text-muted-foreground text-center py-4">
            No tamper alerts recorded. Your devices are secure.
          </p>
        ) : (
          <div className="space-y-3">
            {(data?.data ?? []).map((event) => (
              <div
                key={event.id}
                className="flex items-center justify-between rounded-md border p-3"
              >
                <div className="flex items-center gap-2">
                  <Badge variant="destructive">{capitalize(event.type)}</Badge>
                  <span className="text-sm text-muted-foreground">
                    Device: {event.device_id.slice(0, 12)}...
                  </span>
                </div>
                <span className="text-sm text-muted-foreground">
                  {formatDistanceToNow(event.occurred_at)}
                </span>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
