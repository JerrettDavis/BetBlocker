'use client';

import { useState } from 'react';
import Link from 'next/link';
import { useOrganizations, useCreateOrganization } from '@/hooks/use-organizations';
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
import { capitalize, formatDate } from '@/lib/utils';
import { Plus, Building2 } from 'lucide-react';

export default function OrganizationsPage() {
  const { data, isLoading, error } = useOrganizations();
  const createOrg = useCreateOrganization();
  const [showCreate, setShowCreate] = useState(false);
  const [name, setName] = useState('');
  const [orgType, setOrgType] = useState('family');

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      await createOrg.mutateAsync({ name, org_type: orgType });
      setName('');
      setOrgType('family');
      setShowCreate(false);
    } catch {
      // error is available via createOrg.error
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Organizations</h1>
          <p className="text-sm text-muted-foreground">
            Manage your organizations and their members.
          </p>
        </div>
        <Dialog open={showCreate} onOpenChange={setShowCreate}>
          <DialogTrigger
            render={
              <Button>
                <Plus className="mr-2 h-4 w-4" />
                Create Organization
              </Button>
            }
          />
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Create Organization</DialogTitle>
            </DialogHeader>
            <form onSubmit={handleCreate} className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="org-name">Name</Label>
                <Input
                  id="org-name"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="Organization name"
                  required
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="org-type">Type</Label>
                <select
                  id="org-type"
                  value={orgType}
                  onChange={(e) => setOrgType(e.target.value)}
                  className="flex h-8 w-full rounded-lg border border-input bg-transparent px-2.5 py-1 text-sm"
                >
                  <option value="family">Family</option>
                  <option value="clinical">Clinical</option>
                  <option value="enterprise">Enterprise</option>
                  <option value="government">Government</option>
                </select>
              </div>
              {createOrg.error && (
                <p className="text-sm text-destructive">{createOrg.error.message}</p>
              )}
              <DialogFooter>
                <Button type="submit" disabled={createOrg.isPending}>
                  {createOrg.isPending ? 'Creating...' : 'Create'}
                </Button>
              </DialogFooter>
            </form>
          </DialogContent>
        </Dialog>
      </div>

      {isLoading && (
        <div className="space-y-4">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} className="h-16" />
          ))}
        </div>
      )}

      {error && <p className="text-destructive">Failed to load organizations.</p>}

      {data && data.data.length === 0 && (
        <div className="text-center py-12">
          <Building2 className="mx-auto h-12 w-12 text-muted-foreground" />
          <h3 className="mt-4 text-lg font-semibold">No organizations yet</h3>
          <p className="mt-2 text-sm text-muted-foreground">
            Create an organization to manage devices and members together.
          </p>
        </div>
      )}

      {data && data.data.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Your Organizations</CardTitle>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Type</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {data.data.map((org) => (
                  <TableRow key={org.id}>
                    <TableCell className="font-medium">{org.name}</TableCell>
                    <TableCell>
                      <Badge variant="outline">{capitalize(org.org_type)}</Badge>
                    </TableCell>
                    <TableCell className="text-muted-foreground">
                      {formatDate(org.created_at)}
                    </TableCell>
                    <TableCell>
                      <Button variant="ghost" size="sm" asChild>
                        <Link href={`/organizations/${org.id}`}>View</Link>
                      </Button>
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
