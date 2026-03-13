'use client';

import { useDevices } from '@/hooks/use-devices';
import { DeviceCard } from '@/components/devices/device-card';
import { Skeleton } from '@/components/ui/skeleton';

export function SupervisedDevices() {
  const { data, isLoading, error } = useDevices();

  if (isLoading) {
    return (
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {Array.from({ length: 3 }).map((_, i) => (
          <Skeleton key={i} className="h-48" />
        ))}
      </div>
    );
  }

  if (error) {
    return <p className="text-destructive">Failed to load supervised devices.</p>;
  }

  const devices = data?.data ?? [];

  if (devices.length === 0) {
    return (
      <p className="text-sm text-muted-foreground text-center py-8">
        No supervised devices found. You will see devices here when your partners add them.
      </p>
    );
  }

  return (
    <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
      {devices.map((device) => (
        <DeviceCard key={device.id} device={device} />
      ))}
    </div>
  );
}
