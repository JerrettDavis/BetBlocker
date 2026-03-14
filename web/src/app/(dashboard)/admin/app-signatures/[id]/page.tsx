'use client';

import { use } from 'react';
import { useRouter } from 'next/navigation';
import { AppSignatureForm } from '@/components/admin/AppSignatureForm';
import type { AppSignatureFormData } from '@/components/admin/AppSignatureForm';
import { useAppSignature, useUpdateAppSignature } from '@/hooks/use-app-signatures';
import { Skeleton } from '@/components/ui/skeleton';
import Link from 'next/link';

interface PageProps {
  params: Promise<{ id: string }>;
}

export default function EditAppSignaturePage({ params }: PageProps) {
  const { id } = use(params);
  const router = useRouter();

  const { data, isLoading, error } = useAppSignature(id);
  const updateSignature = useUpdateAppSignature();

  const signature = data?.data;

  function handleSubmit(formData: AppSignatureFormData) {
    updateSignature.mutate(
      {
        id,
        data: {
          name: formData.name,
          platforms: formData.platforms,
          category: formData.category,
          status: formData.status as 'active' | 'inactive' | 'pending_review',
          confidence: formData.confidence,
          evidence_url: formData.evidence_url || undefined,
          tags: formData.tags,
          package_names: formData.package_names,
          executable_names: formData.executable_names,
          cert_hashes: formData.cert_hashes,
          display_name_patterns: formData.display_name_patterns,
        },
      },
      {
        onSuccess: () => router.push('/admin/app-signatures'),
      },
    );
  }

  if (isLoading) {
    return (
      <div className="space-y-4">
        <Skeleton className="h-8 w-64" />
        <Skeleton className="h-96" />
      </div>
    );
  }

  if (error || !signature) {
    return (
      <div className="space-y-4">
        <Link href="/admin/app-signatures" className="text-sm text-muted-foreground hover:text-foreground">
          ← Back to App Signatures
        </Link>
        <p className="text-destructive">Signature not found or failed to load.</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Breadcrumbs */}
      <div className="flex items-center gap-2 text-sm text-muted-foreground">
        <Link href="/admin/app-signatures" className="hover:text-foreground">
          App Signatures
        </Link>
        <span>/</span>
        <span className="text-foreground font-medium">{signature.name}</span>
      </div>

      <div>
        <h1 className="text-2xl font-bold">Edit App Signature</h1>
        <p className="text-sm text-muted-foreground mt-1">Update the signature details.</p>
      </div>

      <div className="max-w-2xl">
        <AppSignatureForm
          mode="edit"
          initialData={signature}
          onSubmit={handleSubmit}
          isSubmitting={updateSignature.isPending}
        />
        {updateSignature.isError && (
          <p className="text-sm text-destructive mt-4">
            Failed to save changes. Please try again.
          </p>
        )}
      </div>
    </div>
  );
}
