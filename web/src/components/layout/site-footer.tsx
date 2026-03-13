import Link from 'next/link';
import { Shield } from 'lucide-react';

export function SiteFooter() {
  return (
    <footer className="border-t bg-muted/30">
      <div className="container mx-auto px-4 py-12">
        <div className="grid grid-cols-1 md:grid-cols-4 gap-8">
          <div className="space-y-3">
            <div className="flex items-center gap-2 font-bold text-lg">
              <Shield className="h-5 w-5 text-primary" />
              BetBlocker
            </div>
            <p className="text-sm text-muted-foreground">
              Open-source gambling blocking software. Take back control.
            </p>
          </div>

          <div className="space-y-3">
            <h4 className="font-semibold text-sm">Product</h4>
            <nav className="flex flex-col gap-2 text-sm text-muted-foreground">
              <Link href="/pricing" className="hover:text-foreground">
                Pricing
              </Link>
              <Link href="/about" className="hover:text-foreground">
                About
              </Link>
            </nav>
          </div>

          <div className="space-y-3">
            <h4 className="font-semibold text-sm">Legal</h4>
            <nav className="flex flex-col gap-2 text-sm text-muted-foreground">
              <Link href="/privacy" className="hover:text-foreground">
                Privacy Policy
              </Link>
              <Link href="/terms" className="hover:text-foreground">
                Terms of Service
              </Link>
            </nav>
          </div>

          <div className="space-y-3">
            <h4 className="font-semibold text-sm">Support</h4>
            <nav className="flex flex-col gap-2 text-sm text-muted-foreground">
              <Link href="https://github.com/betblocker" className="hover:text-foreground">
                GitHub
              </Link>
              <Link href="/about#contact" className="hover:text-foreground">
                Contact
              </Link>
            </nav>
          </div>
        </div>

        <div className="mt-8 border-t pt-8 text-center text-sm text-muted-foreground">
          &copy; {new Date().getFullYear()} BetBlocker. Open source under the MIT License.
        </div>
      </div>
    </footer>
  );
}
