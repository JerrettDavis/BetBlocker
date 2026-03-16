# BetBlocker Social Media Kit

---

## Twitter/X Launch Thread

### Tweet 1 (Hook)
We just open-sourced BetBlocker — a free, cross-platform gambling blocking platform with three independent layers of protection.

Built in Rust. Runs on Windows, macOS, Linux, Android, and iOS. Self-hostable. No tracking. No data selling.

Here is why we built it and what makes it different:

### Tweet 2 (Problem)
Most gambling blockers use a single layer of DNS filtering. Change your DNS provider, install a VPN, or open a native app — and the blocking disappears.

For someone in recovery, that single point of failure can mean the difference between a difficult night and a relapse.

### Tweet 3 (Solution)
BetBlocker uses three independent blocking layers:

- DNS/network filtering (catches everything, including hardcoded DNS)
- Application blocking (detects, blocks, and prevents gambling app installs)
- Browser content scanning (catches gambling ads and affiliate links on non-gambling sites)

If one layer fails, the others hold.

### Tweet 4 (Tamper Resistance)
The agent runs as a system service with:

- Mutual watchdog processes
- Binary integrity validation
- Kernel-level file protection
- Hardware-bound credential encryption

Unenrollment is never instant. Self-enrolled users have a 24-72h delay. Partner enrollments require approval.

### Tweet 5 (Privacy)
What BetBlocker does NOT do:

- No keylogging
- No screen capture
- No location tracking
- No browsing history collection
- No data sold or shared

Self-hosted deployments keep all data on your infrastructure. Zero phone-home.

The codebase is open source — every claim is verifiable.

### Tweet 6 (Accountability)
BetBlocker supports accountability partners natively.

Therapists, sponsors, or family members get their own dashboard with device status and tamper alerts — without seeing browsing history.

Treatment programs can manage dozens of client enrollments from a centralized panel.

### Tweet 7 (CTA)
BetBlocker is free to self-host (every feature included) or $10/mo for fully managed hosting.

- Try it: [link]
- GitHub: [link]
- Docs: [link]

If you know someone in recovery from gambling addiction, consider sharing this.

---

## Reddit Posts

### r/opensource

**Title:** BetBlocker — open-source, cross-platform gambling blocking with multi-layer protection (Rust)

**Body:**

Hi r/opensource,

We just released BetBlocker, an open-source platform for blocking gambling across Windows, macOS, Linux, Android, and iOS.

**Why another blocker?** Existing gambling blocking tools mostly use DNS filtering alone, which is trivially bypassed. BetBlocker combines three independent layers — DNS/network filtering, application blocking, and browser content scanning — with system-level tamper resistance. If one layer is circumvented, the others continue to block.

**Why open source matters here.** When a tool claims to block gambling without spying on you, you should be able to verify that. BetBlocker's entire codebase is auditable — the endpoint agent, the API, the web platform, and the blocklist pipeline. Self-hosted deployments include every feature, keep all data on your infrastructure, and have zero phone-home behavior.

**Tech stack:**
- Endpoint agent: Rust (single codebase, compiled per platform with thin native shims)
- API: Rust (Axum), stateless, horizontally scalable
- Web: Next.js (dashboards for users, partners, institutions)
- Data: PostgreSQL + Redis + TimescaleDB
- Deployment: Docker Compose for self-hosted, Kubernetes for managed

**Key features:**
- Three blocking layers (DNS, app, browser)
- Accountability partner and institutional enrollment tiers
- Federated blocklist intelligence (agents contribute anonymized domain metadata)
- Kernel-level tamper resistance per platform
- Cryptographically signed blocklist with delta sync

Self-hosted is free. Managed hosting is $10/month.

GitHub: [link]
Docs: [link]

Contributions welcome — especially from folks with experience in Rust, security tooling, or cross-platform systems programming.

---

### r/rust

**Title:** BetBlocker: cross-platform gambling blocking agent written in Rust (open source)

**Body:**

We just open-sourced BetBlocker, a gambling blocking platform with a Rust core that compiles for Windows, macOS, Linux, Android, and iOS.

**Architecture overview:**

The endpoint agent is a single Rust codebase with a cross-platform core engine and thin native shims per platform:

- **Windows:** Windows Service + WFP callout driver + kernel minifilter
- **macOS:** launchd daemon + Network Extension + System Extension + Endpoint Security
- **Linux:** systemd service + nftables + AppArmor/SELinux
- **Android:** Foreground Service + VpnService + Device Admin/Knox
- **iOS:** Network Extension + NEDNSProxyProvider + MDM

The core engine handles DNS interception and local resolution, blocklist matching (domain, pattern, heuristic), event reporting with privacy-aware filtering, tamper detection and self-healing, and secure API communication (mTLS + certificate pinning).

The central API is also Rust (Axum) — shared types with the agent, single binary deployment. Background workers for blocklist compilation, federated report processing, and automated discovery are Rust as well.

**Why Rust:** Memory safety was non-negotiable for security tooling that runs at the system level with elevated privileges. The cross-compilation story made a single codebase across five platforms realistic. Performance matters for DNS interception on the hot path.

**Interesting challenges we would love community input on:**
- Sandboxing the plugin system (platform shims) while maintaining performance
- Efficient delta sync for blocklists with tens of thousands of entries
- Cross-platform binary integrity validation approaches

GitHub: [link]

---

### r/problemgambling

**Title:** Free, open-source gambling blocker that works across all your devices — BetBlocker

**Body:**

I want to share a tool we have been building for people who are serious about their gambling recovery.

BetBlocker is a free gambling blocker that works on Windows, Mac, Linux, Android, and iPhone. It is different from other blockers in a few important ways:

**It uses three layers of blocking, not just one.** Most blockers only filter DNS (the address lookup when you type a website). BetBlocker also blocks gambling apps from running or being installed, and scans browser content for gambling ads and links on non-gambling sites. If one layer is bypassed, the others keep working.

**It is hard to turn off on impulse.** If you set it up yourself, there is a mandatory waiting period (24-72 hours) before you can unenroll. If you set it up with an accountability partner — a therapist, sponsor, or family member — they have to approve any changes. This is intentional: the goal is to make sure removing protection is a considered decision, not something you do at 2 AM.

**Your accountability partner can see that it is working without seeing your private activity.** Partners get a dashboard showing that your devices are protected and whether any tampering has been attempted. They do not see your browsing history or any personal details.

**It does not spy on you.** No keylogging, no screen recording, no location tracking, no browsing history. The entire codebase is open source, so anyone can verify these claims.

**It is completely free to self-host.** Every feature is included. If you are not technical, the managed version is $10/month.

If you are working on your recovery and want a tool that holds firm when you need it to, check it out: [link]

This is not a replacement for therapy, support groups, or professional help. It is one more layer of support for the work you are already doing.

---

## Hacker News

**Title:** Show HN: BetBlocker -- Open-source, multi-layer gambling blocking in Rust

**Description (comment):**

BetBlocker is a cross-platform gambling blocking platform — endpoint agent in Rust, API in Rust/Axum, web dashboards in Next.js.

The core problem: existing gambling blockers use DNS filtering alone, which is trivially bypassed. BetBlocker layers DNS/network filtering, application blocking, and browser content scanning with system-level tamper resistance (WFP drivers on Windows, System Extensions on macOS, AppArmor/SELinux on Linux, Device Admin on Android, MDM on iOS).

The enrollment model is the interesting design choice: self-enrolled users get time-delayed unenrollment (24-72h), partner-enrolled users require partner approval, and institutional enrollments (court programs, treatment providers) require institutional approval with full audit trails.

Federated blocklist intelligence: agents contribute anonymized domain metadata back to a central pipeline where automated classifiers and human reviewers maintain the blocklist. Self-hosted users can opt in or out of contributing.

Self-hosted is free (docker compose up), managed hosting is $10/month. Same containers, same code — billing is a feature flag.

GitHub: [link]

---

## LinkedIn Post

**BetBlocker: Open-Source Gambling Blocking for Treatment Providers and Recovery Programs**

Today we are releasing BetBlocker — an open-source platform that helps individuals, accountability partners, and institutions enforce gambling abstinence across every device.

For treatment providers and court-mandated recovery programs, compliance monitoring has traditionally meant trusting client self-reports or relying on single-layer blocking tools that are easily circumvented. BetBlocker addresses both problems.

**For institutions, BetBlocker provides:**
- Centralized management of client device enrollments from a single dashboard
- Three independent blocking layers (DNS, application, browser) with cross-cutting tamper resistance
- Full audit trails for compliance reporting
- Tamper detection with immediate alerts to program administrators
- Unenrollment requiring institutional approval
- Aggregated reporting that respects client privacy while confirming program compliance

**The technical foundation:**
- Written in Rust for memory safety and cross-platform compilation
- Runs natively on Windows, macOS, Linux, Android, and iOS
- System-level service with kernel protections per platform
- Self-hostable with Docker Compose for organizations that require on-premises deployment
- Open source — every security claim is auditable

The platform supports three enrollment tiers: individual (self-managed with time-delayed unenrollment), partner (accountability partner approval required), and institutional (full compliance and audit capabilities).

Self-hosted deployment includes every feature at no cost. Managed hosting with priority support is available for organizations that prefer not to maintain infrastructure.

If you work in addiction treatment, judicial recovery programs, or employee assistance programs, I would welcome the opportunity to discuss how BetBlocker can integrate with your practice.

Learn more: [link]

---

## Product Hunt

**Tagline:**
Free, open-source gambling blocker with 3 layers of protection across every platform.

**Description:**
BetBlocker is a cross-platform gambling blocking platform that combines DNS filtering, application blocking, and browser content scanning — with system-level tamper resistance that holds firm when willpower cannot.

Built in Rust for every platform (Windows, macOS, Linux, Android, iOS). Supports accountability partners who can see that blocking works without seeing your browsing history. Treatment providers and courts can manage client enrollments with full audit trails.

Completely free to self-host with every feature included. Open source and auditable — no tracking, no data selling, no browsing history collection.

Managed hosting available at $10/month for those who prefer not to maintain infrastructure.

**Maker Comment:**
We built BetBlocker because existing gambling blockers rely on a single layer of DNS filtering that can be bypassed in minutes. For people in recovery, that single point of failure is not good enough. BetBlocker layers three independent blocking mechanisms with system-level tamper resistance — the same principles used in enterprise endpoint security, applied to protecting recovery.

The entire platform is open source because when a tool claims to protect your privacy, you should be able to verify that claim yourself.

---

## GitHub Repository

**Description (one line, <350 chars):**
Free, open-source gambling blocking platform. Three protection layers (DNS + app + browser), cross-platform Rust agent (Windows/macOS/Linux/Android/iOS), accountability partner support, institutional management, self-hostable. Privacy-first: no tracking, no data selling, auditable code.

**Topics:**
```
gambling-blocker
gambling-addiction
gambling-recovery
rust
axum
cross-platform
open-source
dns-filtering
endpoint-security
tamper-resistance
privacy
self-hosted
accountability
mental-health
harm-reduction
nextjs
docker
typescript
postgresql
security
```
