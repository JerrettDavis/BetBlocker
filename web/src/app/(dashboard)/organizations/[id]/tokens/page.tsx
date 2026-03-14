'use client';

import { useState } from 'react';
import { useParams } from 'next/navigation';
import { useOrgTokens, useCreateOrgToken, useRevokeOrgToken } from '@/hooks/use-organizations';
import { organizations } from '@/lib/api-client';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
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
import { Plus, Key, Trash2, Copy, QrCode } from 'lucide-react';

export default function OrgTokensPage() {
  const params = useParams();
  const orgId = params.id as string;

  const { data, isLoading, error } = useOrgTokens(orgId);
  const createToken = useCreateOrgToken();
  const revokeToken = useRevokeOrgToken();

  const [showCreate, setShowCreate] = useState(false);
  const [showQr, setShowQr] = useState<{ tokenId: number; label: string } | null>(null);
  const [label, setLabel] = useState('');
  const [maxUses, setMaxUses] = useState('');
  const [expiresAt, setExpiresAt] = useState('');
  const [copiedId, setCopiedId] = useState<number | null>(null);

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      await createToken.mutateAsync({
        orgId,
        data: {
          label: label || undefined,
          protection_config: {},
          reporting_config: {},
          unenrollment_policy: {},
          max_uses: maxUses ? parseInt(maxUses, 10) : undefined,
          expires_at: expiresAt || undefined,
        },
      });
      setLabel('');
      setMaxUses('');
      setExpiresAt('');
      setShowCreate(false);
    } catch {
      // error available via createToken.error
    }
  };

  const handleRevoke = (tokenId: number) => {
    if (confirm('Are you sure you want to revoke this token?')) {
      revokeToken.mutate({ orgId, tokenId });
    }
  };

  const handleCopyLink = async (publicId: string, tokenId: number) => {
    const url = `${window.location.origin}/enroll/${publicId}`;
    try {
      await navigator.clipboard.writeText(url);
      setCopiedId(tokenId);
      setTimeout(() => setCopiedId(null), 2000);
    } catch {
      // fallback - ignore
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">Enrollment Tokens</h2>
        <Dialog open={showCreate} onOpenChange={setShowCreate}>
          <DialogTrigger
            render={
              <Button size="sm">
                <Plus className="mr-2 h-4 w-4" />
                Create Token
              </Button>
            }
          />
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Create Enrollment Token</DialogTitle>
            </DialogHeader>
            <form onSubmit={handleCreate} className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="token-label">Label (optional)</Label>
                <Input
                  id="token-label"
                  value={label}
                  onChange={(e) => setLabel(e.target.value)}
                  placeholder="e.g., School enrollment"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="token-max-uses">Max Uses (optional)</Label>
                <Input
                  id="token-max-uses"
                  type="number"
                  value={maxUses}
                  onChange={(e) => setMaxUses(e.target.value)}
                  placeholder="Unlimited"
                  min="1"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="token-expires">Expires At (optional)</Label>
                <Input
                  id="token-expires"
                  type="datetime-local"
                  value={expiresAt}
                  onChange={(e) => setExpiresAt(e.target.value)}
                />
              </div>
              {createToken.error && (
                <p className="text-sm text-destructive">{createToken.error.message}</p>
              )}
              <DialogFooter>
                <Button type="submit" disabled={createToken.isPending}>
                  {createToken.isPending ? 'Creating...' : 'Create'}
                </Button>
              </DialogFooter>
            </form>
          </DialogContent>
        </Dialog>
      </div>

      {/* QR Code Dialog */}
      <Dialog open={!!showQr} onOpenChange={() => setShowQr(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>QR Code{showQr?.label ? `: ${showQr.label}` : ''}</DialogTitle>
          </DialogHeader>
          {showQr && (
            <div className="flex justify-center p-4">
              {/* eslint-disable-next-line @next/next/no-img-element */}
              <img
                src={organizations.getTokenQrUrl(orgId, showQr.tokenId)}
                alt="Enrollment QR Code"
                className="w-64 h-64"
              />
            </div>
          )}
          <DialogFooter showCloseButton />
        </DialogContent>
      </Dialog>

      {isLoading && (
        <div className="space-y-4">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} className="h-12" />
          ))}
        </div>
      )}

      {error && <p className="text-destructive">Failed to load tokens.</p>}

      {data && data.data.length === 0 && (
        <div className="text-center py-12">
          <Key className="mx-auto h-12 w-12 text-muted-foreground" />
          <h3 className="mt-4 text-lg font-semibold">No enrollment tokens</h3>
          <p className="mt-2 text-sm text-muted-foreground">
            Create an enrollment token to allow devices to join this organization.
          </p>
        </div>
      )}

      {data && data.data.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Tokens</CardTitle>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Label</TableHead>
                  <TableHead>Public ID</TableHead>
                  <TableHead>Uses</TableHead>
                  <TableHead>Expires</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {data.data.map((token) => (
                  <TableRow key={token.id}>
                    <TableCell className="font-medium">
                      {token.label ?? '-'}
                    </TableCell>
                    <TableCell>
                      <code className="text-xs bg-muted px-1 py-0.5 rounded">
                        {token.public_id}
                      </code>
                    </TableCell>
                    <TableCell>
                      {token.uses_count}
                      {token.max_uses != null && ` / ${token.max_uses}`}
                    </TableCell>
                    <TableCell className="text-muted-foreground">
                      {token.expires_at ? formatDate(token.expires_at) : 'Never'}
                    </TableCell>
                    <TableCell className="text-muted-foreground">
                      {formatDate(token.created_at)}
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-1">
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleCopyLink(token.public_id, token.id)}
                          title="Copy enrollment link"
                        >
                          <Copy className="h-4 w-4" />
                          {copiedId === token.id && (
                            <span className="text-xs text-green-600">Copied</span>
                          )}
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() =>
                            setShowQr({ tokenId: token.id, label: token.label ?? '' })
                          }
                          title="Show QR code"
                        >
                          <QrCode className="h-4 w-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleRevoke(token.id)}
                          disabled={revokeToken.isPending}
                          title="Revoke token"
                        >
                          <Trash2 className="h-4 w-4 text-destructive" />
                        </Button>
                      </div>
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
