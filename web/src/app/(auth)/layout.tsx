import Link from 'next/link';
import { Shield } from 'lucide-react';

export default function AuthLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex min-h-screen flex-col items-center justify-center bg-muted/30 p-4">
      <Link href="/" className="mb-8 flex items-center gap-2 text-2xl font-bold">
        <Shield className="h-8 w-8 text-primary" />
        BetBlocker
      </Link>
      {children}
      <p className="mt-6 text-sm text-muted-foreground">
        <Link href="/" className="hover:underline">
          Back to home
        </Link>
      </p>
    </div>
  );
}
