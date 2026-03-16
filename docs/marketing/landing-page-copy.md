# BetBlocker Landing Page Copy

---

## Hero Section

**Headline:**
Take back control from gambling. For good.

**Subheadline:**
BetBlocker is a free, open-source platform that blocks gambling across every device you own — with layers of protection designed to hold firm when willpower alone cannot.

**CTA Buttons:**
- Primary: **Get Started Free**
- Secondary: **View on GitHub**

---

## Problem Statement

### Gambling addiction is a public health crisis hiding in plain sight.

- Over 80 million people worldwide struggle with problem gambling.
- Online gambling platforms are engineered to be accessible 24/7, on every device, with fewer barriers than ever.
- A moment of vulnerability at 2 AM should not undo months of recovery.

**Existing tools fall short.** Most gambling blockers rely on a single layer of DNS filtering that can be bypassed in minutes. Some collect your browsing data. Others charge fees that put protection out of reach for the people who need it most. And almost none of them give accountability partners or treatment providers real visibility into whether blocking is actually working.

Recovery deserves better tools.

---

## Solution Overview

### Protection that works in layers — because addiction looks for every gap.

BetBlocker does not rely on a single point of defense. It combines three independent blocking layers — DNS filtering, application control, and browser content scanning — so that if one layer is circumvented, the others hold. Every layer runs at the system level, below where a user can easily interfere, and the platform monitors its own integrity continuously.

This is not a browser plugin you can uninstall in a weak moment. This is infrastructure-grade protection, built with the same security principles used in enterprise endpoint defense — and it is completely free to self-host.

---

## Feature Grid

### Six pillars of protection

#### DNS Blocking
Every gambling domain is intercepted before it can load — at the network level, before your browser even connects. Works across all apps, all browsers, even those with hardcoded DNS settings. Includes encrypted DNS enforcement so requests cannot be rerouted around the filter.

#### App Blocking
Known gambling applications are detected, blocked from launching, and prevented from being installed. BetBlocker maintains a continuously updated database of gambling app signatures across every major app store and platform.

#### Tamper Protection
BetBlocker runs as a system-level service with mutual watchdog processes, binary integrity validation, and kernel-level file protection. On partner and institutional enrollments, any tampering attempt triggers an immediate alert. Unenrollment requires a time delay or partner approval — not a moment of impulse.

#### Accountability Partners
Invite a therapist, sponsor, family member, or trusted friend as an accountability partner. They receive aggregated reports and tamper alerts through their own dashboard. Unenrollment requires their approval. Privacy is respected — partners see that blocking is working, not a record of everything you do online.

#### Organization Management
Treatment programs, court-mandated recovery programs, and therapy practices can manage enrollments across multiple clients and devices from a single dashboard. Full audit trails for compliance reporting. Bulk enrollment and centralized policy management.

#### Privacy by Design
No keylogging. No screen capture. No location tracking. No browsing history collection. No data sold to anyone, ever. BetBlocker reports only the minimum metadata needed for blocking intelligence — and even that is configurable per enrollment. The entire codebase is open source and auditable.

---

## How It Works

### Three steps to real protection

**Step 1: Install**
Download BetBlocker for your platform — Windows, macOS, Linux, Android, or iOS. Installation takes under two minutes. The agent runs quietly in the background as a system service.

**Step 2: Enroll**
Choose your protection level. Enroll yourself with a time-delayed unenrollment window, or invite an accountability partner who must approve any changes. Institutional enrollments are available for treatment providers and court programs.

**Step 3: Protected**
BetBlocker activates immediately. DNS filtering, application blocking, and browser protection work together across every app and browser on your device. The platform syncs with continuously updated blocklists covering tens of thousands of gambling domains worldwide. You get on with your life — BetBlocker handles the rest.

---

## Platform Support

### One platform. Every device.

| Platform | Protection Level | Integration |
|----------|-----------------|-------------|
| **Windows** | Full (DNS + App + Browser + Kernel) | Windows Service, WFP driver, kernel minifilter |
| **macOS** | Full (DNS + App + Browser + System) | launchd daemon, Network Extension, System Extension |
| **Linux** | Full (DNS + App + Browser + MAC) | systemd service, nftables, AppArmor/SELinux |
| **Android** | Full (DNS + App + Browser) | VPN Service, Device Admin, Knox support |
| **iOS** | DNS + Browser (platform limitations) | Network Extension, MDM profile, Screen Time API |

All platforms share a single Rust core engine, ensuring consistent blocking behavior everywhere.

---

## Pricing

### Real protection should not have a price barrier.

#### Free (Self-Hosted)
**$0 forever**

- Full platform — every feature, every blocking layer
- Community blocklist with continuous updates
- Docker Compose deployment: `docker compose up` and you are running
- Community support via GitHub
- No telemetry, no phone-home, completely independent
- Perfect for technically inclined users or organizations with their own infrastructure

**Get Started Self-Hosting**

---

#### Hosted Standard
**$10/month**

- Fully managed — nothing to deploy or maintain
- Priority blocklist updates with faster coverage of new gambling sites
- Automatic agent updates pushed to all enrolled devices
- Email support
- All blocking layers, all platforms
- Ideal for individuals and families who want protection without the technical setup

**Start Free Trial**

---

#### Institutional
**Custom pricing**

- Bulk device licensing for treatment programs, courts, and recovery organizations
- Compliance reporting with full audit trails
- SSO integration
- Dedicated support and SLA
- Centralized management dashboard for staff
- Custom blocklist policies

**Contact Us**

---

## Testimonials

### People who use BetBlocker

> [Testimonial placeholder — recovering individual describing peace of mind from multi-layer blocking and the accountability partner feature]
>
> — [Name], BetBlocker user for [X] months

> [Testimonial placeholder — therapist or counselor describing institutional management features and client compliance tracking]
>
> — [Name], [Title] at [Organization]

> [Testimonial placeholder — accountability partner describing the dashboard experience and how it strengthens their support role]
>
> — [Name], accountability partner

> [Testimonial placeholder — technical user describing the open-source self-hosted experience and privacy architecture]
>
> — [Name], self-hosted user

---

## FAQ

### Frequently Asked Questions

**Can I bypass BetBlocker?**

BetBlocker is specifically designed to resist bypass attempts. It runs at the system level with multiple independent blocking layers — DNS filtering, application control, and browser content scanning. If one layer is circumvented, the others continue to block. The agent monitors its own integrity, uses mutual watchdog processes, and on supported platforms employs kernel-level protections to prevent tampering. On partner and institutional enrollments, any bypass attempt triggers an immediate alert to your accountability partner or program administrator. No blocking tool is theoretically impossible to defeat with enough effort and technical skill, but BetBlocker raises that bar higher than any other solution available — high enough that the effort required creates the pause that recovery depends on.

**Does BetBlocker spy on me?**

No. BetBlocker has explicit privacy boundaries baked into its architecture: no keylogging, no screen capture, no location tracking, no browsing history collection, no microphone or camera access. The only data BetBlocker processes is gambling-related domain metadata — the fact that a gambling site was blocked, not what you were doing before or after. Even this metadata is configurable per enrollment. The entire codebase is open source, so these claims are independently verifiable by anyone.

**Is BetBlocker really free?**

Yes. The self-hosted version of BetBlocker is free, open source, and includes every feature — every blocking layer, every enrollment tier, every reporting capability. There is no artificial feature gating. The hosted version at $10/month covers the cost of running managed infrastructure, providing priority support, and maintaining faster blocklist updates. The hosted and self-hosted platforms run the exact same code.

**What about my privacy?**

Privacy is a core architectural principle, not an afterthought. BetBlocker collects the minimum data necessary for blocking to work. Self-hosted deployments keep all data on your own infrastructure — nothing leaves your network unless you explicitly opt in to federated blocklist contribution. Even on the hosted platform, data is never sold, shared with third parties, or used for advertising. Accountability partners see aggregated reports (blocking is active, no tampering detected), not detailed browsing activity — unless both parties explicitly consent to more detailed reporting.

**Can my therapist or accountability partner see my browsing data?**

No. Accountability partners see that blocking is active, whether any tampering has been attempted, and aggregated block counts. They do not see your browsing history, search queries, or any activity outside of gambling blocking. With mutual consent, more detailed reporting (such as which categories of gambling sites were blocked) can be enabled, but full browsing history is never collected or available to anyone — not even to you through the platform.

**What platforms does BetBlocker support?**

BetBlocker supports Windows, macOS, Linux, Android, and iOS. All platforms share a single Rust core engine that provides consistent blocking behavior. Platform-specific integrations (kernel drivers, system extensions, VPN services) are minimal native shims that plug into the cross-platform core. A single account manages all your devices from one dashboard.

**What happens if I want to unenroll?**

That depends on your enrollment tier. Self-enrolled users can unenroll with a configurable time delay (24 to 72 hours) — long enough for a moment of impulse to pass. Partner-enrolled users require their accountability partner to approve unenrollment through the web dashboard. Institutional enrollments require approval from the enrolling authority. In all cases, the unenrollment process is intentionally deliberate. The point is not to trap you — it is to make sure that the decision to remove protection is a considered one, not an impulsive one.

**Does BetBlocker work offline?**

Yes. BetBlocker caches its full blocklist locally on every enrolled device. DNS filtering, application blocking, and browser protection all function without an internet connection. When connectivity is restored, the agent syncs any pending events and checks for blocklist updates. An internet connection is required only for initial enrollment and periodic blocklist updates.

**How is BetBlocker different from other gambling blockers?**

Most gambling blockers use a single layer of DNS filtering that can be bypassed by changing DNS settings, using a VPN, or switching to a mobile app. BetBlocker uses three independent blocking layers (DNS, application, browser) with cross-cutting tamper resistance. It is the only open-source gambling blocker, so its security claims are verifiable. It supports accountability partners and institutional management natively. And the self-hosted option means you never have to trust a third party with your data.

**Is BetBlocker only for gambling?**

Yes. BetBlocker blocks gambling and only gambling. It is not a general content filter, parental control tool, or internet restriction platform. This focus means the blocklist is curated specifically for gambling domains, apps, and content — with better coverage and fewer false positives than general-purpose filters.

---

## Final CTA

### Recovery is worth protecting.

BetBlocker exists because people in recovery deserve tools that are as serious about their recovery as they are. Multi-layer blocking. Real tamper resistance. Accountability built in. Privacy respected. Open source and free.

**Get Started Free** | **View on GitHub**

---

*BetBlocker is open-source software. It is not a substitute for professional treatment. If you or someone you know is struggling with gambling addiction, please reach out to the National Problem Gambling Helpline at 1-800-522-4700 or visit ncpgambling.org.*
