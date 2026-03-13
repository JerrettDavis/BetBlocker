import type { Metadata } from 'next';
import { SiteHeader } from '@/components/layout/site-header';
import { SiteFooter } from '@/components/layout/site-footer';

export const metadata: Metadata = {
  title: 'About - BetBlocker',
  description:
    'Learn about BetBlocker, our mission, and our commitment to privacy and open source.',
};

export default function AboutPage() {
  return (
    <div className="flex min-h-screen flex-col">
      <SiteHeader />

      <main className="flex-1">
        <section className="container mx-auto px-4 py-24 max-w-3xl">
          <h1 className="text-4xl font-bold">About BetBlocker</h1>

          <div className="mt-8 space-y-8 text-muted-foreground leading-relaxed">
            <div>
              <h2 className="text-xl font-semibold text-foreground">Our Mission</h2>
              <p className="mt-2">
                BetBlocker exists to help people who struggle with problem gambling take back
                control. We believe that effective gambling blocking software should be accessible
                to everyone, regardless of their technical skill or financial situation.
              </p>
            </div>

            <div>
              <h2 className="text-xl font-semibold text-foreground">How It Works</h2>
              <p className="mt-2">
                BetBlocker uses DNS-level blocking to prevent access to gambling websites and
                applications. A lightweight agent runs on your device, intercepting DNS queries and
                blocking those that match our curated blocklist of gambling domains.
              </p>
              <p className="mt-2">
                The system supports accountability partners who can monitor your protection status
                and approve or deny unenrollment requests, providing an extra layer of support for
                recovery.
              </p>
            </div>

            <div>
              <h2 className="text-xl font-semibold text-foreground">Open Source</h2>
              <p className="mt-2">
                BetBlocker is fully open source. We believe in transparency and community-driven
                development. Anyone can audit our code, contribute improvements, or self-host the
                entire platform on their own infrastructure.
              </p>
            </div>

            <div>
              <h2 className="text-xl font-semibold text-foreground">Privacy First</h2>
              <p className="mt-2">
                We take privacy seriously. BetBlocker collects only the minimum data necessary to
                function. We never sell or share user data. Reporting is configurable, and users or
                their partners control exactly what information is visible.
              </p>
              <p className="mt-2">
                For maximum privacy, you can self-host BetBlocker on your own infrastructure,
                ensuring your data never leaves your control.
              </p>
            </div>

            <div id="contact">
              <h2 className="text-xl font-semibold text-foreground">Contact</h2>
              <p className="mt-2">
                For questions, suggestions, or support, please open an issue on our GitHub
                repository or reach out through our community channels.
              </p>
            </div>
          </div>
        </section>
      </main>

      <SiteFooter />
    </div>
  );
}
