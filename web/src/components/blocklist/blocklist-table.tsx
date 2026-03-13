'use client';

import { useState } from 'react';
import { useBlocklistEntries } from '@/hooks/use-blocklist';
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
import { Skeleton } from '@/components/ui/skeleton';
import { capitalize, formatDate } from '@/lib/utils';
import Link from 'next/link';
import { Search, Pencil } from 'lucide-react';

export function BlocklistTable() {
  const [search, setSearch] = useState('');
  const [category, setCategory] = useState<string>('');
  const [status, setStatus] = useState<string>('');
  const [page, setPage] = useState(1);

  const { data, isLoading, error } = useBlocklistEntries({
    search: search || undefined,
    category: category || undefined,
    status: status || undefined,
    page,
    per_page: 20,
  });

  return (
    <div className="space-y-4">
      {/* Filters */}
      <div className="flex flex-wrap gap-3">
        <div className="relative flex-1 min-w-[200px]">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search domains..."
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
            <SelectItem value="pending_review">Pending Review</SelectItem>
            <SelectItem value="inactive">Inactive</SelectItem>
            <SelectItem value="rejected">Rejected</SelectItem>
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
        <p className="text-destructive">Failed to load blocklist entries.</p>
      ) : (
        <>
          <div className="rounded-md border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Domain / Pattern</TableHead>
                  <TableHead>Category</TableHead>
                  <TableHead>Source</TableHead>
                  <TableHead>Confidence</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Added</TableHead>
                  <TableHead className="w-[50px]" />
                </TableRow>
              </TableHeader>
              <TableBody>
                {(data?.data ?? []).length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={7} className="text-center text-muted-foreground">
                      No entries found.
                    </TableCell>
                  </TableRow>
                ) : (
                  (data?.data ?? []).map((entry) => (
                    <TableRow key={entry.id}>
                      <TableCell className="font-mono text-sm">
                        {entry.domain ?? entry.pattern}
                      </TableCell>
                      <TableCell>
                        <Badge variant="outline">{capitalize(entry.category)}</Badge>
                      </TableCell>
                      <TableCell className="text-sm">{capitalize(entry.source)}</TableCell>
                      <TableCell className="text-sm">
                        {(entry.confidence * 100).toFixed(0)}%
                      </TableCell>
                      <TableCell>
                        <Badge
                          variant={
                            entry.status === 'active'
                              ? 'default'
                              : entry.status === 'pending_review'
                                ? 'secondary'
                                : 'destructive'
                          }
                        >
                          {capitalize(entry.status)}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-sm">{formatDate(entry.created_at)}</TableCell>
                      <TableCell>
                        <Button variant="ghost" size="sm" asChild>
                          <Link href={`/admin/blocklist/${entry.id}/edit`}>
                            <Pencil className="h-4 w-4" />
                          </Link>
                        </Button>
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
                Page {data.pagination.page} of {data.pagination.total_pages} ({data.pagination.total}{' '}
                entries)
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
    </div>
  );
}
