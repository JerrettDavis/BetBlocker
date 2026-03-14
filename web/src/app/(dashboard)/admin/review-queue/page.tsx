'use client';

import { useState } from 'react';
import {
  useReviewQueueList,
  useApproveReviewItem,
  useRejectReviewItem,
  useBulkApproveReviewItems,
  useBulkRejectReviewItems,
} from '@/hooks/use-review-queue';
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
import { Checkbox } from '@/components/ui/checkbox';
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
import { Search, Check, X, Eye, ChevronDown, ChevronUp } from 'lucide-react';
import Link from 'next/link';
import type { BlocklistCategory } from '@/lib/api-types';

const CATEGORIES: BlocklistCategory[] = [
  'online_casino',
  'sports_betting',
  'poker',
  'lottery',
  'bingo',
  'fantasy_sports',
  'crypto_gambling',
  'affiliate',
  'payment_processor',
  'other',
];

type SortField = 'confidence' | 'date';

interface ApprovModalState {
  open: boolean;
  domain: string | null;
  bulk: boolean;
  bulkDomains: string[];
}

export default function ReviewQueueAdminPage() {
  const [minConfidence, setMinConfidence] = useState(0);
  const [domainSearch, setDomainSearch] = useState('');
  const [sortField, setSortField] = useState<SortField>('confidence');
  const [sortAsc, setSortAsc] = useState(false);
  const [page, setPage] = useState(1);
  const [selectedDomains, setSelectedDomains] = useState<Set<string>>(new Set());
  const [approveModal, setApproveModal] = useState<ApprovModalState>({
    open: false,
    domain: null,
    bulk: false,
    bulkDomains: [],
  });
  const [selectedCategory, setSelectedCategory] = useState<BlocklistCategory>('online_casino');

  const { data, isLoading, error } = useReviewQueueList({
    min_confidence: minConfidence > 0 ? minConfidence / 100 : undefined,
    sort: sortField === 'confidence' ? 'confidence' : 'date',
    page,
    per_page: 20,
  });

  const approveItem = useApproveReviewItem();
  const rejectItem = useRejectReviewItem();
  const bulkApprove = useBulkApproveReviewItems();
  const bulkReject = useBulkRejectReviewItems();

  const items = data?.data ?? [];

  const filteredItems = domainSearch
    ? items.filter((item) =>
        item.domain.toLowerCase().includes(domainSearch.toLowerCase()),
      )
    : items;

  const sortedItems = [...filteredItems].sort((a, b) => {
    let cmp = 0;
    if (sortField === 'confidence') {
      cmp = a.aggregated_confidence - b.aggregated_confidence;
    } else {
      cmp = new Date(a.first_reported_at).getTime() - new Date(b.first_reported_at).getTime();
    }
    return sortAsc ? cmp : -cmp;
  });

  function toggleSort(field: SortField) {
    if (sortField === field) {
      setSortAsc(!sortAsc);
    } else {
      setSortField(field);
      setSortAsc(false);
    }
  }

  function toggleSelect(domain: string) {
    setSelectedDomains((prev) => {
      const next = new Set(prev);
      if (next.has(domain)) {
        next.delete(domain);
      } else {
        next.add(domain);
      }
      return next;
    });
  }

  function toggleSelectAll() {
    if (selectedDomains.size === sortedItems.length && sortedItems.length > 0) {
      setSelectedDomains(new Set());
    } else {
      setSelectedDomains(new Set(sortedItems.map((i) => i.domain)));
    }
  }

  function handleApprove(domain: string) {
    setApproveModal({ open: true, domain, bulk: false, bulkDomains: [] });
  }

  function handleBulkApprove() {
    setApproveModal({
      open: true,
      domain: null,
      bulk: true,
      bulkDomains: Array.from(selectedDomains),
    });
  }

  function handleConfirmApprove() {
    if (approveModal.bulk) {
      bulkApprove.mutate(
        { domains: approveModal.bulkDomains, category: selectedCategory },
        {
          onSuccess: () => {
            setApproveModal({ open: false, domain: null, bulk: false, bulkDomains: [] });
            setSelectedDomains(new Set());
          },
        },
      );
    } else if (approveModal.domain) {
      approveItem.mutate(
        { domain: approveModal.domain, category: selectedCategory },
        {
          onSuccess: () => {
            setApproveModal({ open: false, domain: null, bulk: false, bulkDomains: [] });
          },
        },
      );
    }
  }

  function handleReject(domain: string) {
    rejectItem.mutate({ domain });
  }

  function handleBulkReject() {
    bulkReject.mutate(Array.from(selectedDomains), {
      onSuccess: () => setSelectedDomains(new Set()),
    });
  }

  const SortIcon = ({ field }: { field: SortField }) => {
    if (sortField !== field) return null;
    return sortAsc ? (
      <ChevronUp className="inline h-3 w-3 ml-1" />
    ) : (
      <ChevronDown className="inline h-3 w-3 ml-1" />
    );
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Review Queue</h1>
          <p className="text-sm text-muted-foreground">
            Review discovery candidates and approve or reject them for the blocklist.
          </p>
        </div>
      </div>

      {/* Filters */}
      <div className="flex flex-wrap gap-3 items-center">
        <div className="relative flex-1 min-w-[200px]">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search domain..."
            value={domainSearch}
            onChange={(e) => {
              setDomainSearch(e.target.value);
              setPage(1);
            }}
            className="pl-9"
          />
        </div>

        <div className="flex items-center gap-2">
          <label className="text-sm text-muted-foreground whitespace-nowrap">
            Min confidence: {minConfidence}%
          </label>
          <input
            type="range"
            min={0}
            max={100}
            step={5}
            value={minConfidence}
            onChange={(e) => {
              setMinConfidence(Number(e.target.value));
              setPage(1);
            }}
            className="w-32"
          />
        </div>
      </div>

      {/* Bulk actions */}
      {selectedDomains.size > 0 && (
        <div className="flex items-center gap-3 p-3 bg-muted rounded-md">
          <span className="text-sm font-medium">{selectedDomains.size} selected</span>
          <Button
            size="sm"
            variant="outline"
            className="text-green-600 border-green-600 hover:bg-green-50"
            onClick={handleBulkApprove}
            disabled={bulkApprove.isPending}
          >
            <Check className="h-4 w-4 mr-1" />
            Bulk Approve
          </Button>
          <Button
            size="sm"
            variant="outline"
            className="text-red-600 border-red-600 hover:bg-red-50"
            onClick={handleBulkReject}
            disabled={bulkReject.isPending}
          >
            <X className="h-4 w-4 mr-1" />
            Bulk Reject
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => setSelectedDomains(new Set())}
          >
            Clear
          </Button>
        </div>
      )}

      {/* Table */}
      {isLoading ? (
        <div className="space-y-2">
          {Array.from({ length: 5 }).map((_, i) => (
            <Skeleton key={i} className="h-12" />
          ))}
        </div>
      ) : error ? (
        <p className="text-destructive">Failed to load review queue.</p>
      ) : (
        <>
          <div className="rounded-md border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-[40px]">
                    <Checkbox
                      checked={
                        sortedItems.length > 0 && selectedDomains.size === sortedItems.length
                      }
                      onCheckedChange={toggleSelectAll}
                    />
                  </TableHead>
                  <TableHead>Domain</TableHead>
                  <TableHead>Source</TableHead>
                  <TableHead
                    className="cursor-pointer select-none"
                    onClick={() => toggleSort('confidence')}
                  >
                    Confidence
                    <SortIcon field="confidence" />
                  </TableHead>
                  <TableHead>Category Guess</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead
                    className="cursor-pointer select-none"
                    onClick={() => toggleSort('date')}
                  >
                    First Reported
                    <SortIcon field="date" />
                  </TableHead>
                  <TableHead className="w-[130px]">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {sortedItems.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={8} className="text-center text-muted-foreground py-8">
                      No items pending review.
                    </TableCell>
                  </TableRow>
                ) : (
                  sortedItems.map((item) => (
                    <TableRow key={item.domain}>
                      <TableCell>
                        <Checkbox
                          checked={selectedDomains.has(item.domain)}
                          onCheckedChange={() => toggleSelect(item.domain)}
                        />
                      </TableCell>
                      <TableCell className="font-mono text-sm">{item.domain}</TableCell>
                      <TableCell className="text-sm text-muted-foreground">
                        {item.report_count} report{item.report_count !== 1 ? 's' : ''}
                      </TableCell>
                      <TableCell>
                        <Badge
                          variant={
                            item.aggregated_confidence >= 0.8
                              ? 'default'
                              : item.aggregated_confidence >= 0.5
                                ? 'secondary'
                                : 'outline'
                          }
                        >
                          {(item.aggregated_confidence * 100).toFixed(0)}%
                        </Badge>
                      </TableCell>
                      <TableCell>
                        <div className="flex flex-wrap gap-1">
                          {item.top_heuristic_matches.slice(0, 2).map((h) => (
                            <Badge key={h} variant="outline" className="text-xs">
                              {h}
                            </Badge>
                          ))}
                        </div>
                      </TableCell>
                      <TableCell>
                        <Badge variant="secondary">Pending</Badge>
                      </TableCell>
                      <TableCell className="text-sm">
                        {formatDate(item.first_reported_at)}
                      </TableCell>
                      <TableCell>
                        <div className="flex gap-1">
                          <Button
                            size="sm"
                            variant="ghost"
                            className="text-green-600"
                            title="Approve"
                            onClick={() => handleApprove(item.domain)}
                            disabled={approveItem.isPending}
                          >
                            <Check className="h-4 w-4" />
                          </Button>
                          <Button
                            size="sm"
                            variant="ghost"
                            className="text-red-600"
                            title="Reject"
                            onClick={() => handleReject(item.domain)}
                            disabled={rejectItem.isPending}
                          >
                            <X className="h-4 w-4" />
                          </Button>
                          <Button size="sm" variant="ghost" title="View details" asChild>
                            <Link
                              href={`/admin/review-queue/${encodeURIComponent(item.domain)}`}
                            >
                              <Eye className="h-4 w-4" />
                            </Link>
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
                {data.pagination.total} items)
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

      {/* Approve modal with category picker */}
      <Dialog
        open={approveModal.open}
        onOpenChange={(open) =>
          !open && setApproveModal({ open: false, domain: null, bulk: false, bulkDomains: [] })
        }
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {approveModal.bulk
                ? `Approve ${approveModal.bulkDomains.length} items`
                : `Approve: ${approveModal.domain}`}
            </DialogTitle>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div>
              <label className="text-sm font-medium mb-1 block">Category</label>
              <Select
                value={selectedCategory}
                onValueChange={(v) => setSelectedCategory(v as BlocklistCategory)}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {CATEGORIES.map((cat) => (
                    <SelectItem key={cat} value={cat}>
                      {capitalize(cat)}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() =>
                setApproveModal({ open: false, domain: null, bulk: false, bulkDomains: [] })
              }
            >
              Cancel
            </Button>
            <Button
              onClick={handleConfirmApprove}
              disabled={approveItem.isPending || bulkApprove.isPending}
            >
              Approve
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
