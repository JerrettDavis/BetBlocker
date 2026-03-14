'use client';

import { useState, useEffect } from 'react';
import { useParams, useRouter } from 'next/navigation';
import { useOrganization, useUpdateOrganization, useDeleteOrganization } from '@/hooks/use-organizations';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { Separator } from '@/components/ui/separator';
import { Settings, Trash2 } from 'lucide-react';

export default function OrgSettingsPage() {
  const params = useParams();
  const router = useRouter();
  const orgId = params.id as string;

  const { data, isLoading, error } = useOrganization(orgId);
  const updateOrg = useUpdateOrganization();
  const deleteOrg = useDeleteOrganization();

  const org = data?.data;

  const [name, setName] = useState('');
  const [orgType, setOrgType] = useState('');
  const [defaultProtectionConfig, setDefaultProtectionConfig] = useState('');
  const [defaultReportingConfig, setDefaultReportingConfig] = useState('');
  const [defaultUnenrollmentPolicy, setDefaultUnenrollmentPolicy] = useState('');

  useEffect(() => {
    if (org) {
      setName(org.name);
      setOrgType(org.org_type);
      setDefaultProtectionConfig(
        org.default_protection_config ? JSON.stringify(org.default_protection_config, null, 2) : '',
      );
      setDefaultReportingConfig(
        org.default_reporting_config ? JSON.stringify(org.default_reporting_config, null, 2) : '',
      );
      setDefaultUnenrollmentPolicy(
        org.default_unenrollment_policy
          ? JSON.stringify(org.default_unenrollment_policy, null, 2)
          : '',
      );
    }
  }, [org]);

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    const updateData: Record<string, unknown> = { name, org_type: orgType };

    if (defaultProtectionConfig.trim()) {
      try {
        updateData.default_protection_config = JSON.parse(defaultProtectionConfig);
      } catch {
        return; // invalid JSON
      }
    }
    if (defaultReportingConfig.trim()) {
      try {
        updateData.default_reporting_config = JSON.parse(defaultReportingConfig);
      } catch {
        return;
      }
    }
    if (defaultUnenrollmentPolicy.trim()) {
      try {
        updateData.default_unenrollment_policy = JSON.parse(defaultUnenrollmentPolicy);
      } catch {
        return;
      }
    }

    try {
      await updateOrg.mutateAsync({ id: orgId, data: updateData });
    } catch {
      // error available via updateOrg.error
    }
  };

  const handleDelete = async () => {
    if (
      confirm(
        'Are you sure you want to delete this organization? This action cannot be undone.',
      )
    ) {
      try {
        await deleteOrg.mutateAsync(orgId);
        router.push('/organizations');
      } catch {
        // error available via deleteOrg.error
      }
    }
  };

  if (isLoading) {
    return (
      <div className="space-y-4">
        <Skeleton className="h-64" />
      </div>
    );
  }

  if (error || !org) {
    return <p className="text-destructive">Failed to load organization settings.</p>;
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Settings className="h-5 w-5" />
            Organization Settings
          </CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSave} className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="settings-name">Name</Label>
              <Input
                id="settings-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                required
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="settings-type">Type</Label>
              <select
                id="settings-type"
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

            <Separator />

            <div className="space-y-2">
              <Label htmlFor="settings-protection">Default Protection Config (JSON)</Label>
              <textarea
                id="settings-protection"
                value={defaultProtectionConfig}
                onChange={(e) => setDefaultProtectionConfig(e.target.value)}
                className="flex min-h-[80px] w-full rounded-lg border border-input bg-transparent px-2.5 py-2 text-sm font-mono"
                placeholder="{}"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="settings-reporting">Default Reporting Config (JSON)</Label>
              <textarea
                id="settings-reporting"
                value={defaultReportingConfig}
                onChange={(e) => setDefaultReportingConfig(e.target.value)}
                className="flex min-h-[80px] w-full rounded-lg border border-input bg-transparent px-2.5 py-2 text-sm font-mono"
                placeholder="{}"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="settings-unenrollment">Default Unenrollment Policy (JSON)</Label>
              <textarea
                id="settings-unenrollment"
                value={defaultUnenrollmentPolicy}
                onChange={(e) => setDefaultUnenrollmentPolicy(e.target.value)}
                className="flex min-h-[80px] w-full rounded-lg border border-input bg-transparent px-2.5 py-2 text-sm font-mono"
                placeholder="{}"
              />
            </div>

            {updateOrg.error && (
              <p className="text-sm text-destructive">{updateOrg.error.message}</p>
            )}
            {updateOrg.isSuccess && (
              <p className="text-sm text-green-600">Settings saved successfully.</p>
            )}

            <Button type="submit" disabled={updateOrg.isPending}>
              {updateOrg.isPending ? 'Saving...' : 'Save Changes'}
            </Button>
          </form>
        </CardContent>
      </Card>

      <Card className="border-destructive">
        <CardHeader>
          <CardTitle className="text-destructive flex items-center gap-2">
            <Trash2 className="h-5 w-5" />
            Danger Zone
          </CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground mb-4">
            Deleting this organization will remove all members, device assignments, and enrollment
            tokens. This action cannot be undone.
          </p>
          {deleteOrg.error && (
            <p className="text-sm text-destructive mb-4">{deleteOrg.error.message}</p>
          )}
          <Button
            variant="destructive"
            onClick={handleDelete}
            disabled={deleteOrg.isPending}
          >
            {deleteOrg.isPending ? 'Deleting...' : 'Delete Organization'}
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
