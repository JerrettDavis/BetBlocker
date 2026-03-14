'use client';

import Link from 'next/link';
import { useParams, usePathname } from 'next/navigation';
import { useOrganization } from '@/hooks/use-organizations';
import { cn } from '@/lib/utils';
import { Skeleton } from '@/components/ui/skeleton';
import { ArrowLeft } from 'lucide-react';

const tabs = [
  { label: 'Overview', href: '' },
  { label: 'Members', href: '/members' },
  { label: 'Devices', href: '/devices' },
  { label: 'Tokens', href: '/tokens' },
  { label: 'Settings', href: '/settings' },
];

export default function OrgDetailLayout({ children }: { children: React.ReactNode }) {
  const params = useParams();
  const pathname = usePathname();
  const orgId = params.id as string;
  const { data, isLoading } = useOrganization(orgId);
  const org = data?.data;

  const basePath = `/organizations/${orgId}`;

  return (
    <div className="space-y-6">
      <div>
        <Link
          href="/organizations"
          className="inline-flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground mb-2"
        >
          <ArrowLeft className="h-4 w-4" />
          Back to Organizations
        </Link>
        {isLoading ? (
          <Skeleton className="h-8 w-48" />
        ) : (
          <h1 className="text-2xl font-bold">{org?.name ?? 'Organization'}</h1>
        )}
      </div>

      <nav className="flex gap-1 border-b">
        {tabs.map((tab) => {
          const tabPath = `${basePath}${tab.href}`;
          const isActive = pathname === tabPath;
          return (
            <Link
              key={tab.href}
              href={tabPath}
              className={cn(
                'px-4 py-2 text-sm font-medium border-b-2 -mb-px transition-colors',
                isActive
                  ? 'border-primary text-foreground'
                  : 'border-transparent text-muted-foreground hover:text-foreground',
              )}
            >
              {tab.label}
            </Link>
          );
        })}
      </nav>

      {children}
    </div>
  );
}
