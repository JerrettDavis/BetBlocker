'use client';

import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Checkbox } from '@/components/ui/checkbox';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { capitalize } from '@/lib/utils';
import { Plus, X } from 'lucide-react';
import type { AppSignature, AppSignaturePlatform, BlocklistCategory } from '@/lib/api-types';

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

const PLATFORMS: AppSignaturePlatform[] = ['windows', 'macos', 'linux', 'android', 'ios', 'all'];

const STATUS_OPTIONS = ['active', 'inactive', 'pending_review'];

export interface AppSignatureFormData {
  name: string;
  platforms: AppSignaturePlatform[];
  category: BlocklistCategory;
  status: string;
  confidence: number;
  evidence_url: string;
  tags: string[];
  package_names: string[];
  executable_names: string[];
  cert_hashes: string[];
  display_name_patterns: string[];
}

interface AppSignatureFormProps {
  initialData?: Partial<AppSignature>;
  onSubmit: (data: AppSignatureFormData) => void;
  isSubmitting?: boolean;
  mode: 'create' | 'edit';
}

function ArrayField({
  label,
  values,
  onChange,
  placeholder,
}: {
  label: string;
  values: string[];
  onChange: (values: string[]) => void;
  placeholder?: string;
}) {
  function addItem() {
    onChange([...values, '']);
  }

  function removeItem(index: number) {
    onChange(values.filter((_, i) => i !== index));
  }

  function updateItem(index: number, value: string) {
    onChange(values.map((v, i) => (i === index ? value : v)));
  }

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <Label>{label}</Label>
        <Button type="button" variant="outline" size="sm" onClick={addItem}>
          <Plus className="h-3 w-3 mr-1" />
          Add
        </Button>
      </div>
      {values.length === 0 && (
        <p className="text-sm text-muted-foreground">No entries. Click Add to add one.</p>
      )}
      {values.map((value, index) => (
        <div key={index} className="flex gap-2">
          <Input
            value={value}
            onChange={(e) => updateItem(index, e.target.value)}
            placeholder={placeholder}
            className="flex-1"
          />
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => removeItem(index)}
            className="text-destructive"
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
      ))}
    </div>
  );
}

export function AppSignatureForm({
  initialData,
  onSubmit,
  isSubmitting,
  mode,
}: AppSignatureFormProps) {
  const [name, setName] = useState(initialData?.name ?? '');
  const [platforms, setPlatforms] = useState<AppSignaturePlatform[]>(
    initialData?.platforms ?? [],
  );
  const [category, setCategory] = useState<BlocklistCategory>(
    initialData?.category ?? 'online_casino',
  );
  const [status, setStatus] = useState<'active' | 'inactive' | 'pending_review'>(
    (initialData?.status as 'active' | 'inactive' | 'pending_review') ?? 'pending_review',
  );
  const [confidence, setConfidence] = useState(
    initialData?.confidence !== undefined ? Math.round(initialData.confidence * 100) : 80,
  );
  const [evidenceUrl, setEvidenceUrl] = useState(initialData?.evidence_url ?? '');
  const [tags, setTags] = useState<string[]>(initialData?.tags ?? []);
  const [packageNames, setPackageNames] = useState<string[]>(initialData?.package_names ?? []);
  const [executableNames, setExecutableNames] = useState<string[]>(
    initialData?.executable_names ?? [],
  );
  const [certHashes, setCertHashes] = useState<string[]>(initialData?.cert_hashes ?? []);
  const [displayNamePatterns, setDisplayNamePatterns] = useState<string[]>(
    initialData?.display_name_patterns ?? [],
  );
  const [errors, setErrors] = useState<Record<string, string>>({});

  function togglePlatform(platform: AppSignaturePlatform) {
    setPlatforms((prev) =>
      prev.includes(platform) ? prev.filter((p) => p !== platform) : [...prev, platform],
    );
  }

  function validate(): boolean {
    const newErrors: Record<string, string> = {};
    if (!name.trim()) {
      newErrors['name'] = 'Name is required.';
    }
    const hasIdentifier =
      packageNames.some((v) => v.trim()) ||
      executableNames.some((v) => v.trim()) ||
      certHashes.some((v) => v.trim()) ||
      displayNamePatterns.some((v) => v.trim());
    if (!hasIdentifier) {
      newErrors['identifiers'] =
        'At least one identifier (package name, executable name, cert hash, or display name pattern) is required.';
    }
    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!validate()) return;

    onSubmit({
      name: name.trim(),
      platforms,
      category,
      status,
      confidence: confidence / 100,
      evidence_url: evidenceUrl.trim(),
      tags: tags.filter((t) => t.trim()),
      package_names: packageNames.filter((v) => v.trim()),
      executable_names: executableNames.filter((v) => v.trim()),
      cert_hashes: certHashes.filter((v) => v.trim()),
      display_name_patterns: displayNamePatterns.filter((v) => v.trim()),
    });
  }

  return (
    <form onSubmit={handleSubmit} className="space-y-8">
      {/* Basic info */}
      <div className="space-y-4">
        <h2 className="text-lg font-semibold">Basic Information</h2>

        <div className="space-y-2">
          <Label htmlFor="name">
            Name <span className="text-destructive">*</span>
          </Label>
          <Input
            id="name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. Bet365 Mobile App"
          />
          {errors['name'] && <p className="text-sm text-destructive">{errors['name']}</p>}
        </div>

        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <div className="space-y-2">
            <Label>Category</Label>
            <Select value={category} onValueChange={(v) => setCategory(v as BlocklistCategory)}>
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

          <div className="space-y-2">
            <Label>Status</Label>
            <Select
              value={status}
              onValueChange={(v) => {
                if (v) setStatus(v as 'active' | 'inactive' | 'pending_review');
              }}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {STATUS_OPTIONS.map((s) => (
                  <SelectItem key={s} value={s}>
                    {capitalize(s)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>

        <div className="space-y-2">
          <Label>
            Confidence: {confidence}%
          </Label>
          <input
            type="range"
            min={0}
            max={100}
            step={1}
            value={confidence}
            onChange={(e) => setConfidence(Number(e.target.value))}
            className="w-full"
          />
        </div>

        <div className="space-y-2">
          <Label htmlFor="evidence_url">Evidence URL</Label>
          <Input
            id="evidence_url"
            type="url"
            value={evidenceUrl}
            onChange={(e) => setEvidenceUrl(e.target.value)}
            placeholder="https://..."
          />
        </div>
      </div>

      {/* Platforms */}
      <div className="space-y-3">
        <h2 className="text-lg font-semibold">Platforms</h2>
        <div className="flex flex-wrap gap-4">
          {PLATFORMS.map((platform) => (
            <label key={platform} className="flex items-center gap-2 cursor-pointer">
              <Checkbox
                checked={platforms.includes(platform)}
                onCheckedChange={() => togglePlatform(platform)}
              />
              <span className="text-sm capitalize">{platform}</span>
            </label>
          ))}
        </div>
      </div>

      {/* Identifiers */}
      <div className="space-y-6">
        <div>
          <h2 className="text-lg font-semibold">Identifiers</h2>
          <p className="text-sm text-muted-foreground">
            At least one identifier field must be non-empty.
          </p>
          {errors['identifiers'] && (
            <p className="text-sm text-destructive mt-1">{errors['identifiers']}</p>
          )}
        </div>

        <ArrayField
          label="Package Names"
          values={packageNames}
          onChange={setPackageNames}
          placeholder="com.example.app"
        />

        <ArrayField
          label="Executable Names"
          values={executableNames}
          onChange={setExecutableNames}
          placeholder="example.exe"
        />

        <ArrayField
          label="Certificate Hashes"
          values={certHashes}
          onChange={setCertHashes}
          placeholder="sha256:abc123..."
        />

        <ArrayField
          label="Display Name Patterns"
          values={displayNamePatterns}
          onChange={setDisplayNamePatterns}
          placeholder="Bet365*"
        />
      </div>

      {/* Tags */}
      <div className="space-y-4">
        <h2 className="text-lg font-semibold">Tags</h2>
        <ArrayField label="Tags" values={tags} onChange={setTags} placeholder="tag-name" />
      </div>

      {/* Submit */}
      <div className="flex gap-3">
        <Button type="submit" disabled={isSubmitting}>
          {isSubmitting ? 'Saving...' : mode === 'create' ? 'Create Signature' : 'Save Changes'}
        </Button>
      </div>
    </form>
  );
}
