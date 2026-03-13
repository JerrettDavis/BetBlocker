'use client';

import { Badge } from '@/components/ui/badge';
import type { DeviceStatus } from '@/lib/api-types';
import { cn } from '@/lib/utils';

const statusConfig: Record<DeviceStatus, { label: string; className: string }> = {
  active: { label: 'Active', className: 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200' },
  pending: { label: 'Pending', className: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200' },
  offline: { label: 'Offline', className: 'bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-200' },
  unenrolling: { label: 'Unenrolling', className: 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200' },
  unenrolled: { label: 'Unenrolled', className: 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200' },
};

interface DeviceStatusBadgeProps {
  status: DeviceStatus;
}

export function DeviceStatusBadge({ status }: DeviceStatusBadgeProps) {
  const config = statusConfig[status];
  return (
    <Badge variant="outline" className={cn('border-0', config.className)}>
      {config.label}
    </Badge>
  );
}
