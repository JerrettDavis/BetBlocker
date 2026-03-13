# BetBlocker — Vision & Design Document

**Date:** 2026-03-12
**Status:** Draft
**Authors:** JD + Claude

---

## 1. Mission

BetBlocker is a cross-platform gambling blocking platform that helps individuals, accountability partners, and institutions enforce gambling abstinence on enrolled devices. It provides the most aggressive, tamper-resistant blocking available — from DNS filtering to kernel-level protections — while respecting privacy and giving enrollment authorities appropriate oversight.

## 2. Core Invariant

**The enrollment authority determines the unenrollment authority, the reporting visibility, and the bypass protection level.**

Every policy decision in the system flows through this principle.

## 3. Enrollment Tiers

| Tier | Enrolled By | Unenrollment | Reporting | Bypass Protection |
|------|------------|--------------|-----------|-------------------|
| **Self** | Individual user | Time-delayed (configurable 24-72h), notifications sent | User chooses own level, can opt out | Standard (DNS + app + browser layers) |
| **Partner** | Accountability partner | Requires partner approval via web panel | Aggregated by default, detailed with mutual consent | Elevated (same layers + tamper alerts to partner) |
| **Authority** | Court/program/institution | Requires institutional approval + audit trail | Mandated by authority, full audit log | Maximum (all layers + compliance reporting) |

## 4. Domain Model

- **Account** — a person (user, partner, or authority representative)
- **Organization** — optional grouping (therapy practice, court program, family)
- **Device** — an enrolled endpoint, owned by an Account, governed by an Enrollment
- **Enrollment** — the binding between a Device and its authority. Contains tier, protection config, reporting config, and unenrollment policy. This is the heart of the system.
- **Blocklist** — the centralized gambling intelligence (curated + automated + federated)
- **Event** — any reportable action (block, bypass attempt, tamper detection, enrollment change)

## 5. System Architecture

### 5.1 Endpoint Agent (Rust)

Single Rust codebase compiled per platform with thin native integration layers.

**Core engine (cross-platform Rust):**
- DNS interception and local resolver
- Blocklist matching (domain, pattern, heuristic)
- Event reporting with privacy-aware filtering
- Tamper detection and self-healing
- Secure communication with API (mTLS + certificate pinning)
- Plugin system (Rust traits with platform-specific implementations)

**Platform shims (minimal native code):**

| Platform | Service Model | Network Hook | Deep Integration |
|----------|--------------|--------------|------------------|
| Windows | Windows Service | WFP (Windows Filtering Platform) driver | Kernel minifilter for file protection |
| macOS | launchd daemon | Network Extension framework | System Extension + Endpoint Security |
| Linux | systemd service | iptables/nftables | AppArmor/SELinux MAC policies |
| Android | Foreground Service | VpnService (local VPN) | Device Admin/Owner, Knox on Samsung |
| iOS | Network Extension | NEDNSProxyProvider | MDM profile, Screen Time API |

### 5.2 Central API (Rust — Axum)

Single binary, stateless, horizontally scalable.

Responsibilities:
- Authentication & authorization (JWT + refresh tokens)
- Enrollment management (create, modify, unenroll with policy enforcement)
- Device communication (blocklist sync, config push, event ingestion)
- Billing integration (Stripe) — hosted tier only, disabled via env flag for self-hosted
- Blocklist management (curated CRUD, automated discovery intake, federated report ingestion + review queue)
- Reporting engine (aggregation, per-enrollment visibility rules)

### 5.3 Web Platform (Next.js)

- Marketing/landing site (static pages)
- User dashboard (device status, reports, enrollment management)
- Partner dashboard (managed devices, approval queue, reports)
- Authority dashboard (compliance views, audit logs, bulk management)
- Admin panel (blocklist management, review queue, platform analytics)
- Self-hosted admin (subset — no billing, no marketing pages)

### 5.4 Background Worker (Rust)

Async job processor:
- Blocklist compilation and delta generation
- Federated report processing and classification
- Automated discovery pipeline (crawlers, content analyzers)
- Scheduled analytics aggregation
- Heartbeat timeout detection and alerting

### 5.5 Data Stores

| Store | Purpose | Hosted | Self-hosted |
|-------|---------|--------|-------------|
| PostgreSQL | Primary data (accounts, enrollments, devices, blocklist metadata) | Managed (RDS/CloudSQL) | Self-managed or containerized |
| Redis | Session cache, real-time device status, pub/sub for push | Managed (ElastiCache) | Containerized |
| TimescaleDB | Event analytics, time-series reporting | Managed or containerized | Containerized |

## 6. Blocking Layers

### Layer 1: DNS/Network Blocking (Phase 1)

- Local DNS resolver intercepts all queries before they leave the device
- Blocked domains return NXDOMAIN or redirect to interstitial page
- HOSTS file plugin as redundant fallback (survives agent crashes)
- VPN/proxy detection: monitors DNS config changes, VPN tunnels, proxy settings
- Encrypted DNS enforcement: blocks or co-opts DoH/DoT — agent becomes the device's DoH provider
- Platform network hooks (WFP, NetworkExtension, VpnService) ensure apps with hardcoded DNS can't bypass

### Layer 2: Application Blocking (Phase 2)

- App inventory scanner: enumerates installed apps, matches against gambling app signatures (package names, bundle IDs, code signing certs)
- Launch interception: detects and blocks/kills gambling app processes
- Install prevention: monitors package manager activity, blocks gambling app installations
- App store content filtering: coordinates with browser layer to block gambling app store pages

### Layer 3: Browser/Content Blocking (Phase 3)

- Browser extension (Chrome, Firefox, Safari, Edge): content scripts scan for gambling elements — ads, iframes, affiliate links, promotions
- Keyword and visual heuristic matching on non-blocked domains
- Search result filtering: removes gambling results
- Extension tamper protection: agent monitors extension presence, alerts on removal

### Cross-cutting: Federated Intelligence

- Agents report blocked-attempt metadata (domain, category, timestamp) per enrollment visibility rules
- Heuristic hits on unknown domains reported to central review queue
- **Never full browsing history** — only blocked/flagged domain metadata
- Central pipeline: automated classifiers process reports, human review before blocklist promotion
- Blocklist versioning: incremental delta sync, cryptographically signed by API

## 7. Tamper Resistance & Anti-Bypass

### Agent Self-Protection

- Runs as system-level service (SYSTEM/root/launchd) — unprivileged users cannot stop it
- Binary integrity: validates own hash on startup and periodically, self-repairs from cached signed copy
- Watchdog: secondary process monitors primary agent, mutual supervision
- Config encryption: enrollment credentials encrypted with hardware-bound keys (TPM, Keychain, Keystore)

### OS-Level Protections

- **Windows:** WFP callout driver persists blocking if agent terminated; kernel minifilter prevents agent file deletion
- **macOS:** System Extension + Endpoint Security framework; removal requires admin + reboot (agent detects and reports)
- **Linux:** systemd `ProtectSystem=strict`, immutable file attributes, AppArmor/SELinux policies
- **Android:** Device Administrator/Owner enrollment; undeletable without factory reset; Knox integration on Samsung
- **iOS:** MDM profile enrollment; removal requires MDM authority (maps to enrollment authority model)

### Network Bypass Detection

- Monitors for new VPN connections, proxy changes, DNS modifications, Tor usage
- Detects alternative network paths circumventing filtering
- Response configurable per tier: log-only, alert partner, or lock down to known-safe destinations

### Heartbeat & Dead-Man's Switch

- Periodic heartbeats to API with status, blocklist version, integrity check
- Missed heartbeat window triggers accountability partner/authority alert
- Self-enrolled configures own threshold; partner/authority tiers enforce minimums

### Explicit Privacy Boundaries

- No keylogging or screen capture
- No full browsing history collection
- No microphone, camera, or location access
- No data sold or shared with third parties

## 8. Deployment Architecture

### Core Principle

One artifact, two deployment models. The hosted platform runs the exact same containers self-hosters run.

### Container Topology

| Container | Contents | Size |
|-----------|----------|------|
| `betblocker-api` | Rust binary (Axum), stateless | ~10-20MB |
| `betblocker-web` | Next.js app, SSR + static | Standard Node image |
| `betblocker-worker` | Rust binary, async jobs | ~10-20MB |
| `betblocker-db` | PostgreSQL | Standard PG image |
| `betblocker-cache` | Redis | Standard Redis image |
| `betblocker-analytics` | TimescaleDB | Standard TS image |

### Hosted Platform

- Kubernetes on AWS (application layer remains cloud-agnostic)
- Managed database, cache, analytics
- CDN for web assets + agent binary distribution
- Stripe billing active
- Automated discovery pipeline running
- Federated report review queue staffed

### Self-Hosted Platform

- `docker-compose.yml` — single file, `docker compose up` gets you running
- Optional Helm chart for Kubernetes
- Billing disabled via environment flag
- Blocklist syncs from public BetBlocker community feed (free)
- Federated reports contribute back to central blocklist (opt-in)
- No phone-home, no telemetry unless explicitly opted in

### Agent Distribution

- Hosted: download from betblocker.com, auto-updates via API
- Self-hosted: download from own instance, updates with deployment
- All binaries cryptographically signed; agent validates signature matches configured authority

### Configuration Hierarchy

```
API sets policy -> Enrollment overrides -> Platform defaults
```

Switching hosted <-> self-hosted requires device re-enrollment.

## 9. Technology Stack

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| Endpoint agent core | Rust | Memory safety, native performance, compiles to all targets, industry standard for security tooling |
| Platform shims | Native per OS (C/Swift/Kotlin/C#) | Minimal — only for OS-specific APIs that Rust FFI can't reach directly |
| Central API | Rust (Axum) | Shared types with agent, single binary deployment, hardened backend |
| Background worker | Rust | Same codebase as API, shared domain logic |
| Web platform | Next.js (React + TypeScript) | Developer velocity for UI, SSR for dashboards, static export for marketing |
| Primary database | PostgreSQL | Battle-tested, rich feature set, strong ecosystem |
| Cache | Redis | Session management, real-time status, pub/sub |
| Analytics | TimescaleDB | Time-series optimized, PostgreSQL compatible |
| Containerization | Docker + docker-compose + Helm | Portable across hosted and self-hosted |
| Orchestration (hosted) | Kubernetes | Horizontal scaling, rolling updates |
| Billing | Stripe | Industry standard, handles subscriptions |

## 10. Phasing Plan

### Phase 1 — Foundation (MVP)

**Central API:**
- Authentication, account management, enrollment CRUD
- Device registration, heartbeat, blocklist sync endpoint
- Stripe billing integration (hosted tier)

**Web Platform:**
- Marketing site, registration, login
- Device dashboard, basic partner invitation flow
- Enrollment management panel
- Admin blocklist management interface

**Endpoint Agent (all 5 platforms):**
- DNS/network blocking (Layer 1)
- HOSTS file plugin
- Blocklist sync + delta updates
- Heartbeat + basic status reporting
- Service-level tamper resistance + watchdog
- Event reporting for blocked attempts

**Blocklist:**
- Seeded with known gambling domains (public lists)
- Manual curation via admin panel

**Enrollment flows:**
- Self-enrollment with time-delayed unenrollment
- Partner enrollment with partner-approval unenrollment

**Self-hosted:**
- docker-compose deployment
- Setup documentation

**Reporting:**
- Device status dashboard
- Blocked attempt counts
- Tamper alerts

### Phase 2 — Depth

- Application blocking (Layer 2): app scanning, launch interception, install prevention
- Enhanced tamper resistance: kernel-level protections per platform (WFP, System Extension, Device Admin)
- Automated discovery pipeline: domain crawlers, content classifiers, review queue
- Federated reporting: agents contribute unknown domain intelligence
- Advanced reporting: time-series analytics, trends, exportable reports
- Organization support: group partners/devices under orgs
- VPN/proxy/Tor detection and response

### Phase 3 — Breadth

- Browser/content blocking (Layer 3): extensions, content scanning, search filtering
- Authority tier: court/program enrollment, compliance reporting, bulk management, audit trails
- Partnership integrations: therapy platform APIs, accountability app integrations
- Tiered subscriptions: premium tiers bundling partner services
- Mobile hardening: Knox, iOS MDM profiles
- Public API for third-party recovery app integration

### Phase 4 — Scale

- ML-powered gambling site discovery and classification
- Network effect analytics: intelligence quality metrics, coverage scoring
- Multi-language/region support: localized blocklists for regional operators
- Enterprise/institutional tier: bulk licensing, SSO, dedicated support, SLA
- Open source community: contributor program for blocklist and plugin development

## 11. Monetization

| Tier | Price | Includes |
|------|-------|----------|
| **Self-hosted** | Free forever | Full platform, community blocklist, community support |
| **Hosted Standard** | $10/month | Managed deployment, priority blocklist updates, email support, auto-updates |
| **Hosted Partner** | $15/month (future) | Standard + therapy integrations, advanced reporting, priority support |
| **Institutional** | Custom pricing (future) | Bulk licensing, compliance reporting, SSO, dedicated support, SLA |

## 12. Growth Strategy

**Flywheel:**
1. Free self-hosted builds trust in recovery communities
2. Every enrolled device strengthens federated intelligence
3. Accountability partners become evangelists
4. Therapy/court partnerships create institutional adoption channels
5. Open source credibility attracts security researchers

**Key Metrics:**
- Enrolled devices (total + active heartbeats)
- Block rate (blocked attempts / total gambling DNS queries)
- Bypass rate (successful bypasses / enrolled device-days)
- Federated contribution rate (reporting agents / total agents)
- Self-hosted to hosted conversion rate
- Churn (unenrollments / enrollments per month)
- Time-to-re-enroll (unenrolled users returning)

## 13. Explicit Non-Goals

- BetBlocker is not spyware — no keylogging, screen capture, location tracking
- BetBlocker is not a general content filter — it blocks gambling only
- BetBlocker does not sell data — ever
- BetBlocker does not require an internet connection to block (local blocklist cache works offline)
- BetBlocker does not lock users out of their devices — it blocks gambling access, nothing else
