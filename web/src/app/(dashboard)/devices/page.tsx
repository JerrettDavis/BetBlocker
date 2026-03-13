import type { Metadata } from 'next';
import { DeviceList } from '@/components/devices/device-list';
import { Button } from '@/components/ui/button';
import Link from 'next/link';
import { Plus } from 'lucide-react';

export const metadata: Metadata = {
  title: 'Devices - BetBlocker',
};

export default function DevicesPage() {
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Devices</h1>
          <p className="text-sm text-muted-foreground">Manage your protected devices.</p>
        </div>
        <Button asChild>
          <Link href="/devices/add">
            <Plus className="mr-2 h-4 w-4" />
            Add Device
          </Link>
        </Button>
      </div>
      <DeviceList />
    </div>
  );
}
