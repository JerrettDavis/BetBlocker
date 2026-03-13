import { SiteHeader } from '@/components/layout/site-header';
import { SiteFooter } from '@/components/layout/site-footer';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import Link from 'next/link';
import { Shield, Users, Monitor, Wifi, Lock, Eye } from 'lucide-react';

export default function HomePage() {
  return (
    <div className="flex min-h-screen flex-col">
      <SiteHeader />

      <main className="flex-1">
        {/* Hero */}
        <section className="container mx-auto px-4 py-24 text-center">
          <h1 className="text-4xl font-bold tracking-tight sm:text-6xl">
            Take back control of your
            <span className="text-primary"> gambling habits</span>
          </h1>
          <p className="mx-auto mt-6 max-w-2xl text-lg text-muted-foreground">
            BetBlocker blocks gambling websites and apps across all your devices. Self-hosted or
            managed, with accountability partner support and privacy-first design.
          </p>
          <div className="mt-10 flex items-center justify-center gap-4">
            <Button size="lg" asChild>
              <Link href="/register">Get Started Free</Link>
            </Button>
            <Button size="lg" variant="outline" asChild>
              <Link href="/pricing">Learn More</Link>
            </Button>
          </div>
        </section>

        {/* Features */}
        <section className="border-t bg-muted/30 py-24">
          <div className="container mx-auto px-4">
            <h2 className="text-center text-3xl font-bold">How BetBlocker protects you</h2>
            <p className="mx-auto mt-4 max-w-xl text-center text-muted-foreground">
              Comprehensive protection that works at every level.
            </p>
            <div className="mt-16 grid gap-8 md:grid-cols-3">
              <Card>
                <CardHeader>
                  <Wifi className="mb-2 h-10 w-10 text-primary" />
                  <CardTitle>DNS-Level Blocking</CardTitle>
                </CardHeader>
                <CardContent className="text-muted-foreground">
                  Blocks gambling domains at the network level before they even load. Works across
                  all browsers and apps on your device.
                </CardContent>
              </Card>
              <Card>
                <CardHeader>
                  <Users className="mb-2 h-10 w-10 text-primary" />
                  <CardTitle>Accountability Partners</CardTitle>
                </CardHeader>
                <CardContent className="text-muted-foreground">
                  Invite a trusted person to oversee your protection. They can view reports and
                  approve unenrollment requests.
                </CardContent>
              </Card>
              <Card>
                <CardHeader>
                  <Monitor className="mb-2 h-10 w-10 text-primary" />
                  <CardTitle>Multi-Device Support</CardTitle>
                </CardHeader>
                <CardContent className="text-muted-foreground">
                  Protect all your devices from a single dashboard. Windows, macOS, Linux, Android,
                  and iOS supported.
                </CardContent>
              </Card>
            </div>
          </div>
        </section>

        {/* How It Works */}
        <section className="py-24">
          <div className="container mx-auto px-4">
            <h2 className="text-center text-3xl font-bold">Get started in three steps</h2>
            <div className="mt-16 grid gap-8 md:grid-cols-3">
              {[
                {
                  step: '1',
                  title: 'Install',
                  description:
                    'Download and install the BetBlocker agent on your device. Takes less than 2 minutes.',
                },
                {
                  step: '2',
                  title: 'Configure',
                  description:
                    'Choose your protection level, set up an accountability partner, and customize your settings.',
                },
                {
                  step: '3',
                  title: 'Protect',
                  description:
                    'BetBlocker works silently in the background, blocking gambling content and reporting any bypass attempts.',
                },
              ].map((item) => (
                <div key={item.step} className="text-center">
                  <div className="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-primary text-xl font-bold text-primary-foreground">
                    {item.step}
                  </div>
                  <h3 className="mt-4 text-xl font-semibold">{item.title}</h3>
                  <p className="mt-2 text-muted-foreground">{item.description}</p>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* Trust */}
        <section className="border-t bg-muted/30 py-24">
          <div className="container mx-auto px-4">
            <h2 className="text-center text-3xl font-bold">Built on trust</h2>
            <div className="mt-16 grid gap-8 md:grid-cols-3">
              <div className="text-center">
                <Shield className="mx-auto h-10 w-10 text-primary" />
                <h3 className="mt-4 text-lg font-semibold">Open Source</h3>
                <p className="mt-2 text-sm text-muted-foreground">
                  Fully open source. Audit the code yourself.
                </p>
              </div>
              <div className="text-center">
                <Lock className="mx-auto h-10 w-10 text-primary" />
                <h3 className="mt-4 text-lg font-semibold">Privacy First</h3>
                <p className="mt-2 text-sm text-muted-foreground">
                  Your data stays yours. Self-host for complete control.
                </p>
              </div>
              <div className="text-center">
                <Eye className="mx-auto h-10 w-10 text-primary" />
                <h3 className="mt-4 text-lg font-semibold">No Tracking</h3>
                <p className="mt-2 text-sm text-muted-foreground">
                  No analytics, no ads, no selling your data. Ever.
                </p>
              </div>
            </div>
          </div>
        </section>
      </main>

      <SiteFooter />
    </div>
  );
}
