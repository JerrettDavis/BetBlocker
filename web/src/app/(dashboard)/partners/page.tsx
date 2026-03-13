import type { Metadata } from 'next';
import { PartnerList } from '@/components/partners/partner-list';
import { Button } from '@/components/ui/button';
import Link from 'next/link';
import { UserPlus } from 'lucide-react';

export const metadata: Metadata = {
  title: 'Partners - BetBlocker',
};

export default function PartnersPage() {
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Partners</h1>
          <p className="text-sm text-muted-foreground">
            Manage your accountability partners.
          </p>
        </div>
        <Button asChild>
          <Link href="/partners/invite">
            <UserPlus className="mr-2 h-4 w-4" />
            Invite Partner
          </Link>
        </Button>
      </div>
      <PartnerList />
    </div>
  );
}
