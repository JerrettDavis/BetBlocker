'use client';

import { useState } from 'react';
import { useParams } from 'next/navigation';
import {
  useOrgMembers,
  useInviteOrgMember,
  useUpdateMemberRole,
  useRemoveOrgMember,
} from '@/hooks/use-organizations';
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
import { Skeleton } from '@/components/ui/skeleton';
import { capitalize, formatDate } from '@/lib/utils';
import { UserPlus, Trash2, Users } from 'lucide-react';
import type { OrgMemberRole } from '@/lib/api-types';

export default function OrgMembersPage() {
  const params = useParams();
  const orgId = params.id as string;

  const { data, isLoading, error } = useOrgMembers(orgId);
  const inviteMember = useInviteOrgMember();
  const updateRole = useUpdateMemberRole();
  const removeMember = useRemoveOrgMember();

  const [email, setEmail] = useState('');
  const [role, setRole] = useState<OrgMemberRole>('member');

  const handleInvite = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      await inviteMember.mutateAsync({ orgId, data: { email, role } });
      setEmail('');
      setRole('member');
    } catch {
      // error available via inviteMember.error
    }
  };

  const handleRoleChange = (memberId: number, newRole: OrgMemberRole) => {
    updateRole.mutate({ orgId, memberId: String(memberId), role: newRole });
  };

  const handleRemove = (memberId: number) => {
    if (confirm('Are you sure you want to remove this member?')) {
      removeMember.mutate({ orgId, memberId: String(memberId) });
    }
  };

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <UserPlus className="h-5 w-5" />
            Invite Member
          </CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleInvite} className="flex items-end gap-4">
            <div className="flex-1 space-y-2">
              <Label htmlFor="invite-email">Email</Label>
              <Input
                id="invite-email"
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="member@example.com"
                required
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="invite-role">Role</Label>
              <select
                id="invite-role"
                value={role}
                onChange={(e) => setRole(e.target.value as OrgMemberRole)}
                className="flex h-8 rounded-lg border border-input bg-transparent px-2.5 py-1 text-sm"
              >
                <option value="member">Member</option>
                <option value="admin">Admin</option>
                <option value="owner">Owner</option>
              </select>
            </div>
            <Button type="submit" disabled={inviteMember.isPending}>
              {inviteMember.isPending ? 'Inviting...' : 'Invite'}
            </Button>
          </form>
          {inviteMember.error && (
            <p className="mt-2 text-sm text-destructive">{inviteMember.error.message}</p>
          )}
        </CardContent>
      </Card>

      {isLoading && (
        <div className="space-y-4">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} className="h-12" />
          ))}
        </div>
      )}

      {error && <p className="text-destructive">Failed to load members.</p>}

      {data && data.data.length === 0 && (
        <div className="text-center py-12">
          <Users className="mx-auto h-12 w-12 text-muted-foreground" />
          <h3 className="mt-4 text-lg font-semibold">No members yet</h3>
          <p className="mt-2 text-sm text-muted-foreground">
            Invite members to your organization using the form above.
          </p>
        </div>
      )}

      {data && data.data.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Members</CardTitle>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Email</TableHead>
                  <TableHead>Role</TableHead>
                  <TableHead>Joined</TableHead>
                  <TableHead></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {data.data.map((member) => (
                  <TableRow key={member.id}>
                    <TableCell className="font-medium">
                      {member.display_name ?? 'Unknown'}
                    </TableCell>
                    <TableCell className="text-muted-foreground">
                      {member.email ?? '-'}
                    </TableCell>
                    <TableCell>
                      <select
                        value={member.role}
                        onChange={(e) =>
                          handleRoleChange(member.id, e.target.value as OrgMemberRole)
                        }
                        className="rounded-lg border border-input bg-transparent px-2 py-1 text-sm"
                        disabled={updateRole.isPending}
                      >
                        <option value="member">Member</option>
                        <option value="admin">Admin</option>
                        <option value="owner">Owner</option>
                      </select>
                    </TableCell>
                    <TableCell className="text-muted-foreground">
                      {formatDate(member.joined_at)}
                    </TableCell>
                    <TableCell>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleRemove(member.id)}
                        disabled={removeMember.isPending}
                      >
                        <Trash2 className="h-4 w-4 text-destructive" />
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
