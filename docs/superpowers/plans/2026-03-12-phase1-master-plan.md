# BetBlocker Phase 1 — Master Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the BetBlocker Phase 1 MVP: DNS-level gambling blocking on all platforms, central API, web dashboard, partner flows, billing, and self-hosted deployment.

**Architecture:** Rust monorepo (Cargo workspace) with shared `bb-common` types. Axum API, Next.js web, PostgreSQL + Redis + TimescaleDB. Endpoint agents compiled per-platform from shared `bb-agent-core` crate with platform-specific shims. Docker-based deployment for both hosted and self-hosted.

**Tech Stack:** Rust (agent + API + worker), TypeScript/Next.js (web), PostgreSQL, Redis, TimescaleDB, Docker, Protobuf

**Reference Docs:**
- Vision: `docs/plans/2026-03-12-betblocker-vision-design.md`
- ADRs: `docs/architecture/adrs/ADR-001` through `ADR-007`
- API Spec: `docs/architecture/api-spec.md`
- DB Schema: `docs/architecture/database-schema.md`
- Agent Protocol: `docs/architecture/agent-protocol.md`
- Threat Model: `docs/architecture/threat-model.md`
- Repo Structure: `docs/architecture/repo-structure.md`

---

## Sub-Plan Dependency Graph

```
Sub-Plan 1: Foundation (repo scaffold, types, DB, dev infra)
    │
    ├──> Sub-Plan 2: API Server (auth, devices, enrollments, partners, blocklist, events, billing)
    │        │
    │        └──> Sub-Plan 5: Web Platform (Next.js — needs API to integrate against)
    │
    ├──> Sub-Plan 3: Agent Core (plugin system, blocklist matcher, DNS resolver, HOSTS plugin)
    │        │
    │        └──> Sub-Plan 4: Agent Communication + Platform (mTLS, heartbeat, sync, Linux shim)
    │
    └──> Sub-Plan 6: Deployment (Dockerfiles, docker-compose, CI — needs all artifacts)
```

## Sub-Plans

### Sub-Plan 1: Foundation
**File:** `2026-03-12-phase1-sp1-foundation.md`
**Blocks:** Everything
**Estimated tasks:** ~25
**Deliverables:**
- Cargo workspace with all crate stubs (`bb-common`, `bb-proto`, `bb-api`, `bb-worker`, `bb-agent-core`, `bb-agent-plugins`, `bb-agent-linux`, `bb-cli`)
- `bb-common`: domain types (Account, Device, Enrollment, Event, BlocklistEntry, Organization, Partner) with serde support
- `bb-proto`: protobuf definitions for agent-API protocol
- PostgreSQL migrations (V001-V021 from schema doc)
- `docker-compose.yml` for dev infra (PostgreSQL, Redis, TimescaleDB)
- `justfile` with dev commands
- `rust-toolchain.toml`, `.cargo/config.toml`
- CI skeleton (GitHub Actions)

### Sub-Plan 2: API Server
**File:** `2026-03-12-phase1-sp2-api.md`
**Depends on:** Sub-Plan 1
**Estimated tasks:** ~40
**Deliverables:**
- Axum server scaffold with middleware (auth, error handling, request ID, logging, rate limiting)
- Auth module: register, login, JWT (Ed25519), refresh token rotation, logout, password reset
- Device endpoints: registration (enrollment token), heartbeat, config fetch
- Enrollment endpoints: CRUD, unenrollment request/approval with policy enforcement
- Partner endpoints: invite, accept, list, remove
- Blocklist endpoints: version check, delta sync, admin CRUD, federated report ingestion
- Event endpoints: batch ingestion, query with enrollment-scoped visibility, summary
- Billing endpoints: Stripe subscribe, status, webhook, cancel (behind feature flag)
- Integration tests against real PostgreSQL

### Sub-Plan 3: Agent Core
**File:** `2026-03-12-phase1-sp3-agent-core.md`
**Depends on:** Sub-Plan 1
**Estimated tasks:** ~30
**Deliverables:**
- `bb-agent-core`: plugin registry, blocklist loader/matcher (mmap cache), event system, config manager
- `bb-agent-plugins`: `BlockingPlugin` + `DnsBlockingPlugin` traits, `PluginInstance` enum
- DNS Resolver Plugin: local DNS resolver using `hickory-dns`, blocklist interception, upstream forwarding
- HOSTS File Plugin: writes blocked domains to HOSTS file, monitors for tampering
- Plugin lifecycle: init → activate → deactivate, health checks
- Unit tests for blocklist matching, plugin registry, DNS resolution

### Sub-Plan 4: Agent Communication + Linux Platform
**File:** `2026-03-12-phase1-sp4-agent-comms.md`
**Depends on:** Sub-Plan 1, Sub-Plan 3
**Estimated tasks:** ~30
**Deliverables:**
- `bb-agent-core` API client: mTLS with certificate pinning, protobuf serialization
- Device registration flow (enrollment token → CSR → certificate)
- Heartbeat sender with tier-differentiated intervals
- Blocklist sync (delta + full, signature verification)
- Event reporter (batched, privacy-filtered per enrollment tier)
- Watchdog process (mutual supervision)
- Binary integrity checker
- `bb-agent-linux`: systemd service, nftables DNS redirect
- Agent binary entrypoint with graceful shutdown
- Integration tests (agent ↔ API)

### Sub-Plan 5: Web Platform
**File:** `2026-03-12-phase1-sp5-web.md`
**Depends on:** Sub-Plan 2
**Estimated tasks:** ~25
**Deliverables:**
- Next.js project scaffold (App Router, TypeScript, Tailwind CSS)
- Marketing/landing pages (static)
- Auth pages: register, login, forgot password, reset password
- User dashboard: device list, device status, enrollment management
- Partner invitation flow: invite partner, accept invitation
- Partner dashboard: supervised devices, approval queue
- Blocklist admin panel: search, add, edit, review queue
- Basic reporting: block count charts, tamper alerts
- API client library (TypeScript, generated from API spec types)

### Sub-Plan 6: Deployment
**File:** `2026-03-12-phase1-sp6-deployment.md`
**Depends on:** Sub-Plans 2, 3, 4, 5
**Estimated tasks:** ~15
**Deliverables:**
- `Dockerfile.api`: multi-stage build, musl static binary, scratch image
- `Dockerfile.worker`: same pattern
- `Dockerfile.web`: Next.js standalone build
- `Dockerfile.agent-linux`: agent binary for Linux
- `docker-compose.yml`: full stack (API, web, worker, PostgreSQL, Redis, TimescaleDB)
- `docker-compose.dev.yml`: dev overrides (hot reload, debug ports)
- Self-hosted setup script (first-run: generate CA, run migrations, seed blocklist)
- Helm chart skeleton
- GitHub Actions CI: lint, test, build, sign, publish

## Execution Strategy

**Wave 1 (serial):** Sub-Plan 1 — Foundation. Must complete before anything else.

**Wave 2 (parallel):** Sub-Plans 2 + 3 — API and Agent Core can be built simultaneously since they share only `bb-common` types (completed in Wave 1).

**Wave 3 (parallel):** Sub-Plans 4 + 5 — Agent Comms (needs Agent Core) and Web (needs API) can proceed in parallel.

**Wave 4 (serial):** Sub-Plan 6 — Deployment. Needs all artifacts.

## Definition of Done — Phase 1 MVP

- [ ] User can register an account on the web platform
- [ ] User can invite an accountability partner
- [ ] Partner can accept invitation
- [ ] User can create an enrollment (self or partner)
- [ ] User can generate an enrollment token for a device
- [ ] Agent can be installed on Linux, enroll via token, and begin DNS blocking
- [ ] Blocked gambling domains return NXDOMAIN
- [ ] HOSTS file is updated as fallback
- [ ] Agent sends heartbeats; dashboard shows device status
- [ ] Agent reports blocked attempt counts
- [ ] Self-enrolled user can request time-delayed unenrollment
- [ ] Partner-enrolled device requires partner approval to unenroll
- [ ] Blocklist can be managed via admin panel
- [ ] Agents sync blocklist deltas
- [ ] Stripe billing works for hosted tier
- [ ] Self-hosted deployment works via docker-compose
- [ ] All code has tests; CI is green
