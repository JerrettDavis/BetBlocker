'use client';

import { useState } from 'react';
import { useParams } from 'next/navigation';
import { useOrgDevices, useAssignOrgDevice, useUnassignOrgDevice } from '@/hooks/use-organizations';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
  DialogFooter,
} from '@/components/ui/dialog';
import { Skeleton } from '@/components/ui/skeleton';
import { formatDate } from '@/lib/utils';
import { Plus, Monitor, Trash2 } from 'lucide-react';

export default function OrgDevicesPage() {
  const params = useParams();
  const orgId = params.id as string;

  const { data, isLoading, error } = useOrgDevices(orgId);
  const assignDevice = useAssignOrgDevice();
  const unassignDevice = useUnassignOrgDevice();

  const [showAssign, setShowAssign] = useState(false);
  const [deviceId, setDeviceId] = useState('');

  const handleAssign = async (e: React.FormEvent) => {
    e.preventDefault();
    const id = parseInt(deviceId, 10);
    if (isNaN(id)) return;
    try {
      await assignDevice.mutateAsync({ orgId, device_id: id });
      setDeviceId('');
      setShowAssign(false);
    } catch {
      // error available via assignDevice.error
    }
  };

  const handleUnassign = (deviceId: number) => {
    if (confirm('Are you sure you want to unassign this device?')) {
      unassignDevice.mutate({ orgId, deviceId });
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">Devices</h2>
        <Dialog open={showAssign} onOpenChange={setShowAssign}>
          <DialogTrigger
            render={
              <Button size="sm">
                <Plus className="mr-2 h-4 w-4" />
                Assign Device
              </Button>
            }
          />
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Assign Device</DialogTitle>
            </DialogHeader>
            <form onSubmit={handleAssign} className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="device-id">Device ID</Label>
                <Input
                  id="device-id"
                  type="number"
                  value={deviceId}
                  onChange={(e) => setDeviceId(e.target.value)}
                  placeholder="Enter device ID"
                  required
                />
              </div>
              {assignDevice.error && (
                <p className="text-sm text-destructive">{assignDevice.error.message}</p>
              )}
              <DialogFooter>
                <Button type="submit" disabled={assignDevice.isPending}>
                  {assignDevice.isPending ? 'Assigning...' : 'Assign'}
                </Button>
              </DialogFooter>
            </form>
          </DialogContent>
        </Dialog>
      </div>

      {isLoading && (
        <div className="space-y-4">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} className="h-12" />
          ))}
        </div>
      )}

      {error && <p className="text-destructive">Failed to load devices.</p>}

      {data && data.data.length === 0 && (
        <div className="text-center py-12">
          <Monitor className="mx-auto h-12 w-12 text-muted-foreground" />
          <h3 className="mt-4 text-lg font-semibold">No devices assigned</h3>
          <p className="mt-2 text-sm text-muted-foreground">
            Assign devices to this organization to manage them centrally.
          </p>
        </div>
      )}

      {data && data.data.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Assigned Devices</CardTitle>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Device ID</TableHead>
                  <TableHead>Assigned At</TableHead>
                  <TableHead></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {data.data.map((device) => (
                  <TableRow key={device.id}>
                    <TableCell className="font-medium">{device.device_id}</TableCell>
                    <TableCell className="text-muted-foreground">
                      {formatDate(device.assigned_at)}
                    </TableCell>
                    <TableCell>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleUnassign(device.device_id)}
                        disabled={unassignDevice.isPending}
                      >
                        <Trash2 className="h-4 w-4 text-destructive" />
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
