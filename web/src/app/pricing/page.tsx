import type { Metadata } from 'next';
import { SiteHeader } from '@/components/layout/site-header';
import { SiteFooter } from '@/components/layout/site-footer';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import Link from 'next/link';
import { Check } from 'lucide-react';

export const metadata: Metadata = {
  title: 'Pricing - BetBlocker',
  description: 'BetBlocker pricing plans. Self-hosted for free or managed hosting for $10/month.',
};

export default function PricingPage() {
  return (
    <div className="flex min-h-screen flex-col">
      <SiteHeader />

      <main className="flex-1">
        <section className="container mx-auto px-4 py-24">
          <h1 className="text-center text-4xl font-bold">Simple, transparent pricing</h1>
          <p className="mx-auto mt-4 max-w-xl text-center text-muted-foreground">
            Choose the plan that works for you. Both plans include the same powerful blocking
            features.
          </p>

          <div className="mt-16 grid gap-8 md:grid-cols-2 max-w-3xl mx-auto">
            <Card className="relative">
              <CardHeader>
                <CardTitle className="text-2xl">Self-Hosted</CardTitle>
                <CardDescription>Host on your own infrastructure</CardDescription>
                <p className="text-4xl font-bold mt-4">
                  Free
                  <span className="text-base font-normal text-muted-foreground"> forever</span>
                </p>
              </CardHeader>
              <CardContent>
                <ul className="space-y-3">
                  {[
                    'Unlimited devices',
                    'Full control over your data',
                    'Your infrastructure',
                    'Community support',
                    'All blocking features',
                    'Accountability partners',
                  ].map((feature) => (
                    <li key={feature} className="flex items-center gap-2 text-sm">
                      <Check className="h-4 w-4 text-primary" />
                      {feature}
                    </li>
                  ))}
                </ul>
                <Button className="w-full mt-8" variant="outline" asChild>
                  <Link href="/register">Get Started</Link>
                </Button>
              </CardContent>
            </Card>

            <Card className="relative border-primary">
              <CardHeader>
                <CardTitle className="text-2xl">Managed</CardTitle>
                <CardDescription>We host it for you</CardDescription>
                <p className="text-4xl font-bold mt-4">
                  $10
                  <span className="text-base font-normal text-muted-foreground"> /month</span>
                </p>
              </CardHeader>
              <CardContent>
                <ul className="space-y-3">
                  {[
                    'Unlimited devices',
                    'Hosted and managed for you',
                    'Automatic updates',
                    'Email support',
                    'All blocking features',
                    'Same privacy guarantees',
                  ].map((feature) => (
                    <li key={feature} className="flex items-center gap-2 text-sm">
                      <Check className="h-4 w-4 text-primary" />
                      {feature}
                    </li>
                  ))}
                </ul>
                <Button className="w-full mt-8" asChild>
                  <Link href="/register">Start Free Trial</Link>
                </Button>
              </CardContent>
            </Card>
          </div>

          {/* FAQ */}
          <div className="mt-24 max-w-2xl mx-auto">
            <h2 className="text-center text-2xl font-bold">Frequently asked questions</h2>
            <div className="mt-8 space-y-6">
              {[
                {
                  q: 'What happens if I cancel my managed plan?',
                  a: 'You can export all your data and switch to self-hosting at any time. We provide migration tools to make the transition seamless.',
                },
                {
                  q: 'Can I switch between plans?',
                  a: 'Yes. You can switch from self-hosted to managed or vice versa at any time. Your configuration and partner relationships are preserved.',
                },
                {
                  q: 'Is my data private?',
                  a: 'Absolutely. We never sell or share your data. The managed plan uses the same privacy-first architecture as self-hosted. All data is encrypted at rest and in transit.',
                },
                {
                  q: 'Do I need technical knowledge for self-hosting?',
                  a: 'Basic familiarity with running a server is helpful. We provide Docker images and detailed guides to make setup straightforward.',
                },
              ].map((faq) => (
                <div key={faq.q}>
                  <h3 className="font-semibold">{faq.q}</h3>
                  <p className="mt-1 text-sm text-muted-foreground">{faq.a}</p>
                </div>
              ))}
            </div>
          </div>
        </section>
      </main>

      <SiteFooter />
    </div>
  );
}
