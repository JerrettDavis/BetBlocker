'use client';

import { Laptop, Smartphone } from 'lucide-react';
import type { DevicePlatform } from '@/lib/api-types';
import { cn } from '@/lib/utils';

interface PlatformIconProps {
  platform: DevicePlatform;
  className?: string;
}

export function PlatformIcon({ platform, className }: PlatformIconProps) {
  const isMobile = platform === 'android' || platform === 'ios';
  const Icon = isMobile ? Smartphone : Laptop;
  return <Icon className={cn('h-5 w-5', className)} />;
}
