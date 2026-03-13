'use client';

import Link from 'next/link';
import type { Device } from '@/lib/api-types';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { PlatformIcon } from './platform-icon';
import { DeviceStatusBadge } from './device-status-badge';
import { formatDistanceToNow } from '@/lib/utils';

interface DeviceCardProps {
  device: Device;
}

export function DeviceCard({ device }: DeviceCardProps) {
  return (
    <Link href={`/devices/${device.id}`}>
      <Card className="transition-colors hover:bg-muted/50">
        <CardHeader className="flex flex-row items-center gap-3 pb-2">
          <PlatformIcon platform={device.platform} className="h-8 w-8 text-muted-foreground" />
          <div className="flex-1">
            <CardTitle className="text-base">{device.name}</CardTitle>
            <p className="text-sm text-muted-foreground">
              {device.platform} {device.os_version}
            </p>
          </div>
          <DeviceStatusBadge status={device.status} />
        </CardHeader>
        <CardContent>
          <dl className="grid grid-cols-2 gap-2 text-sm">
            <div>
              <dt className="text-muted-foreground">Last heartbeat</dt>
              <dd>
                {device.last_heartbeat_at
                  ? formatDistanceToNow(device.last_heartbeat_at)
                  : 'Never'}
              </dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Blocklist</dt>
              <dd>v{device.blocklist_version}</dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Agent</dt>
              <dd>{device.agent_version}</dd>
            </div>
          </dl>
        </CardContent>
      </Card>
    </Link>
  );
}
