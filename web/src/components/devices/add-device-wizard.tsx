'use client';

import { useState } from 'react';
import { useCreateEnrollment } from '@/hooks/use-enrollments';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Checkbox } from '@/components/ui/checkbox';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Copy, Check } from 'lucide-react';

type Step = 'configure' | 'token' | 'instructions';

export function AddDeviceWizard() {
  const [step, setStep] = useState<Step>('configure');
  const [tier, setTier] = useState('self');
  const [dnsBlocking, setDnsBlocking] = useState(true);
  const [tamperResponse, setTamperResponse] = useState('alert_user');
  const [enrollmentId, setEnrollmentId] = useState('');
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const createEnrollment = useCreateEnrollment();

  async function handleCreate() {
    setError(null);
    try {
      const res = await createEnrollment.mutateAsync({
        device_id: 'pending',
        tier,
        protection_config: {
          dns_blocking: dnsBlocking,
          tamper_response: tamperResponse as 'log' | 'alert_user' | 'alert_partner' | 'alert_authority',
        },
      });
      setEnrollmentId(res.data.id);
      setStep('token');
    } catch (err: unknown) {
      const apiErr = err as { message?: string };
      setError(apiErr.message ?? 'Failed to create enrollment.');
    }
  }

  function handleCopy() {
    navigator.clipboard.writeText(enrollmentId);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  return (
    <div className="max-w-2xl mx-auto">
      {/* Steps indicator */}
      <div className="flex items-center justify-center gap-4 mb-8">
        {[
          { key: 'configure', label: '1. Configure' },
          { key: 'token', label: '2. Token' },
          { key: 'instructions', label: '3. Install' },
        ].map((s) => (
          <div
            key={s.key}
            className={`text-sm font-medium px-3 py-1 rounded-full ${
              step === s.key
                ? 'bg-primary text-primary-foreground'
                : 'bg-muted text-muted-foreground'
            }`}
          >
            {s.label}
          </div>
        ))}
      </div>

      {step === 'configure' && (
        <Card>
          <CardHeader>
            <CardTitle>Configure Protection</CardTitle>
            <CardDescription>
              Choose the protection level for your new device.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            {error && (
              <Alert variant="destructive">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}

            <div className="space-y-2">
              <Label>Enrollment Tier</Label>
              <Select value={tier} onValueChange={(v) => v && setTier(v)}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="self">Self - Time-delayed unenrollment</SelectItem>
                  <SelectItem value="partner">
                    Partner - Requires partner approval to unenroll
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-4">
              <Label>Protection Features</Label>
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="dns"
                  checked={dnsBlocking}
                  onCheckedChange={(v) => setDnsBlocking(!!v)}
                />
                <label htmlFor="dns" className="text-sm">
                  DNS-level blocking
                </label>
              </div>
            </div>

            <div className="space-y-2">
              <Label>Tamper Response</Label>
              <Select value={tamperResponse} onValueChange={(v) => v && setTamperResponse(v)}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="log">Log only</SelectItem>
                  <SelectItem value="alert_user">Alert user</SelectItem>
                  <SelectItem value="alert_partner">Alert partner</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <Button onClick={handleCreate} disabled={createEnrollment.isPending} className="w-full">
              {createEnrollment.isPending ? 'Creating...' : 'Create Enrollment'}
            </Button>
          </CardContent>
        </Card>
      )}

      {step === 'token' && (
        <Card>
          <CardHeader>
            <CardTitle>Enrollment Token</CardTitle>
            <CardDescription>
              Use this token when installing the BetBlocker agent on your device.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            <div className="flex items-center gap-2 rounded-md border bg-muted p-4">
              <code className="flex-1 text-sm break-all font-mono">{enrollmentId}</code>
              <Button variant="ghost" size="sm" onClick={handleCopy}>
                {copied ? (
                  <Check className="h-4 w-4 text-green-600" />
                ) : (
                  <Copy className="h-4 w-4" />
                )}
              </Button>
            </div>
            <Alert>
              <AlertDescription>
                Keep this token secure. It will be needed during device setup.
              </AlertDescription>
            </Alert>
            <Button onClick={() => setStep('instructions')} className="w-full">
              Next: Install Instructions
            </Button>
          </CardContent>
        </Card>
      )}

      {step === 'instructions' && (
        <Card>
          <CardHeader>
            <CardTitle>Install BetBlocker Agent</CardTitle>
            <CardDescription>
              Follow the instructions for your platform.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            <div>
              <h3 className="font-semibold mb-2">Linux</h3>
              <div className="rounded-md bg-muted p-4">
                <code className="text-sm">
                  curl -fsSL https://get.betblocker.com | sh -s -- --token {enrollmentId}
                </code>
              </div>
            </div>

            <div>
              <h3 className="font-semibold mb-2">macOS</h3>
              <p className="text-sm text-muted-foreground">
                macOS agent coming soon. Use the enrollment token when it becomes available.
              </p>
            </div>

            <div>
              <h3 className="font-semibold mb-2">Windows</h3>
              <p className="text-sm text-muted-foreground">
                Windows agent coming soon. Use the enrollment token when it becomes available.
              </p>
            </div>

            <div>
              <h3 className="font-semibold mb-2">Android / iOS</h3>
              <p className="text-sm text-muted-foreground">
                Mobile agents coming soon. You will be able to scan a QR code during setup.
              </p>
            </div>

            <Button variant="outline" asChild className="w-full">
              <a href="/devices">Go to Devices</a>
            </Button>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
