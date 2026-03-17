---
title: Repository Structure
description: Crate and directory layout
---


**Date:** 2026-03-12
**Status:** Proposed
**Relates to:** [Vision & Design Document](../plans/2026-03-12-betblocker-vision-design.md)

---

## 1. Monorepo Layout

```
betblocker/
├── Cargo.toml                 # Rust workspace root
├── Cargo.lock
├── rust-toolchain.toml        # Pinned Rust toolchain + targets
├── .cargo/
│   └── config.toml            # Cross-compilation linker configs
├── crates/
│   ├── bb-common/             # Shared domain types, crypto, blocklist format
│   ├── bb-proto/              # Protocol definitions + generated code
│   ├── bb-agent-core/         # Cross-platform blocking engine
│   ├── bb-agent-windows/      # Windows platform shim
│   ├── bb-agent-macos/        # macOS platform shim
│   ├── bb-agent-linux/        # Linux platform shim
│   ├── bb-agent-android/      # Android platform shim
│   ├── bb-agent-ios/          # iOS platform shim
│   ├── bb-agent-plugins/      # Plugin trait definitions + built-in plugins
│   ├── bb-api/                # Axum API server
│   ├── bb-worker/             # Background job processor
│   └── bb-cli/                # Admin CLI tool
├── web/                       # Next.js application
├── deploy/
│   ├── docker/
│   │   ├── Dockerfile.api
│   │   ├── Dockerfile.worker
│   │   ├── Dockerfile.web
│   │   └── Dockerfile.agent-linux
│   ├── helm/
│   │   └── betblocker/
│   └── docker-compose.yml
├── migrations/                # SQL migrations (shared by API and worker)
├── tests/
│   ├── integration/           # API + DB integration tests
│   ├── e2e/                   # Agent + API end-to-end tests
│   └── fixtures/              # Shared test data
├── tools/
│   ├── ci/                    # CI pipeline scripts
│   ├── signing/               # Binary signing scripts and config
│   ├── blocklist-seed/        # Initial blocklist seed data
│   └── dev-setup.sh           # One-command local setup
├── configs/
│   ├── api.example.toml       # Example API config for self-hosted
│   ├── agent.example.toml     # Example agent config
│   └── worker.example.toml    # Example worker config
├── docs/
├── .github/
│   └── workflows/
│       ├── pr.yml
│       ├── merge.yml
│       └── release.yml
├── .env.example
├── justfile                   # Task runner (replaces Makefile)
└── deny.toml                  # cargo-deny configuration
```

---

## 2. Directory Details

### 2.1 `crates/bb-common/`

**Purpose:** Shared domain types, error types, crypto primitives, and the blocklist wire format. This is the single source of truth for types that cross boundaries between agent, API, and worker.

**Key files:**

```
bb-common/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── domain/
    │   ├── mod.rs
    │   ├── account.rs          # Account, AccountId
    │   ├── device.rs           # Device, DeviceId, Heartbeat
    │   ├── enrollment.rs       # Enrollment, EnrollmentTier, UnenrollmentPolicy
    │   ├── blocklist.rs        # BlocklistEntry, BlocklistVersion, DeltaPatch
    │   └── event.rs            # BlockEvent, TamperEvent, EventEnvelope
    ├── crypto/
    │   ├── mod.rs
    │   ├── signing.rs          # Ed25519 blocklist/binary signing, verification
    │   ├── certificates.rs     # mTLS certificate handling, pinning logic
    │   └── encryption.rs       # Envelope encryption for enrollment credentials
    ├── blocklist/
    │   ├── mod.rs
    │   ├── format.rs           # Binary blocklist format (compact, memory-mapped)
    │   ├── matcher.rs          # Domain matching (exact, wildcard, pattern)
    │   └── delta.rs            # Delta encoding/decoding for sync
    ├── error.rs                # Unified error types (thiserror)
    └── config.rs               # Shared config primitives (Duration wrappers, URL types)
```

**Cargo.toml:**

```toml
[package]
name = "bb-common"
version = "0.1.0"
edition = "2024"

[features]
default = []
hosted = ["stripe-types"]     # Enables billing-related types
stripe-types = []

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v7", "serde"] }
ed25519-dalek = "2"
base64 = "0.22"
```

**Dependencies:** None within the workspace. This is a leaf crate.

---

### 2.2 `crates/bb-proto/`

**Purpose:** Protocol buffer definitions and generated Rust code for agent-to-API communication. Uses `prost` for code generation. The protobuf schemas are the canonical definition of the wire protocol; the generated Rust types are re-exported for both agent and API crates to depend on.

**Key files:**

```
bb-proto/
├── Cargo.toml
├── build.rs                    # prost-build code generation
└── proto/
    ├── agent.proto             # Heartbeat, SyncRequest, SyncResponse
    ├── blocklist.proto         # BlocklistDelta, BlocklistManifest
    ├── events.proto            # EventBatch, BlockEvent, TamperEvent
    └── config.proto            # AgentConfig, EnrollmentConfig
```

**Cargo.toml:**

```toml
[package]
name = "bb-proto"
version = "0.1.0"
edition = "2024"

[dependencies]
prost = "0.13"

[build-dependencies]
prost-build = "0.13"
```

**build.rs:**

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::compile_protos(
        &[
            "proto/agent.proto",
            "proto/blocklist.proto",
            "proto/events.proto",
            "proto/config.proto",
        ],
        &["proto/"],
    )?;
    Ok(())
}
```

**Dependencies:** None within the workspace.

---

### 2.3 `crates/bb-agent-core/`

**Purpose:** The cross-platform blocking engine. Contains all logic that does not touch OS-specific APIs: blocklist loading, DNS resolution logic, event batching, heartbeat scheduling, tamper detection state machines, and the plugin host. Platform shims implement the `PlatformBridge` trait that this crate defines.

**Key files:**

```
bb-agent-core/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── engine.rs               # Main blocking engine lifecycle
    ├── dns/
    │   ├── mod.rs
    │   ├── resolver.rs         # Local DNS resolver (trust-dns/hickory)
    │   └── interceptor.rs      # DNS query interception and filtering
    ├── blocklist/
    │   ├── mod.rs
    │   ├── store.rs            # On-disk blocklist cache, mmap loading
    │   └── sync.rs             # Delta sync with API
    ├── heartbeat.rs            # Periodic heartbeat + status reporting
    ├── events/
    │   ├── mod.rs
    │   ├── collector.rs        # Event batching and privacy filtering
    │   └── reporter.rs         # Batched event upload to API
    ├── tamper/
    │   ├── mod.rs
    │   ├── integrity.rs        # Binary self-hash validation
    │   └── watchdog.rs         # Watchdog protocol (cross-process monitoring)
    ├── platform.rs             # PlatformBridge trait definition
    ├── plugin/
    │   ├── mod.rs
    │   ├── host.rs             # Plugin lifecycle management
    │   ├── traits.rs           # Plugin trait (BlockingPlugin, ReportingPlugin)
    │   └── hosts_file.rs       # Built-in HOSTS file plugin (cross-platform)
    ├── api_client.rs           # HTTP client for API communication (reqwest + mTLS)
    └── config.rs               # Agent runtime config loading
```

**Cargo.toml:**

```toml
[package]
name = "bb-agent-core"
version = "0.1.0"
edition = "2024"

[dependencies]
bb-common = { path = "../bb-common" }
bb-proto = { path = "../bb-proto" }
tokio = { version = "1", features = ["full"] }
hickory-resolver = "0.25"
reqwest = { version = "0.12", features = ["rustls-tls", "json"] }
tracing = "0.1"
```

**Dependencies within workspace:** `bb-common`, `bb-proto`.

---

### 2.4 `crates/bb-agent-{platform}/`

Each platform shim is a separate crate that produces the final agent binary for that platform. The shim implements the `PlatformBridge` trait from `bb-agent-core` and contains the `fn main()`.

**Shared structure (example: bb-agent-windows):**

```
bb-agent-windows/
├── Cargo.toml
├── build.rs                    # Windows resource embedding, manifest
└── src/
    ├── main.rs                 # Service entry point (windows-service crate)
    ├── bridge.rs               # PlatformBridge implementation
    ├── wfp.rs                  # Windows Filtering Platform integration
    ├── service.rs              # Windows Service lifecycle
    └── installer.rs            # Self-registration as Windows Service
```

**Platform-specific notes:**

| Crate | Binary name | Extra dependencies | Build notes |
|-------|------------|-------------------|-------------|
| `bb-agent-windows` | `betblocker-agent.exe` | `windows-service`, `windows` (winapi) | Requires MSVC toolchain; embeds app manifest |
| `bb-agent-macos` | `betblocker-agent` | `objc2`, system extension bindings | Builds `.app` bundle; requires Apple signing |
| `bb-agent-linux` | `betblocker-agent` | `nix` | Produces static binary via musl; systemd unit template |
| `bb-agent-android` | `libbetblocker.so` | JNI bindings (`jni` crate) | Cross-compiled to `aarch64-linux-android` + `armv7-linux-androideabi`; bundled in AAR |
| `bb-agent-ios` | `libbetblocker.a` | Swift bridge (via `swift-bridge`) | Cross-compiled to `aarch64-apple-ios`; packaged as XCFramework |

**Cargo.toml (bb-agent-windows example):**

```toml
[package]
name = "bb-agent-windows"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "betblocker-agent"
path = "src/main.rs"

[dependencies]
bb-agent-core = { path = "../bb-agent-core" }
bb-common = { path = "../bb-common" }
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
windows-service = "0.7"
windows = { version = "0.58", features = [
    "Win32_NetworkManagement_WindowsFilteringPlatform",
    "Win32_Security",
] }

[build-dependencies]
winresource = "0.1"
```

**Dependencies within workspace:** `bb-agent-core`, `bb-common`.

---

### 2.5 `crates/bb-agent-plugins/`

**Purpose:** Plugin trait definitions (re-exported from `bb-agent-core` for external plugin authors) and built-in plugins beyond the HOSTS file plugin. Phase 2+ plugins like app-blocking and browser integration live here.

```
bb-agent-plugins/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── app_blocker/            # Phase 2: application blocking plugin
    │   ├── mod.rs
    │   ├── scanner.rs
    │   └── signatures.rs
    └── browser/                # Phase 3: browser integration plugin
        ├── mod.rs
        └── extension_manager.rs
```

**Dependencies within workspace:** `bb-agent-core`, `bb-common`.

---

### 2.6 `crates/bb-api/`

**Purpose:** The Axum HTTP API server. Stateless, single binary. Handles authentication, enrollment management, device communication, billing (hosted only), and blocklist CRUD.

**Key files:**

```
bb-api/
├── Cargo.toml
└── src/
    ├── main.rs                 # Server startup, graceful shutdown
    ├── app.rs                  # Router composition
    ├── config.rs               # Config loading (env + TOML)
    ├── db/
    │   ├── mod.rs
    │   ├── pool.rs             # SQLx connection pool setup
    │   └── repo/
    │       ├── mod.rs
    │       ├── account.rs
    │       ├── device.rs
    │       ├── enrollment.rs
    │       └── blocklist.rs
    ├── routes/
    │   ├── mod.rs
    │   ├── auth.rs             # Login, register, refresh tokens
    │   ├── device.rs           # Device registration, heartbeat, sync
    │   ├── enrollment.rs       # Enrollment CRUD, unenrollment flow
    │   ├── blocklist.rs        # Blocklist management, delta endpoint
    │   ├── reports.rs          # Reporting engine endpoints
    │   ├── admin.rs            # Admin-only routes
    │   └── billing.rs          # Stripe webhooks + subscription mgmt (hosted only)
    ├── middleware/
    │   ├── mod.rs
    │   ├── auth.rs             # JWT extraction + validation
    │   ├── device_auth.rs      # mTLS device certificate validation
    │   └── rate_limit.rs
    ├── services/
    │   ├── mod.rs
    │   ├── enrollment.rs       # Enrollment business logic
    │   ├── blocklist.rs        # Blocklist compilation, signing
    │   └── billing.rs          # Stripe integration (behind #[cfg(feature = "hosted")])
    └── error.rs                # API error types -> HTTP responses
```

**Cargo.toml:**

```toml
[package]
name = "bb-api"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "betblocker-api"
path = "src/main.rs"

[features]
default = []
hosted = ["bb-common/hosted", "stripe"]

[dependencies]
bb-common = { path = "../bb-common" }
bb-proto = { path = "../bb-proto" }
axum = "0.8"
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "uuid", "chrono", "json"] }
redis = { version = "0.27", features = ["tokio-comp"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace", "compression-gzip"] }
jsonwebtoken = "9"
argon2 = "0.5"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"
stripe = { version = "0.35", optional = true }

[dev-dependencies]
reqwest = { version = "0.12", features = ["json"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres"] }
testcontainers = "0.23"
```

**Dependencies within workspace:** `bb-common`, `bb-proto`.

---

### 2.7 `crates/bb-worker/`

**Purpose:** Background job processor. Shares domain logic and database access patterns with `bb-api` but runs as a separate binary. Handles blocklist compilation, federated report processing, discovery pipeline, analytics aggregation, and heartbeat timeout detection.

```
bb-worker/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── config.rs
    ├── jobs/
    │   ├── mod.rs
    │   ├── blocklist_compile.rs    # Compile blocklist + generate deltas
    │   ├── federated_ingest.rs     # Process federated agent reports
    │   ├── discovery.rs            # Automated gambling site discovery
    │   ├── analytics.rs            # Aggregate time-series data
    │   └── heartbeat_monitor.rs    # Detect missed heartbeats, fire alerts
    ├── scheduler.rs                # Cron-like job scheduling
    └── queue.rs                    # Redis-backed job queue consumer
```

**Cargo.toml:**

```toml
[package]
name = "bb-worker"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "betblocker-worker"
path = "src/main.rs"

[features]
default = []
hosted = ["bb-common/hosted"]

[dependencies]
bb-common = { path = "../bb-common" }
bb-proto = { path = "../bb-proto" }
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "uuid", "chrono", "json"] }
redis = { version = "0.27", features = ["tokio-comp"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"
```

**Dependencies within workspace:** `bb-common`, `bb-proto`.

---

### 2.8 `crates/bb-cli/`

**Purpose:** Admin CLI for self-hosted operators. Manages database migrations, blocklist import/export, enrollment administration, and system health checks.

```
bb-cli/
├── Cargo.toml
└── src/
    ├── main.rs
    └── commands/
        ├── mod.rs
        ├── migrate.rs          # Run database migrations
        ├── blocklist.rs        # Import/export blocklist
        ├── enrollment.rs       # Manual enrollment management
        └── health.rs           # System health check
```

**Dependencies within workspace:** `bb-common`.

---

### 2.9 `web/`

**Purpose:** Next.js application serving all web interfaces: marketing site, user/partner/authority dashboards, and admin panel.

```
web/
├── package.json
├── next.config.ts
├── tsconfig.json
├── tailwind.config.ts
├── .env.local.example
├── public/
│   └── images/
├── src/
│   ├── app/                    # App Router
│   │   ├── (marketing)/        # Route group: landing, pricing, about
│   │   │   ├── page.tsx
│   │   │   └── pricing/page.tsx
│   │   ├── (dashboard)/        # Route group: authenticated views
│   │   │   ├── layout.tsx      # Sidebar, auth guard
│   │   │   ├── devices/page.tsx
│   │   │   ├── enrollments/page.tsx
│   │   │   └── reports/page.tsx
│   │   ├── (partner)/          # Route group: partner dashboard
│   │   ├── (authority)/        # Route group: authority dashboard (Phase 3)
│   │   ├── (admin)/            # Route group: admin panel
│   │   │   ├── blocklist/page.tsx
│   │   │   └── review-queue/page.tsx
│   │   ├── api/                # Next.js API routes (BFF pattern)
│   │   └── layout.tsx          # Root layout
│   ├── components/
│   │   ├── ui/                 # Shadcn-style primitives
│   │   └── domain/             # Domain-specific components
│   ├── lib/
│   │   ├── api-client.ts       # Typed client for Rust API
│   │   ├── auth.ts             # Auth helpers (JWT, session)
│   │   └── config.ts           # Runtime config (reads NEXT_PUBLIC_ vars)
│   └── types/
│       └── api.ts              # TypeScript types mirroring bb-common domain types
├── tests/
│   ├── unit/
│   └── e2e/                    # Playwright tests
└── playwright.config.ts
```

**package.json key scripts:**

```json
{
  "scripts": {
    "dev": "next dev",
    "build": "next build",
    "start": "next start",
    "lint": "next lint",
    "typecheck": "tsc --noEmit",
    "test": "vitest",
    "test:e2e": "playwright test",
    "generate:api-types": "node scripts/generate-api-types.mjs"
  }
}
```

**Dependencies on other directories:** Calls `bb-api` at runtime via HTTP. Types in `web/src/types/api.ts` are manually synchronized with `bb-common/src/domain/` (automated via `generate:api-types` script that reads the OpenAPI spec exported by `bb-api`).

---

### 2.10 `deploy/`

**Purpose:** All deployment configuration. Docker images, docker-compose for self-hosted, Helm chart for hosted Kubernetes.

```
deploy/
├── docker/
│   ├── Dockerfile.api
│   ├── Dockerfile.worker
│   ├── Dockerfile.web
│   └── Dockerfile.agent-linux
├── helm/
│   └── betblocker/
│       ├── Chart.yaml
│       ├── values.yaml
│       ├── values.production.yaml
│       └── templates/
│           ├── api-deployment.yaml
│           ├── worker-deployment.yaml
│           ├── web-deployment.yaml
│           ├── ingress.yaml
│           ├── configmap.yaml
│           └── secrets.yaml
└── docker-compose.yml
```

**Dependencies:** References built artifacts from `crates/` and `web/`.

---

### 2.11 `migrations/`

**Purpose:** SQL migration files managed by `sqlx`. Shared between `bb-api` (applies on startup in dev) and `bb-cli` (applies explicitly in production).

```
migrations/
├── 20260312000001_create_accounts.sql
├── 20260312000002_create_organizations.sql
├── 20260312000003_create_devices.sql
├── 20260312000004_create_enrollments.sql
├── 20260312000005_create_blocklist.sql
├── 20260312000006_create_events.sql
└── 20260312000007_create_timescaledb_hypertables.sql
```

---

### 2.12 `tests/`

```
tests/
├── integration/
│   ├── api/                    # API tests against real Postgres (testcontainers)
│   │   ├── auth_test.rs
│   │   ├── enrollment_test.rs
│   │   └── device_sync_test.rs
│   └── worker/
│       └── blocklist_compile_test.rs
├── e2e/
│   ├── agent_api_sync.rs       # Agent syncs blocklist from running API
│   └── enrollment_flow.rs      # Full enrollment lifecycle
└── fixtures/
    ├── seed_blocklist.json
    └── test_certificates/
```

---

### 2.13 `tools/`

```
tools/
├── ci/
│   ├── cross-compile.sh        # Cross-compilation helper
│   ├── sign-binary.sh          # Binary signing wrapper
│   └── changelog.sh            # Changelog generation from conventional commits
├── signing/
│   ├── README.md               # Signing setup docs (keys stored in CI secrets)
│   └── verify.sh               # Verify a signed binary locally
├── blocklist-seed/
│   └── gambling-domains.csv    # Initial blocklist seed (public sources)
└── dev-setup.sh                # Install toolchain, start docker services
```

---

## 3. Rust Workspace Configuration

### Root `Cargo.toml`

```toml
[workspace]
resolver = "3"
members = [
    "crates/bb-common",
    "crates/bb-proto",
    "crates/bb-agent-core",
    "crates/bb-agent-windows",
    "crates/bb-agent-macos",
    "crates/bb-agent-linux",
    "crates/bb-agent-android",
    "crates/bb-agent-ios",
    "crates/bb-agent-plugins",
    "crates/bb-api",
    "crates/bb-worker",
    "crates/bb-cli",
]

# Platform shims are only buildable on their target (or via cross-compilation).
# Default members exclude platform shims so `cargo build` works on any dev machine.
default-members = [
    "crates/bb-common",
    "crates/bb-proto",
    "crates/bb-agent-core",
    "crates/bb-agent-plugins",
    "crates/bb-api",
    "crates/bb-worker",
    "crates/bb-cli",
]

[workspace.package]
edition = "2024"
license = "AGPL-3.0-or-later"
repository = "https://github.com/betblocker/betblocker"

[workspace.dependencies]
# Pin shared dependency versions here; crates reference via { workspace = true }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "uuid", "chrono", "json"] }
uuid = { version = "1", features = ["v7", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"

[workspace.lints.rust]
unsafe_code = "deny"

[workspace.lints.clippy]
all = "warn"
pedantic = "warn"

[profile.release]
lto = "thin"
strip = true
codegen-units = 1
panic = "abort"          # Smaller binaries, no unwinding overhead
```

### `rust-toolchain.toml`

```toml
[toolchain]
channel = "1.85"
components = ["rustfmt", "clippy", "llvm-tools"]
targets = [
    "x86_64-unknown-linux-musl",
    "x86_64-pc-windows-msvc",
    "aarch64-apple-darwin",
    "aarch64-linux-android",
    "aarch64-apple-ios",
]
```

### `.cargo/config.toml`

```toml
# Cross-compilation linker settings. CI sets these env vars to the correct
# cross-toolchain paths. Developers targeting their own platform do not need these.

[target.x86_64-unknown-linux-musl]
linker = "x86_64-linux-musl-gcc"

[target.aarch64-linux-android]
linker = "aarch64-linux-android33-clang"

[target.aarch64-apple-ios]
# Uses default Xcode toolchain

[target.aarch64-apple-darwin]
# Uses default Xcode toolchain

# Alias for common workflows
[alias]
bb-api = "run -p bb-api --"
bb-worker = "run -p bb-worker --"
bb-cli = "run -p bb-cli --"
bb-test = "test --workspace --exclude bb-agent-windows --exclude bb-agent-macos --exclude bb-agent-linux --exclude bb-agent-android --exclude bb-agent-ios"
```

---

## 4. Cross-Compilation Strategy

### 4.1 Architecture: Separate Binaries, Not Conditional Compilation

Each platform shim is its own crate producing its own binary. This is preferable to a single crate with `#[cfg(target_os)]` because:

- **Build isolation** -- a Windows developer running `cargo build` does not pull in macOS-only dependencies or fail on missing system headers.
- **Dependency clarity** -- each platform crate declares exactly the system dependencies it needs.
- **CI simplicity** -- each platform binary is built in a dedicated CI job with the appropriate toolchain.
- **Binary size** -- no dead code from other platforms gets linked.

The shared logic lives in `bb-agent-core`. Platform shims are thin: typically 500-2000 lines implementing the `PlatformBridge` trait and the service entry point.

### 4.2 CI Build Matrix

| Target | CI Runner | Toolchain | Output |
|--------|-----------|-----------|--------|
| Windows x86_64 | `windows-latest` | MSVC | `betblocker-agent.exe` |
| macOS aarch64 | `macos-14` (Apple Silicon) | Xcode + Rust | `betblocker-agent` (in .app bundle) |
| Linux x86_64 | `ubuntu-latest` | musl-cross | `betblocker-agent` (static binary) |
| Android aarch64 + armv7 | `ubuntu-latest` | Android NDK r26 | `libbetblocker.so` (AAR) |
| iOS aarch64 | `macos-14` | Xcode + Rust | `libbetblocker.a` (XCFramework) |
| API (Linux) | `ubuntu-latest` | musl-cross | `betblocker-api` (Docker image) |
| Worker (Linux) | `ubuntu-latest` | musl-cross | `betblocker-worker` (Docker image) |

### 4.3 Feature Flags

```
bb-common features:
  hosted          -- Includes billing-related types (SubscriptionStatus, PlanTier)

bb-api features:
  hosted          -- Enables Stripe billing routes, subscription middleware.
                     Activates bb-common/hosted transitively.

bb-worker features:
  hosted          -- Enables billing-related jobs (subscription reminders, usage metering).

bb-agent-core features:
  (none currently -- all agent features are always compiled)
```

Hosted builds pass `--features hosted` in CI. Self-hosted builds use default features. The binary is identical except for the presence/absence of billing code paths.

---

## 5. Docker Build Strategy

### 5.1 API Multi-Stage Build (`deploy/docker/Dockerfile.api`)

```dockerfile
# Stage 1: Build
FROM rust:1.85-bookworm AS builder

RUN apt-get update && apt-get install -y musl-tools

WORKDIR /build
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/ crates/

# Build only the API binary, statically linked via musl
RUN cargo build --release --target x86_64-unknown-linux-musl -p bb-api \
    --features hosted

# Stage 2: Runtime
FROM scratch

COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/betblocker-api /betblocker-api
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

EXPOSE 8080
ENTRYPOINT ["/betblocker-api"]
```

**Result:** ~15MB image from `scratch`. No shell, no OS, minimal attack surface.

The worker Dockerfile is identical except it builds `bb-worker` instead of `bb-api`.

### 5.2 Web Multi-Stage Build (`deploy/docker/Dockerfile.web`)

```dockerfile
# Stage 1: Dependencies
FROM node:22-alpine AS deps
WORKDIR /app
COPY web/package.json web/package-lock.json ./
RUN npm ci

# Stage 2: Build
FROM node:22-alpine AS builder
WORKDIR /app
COPY --from=deps /app/node_modules ./node_modules
COPY web/ .
RUN npm run build

# Stage 3: Runtime
FROM node:22-alpine
WORKDIR /app

RUN addgroup -g 1001 -S betblocker && \
    adduser -S betblocker -u 1001

COPY --from=builder /app/.next/standalone ./
COPY --from=builder /app/.next/static ./.next/static
COPY --from=builder /app/public ./public

USER betblocker
EXPOSE 3000
CMD ["node", "server.js"]
```

### 5.3 Docker Compose (Self-Hosted)

```yaml
# deploy/docker-compose.yml
services:
  api:
    image: ghcr.io/betblocker/api:latest
    ports:
      - "8080:8080"
    environment:
      DATABASE_URL: postgres://betblocker:${DB_PASSWORD}@db:5432/betblocker
      REDIS_URL: redis://cache:6379
      BETBLOCKER_HOSTED: "false"
      BETBLOCKER_SIGNING_KEY_PATH: /run/secrets/signing_key
    secrets:
      - signing_key
    depends_on:
      db:
        condition: service_healthy

  worker:
    image: ghcr.io/betblocker/worker:latest
    environment:
      DATABASE_URL: postgres://betblocker:${DB_PASSWORD}@db:5432/betblocker
      REDIS_URL: redis://cache:6379
      BETBLOCKER_HOSTED: "false"
    depends_on:
      db:
        condition: service_healthy

  web:
    image: ghcr.io/betblocker/web:latest
    ports:
      - "3000:3000"
    environment:
      NEXT_PUBLIC_API_URL: http://api:8080
      NEXT_PUBLIC_HOSTED: "false"

  db:
    image: timescale/timescaledb:latest-pg17
    volumes:
      - db_data:/var/lib/postgresql/data
    environment:
      POSTGRES_USER: betblocker
      POSTGRES_PASSWORD: ${DB_PASSWORD}
      POSTGRES_DB: betblocker
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U betblocker"]
      interval: 5s
      timeout: 5s
      retries: 5

  cache:
    image: redis:7-alpine
    volumes:
      - cache_data:/data

volumes:
  db_data:
  cache_data:

secrets:
  signing_key:
    file: ./secrets/signing.key
```

Note: TimescaleDB is a PostgreSQL extension, so a single `timescale/timescaledb` image serves as both the primary database and the analytics store. No separate PostgreSQL container needed.

---

## 6. Binary Signing in CI

### 6.1 Approach

All agent binaries and server binaries are signed before distribution. Two separate signing concerns:

1. **Code signing** (OS-level): Windows Authenticode, macOS codesign + notarization, Android APK signing. Required for the agent to install without security warnings.
2. **Integrity signing** (BetBlocker-level): Ed25519 signature over the binary hash. The agent validates this on self-update. The public key is embedded in the agent at compile time.

### 6.2 CI Signing Flow

```
Build binary
    |
    v
OS code signing (platform-specific)
    |-- Windows: signtool.exe with EV code signing cert (stored in Azure Key Vault, accessed via CI)
    |-- macOS:   codesign + xcrun notarytool (Apple Developer cert in CI keychain)
    |-- Android: apksigner with upload key (key in CI secrets)
    |
    v
BetBlocker integrity signing
    |-- ed25519 sign(sha256(binary)) with release signing key (CI secret)
    |-- Output: binary + binary.sig
    |
    v
Upload to release artifacts
```

### 6.3 Key Management

| Key | Storage | Rotation |
|-----|---------|----------|
| Windows EV cert | Azure Key Vault (HSM-backed) | Per CA policy (annual) |
| Apple Developer cert | GitHub Actions encrypted secret | Annual renewal |
| Android upload key | GitHub Actions encrypted secret | Never (Google manages release key) |
| Ed25519 release key | GitHub Actions encrypted secret + offline backup | Manual rotation with agent update to embed new pubkey |

---

## 7. Development Workflow

### 7.1 Local Development Setup

**Prerequisites:**

- Rust (installed via `rustup`, version pinned by `rust-toolchain.toml`)
- Node.js 22+ and npm
- Docker and docker-compose
- `just` task runner (`cargo install just`)
- `sqlx-cli` (`cargo install sqlx-cli --features postgres`)
- `cargo-deny` (`cargo install cargo-deny`)

**One-command setup:**

```bash
just setup
# Equivalent to:
#   1. Start Postgres + Redis via docker-compose (services only, not app containers)
#   2. Run database migrations
#   3. Seed initial blocklist
#   4. Install web dependencies
```

### 7.2 `justfile` (Task Runner)

```just
# Start infrastructure only (DB + Redis)
infra:
    docker compose -f deploy/docker-compose.yml up -d db cache

# Run database migrations
migrate:
    cargo sqlx migrate run --source migrations

# Run API server locally
api: infra migrate
    cargo bb-api

# Run worker locally
worker: infra migrate
    cargo bb-worker

# Run web dev server
web:
    cd web && npm run dev

# Run API + web together for frontend development
dev: infra migrate
    just api &
    just web

# Build agent for current platform (detects OS)
agent:
    #!/usr/bin/env bash
    case "$(uname -s)" in
        Linux*)  cargo build -p bb-agent-linux ;;
        Darwin*) cargo build -p bb-agent-macos ;;
        MINGW*|MSYS*|CYGWIN*) cargo build -p bb-agent-windows ;;
    esac

# Run all workspace tests (excludes platform shims)
test:
    cargo bb-test

# Run API integration tests (requires running Postgres)
test-integration: infra migrate
    cargo test --test '*' -p bb-api
    cargo test -p bb-worker --test '*'

# Run web unit tests
test-web:
    cd web && npm test

# Full CI check locally
check:
    cargo fmt --check
    cargo clippy --workspace --exclude bb-agent-windows --exclude bb-agent-macos \
        --exclude bb-agent-linux --exclude bb-agent-android --exclude bb-agent-ios
    cargo bb-test
    cargo deny check
    cd web && npm run lint && npm run typecheck

# Build release Docker images locally
docker-build:
    docker build -f deploy/docker/Dockerfile.api -t betblocker-api:local .
    docker build -f deploy/docker/Dockerfile.worker -t betblocker-worker:local .
    docker build -f deploy/docker/Dockerfile.web -t betblocker-web:local web/

# Full local stack via docker-compose
up:
    docker compose -f deploy/docker-compose.yml up --build

# Setup from scratch
setup: infra
    just migrate
    cargo run -p bb-cli -- blocklist import tools/blocklist-seed/gambling-domains.csv
    cd web && npm ci
```

### 7.3 Frontend Development Workflow

For frontend work, you do not need Rust installed beyond having the API running. The simplest path:

1. `just infra` -- starts Postgres and Redis.
2. Run a pre-built API binary or `just api`.
3. `just web` -- starts Next.js dev server with hot reload.

The web app talks to the API at `http://localhost:8080` (configurable via `NEXT_PUBLIC_API_URL`).

### 7.4 Agent Development Workflow

Agent development requires building and testing on the target platform.

**On your own OS:**

```bash
just agent            # Builds agent for current OS
cargo test -p bb-agent-core   # Test cross-platform logic (always works)
```

**Testing against a running API:**

```bash
just api              # Start API locally
# In another terminal:
cargo run -p bb-agent-linux -- --api-url http://localhost:8080 --config configs/agent.example.toml
```

**Cross-platform testing** happens in CI only. Individual developers test on their own OS; CI tests all five platforms.

---

## 8. Testing Strategy

### 8.1 Test Pyramid

```
                    /  E2E  \           Agent + API together (CI only)
                   /----------\
                  / Integration \        API + real Postgres (testcontainers)
                 /----------------\
                /    Unit Tests    \     Per-crate, fast, no I/O
               /--------------------\
```

### 8.2 Unit Tests

Every crate has `#[cfg(test)]` modules co-located with the code they test. These must not depend on external services.

```bash
cargo bb-test          # All workspace unit tests
cargo test -p bb-common       # Just one crate
```

Key areas:
- `bb-common`: Blocklist matcher correctness, delta encoding round-trips, crypto verification.
- `bb-agent-core`: DNS resolver behavior, event batching logic, plugin host lifecycle.
- `bb-api`: Route handler logic with mocked repositories, JWT validation.

### 8.3 Integration Tests

Located in `tests/integration/`. Use `testcontainers` to spin up real Postgres and Redis instances. Test the API end-to-end through HTTP.

```bash
just test-integration
```

Tests cover:
- Account creation through API -> verify in DB.
- Enrollment lifecycle (create, modify, unenroll with policy enforcement).
- Device registration, heartbeat, blocklist sync flow.
- Billing webhook processing (hosted feature).

### 8.4 End-to-End Tests

Located in `tests/e2e/`. These run the full stack: API, worker, and a Linux agent (in CI, inside a Docker container). They verify that:

- An agent can register with the API, receive a blocklist, and block DNS queries.
- Blocklist updates propagate from admin action -> worker compilation -> API delta endpoint -> agent sync.
- Heartbeat timeouts trigger alerts.

E2E tests run on merge to main and on release, not on every PR (they are slow).

### 8.5 Web Tests

- **Unit/component tests** (Vitest): `cd web && npm test`
- **E2E browser tests** (Playwright): `cd web && npm run test:e2e` (requires API running)

---

## 9. CI Pipeline Design

### 9.1 On Pull Request (`pr.yml`)

Runs on every PR. Must pass before merge. Optimized for speed.

```yaml
jobs:
  rust-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --check
      - run: cargo clippy --workspace --default-members -- -D warnings
      - run: cargo test --workspace --default-members
      - run: cargo deny check

  rust-integration:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: timescale/timescaledb:latest-pg17
        env:
          POSTGRES_PASSWORD: test
        ports: [5432:5432]
      redis:
        image: redis:7-alpine
        ports: [6379:6379]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo sqlx migrate run --source migrations
        env:
          DATABASE_URL: postgres://postgres:test@localhost/postgres
      - run: cargo test -p bb-api --test '*'
        env:
          DATABASE_URL: postgres://postgres:test@localhost/postgres
          REDIS_URL: redis://localhost:6379

  web-check:
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: web
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: npm
          cache-dependency-path: web/package-lock.json
      - run: npm ci
      - run: npm run lint
      - run: npm run typecheck
      - run: npm test

  # Build-check platform shims (compile only, no run) on their native runners
  agent-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo check -p bb-agent-windows

  agent-macos:
    runs-on: macos-14
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo check -p bb-agent-macos
```

### 9.2 On Merge to Main (`merge.yml`)

Builds all artifacts and runs the full test suite including E2E.

```yaml
jobs:
  build-api:
    # ... builds Docker image, pushes to ghcr.io with :main tag

  build-worker:
    # ... builds Docker image, pushes to ghcr.io with :main tag

  build-web:
    # ... builds Docker image, pushes to ghcr.io with :main tag

  build-agent-windows:
    runs-on: windows-latest
    # ... builds release binary, uploads as artifact

  build-agent-macos:
    runs-on: macos-14
    # ... builds release binary, creates .app bundle, uploads as artifact

  build-agent-linux:
    runs-on: ubuntu-latest
    # ... builds static musl binary, uploads as artifact

  build-agent-android:
    runs-on: ubuntu-latest
    # ... cross-compiles with Android NDK, packages AAR, uploads as artifact

  build-agent-ios:
    runs-on: macos-14
    # ... cross-compiles, packages XCFramework, uploads as artifact

  e2e:
    needs: [build-api, build-agent-linux]
    runs-on: ubuntu-latest
    # ... spins up API + worker + Postgres + Redis in Docker, runs Linux agent E2E tests

  web-e2e:
    needs: [build-api, build-web]
    runs-on: ubuntu-latest
    # ... spins up full stack, runs Playwright tests
```

### 9.3 On Release Tag (`release.yml`)

Triggered by pushing a version tag (`v*`). Builds, signs, and publishes everything.

```yaml
on:
  push:
    tags: ["v*"]

jobs:
  # 1. Build all artifacts (same as merge.yml)
  # 2. Sign all binaries (OS code signing + Ed25519)
  # 3. Run full test suite
  # 4. Create GitHub Release with:
  #    - Signed agent binaries (Windows, macOS, Linux)
  #    - Android AAR
  #    - iOS XCFramework
  #    - Changelog (auto-generated from conventional commits)
  # 5. Push Docker images with version tag to ghcr.io
  # 6. Update Helm chart version
```

---

## 10. Release Process

### 10.1 Versioning

Semantic versioning with a single version across all crates and the web app. All workspace crate versions stay in sync.

```
v0.1.0  -- Phase 1 MVP
v0.2.0  -- Phase 1 feature complete
v1.0.0  -- Production-ready
```

Agent-to-API protocol changes that break backward compatibility require a major version bump. The API must support the current and previous major version simultaneously during a transition period.

### 10.2 Release Steps

1. Ensure `main` is green (all CI checks pass).
2. Update version in workspace `Cargo.toml` and `web/package.json`.
3. Run `just changelog` to generate changelog from conventional commits since last tag.
4. Commit: `chore: release v0.2.0`.
5. Tag: `git tag v0.2.0`.
6. Push tag: `git push origin v0.2.0`. CI takes over.
7. CI builds, tests, signs, publishes GitHub Release and Docker images.

### 10.3 Changelog Generation

Uses conventional commits (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`). The `tools/ci/changelog.sh` script groups commits by type and generates a markdown changelog. Example:

```markdown
## v0.2.0 (2026-04-15)

### Features
- feat(agent): add VPN detection on Windows (#142)
- feat(api): partner invitation email flow (#138)

### Fixes
- fix(blocklist): delta sync fails on empty initial blocklist (#145)
- fix(web): dashboard loading state flicker (#141)
```

### 10.4 Binary Distribution

| Channel | Mechanism | Audience |
|---------|-----------|----------|
| GitHub Releases | Direct download of signed binaries | Self-hosted operators, manual installs |
| Docker (ghcr.io) | `docker pull ghcr.io/betblocker/{api,worker,web}:v0.2.0` | Self-hosted Docker deployments |
| Hosted CDN | betblocker.com/download (redirects to CDN) | Hosted users, agent auto-update |
| Helm | `helm repo add betblocker ...` | Kubernetes deployments |

---

## 11. Shared Code Strategy

### 11.1 What Goes in `bb-common`

| Category | Examples | Why shared |
|----------|----------|-----------|
| Domain types | `Account`, `Device`, `Enrollment`, `BlocklistEntry` | API, worker, agent, and CLI all operate on these |
| Error types | `BetBlockerError` enum | Consistent error handling across crates |
| Crypto primitives | Ed25519 signing/verification, certificate pinning logic | Agent verifies what API signs |
| Blocklist format | Binary format, delta encoding, domain matcher | Agent reads what worker compiles |
| Config primitives | Duration wrappers, URL types | Shared config parsing |

### 11.2 What Does NOT Go in `bb-common`

- Database models (SQLx types live in `bb-api` and `bb-worker`).
- HTTP routing or middleware (lives in `bb-api`).
- Platform-specific code (lives in `bb-agent-{platform}`).
- Job scheduling logic (lives in `bb-worker`).

The goal: `bb-common` compiles on every target including iOS and Android with no system dependencies beyond `std`.

### 11.3 Proto as Contract

The `.proto` files in `bb-proto/proto/` define the binary wire format between agent and API. Both `bb-agent-core` and `bb-api` depend on `bb-proto` and use the generated Rust structs. Changes to proto files require version negotiation (the `SyncRequest` message includes a `protocol_version` field).

### 11.4 API-to-Web Type Sharing

The API exposes an OpenAPI spec (generated from Axum route metadata via `utoipa`). A build script in `web/` runs `openapi-typescript` to generate TypeScript types from this spec:

```bash
# web/scripts/generate-api-types.mjs
# 1. Fetches OpenAPI spec from running API (or reads static export)
# 2. Generates web/src/types/api.ts
```

This is run manually when API types change. It is not part of the automated build. A CI check verifies the generated types are up to date.

---

## 12. Configuration

### 12.1 Environment Variable Conventions

All BetBlocker-specific environment variables use the `BETBLOCKER_` prefix. Standard conventions like `DATABASE_URL` and `REDIS_URL` are used for infrastructure.

```bash
# Infrastructure (no prefix, standard conventions)
DATABASE_URL=postgres://user:pass@host:5432/betblocker
REDIS_URL=redis://host:6379

# Application config (BETBLOCKER_ prefix)
BETBLOCKER_HOSTED=true|false              # Master toggle for hosted vs self-hosted
BETBLOCKER_SIGNING_KEY_PATH=/path/to/key  # Ed25519 signing key for blocklist/binary signing
BETBLOCKER_JWT_SECRET=...                 # JWT signing secret
BETBLOCKER_LOG_LEVEL=info                 # Tracing level
BETBLOCKER_LOG_FORMAT=json|pretty         # JSON for production, pretty for dev

# Hosted-only (ignored when BETBLOCKER_HOSTED=false)
STRIPE_SECRET_KEY=sk_...
STRIPE_WEBHOOK_SECRET=whsec_...
BETBLOCKER_CDN_URL=https://cdn.betblocker.com

# Web (NEXT_PUBLIC_ prefix for client-side access)
NEXT_PUBLIC_API_URL=http://localhost:8080
NEXT_PUBLIC_HOSTED=true|false
```

### 12.2 Config File Format (TOML)

For self-hosted deployments, a TOML config file can replace environment variables. Environment variables take precedence over the config file.

**`configs/api.example.toml`:**

```toml
[server]
host = "0.0.0.0"
port = 8080

[database]
url = "postgres://betblocker:password@localhost:5432/betblocker"
max_connections = 20

[redis]
url = "redis://localhost:6379"

[auth]
jwt_secret = "CHANGE_ME"
jwt_expiry_seconds = 3600
refresh_token_expiry_days = 30

[blocklist]
signing_key_path = "/etc/betblocker/signing.key"
delta_retention_days = 30

[logging]
level = "info"
format = "json"           # "json" or "pretty"
```

**`configs/agent.example.toml`:**

```toml
[api]
url = "https://api.betblocker.com"
# For self-hosted:
# url = "https://your-server.example.com:8080"

[dns]
listen_address = "127.0.0.1:53"
upstream = ["1.1.1.1:53", "8.8.8.8:53"]

[heartbeat]
interval_seconds = 300

[blocklist]
cache_path = "/var/lib/betblocker/blocklist.bin"
sync_interval_seconds = 3600

[logging]
level = "info"
path = "/var/log/betblocker/agent.log"
```

### 12.3 Feature Flag Behavior

The `BETBLOCKER_HOSTED` environment variable is a runtime flag that works in conjunction with compile-time feature flags:

| Scenario | Compile-time | Runtime | Effect |
|----------|-------------|---------|--------|
| Hosted production | `--features hosted` | `BETBLOCKER_HOSTED=true` | Billing routes active, Stripe connected |
| Self-hosted Docker | default features | `BETBLOCKER_HOSTED=false` | Billing routes not compiled in, no Stripe dependency |
| Development (testing billing) | `--features hosted` | `BETBLOCKER_HOSTED=true` | Stripe test mode |
| Development (normal) | default features | not set (defaults to false) | No billing code |

The compile-time flag removes billing code entirely from the binary. The runtime flag is a secondary guard for operational flexibility (you could compile with `hosted` but run with `BETBLOCKER_HOSTED=false` to temporarily disable billing without redeploying).

For the web app, `NEXT_PUBLIC_HOSTED` controls whether billing UI, marketing pages, and hosted-specific features are rendered. This is a runtime check (no separate build needed).

### 12.4 Secrets Management

| Environment | Approach |
|-------------|----------|
| Local development | `.env` file (git-ignored), example in `.env.example` |
| CI | GitHub Actions encrypted secrets |
| Self-hosted Docker | Docker secrets (mounted as files) or env vars |
| Hosted production | AWS Secrets Manager, injected into pods via Kubernetes ExternalSecrets |

Secrets that appear in config:
- `DATABASE_URL` (contains password)
- `BETBLOCKER_JWT_SECRET`
- `BETBLOCKER_SIGNING_KEY_PATH` (path to key file, not the key itself)
- `STRIPE_SECRET_KEY` (hosted only)
- `STRIPE_WEBHOOK_SECRET` (hosted only)

The signing key is always referenced by file path, never passed as an environment variable, to avoid it appearing in process listings or crash dumps.

---

## 13. Dependency Management

### 13.1 `deny.toml` (cargo-deny)

```toml
[advisories]
vulnerability = "deny"
unmaintained = "warn"

[licenses]
unlicensed = "deny"
allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib"]
# AGPL-3.0 is our own license; dependencies must be permissively licensed

[bans]
multiple-versions = "warn"
deny = [
    # No OpenSSL -- we use rustls everywhere
    { name = "openssl-sys" },
]

[sources]
unknown-registry = "deny"
unknown-git = "deny"
```

### 13.2 Web Dependencies

Lock file (`package-lock.json`) is committed. `npm audit` runs in CI. No `devDependencies` ship in the production Docker image (multi-stage build ensures this).

---

## 14. Architectural Decision Records

### ADR-001: Monorepo with Rust Workspace

**Status:** Accepted

**Context:** BetBlocker has multiple Rust binaries (API, worker, CLI, 5 agent platforms) that share domain types and protocol definitions. Separate repositories would create version synchronization overhead and make cross-cutting changes (e.g., adding a field to `Enrollment`) require coordinated multi-repo PRs.

**Decision:** Single monorepo with a Cargo workspace. The Next.js app lives alongside as a sibling directory, not a workspace member. All Rust crates share a single `Cargo.lock`.

**Consequences:** Simpler cross-cutting changes and atomic commits. CI is more complex (must selectively build per platform). Repository size will grow, but Rust compilation caching mitigates build time impact.

---

### ADR-002: Separate Binary per Platform (Not Conditional Compilation)

**Status:** Accepted

**Context:** The agent needs to run on 5 platforms. We could use `#[cfg(target_os)]` in a single crate or separate crates per platform.

**Decision:** Separate crate per platform, each depending on `bb-agent-core`. The core crate defines a `PlatformBridge` trait. Each platform crate implements it and provides `fn main()`.

**Consequences:** Clean dependency graphs. No accidental cross-platform code leakage. Each platform crate's `Cargo.toml` lists only its own system dependencies. Downside: some boilerplate in each platform crate's `main.rs` for startup/shutdown. Acceptable given the number of platforms (5) is small and stable.

---

### ADR-003: Feature Flags for Hosted vs Self-Hosted

**Status:** Accepted

**Context:** The hosted platform includes billing (Stripe) and premium features. Self-hosted must not include billing code or depend on Stripe libraries.

**Decision:** Compile-time feature flag (`hosted`) gates billing code. Default build is self-hosted (no billing). Hosted CI builds pass `--features hosted`. Runtime flag `BETBLOCKER_HOSTED` provides an additional toggle.

**Consequences:** Self-hosted binary is smaller and has no Stripe dependency. Two build configurations to test in CI. The compile-time flag means billing code is not just hidden but absent from the binary, which is a stronger guarantee for self-hosted operators inspecting the build.

---

### ADR-004: Static Musl Binaries for Server Components

**Status:** Accepted

**Context:** The API and worker run in Docker containers. We want minimal container images.

**Decision:** Compile server binaries targeting `x86_64-unknown-linux-musl` to produce fully static binaries. Use `FROM scratch` Docker images.

**Consequences:** ~15MB container images. No shell or OS utilities in the container (good for security, harder for debugging -- mitigated by structured logging and health endpoints). No glibc dependency issues across Linux distributions.

---

### ADR-005: TimescaleDB as Single Database Image

**Status:** Accepted

**Context:** The vision document lists PostgreSQL and TimescaleDB as separate data stores. TimescaleDB is a PostgreSQL extension, not a separate database.

**Decision:** Use a single `timescale/timescaledb` Docker image that provides both standard PostgreSQL tables (accounts, enrollments, devices) and TimescaleDB hypertables (events, analytics). One connection string, one migration path.

**Consequences:** Simpler deployment (one database container instead of two). Self-hosted operators manage one database. Slightly larger database image than vanilla PostgreSQL, but the operational simplicity outweighs this.
