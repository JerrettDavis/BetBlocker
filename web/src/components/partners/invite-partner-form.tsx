'use client';

import { useState } from 'react';
import { useRouter } from 'next/navigation';
import { useInvitePartner } from '@/hooks/use-partners';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Checkbox } from '@/components/ui/checkbox';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Alert, AlertDescription } from '@/components/ui/alert';
import type { PartnerRole } from '@/lib/api-types';

export function InvitePartnerForm() {
  const router = useRouter();
  const invitePartner = useInvitePartner();

  const [email, setEmail] = useState('');
  const [role, setRole] = useState<PartnerRole>('accountability_partner');
  const [viewReports, setViewReports] = useState(true);
  const [approveUnenrollment, setApproveUnenrollment] = useState(true);
  const [modifyEnrollment, setModifyEnrollment] = useState(false);
  const [message, setMessage] = useState('');
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    try {
      await invitePartner.mutateAsync({
        email,
        role,
        permissions: {
          view_reports: viewReports,
          approve_unenrollment: approveUnenrollment,
          modify_enrollment: modifyEnrollment,
        },
        message: message || undefined,
      });
      router.push('/partners');
    } catch (err: unknown) {
      const apiErr = err as { code?: string; message?: string };
      if (apiErr.code === 'CANNOT_INVITE_SELF') {
        setError('You cannot invite yourself as a partner.');
      } else if (apiErr.code === 'PARTNER_ALREADY_INVITED') {
        setError('This person has already been invited.');
      } else if (apiErr.code === 'EMAIL_NOT_VERIFIED') {
        setError('Please verify your email before inviting partners.');
      } else {
        setError(apiErr.message ?? 'Failed to send invitation.');
      }
    }
  }

  return (
    <Card className="max-w-lg">
      <CardHeader>
        <CardTitle>Invite a Partner</CardTitle>
        <CardDescription>
          Invite a trusted person to be your accountability partner.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <form onSubmit={handleSubmit} className="space-y-4">
          {error && (
            <Alert variant="destructive">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          <div className="space-y-2">
            <Label htmlFor="email">Partner Email</Label>
            <Input
              id="email"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              required
            />
          </div>

          <div className="space-y-2">
            <Label>Role</Label>
            <Select value={role} onValueChange={(v) => v && setRole(v as PartnerRole)}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="accountability_partner">Accountability Partner</SelectItem>
                <SelectItem value="therapist">Therapist</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-3">
            <Label>Permissions</Label>
            <div className="flex items-center space-x-2">
              <Checkbox
                id="viewReports"
                checked={viewReports}
                onCheckedChange={(v) => setViewReports(!!v)}
              />
              <label htmlFor="viewReports" className="text-sm">
                Can view reports
              </label>
            </div>
            <div className="flex items-center space-x-2">
              <Checkbox
                id="approveUnenrollment"
                checked={approveUnenrollment}
                onCheckedChange={(v) => setApproveUnenrollment(!!v)}
              />
              <label htmlFor="approveUnenrollment" className="text-sm">
                Can approve unenrollment
              </label>
            </div>
            <div className="flex items-center space-x-2">
              <Checkbox
                id="modifyEnrollment"
                checked={modifyEnrollment}
                onCheckedChange={(v) => setModifyEnrollment(!!v)}
              />
              <label htmlFor="modifyEnrollment" className="text-sm">
                Can modify enrollment settings
              </label>
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="message">Personal Message (optional)</Label>
            <Textarea
              id="message"
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              placeholder="A brief message to include in the invitation email..."
              maxLength={500}
            />
          </div>

          <Button type="submit" className="w-full" disabled={invitePartner.isPending}>
            {invitePartner.isPending ? 'Sending...' : 'Send Invitation'}
          </Button>
        </form>
      </CardContent>
    </Card>
  );
}
