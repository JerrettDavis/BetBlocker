import { DeviceDetail } from '@/components/devices/device-detail';

export default async function DeviceDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;
  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold">Device Details</h1>
      <DeviceDetail deviceId={id} />
    </div>
  );
}
