'use client';

import { useRouter } from 'next/navigation';
import { AppSignatureForm } from '@/components/admin/AppSignatureForm';
import type { AppSignatureFormData } from '@/components/admin/AppSignatureForm';
import { useCreateAppSignature } from '@/hooks/use-app-signatures';
import Link from 'next/link';

export default function NewAppSignaturePage() {
  const router = useRouter();
  const createSignature = useCreateAppSignature();

  function handleSubmit(data: AppSignatureFormData) {
    createSignature.mutate(
      {
        name: data.name,
        platforms: data.platforms,
        category: data.category,
        status: data.status,
        confidence: data.confidence,
        evidence_url: data.evidence_url || undefined,
        tags: data.tags,
        package_names: data.package_names,
        executable_names: data.executable_names,
        cert_hashes: data.cert_hashes,
        display_name_patterns: data.display_name_patterns,
      },
      {
        onSuccess: () => router.push('/admin/app-signatures'),
      },
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
        <span className="text-foreground font-medium">New Signature</span>
      </div>

      <div>
        <h1 className="text-2xl font-bold">New App Signature</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Add a new gambling app signature for blocking.
        </p>
      </div>

      <div className="max-w-2xl">
        <AppSignatureForm
          mode="create"
          onSubmit={handleSubmit}
          isSubmitting={createSignature.isPending}
        />
        {createSignature.isError && (
          <p className="text-sm text-destructive mt-4">
            Failed to create signature. Please try again.
          </p>
        )}
      </div>
    </div>
  );
}
