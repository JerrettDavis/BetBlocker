'use client';

import { useDevices } from '@/hooks/use-devices';
import { DeviceCard } from './device-card';
import { Skeleton } from '@/components/ui/skeleton';
import { Button } from '@/components/ui/button';
import Link from 'next/link';
import { Plus, Monitor } from 'lucide-react';

export function DeviceList() {
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
    return (
      <div className="text-center py-12">
        <p className="text-destructive">Failed to load devices. Please try again.</p>
      </div>
    );
  }

  const devices = data?.data ?? [];

  if (devices.length === 0) {
    return (
      <div className="text-center py-12">
        <Monitor className="mx-auto h-12 w-12 text-muted-foreground" />
        <h3 className="mt-4 text-lg font-semibold">No devices yet</h3>
        <p className="mt-2 text-sm text-muted-foreground">
          Add your first device to start blocking gambling content.
        </p>
        <Button className="mt-4" asChild>
          <Link href="/devices/add">
            <Plus className="mr-2 h-4 w-4" />
            Add Device
          </Link>
        </Button>
      </div>
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
