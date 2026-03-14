'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { useAuth } from '@/lib/use-auth';
import { cn } from '@/lib/utils';
import {
  LayoutDashboard,
  Monitor,
  Users,
  HandHeart,
  ListFilter,
  BarChart3,
  Shield,
  Building2,
  ClipboardList,
  Smartphone,
} from 'lucide-react';

const navItems = [
  { href: '/dashboard', label: 'Dashboard', icon: LayoutDashboard },
  { href: '/devices', label: 'Devices', icon: Monitor },
  { href: '/partners', label: 'Partners', icon: Users },
  { href: '/partner-dashboard', label: 'Partner Dashboard', icon: HandHeart, partnerOnly: true },
  { href: '/organizations', label: 'Organizations', icon: Building2 },
  { href: '/reports', label: 'Reports', icon: BarChart3 },
];

const adminItems = [
  { href: '/admin/blocklist', label: 'Blocklist', icon: ListFilter, section: 'Blocklist' },
  { href: '/admin/review-queue', label: 'Review Queue', icon: ClipboardList, section: 'Blocklist' },
  { href: '/admin/app-signatures', label: 'App Signatures', icon: Smartphone, section: 'Blocking' },
];

export function DashboardSidebar() {
  const pathname = usePathname();
  const { user } = useAuth();
  const isAdmin = user?.role === 'admin';

  return (
    <aside className="hidden lg:flex w-64 flex-col border-r bg-muted/30">
      <div className="flex h-16 items-center gap-2 border-b px-6 font-bold text-lg">
        <Shield className="h-5 w-5 text-primary" />
        BetBlocker
      </div>

      <nav className="flex-1 space-y-1 p-4">
        {navItems.map((item) => {
          if (item.partnerOnly && user?.role !== 'partner' && user?.role !== 'admin') {
            return null;
          }
          const isActive = pathname === item.href || pathname.startsWith(item.href + '/');
          return (
            <Link
              key={item.href}
              href={item.href}
              className={cn(
                'flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors',
                isActive
                  ? 'bg-primary text-primary-foreground'
                  : 'text-muted-foreground hover:bg-muted hover:text-foreground',
              )}
            >
              <item.icon className="h-4 w-4" />
              {item.label}
            </Link>
          );
        })}

        {isAdmin && (
          <>
            <div className="pt-4 pb-2">
              <p className="px-3 text-xs font-semibold uppercase text-muted-foreground">Admin</p>
            </div>
            {/* Blocklist section */}
            <div className="pb-1">
              <p className="px-3 text-xs text-muted-foreground/70 mb-1">Blocklist</p>
            </div>
            {adminItems
              .filter((item) => item.section === 'Blocklist')
              .map((item) => {
                const isActive = pathname === item.href || pathname.startsWith(item.href + '/');
                return (
                  <Link
                    key={item.href}
                    href={item.href}
                    className={cn(
                      'flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors',
                      isActive
                        ? 'bg-primary text-primary-foreground'
                        : 'text-muted-foreground hover:bg-muted hover:text-foreground',
                    )}
                  >
                    <item.icon className="h-4 w-4" />
                    {item.label}
                  </Link>
                );
              })}
            {/* Blocking section */}
            <div className="pt-3 pb-1">
              <p className="px-3 text-xs text-muted-foreground/70 mb-1">Blocking</p>
            </div>
            {adminItems
              .filter((item) => item.section === 'Blocking')
              .map((item) => {
                const isActive = pathname === item.href || pathname.startsWith(item.href + '/');
                return (
                  <Link
                    key={item.href}
                    href={item.href}
                    className={cn(
                      'flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors',
                      isActive
                        ? 'bg-primary text-primary-foreground'
                        : 'text-muted-foreground hover:bg-muted hover:text-foreground',
                    )}
                  >
                    <item.icon className="h-4 w-4" />
                    {item.label}
                  </Link>
                );
              })}
          </>
        )}
      </nav>
    </aside>
  );
}
