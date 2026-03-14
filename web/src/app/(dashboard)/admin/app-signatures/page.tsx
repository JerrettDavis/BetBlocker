'use client';

import { useState } from 'react';
import { useAppSignatures, useDeleteAppSignature } from '@/hooks/use-app-signatures';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import { Skeleton } from '@/components/ui/skeleton';
import { capitalize, formatDate } from '@/lib/utils';
import { Search, Plus, Pencil, Trash2 } from 'lucide-react';
import Link from 'next/link';
import type { AppSignature, AppSignaturePlatform } from '@/lib/api-types';

const PLATFORM_BADGE_COLORS: Record<AppSignaturePlatform, string> = {
  windows: 'bg-blue-100 text-blue-800',
  macos: 'bg-gray-100 text-gray-800',
  linux: 'bg-orange-100 text-orange-800',
  android: 'bg-green-100 text-green-800',
  ios: 'bg-purple-100 text-purple-800',
  all: 'bg-slate-100 text-slate-800',
};

function truncateList(items: string[], max = 2): string {
  if (items.length === 0) return '—';
  const shown = items.slice(0, max).join(', ');
  return items.length > max ? `${shown} +${items.length - max}` : shown;
}

export default function AppSignaturesPage() {
  const [search, setSearch] = useState('');
  const [category, setCategory] = useState('');
  const [platform, setPlatform] = useState('');
  const [status, setStatus] = useState('');
  const [page, setPage] = useState(1);
  const [deleteTarget, setDeleteTarget] = useState<AppSignature | null>(null);

  const { data, isLoading, error } = useAppSignatures({
    search: search || undefined,
    category: category || undefined,
    platform: platform || undefined,
    status: status || undefined,
    page,
    per_page: 20,
  });

  const deleteSignature = useDeleteAppSignature();

  function handleDelete() {
    if (!deleteTarget) return;
    deleteSignature.mutate(deleteTarget.id, {
      onSuccess: () => setDeleteTarget(null),
    });
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">App Signatures</h1>
          <p className="text-sm text-muted-foreground">
            Manage gambling app signatures for blocking.
          </p>
        </div>
        <Button asChild>
          <Link href="/admin/app-signatures/new">
            <Plus className="h-4 w-4 mr-2" />
            New Signature
          </Link>
        </Button>
      </div>

      {/* Filters */}
      <div className="flex flex-wrap gap-3">
        <div className="relative flex-1 min-w-[200px]">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search name, package, executable..."
            value={search}
            onChange={(e) => {
              setSearch(e.target.value);
              setPage(1);
            }}
            className="pl-9"
          />
        </div>

        <Select
          value={category}
          onValueChange={(v) => {
            setCategory(v === 'all' ? '' : (v ?? ''));
            setPage(1);
          }}
        >
          <SelectTrigger className="w-[180px]">
            <SelectValue placeholder="Category" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Categories</SelectItem>
            <SelectItem value="online_casino">Online Casino</SelectItem>
            <SelectItem value="sports_betting">Sports Betting</SelectItem>
            <SelectItem value="poker">Poker</SelectItem>
            <SelectItem value="lottery">Lottery</SelectItem>
            <SelectItem value="bingo">Bingo</SelectItem>
            <SelectItem value="fantasy_sports">Fantasy Sports</SelectItem>
            <SelectItem value="crypto_gambling">Crypto Gambling</SelectItem>
            <SelectItem value="affiliate">Affiliate</SelectItem>
            <SelectItem value="other">Other</SelectItem>
          </SelectContent>
        </Select>

        <Select
          value={platform}
          onValueChange={(v) => {
            setPlatform(v === 'all' ? '' : (v ?? ''));
            setPage(1);
          }}
        >
          <SelectTrigger className="w-[150px]">
            <SelectValue placeholder="Platform" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Platforms</SelectItem>
            <SelectItem value="windows">Windows</SelectItem>
            <SelectItem value="macos">macOS</SelectItem>
            <SelectItem value="linux">Linux</SelectItem>
            <SelectItem value="android">Android</SelectItem>
            <SelectItem value="ios">iOS</SelectItem>
          </SelectContent>
        </Select>

        <Select
          value={status}
          onValueChange={(v) => {
            setStatus(v === 'all' ? '' : (v ?? ''));
            setPage(1);
          }}
        >
          <SelectTrigger className="w-[160px]">
            <SelectValue placeholder="Status" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Statuses</SelectItem>
            <SelectItem value="active">Active</SelectItem>
            <SelectItem value="inactive">Inactive</SelectItem>
            <SelectItem value="pending_review">Pending Review</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* Table */}
      {isLoading ? (
        <div className="space-y-2">
          {Array.from({ length: 5 }).map((_, i) => (
            <Skeleton key={i} className="h-12" />
          ))}
        </div>
      ) : error ? (
        <p className="text-destructive">Failed to load app signatures.</p>
      ) : (
        <>
          <div className="rounded-md border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Platforms</TableHead>
                  <TableHead>Category</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Package Names</TableHead>
                  <TableHead>Executable Names</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead className="w-[80px]" />
                </TableRow>
              </TableHeader>
              <TableBody>
                {(data?.data ?? []).length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={8} className="text-center text-muted-foreground py-8">
                      No app signatures found.
                    </TableCell>
                  </TableRow>
                ) : (
                  (data?.data ?? []).map((sig) => (
                    <TableRow key={sig.id}>
                      <TableCell className="font-medium">{sig.name}</TableCell>
                      <TableCell>
                        <div className="flex flex-wrap gap-1">
                          {sig.platforms.length === 0 ? (
                            <span className="text-muted-foreground text-sm">—</span>
                          ) : (
                            sig.platforms.map((p) => (
                              <span
                                key={p}
                                className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${PLATFORM_BADGE_COLORS[p] ?? 'bg-gray-100 text-gray-800'}`}
                              >
                                {p}
                              </span>
                            ))
                          )}
                        </div>
                      </TableCell>
                      <TableCell>
                        <Badge variant="outline">{capitalize(sig.category)}</Badge>
                      </TableCell>
                      <TableCell>
                        <Badge
                          variant={
                            sig.status === 'active'
                              ? 'default'
                              : sig.status === 'pending_review'
                                ? 'secondary'
                                : 'outline'
                          }
                        >
                          {capitalize(sig.status)}
                        </Badge>
                      </TableCell>
                      <TableCell className="font-mono text-xs text-muted-foreground max-w-[160px] truncate">
                        {truncateList(sig.package_names)}
                      </TableCell>
                      <TableCell className="font-mono text-xs text-muted-foreground max-w-[160px] truncate">
                        {truncateList(sig.executable_names)}
                      </TableCell>
                      <TableCell className="text-sm">{formatDate(sig.created_at)}</TableCell>
                      <TableCell>
                        <div className="flex gap-1">
                          <Button variant="ghost" size="sm" asChild>
                            <Link href={`/admin/app-signatures/${sig.id}`}>
                              <Pencil className="h-4 w-4" />
                            </Link>
                          </Button>
                          <Button
                            variant="ghost"
                            size="sm"
                            className="text-destructive"
                            onClick={() => setDeleteTarget(sig)}
                          >
                            <Trash2 className="h-4 w-4" />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  ))
                )}
              </TableBody>
            </Table>
          </div>

          {/* Pagination */}
          {data?.pagination && data.pagination.total_pages > 1 && (
            <div className="flex items-center justify-between">
              <p className="text-sm text-muted-foreground">
                Page {data.pagination.page} of {data.pagination.total_pages} (
                {data.pagination.total} signatures)
              </p>
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  disabled={page <= 1}
                  onClick={() => setPage(page - 1)}
                >
                  Previous
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  disabled={page >= data.pagination.total_pages}
                  onClick={() => setPage(page + 1)}
                >
                  Next
                </Button>
              </div>
            </div>
          )}
        </>
      )}

      {/* Delete confirmation dialog */}
      <Dialog open={!!deleteTarget} onOpenChange={(open) => !open && setDeleteTarget(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete App Signature</DialogTitle>
          </DialogHeader>
          <p className="text-sm text-muted-foreground">
            Are you sure you want to delete{' '}
            <span className="font-medium text-foreground">{deleteTarget?.name}</span>? This
            action cannot be undone.
          </p>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteTarget(null)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={handleDelete}
              disabled={deleteSignature.isPending}
            >
              {deleteSignature.isPending ? 'Deleting...' : 'Delete'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
