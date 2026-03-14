# BetBlocker Phase 2 — Master Implementation Plan ("Depth")

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deepen BetBlocker across all dimensions: application blocking (Layer 2), kernel-level tamper resistance, automated discovery pipeline, federated reporting, advanced analytics, organization support, VPN/proxy/Tor detection, and Windows/macOS platform completion.

**Architecture:** Extends the Phase 1 Rust monorepo. New plugins added via feature-flagged `PluginInstance` variants. Platform shims filled in with native APIs. Server-side features extend bb-api and bb-worker. Discovery and federated pipelines converge on a shared review queue.

**Tech Stack:** Rust, TypeScript/Next.js, PostgreSQL, Redis, TimescaleDB, Docker, Protobuf, Win32 APIs, macOS System Extensions, AppArmor/SELinux, Android Device Owner

**Reference Docs:**
- Phase 2 Design: `docs/plans/2026-03-13-phase2-design.md`
- Phase 1 Master Plan: `docs/superpowers/plans/2026-03-12-phase1-master-plan.md`
- ADRs: `docs/architecture/adrs/ADR-001` through `ADR-007`
- API Spec: `docs/architecture/api-spec.md`
- DB Schema: `docs/architecture/database-schema.md`

---

## Sub-Plan Dependency Graph

```
Phase 1 (Complete)
    |
    +------+------+------+------+------+------+------+
    |      |      |      |      |      |      |      |
    v      v      v      v      v      v      v      v
  SP1    SP2    SP3    SP4    SP5    SP6    SP7   (all start from Phase 1)
  ORG+   DISC+  APP    VPN    WIN    MAC   LINUX+
  RPT    FED    BLOCK  DET    PLAT   PLAT  MOBILE
                                             TAMPER
    |      |      |      |      |      |      |
    |      +--+   |      |      |      |      |
    |      |  |   |      |      |      |      |
    |    DISC->FED |      |      |      |      |
    |    (FED uses |    VPN-4    |      |      |
    |     DISC     |   depends  |      |      |
    |     review   |   on SP5/  |      |      |
    |     queue)   |   SP6/SP7  |      |      |
    |              |   tamper   |      |      |
    |              |            v      v      |
    |              |        WIN+MAC TAMPER     |
    |              |        (in SP5, SP6)      |
    |              |                           |
    v              v                           v
 (independent)  (independent)            (independent)
```

## Sub-Plans

### Sub-Plan 1: Server-Side Features (Organizations + Advanced Reporting)
**File:** `2026-03-13-phase2-sp1-server-features.md`
**Blocks:** Nothing (independent)
**Estimated tasks:** ~20
**Deliverables:**
- Organization CRUD (API, DB, UI)
- Member invitation and management flow
- Device assignment + org-level default enrollment configs
- Bulk enrollment tokens with QR codes
- TimescaleDB continuous aggregates (hourly, daily rollups)
- Analytics API endpoints with enrollment visibility
- Trend analysis engine (time-of-day, category shifts, streaks)
- PDF/CSV report export
- Enhanced dashboard UI with charts and heatmaps

### Sub-Plan 2: Intelligence Pipeline (Discovery + Federated Reporting)
**File:** `2026-03-13-phase2-sp2-intelligence.md`
**Blocks:** Nothing directly (FED depends on DISC classifier internally)
**Estimated tasks:** ~18
**Deliverables:**
- Crawler framework with scheduling and rate limiting
- First crawler (gambling affiliate directories)
- Rule-based content classifier (keyword density, HTML structure, link graph)
- Confidence scoring engine
- Review queue (API, DB, admin UI)
- Additional crawlers (license registries, WHOIS, DNS zones, search)
- Agent-side federated report generation with k-anonymity
- Federated report ingestion endpoint (IP-stripped)
- Aggregation pipeline (dedup, threshold, classifier routing)
- Auto-promotion logic

### Sub-Plan 3: Application Blocking (Layer 2)
**File:** `2026-03-13-phase2-sp3-app-blocking.md`
**Blocks:** Nothing (independent)
**Estimated tasks:** ~15
**Deliverables:**
- `AppSignature` model, database schema, API CRUD, admin UI, seed data
- App inventory scanner (cross-platform abstraction + per-platform impl)
- `AppProcessPlugin` with launch interception (desktop platforms)
- Install prevention monitors (filesystem watchers per platform)
- `PluginInstance::AppProcess` variant with feature flag

### Sub-Plan 4: VPN/Proxy/Tor Detection
**File:** `2026-03-13-phase2-sp4-vpn-detection.md`
**Blocks:** P2-VPN-4 blocked by SP5/SP6/SP7 tamper resistance
**Estimated tasks:** ~12
**Deliverables:**
- Network interface monitoring for VPN tunnels (all platforms)
- System proxy configuration monitoring
- Tor process and exit node detection
- Response logic (Log, Alert modes)
- Event reporting integration
- Block/Lockdown modes (gated behind kernel protections)

### Sub-Plan 5: Windows Platform
**File:** `2026-03-13-phase2-sp5-windows.md`
**Depends on:** Phase 1 agent core
**Estimated tasks:** ~18
**Deliverables:**
- Windows Service lifecycle (SCM registration, control handler, failure recovery)
- DNS configuration monitoring via Win32 APIs
- File/registry ACLs and TPM/DPAPI key storage
- MSI/MSIX installer and auto-update mechanism
- `bb-agent-windows` binary crate
- WFP callout driver (C, IOCTL interface in Rust)
- Kernel minifilter for file protection

### Sub-Plan 6: macOS Platform
**File:** `2026-03-13-phase2-sp6-macos.md`
**Depends on:** Phase 1 agent core
**Estimated tasks:** ~16
**Deliverables:**
- launchd daemon lifecycle
- Network Extension (NEDNSProxyProvider) with XPC bridge to agent
- Keychain integration and file permissions
- pkg installer and notarization pipeline
- `bb-agent-macos` binary crate
- System Extension + Endpoint Security Framework
- Swift bridge for macOS-only APIs

### Sub-Plan 7: Linux + Mobile Tamper Resistance
**File:** `2026-03-13-phase2-sp7-tamper-mobile.md`
**Depends on:** Phase 1 Linux agent, platform shims
**Estimated tasks:** ~14
**Deliverables:**
- Linux AppArmor profile (Ubuntu/Debian)
- Linux SELinux policy module (RHEL/Fedora)
- Linux eBPF DNS interception (stretch goal)
- Android Device Owner provisioning and policy management
- Samsung Knox integration
- iOS MDM profile integration

## Execution Strategy

**Wave 1 (all parallel — no inter-dependencies):**
- Sub-Plan 1: Server-Side Features (ORG + RPT)
- Sub-Plan 2: Intelligence Pipeline (DISC + FED)
- Sub-Plan 3: Application Blocking (APP)
- Sub-Plan 4: VPN Detection (Log/Alert modes only)
- Sub-Plan 5: Windows Platform (service lifecycle, DNS, ACLs, installer)
- Sub-Plan 6: macOS Platform (launchd, Network Extension, Keychain, installer)
- Sub-Plan 7: Linux + Mobile Tamper (AppArmor, SELinux, Device Owner)

**Wave 2 (depends on Wave 1 platform shims):**
- SP5 kernel drivers: WFP callout + minifilter (requires Windows service lifecycle)
- SP6 kernel: System Extension + Endpoint Security (requires macOS launchd + Network Extension)
- SP4 P2-VPN-4: Block/Lockdown modes (requires kernel protections from SP5/SP6/SP7)

## Definition of Done — Phase 2

- [ ] Application blocking: gambling apps detected, terminated, and install-prevented on Windows/macOS/Linux
- [ ] Organizations: CRUD, member management, device assignment, bulk enrollment tokens
- [ ] Advanced reporting: time-series charts, trend analysis, PDF/CSV export
- [ ] Discovery pipeline: crawlers running, classifier scoring, review queue functional
- [ ] Federated reporting: agents submit anonymized reports, aggregation promotes to review queue
- [ ] VPN/proxy/Tor detection: Log and Alert modes functional on all platforms
- [ ] Windows agent: runs as service, DNS blocking, file protection, installer
- [ ] macOS agent: runs as launchd daemon, Network Extension DNS, Keychain, installer
- [ ] Linux tamper: AppArmor/SELinux profiles protecting agent files and processes
- [ ] Android: Device Owner provisioning functional
- [ ] All code has tests; CI is green
