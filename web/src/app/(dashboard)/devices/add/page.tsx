import type { Metadata } from 'next';
import { AddDeviceWizard } from '@/components/devices/add-device-wizard';

export const metadata: Metadata = {
  title: 'Add Device - BetBlocker',
};

export default function AddDevicePage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold">Add a Device</h1>
        <p className="text-sm text-muted-foreground">
          Set up protection for a new device.
        </p>
      </div>
      <AddDeviceWizard />
    </div>
  );
}
