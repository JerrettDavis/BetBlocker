# Phase 2 Design Document -- "Depth"

**Date:** 2026-03-13
**Status:** Draft
**Authors:** JD + Claude
**Depends on:** Phase 1 complete (DNS blocking, basic tamper resistance, enrollment flows, API, web dashboard)

---

## Overview

Phase 1 established BetBlocker's foundation: DNS/network blocking (Layer 1), a plugin system with `DnsResolverPlugin` and `HostsFilePlugin`, basic tamper resistance (service model, watchdog, binary integrity, heartbeats), enrollment flows (self and partner tiers), and the central API and web platform.

Phase 2 -- "Depth" -- deepens every dimension of the system. It adds a second blocking layer (application blocking), hardens tamper resistance with kernel-level protections, builds the automated discovery pipeline, enables federated reporting from agents, introduces organization support, adds VPN/proxy/Tor detection, delivers advanced analytics, and completes the Windows and macOS platform shims (which were Linux-only in Phase 1).

Phase 2 does NOT add:
- Browser/content blocking (Layer 3) -- that is Phase 3
- Authority tier enrollment -- Phase 3
- WASM community plugins -- Phase 4
- ML-powered classification -- Phase 4 (Phase 2 uses rule-based classifiers)

---

## Table of Contents

1. [Application Blocking (Layer 2)](#1-application-blocking-layer-2)
2. [Enhanced Tamper Resistance](#2-enhanced-tamper-resistance)
3. [Automated Discovery Pipeline](#3-automated-discovery-pipeline)
4. [Federated Reporting](#4-federated-reporting)
5. [Advanced Reporting](#5-advanced-reporting)
6. [Organization Support](#6-organization-support)
7. [VPN/Proxy/Tor Detection](#7-vpnproxytor-detection)
8. [Windows and macOS Platform Shims](#8-windows-and-macos-platform-shims)
9. [Dependency Graph](#9-dependency-graph)
10. [Sub-plan Groupings](#10-sub-plan-groupings)

---

## 1. Application Blocking (Layer 2)

### What It Achieves

DNS blocking prevents access to gambling websites, but gambling applications installed locally can bypass DNS entirely (they may use hardcoded IPs, certificate-pinned connections, or embedded content). Layer 2 closes this gap by detecting, blocking, and preventing installation of gambling apps.

### Key Technical Approach

Application blocking has three pillars:

**Pillar 1 -- App Inventory Scanning.** On activation and periodically (configurable, default every 15 minutes), the agent enumerates installed applications and matches them against a gambling app signature database. Signatures include:
- Package names (Android: `com.bet365.app`, iOS: bundle IDs)
- Executable paths and names (Windows: `bet365.exe`, macOS: `Bet365.app`)
- Code signing certificate fingerprints (allows matching regardless of rename)
- Display name fuzzy matching (catches repackaged or white-label apps)

The signature database is part of the blocklist and syncs via the same delta protocol used for domain lists (ADR-003). The `Blocklist` struct in `bb-agent-plugins/src/blocklist/` gains an `app_signatures: Vec<AppSignature>` field.

**Pillar 2 -- Launch Interception.** When a matched app is detected running, the agent terminates the process and logs the event.
- Desktop (Windows/macOS/Linux): process monitoring via platform APIs. The agent watches for new process creation events and checks the executable against the signature database.
- Android: `UsageStatsManager` polling (5-second interval) plus `AccessibilityService` for real-time detection. When a gambling app moves to the foreground, the agent brings itself to the foreground with a blocking overlay.

**Pillar 3 -- Install Prevention.** The agent monitors for new app installations and blocks them if they match the signature database.
- Windows: monitors `%ProgramFiles%`, `%AppData%`, and installer temp directories for new executables via `ReadDirectoryChangesW`.
- macOS: monitors `/Applications` and `~/Applications` via FSEvents.
- Linux: monitors package manager activity (apt, dnf, snap, flatpak) via inotify on package cache directories plus DBUS signal monitoring for PackageKit.
- Android: `BroadcastReceiver` on `ACTION_PACKAGE_ADDED`. With Device Owner enrollment (Phase 2 tamper resistance), the agent can use `DevicePolicyManager.setUninstallBlocked()` and package install restrictions.
- iOS: not possible in the general case due to App Store sandboxing. Relies on MDM app restrictions (Phase 2 tamper resistance) and DNS blocking of gambling app download endpoints.

### New Crates/Files

| Location | Description |
|----------|-------------|
| `crates/bb-agent-plugins/src/app_process/` | `AppProcessPlugin` -- cross-platform app blocking plugin for desktop (Windows, macOS, Linux) |
| `crates/bb-agent-plugins/src/app_process/scanner.rs` | Platform-abstracted app inventory scanner |
| `crates/bb-agent-plugins/src/app_process/interceptor.rs` | Process creation monitoring and kill logic |
| `crates/bb-agent-plugins/src/app_process/install_watcher.rs` | Installation prevention monitors |
| `crates/bb-agent-plugins/src/app_device_admin/` | `AppDeviceAdminPlugin` -- Android-specific app blocking via Device Admin/Owner APIs |
| `crates/bb-common/src/models/app_signature.rs` | `AppSignature` model (package name, cert hash, display patterns) |
| `crates/bb-agent-plugins/src/blocklist/app_signatures.rs` | App signature matching engine (exact match + fuzzy) |
| `crates/bb-api/src/routes/app_signatures.rs` | Admin CRUD endpoints for app signature management |
| `crates/bb-api/src/services/app_signature_service.rs` | App signature business logic |

### Registry Changes

The `PluginInstance` enum gains two new variants:

```rust
#[cfg(feature = "app-process")]
AppProcess(AppProcessPlugin),

#[cfg(feature = "app-device-admin")]
AppDeviceAdmin(AppDeviceAdminPlugin),
```

The `PluginRegistry` gains a `check_app()` method (matching the existing `check_domain()` pattern) that queries all App-layer plugins and short-circuits on the first `Block` decision.

### New Cargo Feature Flags

```toml
app-process = ["bb-plugin-app-process"]
app-device-admin = ["bb-shim-android/device-admin"]
```

### Dependencies on Phase 1

- Plugin system (`BlockingPlugin`, `AppBlockingPlugin` traits, `PluginRegistry`) -- exists, traits defined
- Blocklist sync protocol (ADR-003) -- extend for app signatures
- Event reporting pipeline -- extend with `AppDetected` event type (already defined in ADR-007)
- `AppIdentifier` and `AppMatch` types in `bb-agent-plugins/src/types.rs` -- exist as placeholders

### Estimated Complexity

**L (Large)**. The app inventory scanner is straightforward, but launch interception requires platform-specific process monitoring APIs on four platforms (five minus iOS where it is not possible). Install prevention adds another axis of platform-specific work. The gambling app signature database needs an initial seed and ongoing curation.

### Sub-plan Grouping

- **P2-APP-1:** App signature model, database schema, API CRUD, admin UI, seed data
- **P2-APP-2:** App inventory scanner (cross-platform abstraction + platform implementations)
- **P2-APP-3:** `AppProcessPlugin` with launch interception (desktop platforms)
- **P2-APP-4:** `AppDeviceAdminPlugin` (Android-specific: AccessibilityService, Device Owner integration)
- **P2-APP-5:** Install prevention monitors (all platforms)

---

## 2. Enhanced Tamper Resistance

### What It Achieves

Phase 1 tamper resistance operates entirely in user space: system service model, watchdog, binary integrity checks, heartbeats. This deters casual and motivated users (TA-1, TA-2 in the threat model) but a technical user (TA-3) with admin/root access can defeat user-space protections.

Phase 2 adds kernel-level protections that persist even when the agent process is killed, and platform-specific hardening that makes tampering significantly harder even for users with administrative credentials.

### Key Technical Approach

Each platform gets a kernel-level or system-level protection mechanism. The common thread is: blocking rules and file protection operate at a layer below the agent process, so killing the agent does not disable blocking.

#### Windows: WFP Callout Driver + Kernel Minifilter

**WFP Callout Driver.** A WHQL-signed Windows Filtering Platform callout driver intercepts DNS traffic at the network stack level. Even if the `bb-agent` service is stopped, the WFP rules persist and continue blocking gambling DNS queries. The driver communicates with the agent via IOCTL for blocklist updates.

**Kernel Minifilter.** A WHQL-signed filesystem minifilter prevents modification or deletion of agent files (`C:\Program Files\BetBlocker\*`) by any process other than the agent's own update mechanism. This stops users from simply deleting the agent binary.

**Critical dependency:** Both drivers require WHQL (Windows Hardware Quality Labs) signing. This is a multi-week process involving Microsoft's hardware dev center. The drivers must be developed, tested extensively (Verifier, HLK), submitted, and signed before they can be deployed.

**Protected Process Light (PPL).** If BetBlocker qualifies for Early Launch Anti-Malware (ELAM) registration, the agent can run as a PPL process that even administrator-level processes cannot terminate. This is aspirational for Phase 2 -- ELAM registration has strict Microsoft requirements.

#### macOS: System Extension + Endpoint Security

**System Extension.** A Network Extension System Extension (replacing the App Extension from Phase 1) runs in its own process, managed by the OS, and survives agent process termination. Removal requires admin authentication plus a reboot, and the agent detects deregistration attempts.

**Endpoint Security Framework.** The agent registers an Endpoint Security client to monitor:
- File modifications targeting agent binaries and configuration
- Process kills targeting agent or watchdog PIDs
- Authorization events (attempts to change DNS, install VPN profiles)

Endpoint Security events allow the agent to detect and report tampering in real time, even if it cannot block the action (ES is observe-only for some event types; AUTH events can block).

**Notarization.** The System Extension must be notarized by Apple. This requires a Developer ID certificate and submission to Apple's notarization service.

#### Linux: AppArmor/SELinux MAC Policies

**AppArmor Profile.** For Ubuntu/Debian systems, an AppArmor profile confines the agent's own access (defense-in-depth) and critically, protects agent files and processes from modification by other processes. The profile denies `ptrace` attachment, signal delivery, and file write access to agent paths for all non-agent processes.

**SELinux Policy Module.** For RHEL/Fedora systems, a custom SELinux policy module provides equivalent protection using mandatory access control. The agent runs in a dedicated SELinux domain with a custom type for its files.

**eBPF DNS Interception (experimental).** A BPF program attached to the XDP or TC hook intercepts DNS packets at the kernel level, independent of the agent process. This is a stretch goal for Phase 2 -- eBPF program deployment and lifecycle management adds significant complexity.

#### Android: Device Administrator / Device Owner

**Device Administrator.** The baseline. Prevents app uninstall without deactivating device admin first. Deactivation triggers a tamper alert. Already partially implemented in Phase 1.

**Device Owner.** The full-control mode, provisioned via QR code during device setup (or ADB for already-set-up devices). Device Owner can:
- Silently re-enable VPN if user disconnects
- Prevent app uninstall entirely (no deactivation option)
- Restrict access to Settings screens
- Enforce app install restrictions
- Set up a managed profile that isolates blocking infrastructure

**Samsung Knox.** On Samsung devices, Knox Workspace provides managed VPN (user cannot disconnect), app protection (user cannot force-stop), and enterprise-grade device policy enforcement. Knox integration requires Samsung partnership/SDK access.

#### iOS: MDM Profile Enrollment

**MDM Profile.** For partner and authority tiers, the Network Extension is deployed via a Mobile Device Management profile. An MDM-managed Network Extension cannot be removed by the user -- removal requires the MDM authority (which maps to the enrollment authority). This is the strongest protection iOS offers.

**Supervised Mode.** For institutional deployments, devices can be put into supervised mode (via Apple Configurator or Apple Business Manager), which prevents users from modifying VPN/DNS settings entirely.

**Implementation note:** BetBlocker must operate or partner with an MDM provider. The MDM infrastructure (APNS certificates, MDM server endpoint, profile signing) is a separate operational concern from the agent itself.

#### All Platforms: Kernel-Level File Protection

Across all platforms, the goal is the same: agent binaries, configuration files, and the local blocklist cache are protected at the OS kernel level from modification by any process other than the agent's own update mechanism. The specific mechanism varies (minifilter, Endpoint Security, AppArmor/SELinux, Device Owner, MDM) but the effect is uniform.

### New Crates/Files

| Location | Description |
|----------|-------------|
| `crates/bb-shim-windows/src/wfp.rs` | WFP callout driver IOCTL interface |
| `crates/bb-shim-windows/src/minifilter.rs` | Kernel minifilter IOCTL interface |
| `crates/bb-shim-windows/driver/` | WFP callout driver source (C, built with WDK) |
| `crates/bb-shim-windows/minifilter/` | Filesystem minifilter source (C, built with WDK) |
| `crates/bb-shim-macos/src/system_ext.rs` | System Extension lifecycle management |
| `crates/bb-shim-macos/src/endpoint_security.rs` | Endpoint Security client registration and event handling |
| `crates/bb-shim-macos/bridge/swift/` | Swift bridge for System Extension and ES framework APIs |
| `crates/bb-shim-linux/src/apparmor.rs` | AppArmor profile installation and management |
| `crates/bb-shim-linux/src/selinux.rs` | SELinux policy module installation and management |
| `crates/bb-shim-linux/src/ebpf.rs` | eBPF program loading and management (stretch goal) |
| `deploy/apparmor/betblocker-agent` | AppArmor profile definition |
| `deploy/selinux/betblocker.te` | SELinux type enforcement policy |
| `crates/bb-shim-android/src/device_owner.rs` | Device Owner provisioning and policy management |
| `crates/bb-shim-android/src/knox.rs` | Samsung Knox SDK integration |

### Dependencies on Phase 1

- Watchdog process and mutual supervision (operational in Phase 1; kernel protections augment, not replace)
- Binary integrity checking (Phase 1 validates hashes; Phase 2 prevents modification at the OS level)
- Heartbeat protocol with `ProtectionStatus` struct (extend to report kernel protection status)
- Platform shim crate structure (`bb-shim-windows`, `bb-shim-macos`, `bb-shim-linux`, `bb-shim-android` exist as compilation targets; Phase 2 fills in the kernel-level implementations)
- Windows and macOS platform shims must be completed first (see section 8)

### Estimated Complexity

**XL (Extra Large)**. This is the most complex Phase 2 deliverable. Each platform has a fundamentally different kernel protection mechanism. Windows drivers require WHQL signing (multi-week external process). macOS System Extensions require notarization. Android Device Owner has specific provisioning requirements. iOS MDM requires operational infrastructure. Testing requires admin-level bypass attempts on each platform.

### Sub-plan Grouping

- **P2-TAMPER-1:** Windows WFP callout driver (design, C driver, IOCTL interface in Rust, WHQL process)
- **P2-TAMPER-2:** Windows kernel minifilter (C driver, IOCTL interface, WHQL process)
- **P2-TAMPER-3:** macOS System Extension + Endpoint Security (Swift bridge, notarization)
- **P2-TAMPER-4:** Linux AppArmor + SELinux policies (profile/policy authoring, installation automation)
- **P2-TAMPER-5:** Android Device Owner + Knox integration (provisioning flow, policy management)
- **P2-TAMPER-6:** iOS MDM profile integration (MDM infrastructure, profile signing, Network Extension management)
- **P2-TAMPER-7:** eBPF DNS interception for Linux (stretch goal)

---

## 3. Automated Discovery Pipeline

### What It Achieves

Phase 1's blocklist is seeded from public gambling domain lists and manually curated via the admin panel. This does not scale. New gambling sites launch constantly, using new domains, subdomains, and URL shorteners. The automated discovery pipeline continuously finds new gambling domains and routes them through classification and review before they are added to the blocklist.

### Key Technical Approach

The pipeline runs in the `bb-worker` crate as a set of background jobs.

**Stage 1 -- Domain Crawling.** Scheduled crawlers visit known gambling-adjacent sources:
- Gambling affiliate networks and directories
- Gambling license registries (UKGC, MGA, Curacao) -- many publish licensee lists
- WHOIS registration patterns (registrants who have previously registered gambling domains)
- DNS zone changes for TLDs popular with gambling operators (.bet, .casino, .poker, .games)
- Search engine results for gambling keywords (automated search queries with rotating proxies)

Each crawler produces a stream of candidate domains with source metadata.

**Stage 2 -- Content Classification.** Each candidate domain is analyzed:
- HTTP response analysis: fetch the page, extract text, classify content using rule-based classifiers (keyword density, gambling-specific terminology like "odds", "wager", "deposit bonus", "free spins")
- HTML structure analysis: look for gambling-specific page elements (betting slips, odds tables, deposit forms, responsible gambling notices)
- Certificate and WHOIS analysis: registrant patterns, hosting infrastructure overlap with known gambling sites
- Link graph analysis: does this domain link to or from known gambling domains?

Phase 2 uses rule-based classifiers. ML-based classification is deferred to Phase 4 but the pipeline architecture should support pluggable classifiers so the ML classifier can drop in later.

**Stage 3 -- Confidence Scoring.** Each candidate domain receives a confidence score (0.0 to 1.0) based on the classifier outputs. Domains above a high threshold (e.g., 0.95) are auto-flagged for review with high priority. Domains in the medium range (0.5-0.95) are flagged for standard review. Domains below 0.5 are discarded.

**Stage 4 -- Review Queue.** Flagged domains enter a review queue visible in the admin panel. Reviewers can:
- Approve (add to blocklist with category and confidence)
- Reject (mark as false positive, add to allowlist)
- Defer (needs more investigation)
- Bulk approve/reject for efficiency

The review queue is the same queue used by federated reporting (section 4). Both discovery pipeline outputs and federated reports converge here.

### New Crates/Files

| Location | Description |
|----------|-------------|
| `crates/bb-worker/src/discovery/` | Discovery pipeline module |
| `crates/bb-worker/src/discovery/crawler.rs` | Domain crawler framework (trait + implementations per source type) |
| `crates/bb-worker/src/discovery/crawlers/` | Individual crawler implementations (affiliate, registry, whois, dns_zone, search) |
| `crates/bb-worker/src/discovery/classifier.rs` | Content classifier trait + rule-based implementation |
| `crates/bb-worker/src/discovery/scorer.rs` | Confidence scoring engine |
| `crates/bb-worker/src/discovery/queue.rs` | Review queue job management |
| `crates/bb-api/src/routes/review_queue.rs` | Review queue API endpoints (list, approve, reject, defer) |
| `crates/bb-api/src/services/review_queue_service.rs` | Review queue business logic |
| `crates/bb-common/src/models/review_item.rs` | Review queue item model |
| `web/src/app/admin/review-queue/` | Admin review queue UI |

### Database Schema Additions

```sql
CREATE TABLE discovery_candidates (
    id BIGSERIAL PRIMARY KEY,
    domain TEXT NOT NULL,
    source TEXT NOT NULL,            -- 'crawler:affiliate', 'crawler:registry', 'federated', etc.
    source_metadata JSONB,
    confidence_score FLOAT NOT NULL DEFAULT 0.0,
    classification JSONB,            -- classifier outputs
    status TEXT NOT NULL DEFAULT 'pending', -- pending, approved, rejected, deferred
    reviewed_by BIGINT REFERENCES accounts(id),
    reviewed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_discovery_status ON discovery_candidates(status);
CREATE INDEX idx_discovery_domain ON discovery_candidates(domain);
CREATE UNIQUE INDEX idx_discovery_domain_source ON discovery_candidates(domain, source);
```

### Dependencies on Phase 1

- Blocklist management (admin CRUD, domain model) -- extend with discovery pipeline intake
- Background worker infrastructure (`bb-worker`) -- currently a stub; needs job scheduling framework
- Admin panel -- extend with review queue UI
- Blocklist delta sync protocol -- new domains from the pipeline must flow through the same compilation and distribution mechanism

### Estimated Complexity

**L (Large)**. The crawler framework is moderately complex but each individual crawler is straightforward. The rule-based classifier is simpler than ML but still requires careful tuning to avoid excessive false positives. The review queue is standard CRUD plus a workflow state machine. The main risk is operational: crawlers need rotating proxies, rate limiting, and resilience to site changes.

### Sub-plan Grouping

- **P2-DISC-1:** Crawler framework (trait, scheduling, rate limiting) + first crawler (gambling affiliate directories)
- **P2-DISC-2:** Content classifier (rule-based: keyword density, HTML structure, link graph)
- **P2-DISC-3:** Confidence scoring + review queue (API, database, admin UI)
- **P2-DISC-4:** Additional crawlers (license registries, WHOIS patterns, DNS zone monitoring, search)

---

## 4. Federated Reporting

### What It Achieves

Every enrolled agent encounters domains in the wild. When the agent's heuristic engine flags a domain that is not on the blocklist, that signal is valuable -- it might be a new gambling site. Federated reporting enables agents to contribute these signals to the central platform, where they are aggregated, anonymized, classified, and (when confidence is high enough) promoted to the blocklist.

This creates a network effect: every enrolled device makes the blocklist better for all devices.

### Key Technical Approach

**Agent-Side: Report Generation.** When the DNS resolver encounters a heuristic match (a domain that scores above a threshold on the heuristic classifier but is not on the blocklist), the agent generates a `DnsHeuristicMatch` event (defined in ADR-007). Per the enrollment's reporting config, this event is:
- Queued for federated reporting (default: enabled for all tiers)
- Stripped of device-identifying information before transmission

**Anonymization Pipeline.** Before the agent transmits a federated report, it applies k-anonymity protections:
- **Rotating device tokens:** The agent does not send its real `device_id`. Instead, it generates a rotating pseudonym token that changes every 24 hours. The API cannot link reports from the same device across token rotations.
- **Temporal bucketing:** Timestamps are rounded to 1-hour buckets. The API cannot determine the exact time of the heuristic match.
- **No IP logging:** The API does not log the source IP of federated reports. Reports are received on a dedicated endpoint that strips source IP before processing.
- **Batch submission:** Reports are batched (e.g., every 6 hours) and submitted in a single request, preventing traffic analysis from revealing real-time browsing patterns.

**Central Aggregation.** The `bb-worker` processes incoming federated reports:
1. Deduplicate by domain (multiple agents reporting the same domain)
2. Count unique reporting tokens per domain (proxy for independent reporters)
3. When a domain crosses the reporting threshold (configurable, default: 5 unique tokens), route it to the content classifier (same pipeline as automated discovery, section 3)
4. Classified domains enter the review queue

**Automatic Blocklist Promotion.** When a domain meets all of the following criteria, it can be auto-promoted to the blocklist without human review:
- Reported by >= N unique tokens (configurable, default: 10)
- Content classifier confidence >= 0.95
- Domain is less than 30 days old (WHOIS) -- established domains that are suddenly reported need human review
- Not on the allowlist

Auto-promotion is a configuration flag, disabled by default. Most deployments will use human review for all additions.

### New Crates/Files

| Location | Description |
|----------|-------------|
| `crates/bb-agent-core/src/federated.rs` | Agent-side federated report generation, anonymization, batching |
| `crates/bb-agent-core/src/federated/anonymizer.rs` | k-anonymity: rotating tokens, temporal bucketing |
| `crates/bb-api/src/routes/federated.rs` | Federated report ingestion endpoint (dedicated, IP-stripped) |
| `crates/bb-api/src/services/federated_service.rs` | Aggregation, deduplication, threshold checking |
| `crates/bb-worker/src/federated/` | Background processing: aggregation, classifier routing, promotion |
| `crates/bb-common/src/models/federated_report.rs` | Federated report model |

### Database Schema Additions

```sql
CREATE TABLE federated_reports (
    id BIGSERIAL PRIMARY KEY,
    domain TEXT NOT NULL,
    reporter_token TEXT NOT NULL,      -- rotating pseudonym, not device_id
    heuristic_score FLOAT NOT NULL,
    category_guess TEXT,
    reported_at TIMESTAMPTZ NOT NULL,  -- bucketed to 1-hour granularity
    batch_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_federated_domain ON federated_reports(domain);

CREATE TABLE federated_aggregates (
    id BIGSERIAL PRIMARY KEY,
    domain TEXT NOT NULL UNIQUE,
    unique_reporters INT NOT NULL DEFAULT 0,
    avg_heuristic_score FLOAT NOT NULL DEFAULT 0.0,
    first_reported_at TIMESTAMPTZ NOT NULL,
    last_reported_at TIMESTAMPTZ NOT NULL,
    status TEXT NOT NULL DEFAULT 'collecting', -- collecting, threshold_met, reviewing, promoted, rejected
    discovery_candidate_id BIGINT REFERENCES discovery_candidates(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### Dependencies on Phase 1

- Event reporting pipeline (ADR-007) -- `DnsHeuristicMatch` event type exists
- Heuristic matching in DNS resolver -- must be implemented in Phase 1 or early Phase 2
- Blocklist sync protocol -- promoted domains must flow through the standard sync
- Automated discovery pipeline (section 3) -- federated reports feed into the same classifier and review queue

### Estimated Complexity

**M (Medium)**. The agent-side anonymization is straightforward. The API ingestion endpoint is simple. The aggregation logic is a counting problem. The main design work is in the privacy guarantees (ensuring k-anonymity actually holds under realistic traffic patterns) and in the integration with the discovery pipeline.

### Sub-plan Grouping

- **P2-FED-1:** Agent-side report generation and anonymization (rotating tokens, batching)
- **P2-FED-2:** API ingestion endpoint (IP stripping, storage)
- **P2-FED-3:** Aggregation pipeline (deduplication, threshold checking, classifier routing)
- **P2-FED-4:** Auto-promotion logic and review queue integration

---

## 5. Advanced Reporting

### What It Achieves

Phase 1 delivers basic reporting: device status, blocked attempt counts, tamper alerts. Phase 2 upgrades this to time-series analytics that reveal trends, patterns, and long-term effectiveness of the blocking system. This data is valuable for self-enrolled users tracking their own progress, partners understanding patterns, and (in Phase 3) authorities generating compliance reports.

### Key Technical Approach

**TimescaleDB Continuous Aggregates.** The event data stored in TimescaleDB (Phase 1 stores raw events) is transformed into pre-computed time-series aggregates:
- Hourly, daily, weekly, monthly rollups of blocked attempt counts
- Per-category breakdowns (sports betting, casino, poker, lottery, etc.)
- Per-layer breakdowns (DNS blocked, app blocked, content blocked)
- Tamper event frequency and severity trends

Continuous aggregates are materialized views in TimescaleDB that auto-refresh as new data arrives. They provide sub-second query performance for dashboard views that would otherwise require scanning millions of raw events.

**Trend Analysis.** The reporting engine computes derived metrics:
- Blocked attempts over time (is the urge frequency increasing or decreasing?)
- Time-of-day patterns (are urges concentrated in specific hours?)
- Day-of-week patterns (weekends vs weekdays)
- Category shifts (is the user moving from sports betting to casino?)
- Streak tracking (days since last blocked attempt)

These are computed in the `bb-worker` as scheduled aggregation jobs and stored as pre-computed analytics.

**Exportable Reports.** Users, partners, and authorities can export reports in two formats:
- **PDF:** Formatted report with charts, generated server-side using a Rust PDF library (e.g., `printpdf` or `genpdf`) or via a headless browser rendering a report template.
- **CSV:** Raw data export for custom analysis.

Reports respect enrollment visibility rules (ADR-007): a partner sees only what the enrollment's `ReportingConfig` permits.

**Dashboard Improvements.** The web platform dashboards are enhanced with:
- Interactive time-series charts (line charts, area charts for blocking trends)
- Calendar heatmaps (GitHub-contribution-style visualization of daily blocking activity)
- Comparative views (this week vs last week, this month vs last month)
- Configurable date range selectors

### New Crates/Files

| Location | Description |
|----------|-------------|
| `crates/bb-worker/src/analytics/` | Scheduled analytics aggregation jobs |
| `crates/bb-worker/src/analytics/aggregator.rs` | Time-series rollup computation |
| `crates/bb-worker/src/analytics/trends.rs` | Trend analysis (time-of-day, category shifts, streaks) |
| `crates/bb-api/src/routes/analytics.rs` | Analytics query endpoints (time-series, trends, exports) |
| `crates/bb-api/src/services/analytics_service.rs` | Analytics query logic with enrollment visibility enforcement |
| `crates/bb-api/src/services/export_service.rs` | PDF and CSV report generation |
| `crates/bb-common/src/models/analytics.rs` | Analytics aggregate models |
| `web/src/app/dashboard/analytics/` | Enhanced dashboard UI with charts |
| `migrations/` | TimescaleDB continuous aggregate definitions |

### Database Schema Additions

```sql
-- TimescaleDB hypertable (raw events already stored from Phase 1)
-- Add continuous aggregates:

CREATE MATERIALIZED VIEW hourly_block_stats
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 hour', event_time) AS bucket,
    device_id,
    category,
    blocking_layer,
    COUNT(*) AS block_count
FROM blocking_events
GROUP BY bucket, device_id, category, blocking_layer;

SELECT add_continuous_aggregate_policy('hourly_block_stats',
    start_offset => INTERVAL '3 hours',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour');

-- Daily rollup
CREATE MATERIALIZED VIEW daily_block_stats
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 day', bucket) AS day,
    device_id,
    category,
    blocking_layer,
    SUM(block_count) AS block_count
FROM hourly_block_stats
GROUP BY day, device_id, category, blocking_layer;

-- Pre-computed trend analytics (populated by bb-worker)
CREATE TABLE analytics_trends (
    id BIGSERIAL PRIMARY KEY,
    device_id BIGINT NOT NULL REFERENCES devices(id),
    metric_name TEXT NOT NULL,        -- 'streak_days', 'peak_hour', 'category_shift', etc.
    metric_value JSONB NOT NULL,
    computed_at TIMESTAMPTZ NOT NULL,
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ NOT NULL
);
```

### Dependencies on Phase 1

- Event ingestion and storage in TimescaleDB -- raw events must be flowing
- Device and enrollment models -- analytics are scoped per device and filtered per enrollment visibility
- Web dashboard infrastructure -- Phase 1 delivers the basic dashboard; Phase 2 enhances it
- Authentication and authorization -- analytics endpoints enforce enrollment visibility rules

### Estimated Complexity

**M (Medium)**. TimescaleDB continuous aggregates are well-documented and straightforward to set up. The trend analysis is arithmetic over pre-aggregated data. PDF generation is a moderate effort. The dashboard UI work is the largest portion but uses standard charting libraries.

### Sub-plan Grouping

- **P2-REPORT-1:** TimescaleDB continuous aggregates (schema, migrations, aggregation policies)
- **P2-REPORT-2:** Analytics API endpoints with enrollment visibility enforcement
- **P2-REPORT-3:** Trend analysis engine (time-of-day, category, streaks) in bb-worker
- **P2-REPORT-4:** PDF/CSV export generation
- **P2-REPORT-5:** Dashboard UI improvements (charts, heatmaps, date selectors)

---

## 6. Organization Support

### What It Achieves

Phase 1 supports individual accounts and pairwise partner relationships. Many real-world use cases involve groups: a therapy practice managing multiple clients, a court program with enrolled defendants, a family with multiple devices. Organizations provide a grouping construct that simplifies management, enables shared configuration, and supports bulk operations.

### Key Technical Approach

**Organization Model.** An organization has:
- A name and type (`TherapyPractice`, `CourtProgram`, `Family`, `Other` -- already defined in `bb-common/src/enums.rs` as `OrganizationType`)
- An owner (the account that created it)
- Members with roles (`Owner`, `Admin`, `Member`)
- Devices assigned to the organization
- Default enrollment configuration (protection config, reporting config, unenrollment policy) that applies to new device enrollments

The `Organization` model already exists in `bb-common/src/models/organization.rs` as a placeholder. Phase 2 fills it in with full CRUD, membership management, and device assignment.

**Member Management.** Organization owners and admins can:
- Invite members via email (similar to partner invitation flow)
- Assign roles (Owner, Admin, Member)
- Remove members
- View all devices enrolled under the organization

Members can be either the device users (whose devices are enrolled) or the managers (partners/authorities who oversee enrollments). The role determines visibility and control.

**Device Assignment.** Devices can be assigned to an organization, which means:
- The device inherits the organization's default enrollment configuration
- Organization admins can view the device's status and reports (per enrollment visibility rules)
- Bulk operations (update config, export reports) apply to all org devices

**Organization-Level Default Enrollment Config.** When a new device is enrolled under an organization, it automatically inherits the org's default protection config, reporting config, and unenrollment policy. These can be overridden per-device.

**Bulk Device Enrollment.** Organizations can generate enrollment links or QR codes that pre-configure new devices with the org's settings. This is essential for institutional deployments where IT staff enroll dozens of devices.

### New Crates/Files

| Location | Description |
|----------|-------------|
| `crates/bb-api/src/routes/organizations.rs` | Organization CRUD + membership management endpoints |
| `crates/bb-api/src/services/organization_service.rs` | Organization business logic |
| `crates/bb-common/src/models/org_member.rs` | Organization membership model (account, role, joined_at) |
| `crates/bb-common/src/models/org_device.rs` | Organization-device assignment model |
| `web/src/app/organizations/` | Organization management UI |
| `web/src/app/organizations/members/` | Member invitation and management UI |
| `web/src/app/organizations/devices/` | Device listing and bulk operations UI |
| `web/src/app/organizations/settings/` | Default enrollment config UI |

### Database Schema Additions

```sql
-- organizations table already exists from Phase 1 schema; extend it:
ALTER TABLE organizations ADD COLUMN default_protection_config JSONB;
ALTER TABLE organizations ADD COLUMN default_reporting_config JSONB;
ALTER TABLE organizations ADD COLUMN default_unenrollment_policy JSONB;

CREATE TABLE organization_members (
    id BIGSERIAL PRIMARY KEY,
    organization_id BIGINT NOT NULL REFERENCES organizations(id),
    account_id BIGINT NOT NULL REFERENCES accounts(id),
    role TEXT NOT NULL DEFAULT 'member', -- owner, admin, member
    invited_by BIGINT REFERENCES accounts(id),
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(organization_id, account_id)
);

CREATE TABLE organization_devices (
    id BIGSERIAL PRIMARY KEY,
    organization_id BIGINT NOT NULL REFERENCES organizations(id),
    device_id BIGINT NOT NULL REFERENCES devices(id),
    assigned_by BIGINT REFERENCES accounts(id),
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(organization_id, device_id)
);

-- Bulk enrollment tokens
CREATE TABLE enrollment_tokens (
    id BIGSERIAL PRIMARY KEY,
    public_id UUID NOT NULL UNIQUE DEFAULT gen_random_uuid(),
    organization_id BIGINT NOT NULL REFERENCES organizations(id),
    created_by BIGINT NOT NULL REFERENCES accounts(id),
    protection_config JSONB NOT NULL,
    reporting_config JSONB NOT NULL,
    unenrollment_policy JSONB NOT NULL,
    max_uses INT,                     -- NULL = unlimited
    uses_count INT NOT NULL DEFAULT 0,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### Dependencies on Phase 1

- Account and enrollment models -- exist
- `Organization` model -- exists as a placeholder
- Partner invitation flow -- organization member invitation reuses the same pattern
- Device registration -- extend to support organization assignment at enrollment time
- Authentication and authorization -- organization roles must be checked on all org endpoints

### Estimated Complexity

**M (Medium)**. This is largely standard CRUD with role-based access control. The bulk enrollment token flow is the most novel piece. No kernel-level or platform-specific work.

### Sub-plan Grouping

- **P2-ORG-1:** Organization CRUD (API, database, basic UI)
- **P2-ORG-2:** Member management (invitation flow, roles, member list UI)
- **P2-ORG-3:** Device assignment and org-level default configs
- **P2-ORG-4:** Bulk enrollment tokens (generation, QR codes, redemption flow)

---

## 7. VPN/Proxy/Tor Detection

### What It Achieves

A user who cannot bypass DNS blocking directly may install a VPN, configure a SOCKS proxy, or use Tor to tunnel traffic through an unfiltered network path. VPN/proxy/Tor detection identifies these bypass attempts and responds according to the enrollment tier's policy.

### Key Technical Approach

**VPN Tunnel Detection.** The agent monitors for new network interfaces that indicate a VPN tunnel:
- Windows: `GetAdaptersInfo()` / `NotifyIpInterfaceChange()` -- watch for TAP/TUN adapters, WireGuard interfaces
- macOS: `SCDynamicStore` notifications for interface changes, `utun` interface creation
- Linux: `netlink` socket monitoring for new interfaces, `/sys/class/net/` polling
- Android: `ConnectivityManager` network callbacks, VpnService state changes
- iOS: `NWPathMonitor` for path changes, `NEVPNManager` status

In addition to interface monitoring, the agent watches for known VPN client processes:
- Process names: `openvpn`, `wireguard`, `nordvpn`, `expressvpn`, etc.
- Service names on Windows/macOS/Linux
- Package names on Android

**Proxy Configuration Monitoring.** The agent monitors system proxy settings:
- Windows: registry key `HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings\ProxyServer`
- macOS: System Preferences proxy settings via `CFNetworkCopySystemProxySettings()`
- Linux: environment variables (`http_proxy`, `https_proxy`, `all_proxy`), GNOME/KDE proxy settings
- Android: `Settings.Global.HTTP_PROXY`
- Browser-specific proxy settings (Firefox has its own proxy config independent of OS)

**Tor Exit Node Detection.** The agent maintains a list of known Tor exit node IPs (sourced from the Tor Project's public exit node list, refreshed daily). The agent:
- Monitors for Tor browser/process installation and execution
- Checks outbound connections against the exit node IP list (requires network hook integration)
- Detects `tor` process by name

**Configurable Response.** The response to VPN/proxy/Tor detection is configurable per enrollment tier via the `VpnDetectionMode` enum (already defined in `bb-common/src/enums.rs`):

| Mode | Behavior | Typical Tier |
|------|----------|-------------|
| `Disabled` | No monitoring | -- |
| `Log` | Record event, no user-visible action | Self |
| `Alert` | Record event + notify partner/authority | Partner |
| `Block` | Attempt to disable/block the VPN/proxy + alert | Authority |
| `Lockdown` | Restrict network to essential services until VPN is removed | Authority |

The `Block` and `Lockdown` modes require kernel-level network control (WFP, NetworkExtension, nftables) and are only available when the corresponding tamper resistance features (section 2) are active.

### New Crates/Files

| Location | Description |
|----------|-------------|
| `crates/bb-agent-core/src/bypass_detection/` | Bypass detection module |
| `crates/bb-agent-core/src/bypass_detection/vpn.rs` | VPN tunnel detection (interface monitoring, process detection) |
| `crates/bb-agent-core/src/bypass_detection/proxy.rs` | Proxy configuration monitoring |
| `crates/bb-agent-core/src/bypass_detection/tor.rs` | Tor exit node detection, Tor process detection |
| `crates/bb-agent-core/src/bypass_detection/response.rs` | Response logic per VpnDetectionMode |
| `crates/bb-common/src/models/tor_exit_nodes.rs` | Tor exit node list model (synced from API) |
| `crates/bb-api/src/routes/tor_exits.rs` | Tor exit node list distribution endpoint |
| `crates/bb-worker/src/tor_exits.rs` | Background job to refresh Tor exit node list from Tor Project |

### Dependencies on Phase 1

- Event reporting pipeline -- `VpnDetected` event type (defined in ADR-007)
- `VpnDetectionMode` and `TamperResponse` enums -- exist in `bb-common/src/enums.rs`
- `ProtectionConfig` in enrollment model -- already has `vpn_detection: VpnDetectionMode` field
- Heartbeat protocol -- `ProtectionStatus` already has `vpn_connected: bool` field
- Kernel-level network controls (section 2) -- required for `Block` and `Lockdown` modes

### Estimated Complexity

**M (Medium)**. Interface monitoring and process detection are well-understood techniques. The main complexity is in the breadth of platforms and the number of VPN clients/protocols to detect. The `Log` and `Alert` modes are straightforward; `Block` and `Lockdown` require kernel integration and are gated behind section 2.

### Sub-plan Grouping

- **P2-VPN-1:** Network interface monitoring (VPN tunnel detection) -- all platforms
- **P2-VPN-2:** Proxy configuration monitoring + Tor detection
- **P2-VPN-3:** Response logic (`Log`, `Alert` modes) + event reporting integration
- **P2-VPN-4:** `Block` and `Lockdown` modes (depends on P2-TAMPER kernel controls)

---

## 8. Windows and macOS Platform Shims

### What It Achieves

Phase 1 delivers the agent on Linux (the primary development platform) with stub shim crates for Windows and macOS. Phase 2 completes these shims, making the agent fully functional on Windows and macOS at parity with Linux.

### Key Technical Approach

#### Windows Platform Shim (`bb-shim-windows`)

**Windows Service Integration.** The agent runs as a Windows Service, registered with the Service Control Manager (SCM):
- `SERVICE_WIN32_OWN_PROCESS` type
- `SERVICE_AUTO_START` start type
- `SERVICE_FAILURE_ACTIONS`: restart on failure (0s, 5s, 30s)
- `LocalSystem` service account
- Service control handler for stop, shutdown, and custom control codes (used for config reload)

Implementation via the `windows-service` crate or raw Win32 API calls (`RegisterServiceCtrlHandlerEx`, `SetServiceStatus`).

**WFP DNS Interception.** Phase 1 uses a local DNS resolver that the system is configured to use. Phase 2 adds WFP callout driver integration (from section 2) that forces all DNS traffic through the agent regardless of application DNS settings. The user-space component communicates with the WFP driver via IOCTL.

**File and Registry ACLs.** The installer sets restrictive ACLs:
- Agent directory: SYSTEM=FullControl, Administrators=ReadExecute, Users=ReadExecute, Deny Delete for non-SYSTEM
- Registry keys: protected with restrictive DACLs

**DNS Configuration Monitoring.** Windows-specific DNS change detection via `NotifyIpInterfaceChange()` and periodic polling of `GetAdaptersInfo()`.

#### macOS Platform Shim (`bb-shim-macos`)

**launchd Integration.** The agent runs as a launchd daemon (`/Library/LaunchDaemons/com.betblocker.agent.plist`):
- `KeepAlive = true`
- `RunAtLoad = true`
- `AbandonProcessGroup = true`
- `Program` points to the agent binary in `/Library/Application Support/BetBlocker/`

**Network Extension.** macOS requires a Network Extension (NEDNSProxyProvider) for DNS interception. This requires:
- A separate Network Extension app extension binary
- Entitlements: `com.apple.developer.networking.networkextension`
- Code signing with Developer ID
- Notarization

The Network Extension provides the DNS interception hook. It communicates with the main agent process via XPC for blocklist queries.

**Endpoint Security Framework.** File and process monitoring (from section 2). Requires the `com.apple.developer.endpoint-security.client` entitlement.

**Keychain Integration.** Configuration encryption keys stored in the macOS Keychain with `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly` and non-exportable flag.

### New Crates/Files

| Location | Description |
|----------|-------------|
| `crates/bb-shim-windows/src/service.rs` | Windows Service lifecycle (SCM registration, control handler) |
| `crates/bb-shim-windows/src/acl.rs` | File and registry ACL management |
| `crates/bb-shim-windows/src/dns_monitor.rs` | DNS configuration monitoring via Win32 APIs |
| `crates/bb-shim-windows/src/keystore.rs` | TPM/DPAPI key storage |
| `crates/bb-shim-windows/src/installer.rs` | MSI/MSIX installer helpers |
| `crates/bb-agent-windows/` | Windows agent binary crate (equivalent to `bb-agent-linux`) |
| `crates/bb-shim-macos/src/launchd.rs` | launchd daemon lifecycle |
| `crates/bb-shim-macos/src/network_ext.rs` | Network Extension management and XPC communication |
| `crates/bb-shim-macos/src/keychain.rs` | Keychain key storage |
| `crates/bb-shim-macos/src/dns_monitor.rs` | DNS configuration monitoring via SCDynamicStore |
| `crates/bb-shim-macos/bridge/swift/` | Swift bridge for APIs not accessible via C FFI |
| `crates/bb-agent-macos/` | macOS agent binary crate |

### Dependencies on Phase 1

- Agent core (`bb-agent-core`) -- platform-independent engine that the shims wrap
- Plugin system -- shims integrate with the plugin registry
- Blocklist sync, heartbeat, event reporting -- all platform-independent; shims just provide the service lifecycle
- `bb-agent-linux` -- serves as the reference implementation for how a platform agent binary integrates with the core

### Estimated Complexity

**L (Large)** for Windows, **L (Large)** for macOS. Each platform has significant native API surface. Windows requires Win32 service management, ACL APIs, and (for Phase 2 tamper resistance) driver development. macOS requires Network Extension development (partially in Swift), Keychain integration, and (for tamper resistance) System Extension + Endpoint Security. Both require code signing infrastructure and platform-specific CI.

### Sub-plan Grouping

- **P2-WIN-1:** Windows Service lifecycle (SCM registration, control handler, restart)
- **P2-WIN-2:** Windows DNS configuration monitoring and enforcement
- **P2-WIN-3:** Windows file/registry ACLs and TPM key storage
- **P2-WIN-4:** Windows installer (MSI or MSIX) and auto-update mechanism
- **P2-MAC-1:** macOS launchd daemon lifecycle
- **P2-MAC-2:** macOS Network Extension (NEDNSProxyProvider) with XPC bridge
- **P2-MAC-3:** macOS Keychain integration and file permissions
- **P2-MAC-4:** macOS installer (pkg) and notarization pipeline

---

## 9. Dependency Graph

The following graph shows which Phase 2 features depend on each other and which can be built in parallel. An arrow from A to B means "A must be completed (or substantially completed) before B can begin."

```
Phase 1 (Foundation)
    |
    +----+----+----+----+----+----+----+
    |    |    |    |    |    |    |    |
    v    v    v    v    |    v    v    |
  ORG  DISC  FED  RPT  |   VPN  APP  |
  [6]  [3]  [4]  [5]  |   [7]  [1]  |
    |    |    |         |    |        |
    |    +--->+         |    |        |
    |    |   (FED uses  |    |        |
    |    |    DISC       |    |        |
    |    |    classifier |    |        |
    |    |    + review   |    |        |
    |    |    queue)     |    |        |
    |    |              |    |        |
    |    |              v    |        v
    |    |        WIN+MAC SHIMS [8]
    |    |              |
    |    |              v
    |    |         TAMPER [2]
    |    |              |
    |    |              v
    |    |        VPN Block/Lockdown
    |    |           (P2-VPN-4)
    |    |
    v    v
  (independent)
```

### Parallel Workstreams

The following features have no dependencies on each other and can be developed in parallel:

**Workstream A -- Agent Blocking Depth:**
- Application Blocking (section 1) -- can start immediately after Phase 1
- VPN/Proxy/Tor Detection (section 7, `Log` and `Alert` modes) -- can start immediately

**Workstream B -- Platform Expansion:**
- Windows Platform Shim (section 8, Windows parts) -- can start immediately
- macOS Platform Shim (section 8, macOS parts) -- can start immediately

**Workstream C -- Intelligence Pipeline:**
- Automated Discovery Pipeline (section 3) -- can start immediately
- Federated Reporting (section 4) -- depends on discovery pipeline's classifier and review queue

**Workstream D -- Server-Side Features:**
- Organization Support (section 6) -- can start immediately
- Advanced Reporting (section 5) -- can start immediately

**Workstream E -- Kernel Hardening (depends on Workstream B):**
- Enhanced Tamper Resistance (section 2) -- each platform's kernel protections depend on the corresponding platform shim being completed
- VPN Block/Lockdown modes (P2-VPN-4) -- depends on tamper resistance kernel controls

### Suggested Implementation Order

Given team size constraints and risk management, the recommended order is:

1. **First wave (parallel):** Organization Support (P2-ORG), Advanced Reporting (P2-REPORT), Discovery Pipeline stages 1-2 (P2-DISC-1, P2-DISC-2), Windows Service lifecycle (P2-WIN-1)
2. **Second wave (parallel):** App Signature data model + API (P2-APP-1), macOS launchd lifecycle (P2-MAC-1), VPN detection Log/Alert modes (P2-VPN-1 through P2-VPN-3), Federated Reporting (P2-FED)
3. **Third wave (parallel):** App inventory scanner + AppProcessPlugin (P2-APP-2, P2-APP-3), Windows Network Extension equivalent and DNS monitoring (P2-WIN-2, P2-WIN-3), macOS Network Extension (P2-MAC-2, P2-MAC-3)
4. **Fourth wave (parallel, high-risk):** Windows WFP driver (P2-TAMPER-1), macOS System Extension (P2-TAMPER-3), Linux AppArmor/SELinux (P2-TAMPER-4), Android Device Owner (P2-TAMPER-5)
5. **Fifth wave (depends on fourth):** Windows minifilter (P2-TAMPER-2), iOS MDM (P2-TAMPER-6), VPN Block/Lockdown modes (P2-VPN-4), eBPF stretch goal (P2-TAMPER-7)

---

## 10. Sub-plan Groupings

### Complete Sub-plan Index

| ID | Name | Section | Complexity | Dependencies |
|----|------|---------|------------|-------------|
| P2-APP-1 | App signature model + API + seed data | 1 | S | Phase 1 blocklist management |
| P2-APP-2 | App inventory scanner | 1 | M | P2-APP-1 |
| P2-APP-3 | AppProcessPlugin + launch interception | 1 | L | P2-APP-1, P2-APP-2 |
| P2-APP-4 | AppDeviceAdminPlugin (Android) | 1 | L | P2-APP-1, P2-TAMPER-5 |
| P2-APP-5 | Install prevention monitors | 1 | M | P2-APP-1, P2-APP-3 |
| P2-TAMPER-1 | Windows WFP callout driver | 2 | XL | P2-WIN-1, P2-WIN-2 |
| P2-TAMPER-2 | Windows kernel minifilter | 2 | L | P2-WIN-1 |
| P2-TAMPER-3 | macOS System Extension + Endpoint Security | 2 | XL | P2-MAC-1, P2-MAC-2 |
| P2-TAMPER-4 | Linux AppArmor + SELinux policies | 2 | M | Phase 1 Linux agent |
| P2-TAMPER-5 | Android Device Owner + Knox | 2 | L | Phase 1 Android agent |
| P2-TAMPER-6 | iOS MDM profile integration | 2 | L | Phase 1 iOS agent |
| P2-TAMPER-7 | Linux eBPF DNS interception (stretch) | 2 | L | P2-TAMPER-4 |
| P2-DISC-1 | Crawler framework + first crawler | 3 | M | Phase 1 bb-worker |
| P2-DISC-2 | Content classifier (rule-based) | 3 | M | P2-DISC-1 |
| P2-DISC-3 | Confidence scoring + review queue | 3 | M | P2-DISC-2, Phase 1 admin panel |
| P2-DISC-4 | Additional crawlers | 3 | M | P2-DISC-1 |
| P2-FED-1 | Agent-side report generation + anonymization | 4 | M | Phase 1 event reporting |
| P2-FED-2 | API ingestion endpoint | 4 | S | Phase 1 API |
| P2-FED-3 | Aggregation pipeline | 4 | M | P2-FED-2, P2-DISC-2 |
| P2-FED-4 | Auto-promotion + review queue integration | 4 | S | P2-FED-3, P2-DISC-3 |
| P2-REPORT-1 | TimescaleDB continuous aggregates | 5 | S | Phase 1 event storage |
| P2-REPORT-2 | Analytics API endpoints | 5 | M | P2-REPORT-1 |
| P2-REPORT-3 | Trend analysis engine | 5 | M | P2-REPORT-1 |
| P2-REPORT-4 | PDF/CSV export | 5 | M | P2-REPORT-2 |
| P2-REPORT-5 | Dashboard UI improvements | 5 | M | P2-REPORT-2 |
| P2-ORG-1 | Organization CRUD | 6 | S | Phase 1 account model |
| P2-ORG-2 | Member management | 6 | M | P2-ORG-1 |
| P2-ORG-3 | Device assignment + default configs | 6 | M | P2-ORG-1, P2-ORG-2 |
| P2-ORG-4 | Bulk enrollment tokens | 6 | M | P2-ORG-3 |
| P2-VPN-1 | Network interface monitoring (VPN detection) | 7 | M | Phase 1 agent |
| P2-VPN-2 | Proxy + Tor detection | 7 | M | P2-VPN-1 |
| P2-VPN-3 | Response logic (Log, Alert modes) | 7 | S | P2-VPN-1, P2-VPN-2 |
| P2-VPN-4 | Block + Lockdown modes | 7 | L | P2-VPN-3, P2-TAMPER-1/3/4/5 |
| P2-WIN-1 | Windows Service lifecycle | 8 | M | Phase 1 agent core |
| P2-WIN-2 | Windows DNS monitoring + enforcement | 8 | M | P2-WIN-1 |
| P2-WIN-3 | Windows ACLs + TPM key storage | 8 | M | P2-WIN-1 |
| P2-WIN-4 | Windows installer + auto-update | 8 | M | P2-WIN-1 |
| P2-MAC-1 | macOS launchd lifecycle | 8 | M | Phase 1 agent core |
| P2-MAC-2 | macOS Network Extension | 8 | L | P2-MAC-1 |
| P2-MAC-3 | macOS Keychain + file permissions | 8 | M | P2-MAC-1 |
| P2-MAC-4 | macOS installer + notarization | 8 | M | P2-MAC-1 |

### Summary Statistics

- **Total sub-plans:** 36
- **By complexity:** S=6, M=20, L=7, XL=3
- **Parallelizable at start (no inter-Phase-2 deps):** P2-ORG-1, P2-DISC-1, P2-REPORT-1, P2-WIN-1, P2-MAC-1, P2-VPN-1, P2-APP-1, P2-FED-1 (8 sub-plans)
- **Critical path:** Platform shims -> tamper resistance kernel drivers -> VPN Block/Lockdown mode

---

## Appendix: Risk Register

| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|-----------|
| WHQL driver signing takes longer than expected | Delays Windows kernel protections | High | Start the WHQL submission process early. Ship user-space Windows agent first; kernel protections are an upgrade, not a gate. |
| Apple rejects Network Extension entitlement | Blocks macOS DNS interception | Medium | Apply early with clear privacy justification. Have DNS configuration fallback (configure system DNS to point to agent's resolver). |
| App signature database has poor coverage | Layer 2 is ineffective | Medium | Seed with data from app store searches. Combine with federated reporting to crowdsource app discovery. |
| Rule-based classifiers produce too many false positives | Review queue overwhelmed | Medium | Conservative thresholds initially. Invest in classifier tuning before enabling auto-promotion. |
| Device Owner provisioning is too disruptive for users | Low adoption of Android hardened mode | Medium | Make Device Owner optional, not required. Document the provisioning process clearly. Offer Device Admin as a less invasive alternative. |
| eBPF program deployment fails on older kernels | Linux kernel-level DNS interception unavailable | Medium | Mark as stretch goal. Fall back to nftables rules (Phase 1 approach). |
| MDM infrastructure operational costs | iOS hardened mode expensive to operate | Low | Partner with existing MDM providers rather than building from scratch. Evaluate open-source MDM (MicroMDM). |
