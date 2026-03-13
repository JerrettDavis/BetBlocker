'use client';

import { useState } from 'react';
import { useRequestUnenroll } from '@/hooks/use-enrollments';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Textarea } from '@/components/ui/textarea';
import { Label } from '@/components/ui/label';
import { Alert, AlertDescription } from '@/components/ui/alert';
import type { EnrollmentTier } from '@/lib/api-types';

interface UnenrollDialogProps {
  enrollmentId: string;
  tier: EnrollmentTier;
  onSuccess?: () => void;
}

export function UnenrollDialog({ enrollmentId, tier, onSuccess }: UnenrollDialogProps) {
  const [open, setOpen] = useState(false);
  const [reason, setReason] = useState('');
  const [error, setError] = useState<string | null>(null);
  const requestUnenroll = useRequestUnenroll();

  async function handleConfirm() {
    setError(null);
    try {
      await requestUnenroll.mutateAsync({
        id: enrollmentId,
        reason: reason || undefined,
      });
      setOpen(false);
      onSuccess?.();
    } catch (err: unknown) {
      const apiErr = err as { message?: string };
      setError(apiErr.message ?? 'Failed to request unenrollment.');
    }
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <Button variant="destructive" onClick={() => setOpen(true)}>
        Request Unenrollment
      </Button>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Request Unenrollment</DialogTitle>
          <DialogDescription>
            {tier === 'self'
              ? 'A cooldown period will apply before unenrollment is completed. This gives you time to reconsider.'
              : 'Your accountability partner will be notified and must approve this request.'}
          </DialogDescription>
        </DialogHeader>

        {error && (
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}

        <div className="space-y-2">
          <Label htmlFor="reason">Reason (optional)</Label>
          <Textarea
            id="reason"
            value={reason}
            onChange={(e) => setReason(e.target.value)}
            placeholder="Why do you want to unenroll?"
            maxLength={500}
          />
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => setOpen(false)}>
            Cancel
          </Button>
          <Button
            variant="destructive"
            onClick={handleConfirm}
            disabled={requestUnenroll.isPending}
          >
            {requestUnenroll.isPending ? 'Requesting...' : 'Confirm Unenrollment'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
