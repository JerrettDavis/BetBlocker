import type { Metadata } from 'next';
import { InvitePartnerForm } from '@/components/partners/invite-partner-form';

export const metadata: Metadata = {
  title: 'Invite Partner - BetBlocker',
};

export default function InvitePartnerPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold">Invite a Partner</h1>
        <p className="text-sm text-muted-foreground">
          Send an invitation to a trusted person.
        </p>
      </div>
      <InvitePartnerForm />
    </div>
  );
}
