'use client';

import { useDevice } from '@/hooks/use-devices';
import { useEvents } from '@/hooks/use-events';
import { PlatformIcon } from './platform-icon';
import { DeviceStatusBadge } from './device-status-badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { Badge } from '@/components/ui/badge';
import { formatDistanceToNow, formatDateTime, capitalize } from '@/lib/utils';
import Link from 'next/link';

interface DeviceDetailProps {
  deviceId: string;
}

export function DeviceDetail({ deviceId }: DeviceDetailProps) {
  const { data: deviceData, isLoading: deviceLoading } = useDevice(deviceId);
  const { data: eventsData, isLoading: eventsLoading } = useEvents({
    device_id: deviceId,
    per_page: 10,
  });

  if (deviceLoading) {
    return (
      <div className="space-y-4">
        <Skeleton className="h-32" />
        <Skeleton className="h-48" />
      </div>
    );
  }

  const device = deviceData?.data;
  if (!device) {
    return <p className="text-destructive">Device not found.</p>;
  }

  const events = eventsData?.data ?? [];

  return (
    <div className="space-y-6">
      {/* Device Header */}
      <Card>
        <CardHeader className="flex flex-row items-center gap-4">
          <PlatformIcon platform={device.platform} className="h-12 w-12 text-muted-foreground" />
          <div className="flex-1">
            <CardTitle className="text-xl">{device.name}</CardTitle>
            <p className="text-sm text-muted-foreground">
              {device.platform} {device.os_version} &middot; {device.hostname}
            </p>
          </div>
          <DeviceStatusBadge status={device.status} />
        </CardHeader>
        <CardContent>
          <dl className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
            <div>
              <dt className="text-muted-foreground">Agent Version</dt>
              <dd className="font-medium">{device.agent_version}</dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Blocklist Version</dt>
              <dd className="font-medium">v{device.blocklist_version}</dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Last Heartbeat</dt>
              <dd className="font-medium">
                {device.last_heartbeat_at
                  ? formatDistanceToNow(device.last_heartbeat_at)
                  : 'Never'}
              </dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Registered</dt>
              <dd className="font-medium">{formatDateTime(device.created_at)}</dd>
            </div>
          </dl>
        </CardContent>
      </Card>

      {/* Enrollment */}
      <Card>
        <CardHeader>
          <CardTitle>Enrollment</CardTitle>
        </CardHeader>
        <CardContent>
          {device.enrollment_id ? (
            <div className="flex items-center justify-between">
              <p className="text-sm">
                Enrollment ID:{' '}
                <code className="text-xs bg-muted px-1 py-0.5 rounded">
                  {device.enrollment_id}
                </code>
              </p>
              <div className="flex gap-2">
                <Button variant="outline" size="sm" asChild>
                  <Link href={`/enrollments/${device.enrollment_id}`}>View Details</Link>
                </Button>
                <Button variant="destructive" size="sm" asChild>
                  <Link href={`/enrollments/${device.enrollment_id}/unenroll`}>
                    Request Unenrollment
                  </Link>
                </Button>
              </div>
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">
              This device is not currently enrolled in any protection plan.
            </p>
          )}
        </CardContent>
      </Card>

      {/* Recent Events */}
      <Card>
        <CardHeader>
          <CardTitle>Recent Events</CardTitle>
        </CardHeader>
        <CardContent>
          {eventsLoading ? (
            <div className="space-y-2">
              {Array.from({ length: 5 }).map((_, i) => (
                <Skeleton key={i} className="h-8" />
              ))}
            </div>
          ) : events.length === 0 ? (
            <p className="text-sm text-muted-foreground">No events recorded yet.</p>
          ) : (
            <div className="space-y-2">
              {events.map((event) => (
                <div
                  key={event.id}
                  className="flex items-center justify-between rounded-md border p-3 text-sm"
                >
                  <div className="flex items-center gap-2">
                    <Badge
                      variant={
                        event.severity === 'critical'
                          ? 'destructive'
                          : event.severity === 'warning'
                            ? 'secondary'
                            : 'outline'
                      }
                    >
                      {event.severity}
                    </Badge>
                    <span className="font-medium">{capitalize(event.type)}</span>
                    <span className="text-muted-foreground">{event.category}</span>
                  </div>
                  <span className="text-muted-foreground">
                    {formatDistanceToNow(event.occurred_at)}
                  </span>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
