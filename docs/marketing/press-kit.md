# BetBlocker Press Kit

---

## Project Summary

### 100-Word Version

BetBlocker is a free, open-source gambling blocking platform that protects every device a person owns. Unlike single-layer DNS blockers, BetBlocker combines DNS filtering, application blocking, and browser content scanning with system-level tamper resistance. The endpoint agent is written in Rust and runs natively on Windows, macOS, Linux, Android, and iOS. Accountability partners and treatment providers can monitor blocking health without accessing browsing history. Self-hosted deployments include every feature at no cost. The entire codebase is auditable — no tracking, no data selling, no browsing history collection. Built for people who are serious about gambling recovery.

### 250-Word Version

BetBlocker is a free, open-source, cross-platform gambling blocking platform designed for people in recovery from gambling addiction, their accountability partners, and institutional treatment programs.

Most gambling blockers rely on a single layer of DNS filtering, which can be bypassed by changing DNS settings, using a VPN, or opening a native gambling app. BetBlocker addresses this with three independent blocking layers: DNS and network filtering that intercepts gambling domains before they load, application blocking that prevents gambling apps from launching or being installed, and browser content scanning that catches gambling ads and affiliate links on non-gambling sites. If one layer is circumvented, the others continue to protect.

The endpoint agent is written in Rust and compiles natively for Windows, macOS, Linux, Android, and iOS from a single codebase. It runs as a system-level service with mutual watchdog processes, binary integrity validation, and kernel-level protections per platform. Unenrollment requires either a time delay or accountability partner approval — never an instant toggle.

BetBlocker supports three enrollment tiers: self-enrolled individuals with time-delayed unenrollment, partner-enrolled users whose accountability partner must approve changes, and institutional enrollments for court-mandated programs and treatment providers with full audit trails.

Privacy is an architectural principle. BetBlocker performs no keylogging, screen capture, location tracking, or browsing history collection. The entire codebase is open source and auditable. Self-hosted deployments include every feature at no cost and keep all data on the user's own infrastructure.

### 500-Word Version

BetBlocker is a free, open-source, cross-platform gambling blocking platform built for individuals in recovery from gambling addiction, their accountability partners, and institutional treatment programs. It provides the most comprehensive gambling blocking available — three independent protection layers with system-level tamper resistance — while maintaining strict privacy boundaries and full transparency through open-source code.

**The Problem.** Online gambling is accessible 24 hours a day on every device. For the estimated 80 million people worldwide who struggle with problem gambling, that constant accessibility makes recovery significantly harder. Existing gambling blocking tools rely almost exclusively on DNS filtering — a single layer of protection that can be bypassed by changing DNS providers, installing a VPN, using encrypted DNS, or opening a native gambling application instead of a website. For people in recovery, a tool that can be defeated in a moment of vulnerability is not adequate protection.

**The Solution.** BetBlocker combines three independent blocking layers. DNS and network filtering intercepts gambling domain requests before they reach the network, across all applications and browsers, including those with hardcoded DNS settings. Application blocking detects known gambling apps, prevents them from launching, and blocks new installations. Browser content scanning identifies gambling advertisements, affiliate links, and promotions embedded in non-gambling websites. Each layer operates independently — if one is circumvented, the others continue to block.

**Tamper Resistance.** The BetBlocker agent runs as a system-level service with mutual watchdog processes, periodic binary integrity validation, and platform-specific kernel protections: WFP callout drivers and kernel minifilters on Windows, System Extensions and Endpoint Security on macOS, AppArmor and SELinux policies on Linux, Device Administrator and Knox integration on Android, and MDM profiles on iOS. Unenrollment is intentionally deliberate: self-enrolled users face a configurable 24-to-72-hour delay, partner-enrolled users require their accountability partner to approve, and institutional enrollments require the enrolling authority's approval with a full audit trail.

**Accountability Model.** BetBlocker natively supports accountability partners — therapists, sponsors, family members, or other trusted individuals — who receive their own dashboard showing device protection status and tamper alerts. Partners confirm that blocking is active without accessing browsing history or personal activity. Treatment programs and court-mandated recovery programs can manage enrollments across multiple clients from a centralized dashboard with compliance reporting.

**Privacy Architecture.** BetBlocker performs no keylogging, screen capture, location tracking, or browsing history collection. It collects only the minimum gambling-domain metadata necessary for blocking intelligence, and this is configurable per enrollment. No data is sold or shared with third parties. Self-hosted deployments keep all data on the user's own infrastructure with zero telemetry.

**Technology.** The endpoint agent is written in Rust and compiles natively for Windows, macOS, Linux, Android, and iOS from a single codebase. The central API (Rust, Axum) and background workers share types and domain logic with the agent. The web platform (Next.js) provides dashboards for users, partners, institutions, and administrators. The entire platform ships as Docker containers — self-hosted deployment requires a single `docker compose up` command.

**Pricing.** Self-hosted deployment is free and includes every feature. Managed hosting is $10 per month for individuals and families. Institutional pricing is available for treatment programs and court systems requiring dedicated support and SLAs.

---

## Key Facts and Figures

| Category | Detail |
|----------|--------|
| **License** | [License type — to be confirmed] |
| **Primary language** | Rust |
| **Supported platforms** | 5 (Windows, macOS, Linux, Android, iOS) |
| **Blocking layers** | 3 (DNS/network, application, browser/content) |
| **Enrollment tiers** | 3 (self, partner, institutional) |
| **Crate count** | [To be updated as development progresses] |
| **Test count** | [To be updated as development progresses] |
| **Lines of code** | [To be updated as development progresses] |
| **API framework** | Axum (Rust) |
| **Web framework** | Next.js (React + TypeScript) |
| **Database** | PostgreSQL + Redis + TimescaleDB |
| **Deployment** | Docker Compose (self-hosted), Kubernetes (managed) |
| **Self-hosted price** | Free (all features included) |
| **Managed hosting price** | $10/month (standard) |
| **Data collection** | Minimum gambling-domain metadata only; no browsing history |

---

## Founder Bio

[Placeholder — to be written]

[Name] is [role/background]. [Relevant experience and motivation for building BetBlocker.] [Contact preference.]

---

## Contact Information

- **General inquiries:** [email placeholder]
- **Press inquiries:** [email placeholder]
- **Institutional partnerships:** [email placeholder]
- **Security disclosures:** [email placeholder — responsible disclosure policy link]
- **GitHub:** [repository URL]
- **Website:** [betblocker.com or equivalent]

---

## Logo Usage Guidelines

[Placeholder — to be developed with brand identity]

### When available, this section will include:

- **Primary logo** — full color, for use on light backgrounds
- **Reversed logo** — for use on dark backgrounds
- **Monochrome versions** — black and white variants
- **Icon/mark** — square format for app icons, favicons, and social media avatars
- **Minimum size** — the smallest size at which the logo remains legible
- **Clear space** — the minimum padding required around the logo
- **Color specifications** — primary, secondary, and accent colors with hex, RGB, and CMYK values
- **Prohibited usage** — stretching, recoloring outside approved palette, placing on busy backgrounds, modifying proportions

### File formats to be provided:

- SVG (vector, scalable)
- PNG (transparent background, multiple resolutions: 1x, 2x, 4x)
- ICO (favicon)
- PDF (print)

---

## Boilerplate

*BetBlocker is a free, open-source gambling blocking platform that combines DNS filtering, application blocking, and browser content scanning with system-level tamper resistance across Windows, macOS, Linux, Android, and iOS. Built for individuals in recovery, accountability partners, and treatment institutions. Self-hosted deployments include every feature at no cost. Learn more at [website].*

---

*This press kit was last updated on 2026-03-15. For the latest information, visit [website] or contact [press email].*
