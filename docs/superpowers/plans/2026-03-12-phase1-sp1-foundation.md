# Phase 1 Sub-Plan 1: Foundation

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Set up the Rust monorepo, shared domain types, protobuf definitions, database migrations, dev infrastructure, and build tooling so all other sub-plans can begin.

**Architecture:** Cargo workspace with `bb-common` (shared types), `bb-proto` (protocol definitions), and stub crates for API, worker, agent, and CLI. PostgreSQL + Redis + TimescaleDB via docker-compose for dev.

**Tech Stack:** Rust 1.85+, Cargo workspace, protobuf (prost), PostgreSQL 16, Redis 7, TimescaleDB, Docker Compose, just

**Reference Docs:**
- Repo structure: `docs/architecture/repo-structure.md`
- DB schema: `docs/architecture/database-schema.md`
- ADR-001: `docs/architecture/adrs/ADR-001-rust-for-endpoint-agent-core.md`
- ADR-002: `docs/architecture/adrs/ADR-002-plugin-architecture-for-blocking-layers.md`
- Agent protocol: `docs/architecture/agent-protocol.md`

---

## File Structure

```
betblocker/
├── Cargo.toml                    # Workspace root
├── Cargo.lock
├── rust-toolchain.toml
├── .cargo/config.toml
├── justfile
├── .github/
│   └── workflows/
│       └── ci.yml
├── crates/
│   ├── bb-common/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── models/
│   │       │   ├── mod.rs
│   │       │   ├── account.rs
│   │       │   ├── device.rs
│   │       │   ├── enrollment.rs
│   │       │   ├── event.rs
│   │       │   ├── blocklist.rs
│   │       │   ├── organization.rs
│   │       │   └── partner.rs
│   │       ├── enums.rs
│   │       ├── error.rs
│   │       └── crypto.rs
│   ├── bb-proto/
│   │   ├── Cargo.toml
│   │   ├── build.rs
│   │   ├── proto/
│   │   │   ├── device.proto
│   │   │   ├── heartbeat.proto
│   │   │   ├── blocklist.proto
│   │   │   ├── events.proto
│   │   │   └── config.proto
│   │   └── src/
│   │       └── lib.rs
│   ├── bb-api/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs           # Stub
│   ├── bb-worker/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs           # Stub
│   ├── bb-agent-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs            # Stub
│   ├── bb-agent-plugins/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs            # Stub
│   ├── bb-agent-linux/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs           # Stub
│   └── bb-cli/
│       ├── Cargo.toml
│       └── src/
│           └── main.rs           # Stub
├── migrations/
│   ├── V001__create_enum_types.sql
│   ├── V002__create_accounts.sql
│   ├── V003__create_refresh_tokens.sql
│   ├── V004__create_partner_relationships.sql
│   ├── V005__create_organizations.sql
│   ├── V006__create_organization_members.sql
│   ├── V007__create_devices.sql
│   ├── V008__create_device_certificates.sql
│   ├── V009__create_enrollments.sql
│   ├── V010__create_enrollment_unenroll_requests.sql
│   ├── V011__create_blocklist_entries.sql
│   ├── V012__create_blocklist_versions.sql
│   ├── V013__create_blocklist_version_entries.sql
│   ├── V014__create_federated_reports.sql
│   ├── V015__create_events.sql
│   ├── V016__create_reporting_snapshots.sql
│   ├── V017__create_subscriptions.sql
│   ├── V018__create_audit_log.sql
│   ├── V019__create_rls_policies.sql
│   ├── V020__create_audit_triggers.sql
│   └── V021__seed_blocklist.sql
├── deploy/
│   └── docker-compose.dev.yml
└── tests/
    └── integration/
        └── .gitkeep
```

---

## Chunk 1: Workspace Scaffold

### Task 1: Initialize Cargo workspace and toolchain

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `rust-toolchain.toml`
- Create: `.cargo/config.toml`
- Create: `.gitignore`

- [ ] **Step 1: Create workspace Cargo.toml**

```toml
[workspace]
resolver = "2"
members = [
    "crates/bb-common",
    "crates/bb-proto",
    "crates/bb-api",
    "crates/bb-worker",
    "crates/bb-agent-core",
    "crates/bb-agent-plugins",
    "crates/bb-agent-linux",
    "crates/bb-cli",
]
# Default members excludes platform-specific agent crates on non-matching OS
default-members = [
    "crates/bb-common",
    "crates/bb-proto",
    "crates/bb-api",
    "crates/bb-worker",
    "crates/bb-cli",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "AGPL-3.0-or-later"
repository = "https://github.com/betblocker/betblocker"

[workspace.dependencies]
# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
postcard = { version = "1", features = ["alloc"] }

# Async
tokio = { version = "1", features = ["full"] }

# Web framework
axum = { version = "0.8", features = ["macros"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace", "request-id", "compression-gzip"] }

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "uuid", "chrono", "json", "migrate"] }

# Crypto
ring = "0.17"
rustls = "0.23"
jsonwebtoken = "9"

# Protocol
prost = "0.13"
prost-types = "0.13"

# Utils
uuid = { version = "1", features = ["v7", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
config = "0.14"
dotenvy = "0.15"

# Testing
insta = { version = "1", features = ["json"] }

[workspace.lints.rust]
unsafe_code = "deny"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
unwrap_used = "warn"
expect_used = "warn"
```

- [ ] **Step 2: Create rust-toolchain.toml**

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
targets = [
    "x86_64-unknown-linux-musl",
    "x86_64-pc-windows-msvc",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "aarch64-unknown-linux-musl",
]
```

- [ ] **Step 3: Create .cargo/config.toml**

```toml
[target.x86_64-unknown-linux-musl]
linker = "x86_64-linux-musl-gcc"

[target.aarch64-unknown-linux-musl]
linker = "aarch64-linux-musl-gcc"

# Faster linking for development
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
```

- [ ] **Step 4: Create .gitignore**

```
/target
.env
*.pem
*.key
*.cert
.DS_Store
node_modules/
web/.next/
web/out/
```

- [ ] **Step 5: Verify workspace compiles**

Run: `cargo check`
Expected: Success (no crates exist yet, but workspace is valid syntax — will succeed once crate stubs are added)

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml rust-toolchain.toml .cargo/config.toml .gitignore
git commit -m "chore: initialize Cargo workspace and toolchain config"
```

---

### Task 2: Create bb-common crate with domain enums

**Files:**
- Create: `crates/bb-common/Cargo.toml`
- Create: `crates/bb-common/src/lib.rs`
- Create: `crates/bb-common/src/enums.rs`

- [ ] **Step 1: Create bb-common Cargo.toml**

```toml
[package]
name = "bb-common"
version.workspace = true
edition.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }

[lints]
workspace = true
```

- [ ] **Step 2: Create enums.rs with all domain enums**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountRole {
    User,
    Partner,
    Authority,
    Admin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Windows,
    Macos,
    Linux,
    Android,
    Ios,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceStatus {
    Pending,
    Active,
    Offline,
    Unenrolling,
    Unenrolled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnrollmentTier {
    #[serde(rename = "self")]
    SelfEnrolled,
    Partner,
    Authority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnrollmentStatus {
    Pending,
    Active,
    UnenrollRequested,
    UnenrollApproved,
    Unenrolling,
    Unenrolled,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnenrollmentPolicyType {
    TimeDelayed,
    PartnerApproval,
    AuthorityApproval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnenrollRequestStatus {
    Pending,
    Approved,
    Denied,
    Expired,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartnerRelationshipStatus {
    Pending,
    Active,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartnerRole {
    AccountabilityPartner,
    Therapist,
    AuthorityRep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationType {
    Family,
    TherapyPractice,
    CourtProgram,
    Employer,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrgMemberRole {
    Admin,
    Member,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlocklistSource {
    Curated,
    Automated,
    Federated,
    Community,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlocklistEntryStatus {
    PendingReview,
    Active,
    Inactive,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GamblingCategory {
    OnlineCasino,
    SportsBetting,
    Poker,
    Lottery,
    Bingo,
    FantasySports,
    CryptoGambling,
    Affiliate,
    PaymentProcessor,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Block,
    BypassAttempt,
    TamperDetected,
    TamperSelfHealed,
    VpnDetected,
    EnrollmentCreated,
    EnrollmentModified,
    UnenrollRequested,
    UnenrollCompleted,
    Heartbeat,
    AgentStarted,
    AgentUpdated,
    BlocklistUpdated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventCategory {
    Dns,
    App,
    Browser,
    Tamper,
    Enrollment,
    Heartbeat,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportingLevel {
    None,
    Aggregated,
    Detailed,
    FullAudit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VpnDetectionMode {
    Disabled,
    Log,
    Alert,
    Lockdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TamperResponse {
    Log,
    AlertUser,
    AlertPartner,
    AlertAuthority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionPlan {
    Free,
    Standard,
    PartnerTier,
    Institutional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Trialing,
    Active,
    PastDue,
    Cancelled,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FederatedReportStatus {
    Pending,
    Promoted,
    Rejected,
    Duplicate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockingLayer {
    Dns,
    Application,
    Browser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockDecision {
    Allow,
    Block,
}
```

- [ ] **Step 3: Create lib.rs**

```rust
pub mod enums;
pub mod error;
pub mod models;
```

- [ ] **Step 4: Create error.rs**

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BetBlockerError {
    #[error("not found: {entity} with id {id}")]
    NotFound { entity: &'static str, id: String },

    #[error("unauthorized: {reason}")]
    Unauthorized { reason: String },

    #[error("forbidden: {reason}")]
    Forbidden { reason: String },

    #[error("validation error: {message}")]
    Validation { message: String },

    #[error("conflict: {message}")]
    Conflict { message: String },

    #[error("enrollment policy violation: {message}")]
    PolicyViolation { message: String },

    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}
```

- [ ] **Step 5: Verify compiles**

Run: `cargo check -p bb-common`
Expected: Success

- [ ] **Step 6: Commit**

```bash
git add crates/bb-common/
git commit -m "feat(common): add bb-common crate with domain enums and error types"
```

---

### Task 3: Add domain models to bb-common

**Files:**
- Create: `crates/bb-common/src/models/mod.rs`
- Create: `crates/bb-common/src/models/account.rs`
- Create: `crates/bb-common/src/models/device.rs`
- Create: `crates/bb-common/src/models/enrollment.rs`
- Create: `crates/bb-common/src/models/event.rs`
- Create: `crates/bb-common/src/models/blocklist.rs`
- Create: `crates/bb-common/src/models/organization.rs`
- Create: `crates/bb-common/src/models/partner.rs`

- [ ] **Step 1: Create models/mod.rs**

```rust
pub mod account;
pub mod blocklist;
pub mod device;
pub mod enrollment;
pub mod event;
pub mod organization;
pub mod partner;

pub use account::Account;
pub use blocklist::BlocklistEntry;
pub use device::Device;
pub use enrollment::{Enrollment, ProtectionConfig, ReportingConfig, UnenrollmentPolicy};
pub use event::Event;
pub use organization::Organization;
pub use partner::PartnerRelationship;
```

- [ ] **Step 2: Create account.rs**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::AccountRole;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: i64,
    pub public_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: AccountRole,
    pub email_verified: bool,
    pub mfa_enabled: bool,
    pub timezone: String,
    pub locale: String,
    pub organization_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

- [ ] **Step 3: Create device.rs**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::{DeviceStatus, Platform};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: i64,
    pub public_id: Uuid,
    pub account_id: i64,
    pub name: String,
    pub platform: Platform,
    pub os_version: String,
    pub agent_version: String,
    pub hostname: String,
    pub hardware_id: String,
    pub status: DeviceStatus,
    pub blocklist_version: Option<i64>,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    pub enrollment_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

- [ ] **Step 4: Create enrollment.rs**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::{
    EnrollmentStatus, EnrollmentTier, ReportingLevel, TamperResponse, UnenrollmentPolicyType,
    VpnDetectionMode,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enrollment {
    pub id: i64,
    pub public_id: Uuid,
    pub device_id: i64,
    pub account_id: i64,
    pub enrolled_by: i64,
    pub tier: EnrollmentTier,
    pub status: EnrollmentStatus,
    pub protection_config: ProtectionConfig,
    pub reporting_config: ReportingConfig,
    pub unenrollment_policy: UnenrollmentPolicy,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectionConfig {
    pub dns_blocking: bool,
    pub app_blocking: bool,
    pub browser_blocking: bool,
    pub vpn_detection: VpnDetectionMode,
    pub tamper_response: TamperResponse,
}

impl Default for ProtectionConfig {
    fn default() -> Self {
        Self {
            dns_blocking: true,
            app_blocking: false,
            browser_blocking: false,
            vpn_detection: VpnDetectionMode::Alert,
            tamper_response: TamperResponse::Log,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportingConfig {
    pub level: ReportingLevel,
    pub blocked_attempt_counts: bool,
    pub domain_details: bool,
    pub tamper_alerts: bool,
}

impl Default for ReportingConfig {
    fn default() -> Self {
        Self {
            level: ReportingLevel::Aggregated,
            blocked_attempt_counts: true,
            domain_details: false,
            tamper_alerts: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnenrollmentPolicy {
    #[serde(rename = "type")]
    pub policy_type: UnenrollmentPolicyType,
    pub cooldown_hours: Option<i32>,
    pub requires_approval_from: Option<Uuid>,
}
```

- [ ] **Step 5: Create event.rs**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::{EventCategory, EventSeverity, EventType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: i64,
    pub public_id: Uuid,
    pub device_id: i64,
    pub enrollment_id: i64,
    pub event_type: EventType,
    pub category: EventCategory,
    pub severity: EventSeverity,
    pub metadata: serde_json::Value,
    pub occurred_at: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
}
```

- [ ] **Step 6: Create blocklist.rs**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::{BlocklistEntryStatus, BlocklistSource, GamblingCategory};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlocklistEntry {
    pub id: i64,
    pub public_id: Uuid,
    pub domain: Option<String>,
    pub pattern: Option<String>,
    pub category: GamblingCategory,
    pub source: BlocklistSource,
    pub confidence: f64,
    pub status: BlocklistEntryStatus,
    pub added_by: Option<i64>,
    pub reviewed_by: Option<i64>,
    pub evidence_url: Option<String>,
    pub tags: Vec<String>,
    pub blocklist_version_added: Option<i64>,
    pub blocklist_version_removed: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlocklistVersion {
    pub id: i64,
    pub version_number: i64,
    pub entry_count: i64,
    pub signature: Vec<u8>,
    pub published_at: DateTime<Utc>,
}

/// Delta between two blocklist versions for sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlocklistDelta {
    pub from_version: i64,
    pub to_version: i64,
    pub added: Vec<BlocklistDeltaEntry>,
    pub removed: Vec<String>,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlocklistDeltaEntry {
    pub domain: String,
    pub category: GamblingCategory,
    pub confidence: f64,
}
```

- [ ] **Step 7: Create organization.rs**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::OrganizationType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: i64,
    pub public_id: Uuid,
    pub name: String,
    pub org_type: OrganizationType,
    pub owner_id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

- [ ] **Step 8: Create partner.rs**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::{PartnerRelationshipStatus, PartnerRole};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartnerRelationship {
    pub id: i64,
    pub public_id: Uuid,
    pub account_id: i64,
    pub partner_account_id: i64,
    pub status: PartnerRelationshipStatus,
    pub role: PartnerRole,
    pub invited_by: i64,
    pub invite_token_hash: Option<String>,
    pub invited_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}
```

- [ ] **Step 9: Run tests**

Run: `cargo test -p bb-common`
Expected: Pass (compile check, no tests yet — serde roundtrip tests come next)

- [ ] **Step 10: Write serde roundtrip tests**

Add to `crates/bb-common/src/models/enrollment.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protection_config_default_has_dns_enabled() {
        let config = ProtectionConfig::default();
        assert!(config.dns_blocking);
        assert!(!config.app_blocking);
        assert!(!config.browser_blocking);
    }

    #[test]
    fn protection_config_roundtrips_json() {
        let config = ProtectionConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let roundtripped: ProtectionConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.dns_blocking, roundtripped.dns_blocking);
        assert_eq!(config.vpn_detection, roundtripped.vpn_detection);
    }

    #[test]
    fn unenrollment_policy_self_tier() {
        let policy = UnenrollmentPolicy {
            policy_type: UnenrollmentPolicyType::TimeDelayed,
            cooldown_hours: Some(48),
            requires_approval_from: None,
        };
        let json = serde_json::to_string(&policy).unwrap();
        assert!(json.contains("time_delayed"));
        assert!(json.contains("48"));
    }
}
```

- [ ] **Step 11: Run tests**

Run: `cargo test -p bb-common`
Expected: 3 tests pass

- [ ] **Step 12: Commit**

```bash
git add crates/bb-common/
git commit -m "feat(common): add domain models for all entities"
```

---

### Task 4: Create bb-proto crate with protobuf definitions

**Files:**
- Create: `crates/bb-proto/Cargo.toml`
- Create: `crates/bb-proto/build.rs`
- Create: `crates/bb-proto/proto/device.proto`
- Create: `crates/bb-proto/proto/heartbeat.proto`
- Create: `crates/bb-proto/proto/blocklist.proto`
- Create: `crates/bb-proto/proto/events.proto`
- Create: `crates/bb-proto/src/lib.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "bb-proto"
version.workspace = true
edition.workspace = true

[dependencies]
prost = { workspace = true }

[build-dependencies]
prost-build = "0.13"

[lints]
workspace = true
```

- [ ] **Step 2: Create build.rs**

```rust
use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(
        &[
            "proto/device.proto",
            "proto/heartbeat.proto",
            "proto/blocklist.proto",
            "proto/events.proto",
        ],
        &["proto/"],
    )?;
    Ok(())
}
```

- [ ] **Step 3: Create proto/device.proto**

```protobuf
syntax = "proto3";
package betblocker.device;

message DeviceRegistrationRequest {
  string enrollment_token = 1;
  bytes public_key = 2;
  DeviceFingerprint fingerprint = 3;
  string agent_version = 4;
}

message DeviceFingerprint {
  string os_type = 1;
  string os_version = 2;
  string hardware_id = 3;
  string hostname = 4;
}

message DeviceRegistrationResponse {
  string device_id = 1;
  bytes device_certificate = 2;
  bytes ca_certificate_chain = 3;
  string initial_blocklist_url = 4;
  uint64 initial_blocklist_version = 5;
  bytes initial_blocklist_signature = 6;
  uint64 certificate_expires_at = 7;
}
```

- [ ] **Step 4: Create proto/heartbeat.proto**

```protobuf
syntax = "proto3";
package betblocker.heartbeat;

message HeartbeatRequest {
  string device_id = 1;
  uint64 sequence_number = 2;
  uint64 timestamp = 3;
  string agent_version = 4;
  string os_version = 5;
  uint64 blocklist_version = 6;
  ProtectionStatus protection_status = 7;
  bytes integrity_hash = 8;
  uint64 uptime_seconds = 9;
  ResourceUsage resource_usage = 10;
  uint32 queued_events = 11;
  uint32 queued_reports = 12;
}

message ProtectionStatus {
  LayerStatus dns_blocking = 1;
  LayerStatus hosts_file = 2;
  LayerStatus app_blocking = 3;
  LayerStatus browser_extension = 4;
  LayerStatus network_hook = 5;
  bool watchdog_alive = 6;
  bool config_integrity_ok = 7;
}

enum LayerStatus {
  ACTIVE = 0;
  DEGRADED = 1;
  INACTIVE = 2;
  FAILED = 3;
}

message ResourceUsage {
  float cpu_percent = 1;
  uint64 memory_bytes = 2;
  uint64 disk_cache_bytes = 3;
}

message HeartbeatResponse {
  bool acknowledged = 1;
  uint64 server_timestamp = 2;
  repeated ServerCommand commands = 3;
}

message ServerCommand {
  string command_type = 1;
  bytes payload = 2;
}
```

- [ ] **Step 5: Create proto/blocklist.proto**

```protobuf
syntax = "proto3";
package betblocker.blocklist;

message BlocklistDeltaRequest {
  uint64 current_version = 1;
}

message BlocklistDeltaResponse {
  uint64 from_version = 1;
  uint64 to_version = 2;
  bool full_sync_required = 3;
  repeated BlocklistAddition additions = 4;
  repeated string removals = 5;
  bytes signature = 6;
}

message BlocklistAddition {
  string domain = 1;
  string category = 2;
  float confidence = 3;
}

message FederatedReport {
  string domain = 1;
  string heuristic_type = 2;
  float confidence = 3;
  uint64 timestamp = 4;
}
```

- [ ] **Step 6: Create proto/events.proto**

```protobuf
syntax = "proto3";
package betblocker.events;

message EventBatch {
  string device_id = 1;
  uint64 batch_sequence = 2;
  repeated EventRecord events = 3;
}

message EventRecord {
  string event_type = 1;
  string category = 2;
  string severity = 3;
  bytes metadata = 4;
  uint64 occurred_at = 5;
}

message EventBatchResponse {
  bool acknowledged = 1;
  uint64 events_accepted = 2;
}
```

- [ ] **Step 7: Create src/lib.rs**

```rust
pub mod device {
    include!(concat!(env!("OUT_DIR"), "/betblocker.device.rs"));
}

pub mod heartbeat {
    include!(concat!(env!("OUT_DIR"), "/betblocker.heartbeat.rs"));
}

pub mod blocklist {
    include!(concat!(env!("OUT_DIR"), "/betblocker.blocklist.rs"));
}

pub mod events {
    include!(concat!(env!("OUT_DIR"), "/betblocker.events.rs"));
}
```

- [ ] **Step 8: Verify compiles**

Run: `cargo check -p bb-proto`
Expected: Success (prost generates code from proto files)

- [ ] **Step 9: Commit**

```bash
git add crates/bb-proto/
git commit -m "feat(proto): add protobuf definitions for agent-API protocol"
```

---

### Task 5: Create remaining crate stubs

**Files:**
- Create: `crates/bb-api/Cargo.toml`, `crates/bb-api/src/main.rs`
- Create: `crates/bb-worker/Cargo.toml`, `crates/bb-worker/src/main.rs`
- Create: `crates/bb-agent-core/Cargo.toml`, `crates/bb-agent-core/src/lib.rs`
- Create: `crates/bb-agent-plugins/Cargo.toml`, `crates/bb-agent-plugins/src/lib.rs`
- Create: `crates/bb-agent-linux/Cargo.toml`, `crates/bb-agent-linux/src/main.rs`
- Create: `crates/bb-cli/Cargo.toml`, `crates/bb-cli/src/main.rs`

- [ ] **Step 1: Create bb-api stub**

`crates/bb-api/Cargo.toml`:
```toml
[package]
name = "bb-api"
version.workspace = true
edition.workspace = true

[dependencies]
bb-common = { path = "../bb-common" }
bb-proto = { path = "../bb-proto" }
axum = { workspace = true }
tokio = { workspace = true }
sqlx = { workspace = true }
tower = { workspace = true }
tower-http = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
jsonwebtoken = { workspace = true }
ring = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
dotenvy = { workspace = true }
config = { workspace = true }

[features]
default = ["hosted"]
hosted = []

[lints]
workspace = true
```

`crates/bb-api/src/main.rs`:
```rust
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("BetBlocker API starting...");
    // TODO: Sub-Plan 2 implements the full API
}
```

- [ ] **Step 2: Create bb-worker stub**

`crates/bb-worker/Cargo.toml`:
```toml
[package]
name = "bb-worker"
version.workspace = true
edition.workspace = true

[dependencies]
bb-common = { path = "../bb-common" }
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }

[lints]
workspace = true
```

`crates/bb-worker/src/main.rs`:
```rust
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("BetBlocker Worker starting...");
    // TODO: Background job processing
}
```

- [ ] **Step 3: Create bb-agent-core stub**

`crates/bb-agent-core/Cargo.toml`:
```toml
[package]
name = "bb-agent-core"
version.workspace = true
edition.workspace = true

[dependencies]
bb-common = { path = "../bb-common" }
bb-proto = { path = "../bb-proto" }
tokio = { workspace = true }
tracing = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }

[lints]
workspace = true
```

`crates/bb-agent-core/src/lib.rs`:
```rust
//! BetBlocker Agent Core — cross-platform blocking engine.
//!
//! This crate contains the platform-independent blocking logic:
//! plugin registry, blocklist matching, event system, and configuration.
```

- [ ] **Step 4: Create bb-agent-plugins stub**

`crates/bb-agent-plugins/Cargo.toml`:
```toml
[package]
name = "bb-agent-plugins"
version.workspace = true
edition.workspace = true

[dependencies]
bb-common = { path = "../bb-common" }
bb-agent-core = { path = "../bb-agent-core" }
thiserror = { workspace = true }
tracing = { workspace = true }

[lints]
workspace = true
```

`crates/bb-agent-plugins/src/lib.rs`:
```rust
//! BetBlocker Agent Plugins — blocking plugin trait definitions and built-in plugins.
```

- [ ] **Step 5: Create bb-agent-linux stub**

`crates/bb-agent-linux/Cargo.toml`:
```toml
[package]
name = "bb-agent-linux"
version.workspace = true
edition.workspace = true

[dependencies]
bb-common = { path = "../bb-common" }
bb-agent-core = { path = "../bb-agent-core" }
bb-agent-plugins = { path = "../bb-agent-plugins" }
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }

[lints]
workspace = true

[target.'cfg(target_os = "linux")'.dependencies]
# Linux-specific deps will go here
```

`crates/bb-agent-linux/src/main.rs`:
```rust
fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("BetBlocker Agent (Linux) starting...");
    // TODO: Sub-Plan 4 implements the full agent
}
```

- [ ] **Step 6: Create bb-cli stub**

`crates/bb-cli/Cargo.toml`:
```toml
[package]
name = "bb-cli"
version.workspace = true
edition.workspace = true

[dependencies]
bb-common = { path = "../bb-common" }
tokio = { workspace = true }
tracing = { workspace = true }

[lints]
workspace = true
```

`crates/bb-cli/src/main.rs`:
```rust
fn main() {
    println!("BetBlocker CLI — admin tool");
    // TODO: CLI commands for admin operations
}
```

- [ ] **Step 7: Verify entire workspace compiles**

Run: `cargo check`
Expected: All crates compile successfully

- [ ] **Step 8: Commit**

```bash
git add crates/
git commit -m "feat: add stub crates for API, worker, agent, and CLI"
```

---

## Chunk 2: Database & Dev Infrastructure

### Task 6: Create docker-compose for dev infrastructure

**Files:**
- Create: `deploy/docker-compose.dev.yml`
- Create: `.env.example`

- [ ] **Step 1: Create docker-compose.dev.yml**

```yaml
services:
  postgres:
    image: timescale/timescaledb:latest-pg16
    ports:
      - "5432:5432"
    environment:
      POSTGRES_DB: betblocker
      POSTGRES_USER: betblocker
      POSTGRES_PASSWORD: betblocker_dev
    volumes:
      - pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U betblocker"]
      interval: 5s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 5s
      retries: 5

volumes:
  pgdata:
```

- [ ] **Step 2: Create .env.example**

```bash
# Database
DATABASE_URL=postgres://betblocker:betblocker_dev@localhost:5432/betblocker

# Redis
REDIS_URL=redis://localhost:6379

# API
API_HOST=0.0.0.0
API_PORT=3000
JWT_SECRET_KEY=GENERATE_ME_WITH_openssl_rand_base64_32
RUST_LOG=bb_api=debug,tower_http=debug

# Billing (hosted only)
BILLING_ENABLED=false
STRIPE_SECRET_KEY=sk_test_...
STRIPE_WEBHOOK_SECRET=whsec_...

# Agent
BETBLOCKER_API_URL=http://localhost:3000
```

- [ ] **Step 3: Verify docker-compose starts**

Run: `docker compose -f deploy/docker-compose.dev.yml up -d`
Expected: PostgreSQL and Redis containers running, healthchecks pass

- [ ] **Step 4: Commit**

```bash
git add deploy/ .env.example
git commit -m "chore: add dev docker-compose with PostgreSQL/TimescaleDB and Redis"
```

---

### Task 7: Create database migrations

**Files:**
- Create: `migrations/V001__create_enum_types.sql` through `migrations/V021__seed_blocklist.sql`

Reference: `docs/architecture/database-schema.md` contains the full DDL. Extract each CREATE TABLE and its indexes into the corresponding numbered migration file.

- [ ] **Step 1: Create V001 — enum types**

`migrations/V001__create_enum_types.sql`:
```sql
-- Enum types for BetBlocker
CREATE TYPE account_role AS ENUM ('user', 'partner', 'authority', 'admin');
CREATE TYPE platform_type AS ENUM ('windows', 'macos', 'linux', 'android', 'ios');
CREATE TYPE device_status AS ENUM ('pending', 'active', 'offline', 'unenrolling', 'unenrolled');
CREATE TYPE enrollment_tier AS ENUM ('self', 'partner', 'authority');
CREATE TYPE enrollment_status AS ENUM ('pending', 'active', 'unenroll_requested', 'unenroll_approved', 'unenrolling', 'unenrolled', 'expired');
CREATE TYPE unenrollment_policy_type AS ENUM ('time_delayed', 'partner_approval', 'authority_approval');
CREATE TYPE unenroll_request_status AS ENUM ('pending', 'approved', 'denied', 'expired', 'cancelled');
CREATE TYPE partner_relationship_status AS ENUM ('pending', 'active', 'revoked');
CREATE TYPE partner_role AS ENUM ('accountability_partner', 'therapist', 'authority_rep');
CREATE TYPE organization_type AS ENUM ('family', 'therapy_practice', 'court_program', 'employer', 'other');
CREATE TYPE org_member_role AS ENUM ('admin', 'member');
CREATE TYPE blocklist_source AS ENUM ('curated', 'automated', 'federated', 'community');
CREATE TYPE blocklist_entry_status AS ENUM ('pending_review', 'active', 'inactive', 'rejected');
CREATE TYPE gambling_category AS ENUM ('online_casino', 'sports_betting', 'poker', 'lottery', 'bingo', 'fantasy_sports', 'crypto_gambling', 'affiliate', 'payment_processor', 'other');
CREATE TYPE event_type AS ENUM ('block', 'bypass_attempt', 'tamper_detected', 'tamper_self_healed', 'vpn_detected', 'enrollment_created', 'enrollment_modified', 'unenroll_requested', 'unenroll_completed', 'heartbeat', 'agent_started', 'agent_updated', 'blocklist_updated');
CREATE TYPE event_category AS ENUM ('dns', 'app', 'browser', 'tamper', 'enrollment', 'heartbeat', 'system');
CREATE TYPE event_severity AS ENUM ('info', 'warning', 'critical');
CREATE TYPE reporting_level AS ENUM ('none', 'aggregated', 'detailed', 'full_audit');
CREATE TYPE vpn_detection_mode AS ENUM ('disabled', 'log', 'alert', 'lockdown');
CREATE TYPE tamper_response AS ENUM ('log', 'alert_user', 'alert_partner', 'alert_authority');
CREATE TYPE subscription_plan AS ENUM ('free', 'standard', 'partner_tier', 'institutional');
CREATE TYPE subscription_status AS ENUM ('trialing', 'active', 'past_due', 'cancelled', 'expired');
CREATE TYPE federated_report_status AS ENUM ('pending', 'promoted', 'rejected', 'duplicate');
```

- [ ] **Step 2: Create V002 — accounts**

`migrations/V002__create_accounts.sql`:
```sql
CREATE TABLE accounts (
    id BIGSERIAL PRIMARY KEY,
    public_id UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    display_name VARCHAR(100) NOT NULL,
    role account_role NOT NULL DEFAULT 'user',
    email_verified BOOLEAN NOT NULL DEFAULT FALSE,
    email_verification_token VARCHAR(255),
    mfa_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    mfa_secret VARCHAR(255),
    timezone VARCHAR(50) NOT NULL DEFAULT 'UTC',
    locale VARCHAR(10) NOT NULL DEFAULT 'en-US',
    organization_id BIGINT, -- FK added after organizations table
    locked_until TIMESTAMPTZ,
    failed_login_attempts INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_accounts_email ON accounts (email);
CREATE INDEX idx_accounts_public_id ON accounts (public_id);
```

- [ ] **Step 3: Create V003 through V018 — remaining tables**

Create each migration file following the DDL in `docs/architecture/database-schema.md`. Key tables:
- V003: `refresh_tokens`
- V004: `partner_relationships`
- V005: `organizations`
- V006: `organization_members` (with FK to organizations)
- V007: `devices`
- V008: `device_certificates`
- V009: `enrollments`
- V010: `enrollment_unenroll_requests`
- V011: `blocklist_entries`
- V012: `blocklist_versions`
- V013: `blocklist_version_entries` (join table)
- V014: `federated_reports`
- V015: `events` (with range partitioning by month)
- V016: `reporting_snapshots`
- V017: `subscriptions`
- V018: `audit_log`

Each migration must include:
- CREATE TABLE with all columns, types, constraints, defaults
- All indexes (including partial indexes)
- Foreign keys with appropriate CASCADE/RESTRICT behavior

- [ ] **Step 4: Create V019 — RLS policies**

`migrations/V019__create_rls_policies.sql`:
```sql
-- Enable RLS on tenant-scoped tables
ALTER TABLE accounts ENABLE ROW LEVEL SECURITY;
ALTER TABLE devices ENABLE ROW LEVEL SECURITY;
ALTER TABLE enrollments ENABLE ROW LEVEL SECURITY;
ALTER TABLE partner_relationships ENABLE ROW LEVEL SECURITY;
ALTER TABLE subscriptions ENABLE ROW LEVEL SECURITY;

-- Application role for API connections
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'bb_api') THEN
        CREATE ROLE bb_api LOGIN;
    END IF;
END
$$;

-- RLS policies: accounts
CREATE POLICY accounts_own_row ON accounts
    FOR ALL TO bb_api
    USING (id = current_setting('app.current_account_id', true)::bigint);

-- RLS policies: devices
CREATE POLICY devices_own_row ON devices
    FOR ALL TO bb_api
    USING (account_id = current_setting('app.current_account_id', true)::bigint);

-- Additional policies for partner visibility will be added as needed
```

- [ ] **Step 5: Create V020 — audit triggers**

`migrations/V020__create_audit_triggers.sql`:
```sql
CREATE OR REPLACE FUNCTION fn_audit_trigger()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO audit_log (table_name, record_id, action, old_values, new_values, actor_id)
    VALUES (
        TG_TABLE_NAME,
        COALESCE(NEW.id, OLD.id),
        TG_OP,
        CASE WHEN TG_OP IN ('UPDATE', 'DELETE') THEN to_jsonb(OLD) END,
        CASE WHEN TG_OP IN ('INSERT', 'UPDATE') THEN to_jsonb(NEW) END,
        current_setting('app.current_account_id', true)::bigint
    );
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

-- Attach to security-critical tables
CREATE TRIGGER trg_audit_accounts AFTER INSERT OR UPDATE OR DELETE ON accounts FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();
CREATE TRIGGER trg_audit_enrollments AFTER INSERT OR UPDATE OR DELETE ON enrollments FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();
CREATE TRIGGER trg_audit_devices AFTER INSERT OR UPDATE OR DELETE ON devices FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();
CREATE TRIGGER trg_audit_unenroll_requests AFTER INSERT OR UPDATE OR DELETE ON enrollment_unenroll_requests FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();
CREATE TRIGGER trg_audit_partner_relationships AFTER INSERT OR UPDATE OR DELETE ON partner_relationships FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();
CREATE TRIGGER trg_audit_blocklist_entries AFTER INSERT OR UPDATE OR DELETE ON blocklist_entries FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();
CREATE TRIGGER trg_audit_device_certificates AFTER INSERT OR UPDATE OR DELETE ON device_certificates FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();
CREATE TRIGGER trg_audit_subscriptions AFTER INSERT OR UPDATE OR DELETE ON subscriptions FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();
```

- [ ] **Step 6: Create V021 — seed blocklist**

`migrations/V021__seed_blocklist.sql`:
```sql
-- Seed with well-known gambling domains
-- Source: public gambling domain blocklists
INSERT INTO blocklist_entries (domain, category, source, confidence, status, tags, created_at, updated_at)
VALUES
    ('bet365.com', 'sports_betting', 'curated', 1.0, 'active', ARRAY['major_operator', 'uk'], NOW(), NOW()),
    ('draftkings.com', 'sports_betting', 'curated', 1.0, 'active', ARRAY['major_operator', 'us'], NOW(), NOW()),
    ('fanduel.com', 'sports_betting', 'curated', 1.0, 'active', ARRAY['major_operator', 'us'], NOW(), NOW()),
    ('pokerstars.com', 'poker', 'curated', 1.0, 'active', ARRAY['major_operator'], NOW(), NOW()),
    ('888casino.com', 'online_casino', 'curated', 1.0, 'active', ARRAY['major_operator'], NOW(), NOW()),
    ('betway.com', 'sports_betting', 'curated', 1.0, 'active', ARRAY['major_operator'], NOW(), NOW()),
    ('williamhill.com', 'sports_betting', 'curated', 1.0, 'active', ARRAY['major_operator', 'uk'], NOW(), NOW()),
    ('bovada.lv', 'online_casino', 'curated', 1.0, 'active', ARRAY['major_operator'], NOW(), NOW()),
    ('betmgm.com', 'sports_betting', 'curated', 1.0, 'active', ARRAY['major_operator', 'us'], NOW(), NOW()),
    ('caesars.com', 'online_casino', 'curated', 1.0, 'active', ARRAY['major_operator', 'us'], NOW(), NOW()),
    ('paddypower.com', 'sports_betting', 'curated', 1.0, 'active', ARRAY['major_operator', 'uk'], NOW(), NOW()),
    ('ladbrokes.com', 'sports_betting', 'curated', 1.0, 'active', ARRAY['major_operator', 'uk'], NOW(), NOW()),
    ('unibet.com', 'sports_betting', 'curated', 1.0, 'active', ARRAY['major_operator'], NOW(), NOW()),
    ('stake.com', 'crypto_gambling', 'curated', 1.0, 'active', ARRAY['crypto', 'major_operator'], NOW(), NOW()),
    ('roobet.com', 'crypto_gambling', 'curated', 1.0, 'active', ARRAY['crypto'], NOW(), NOW());

-- Create initial blocklist version
INSERT INTO blocklist_versions (version_number, entry_count, signature, published_at)
VALUES (1, 15, E'\\x00', NOW());
```

- [ ] **Step 7: Verify migrations run against dev database**

Run:
```bash
docker compose -f deploy/docker-compose.dev.yml up -d
# Use sqlx-cli or psql to run migrations
cargo install sqlx-cli --no-default-features --features postgres
DATABASE_URL=postgres://betblocker:betblocker_dev@localhost:5432/betblocker sqlx migrate run --source migrations
```
Expected: All 21 migrations run successfully

- [ ] **Step 8: Commit**

```bash
git add migrations/
git commit -m "feat(db): add all Phase 1 database migrations (V001-V021)"
```

---

### Task 8: Create justfile and CI skeleton

**Files:**
- Create: `justfile`
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Create justfile**

```just
# BetBlocker development commands

# Start dev infrastructure (PostgreSQL + Redis)
infra:
    docker compose -f deploy/docker-compose.dev.yml up -d

# Stop dev infrastructure
infra-down:
    docker compose -f deploy/docker-compose.dev.yml down

# Run database migrations
migrate:
    sqlx migrate run --source migrations

# Run API server
api:
    cargo run -p bb-api

# Run worker
worker:
    cargo run -p bb-worker

# Run web dev server
web:
    cd web && npm run dev

# Build agent for current OS
agent:
    cargo build -p bb-agent-linux

# Run all tests
test:
    cargo test --workspace

# Run clippy
lint:
    cargo clippy --workspace -- -D warnings

# Format check
fmt-check:
    cargo fmt --all -- --check

# Format
fmt:
    cargo fmt --all

# Full CI check (format + lint + test)
ci: fmt-check lint test
```

- [ ] **Step 2: Create CI workflow**

`.github/workflows/ci.yml`:
```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  check:
    name: Check & Lint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - name: Format check
        run: cargo fmt --all -- --check
      - name: Clippy
        run: cargo clippy --workspace -- -D warnings

  test:
    name: Test
    runs-on: ubuntu-latest
    services:
      postgres:
        image: timescale/timescaledb:latest-pg16
        env:
          POSTGRES_DB: betblocker_test
          POSTGRES_USER: betblocker
          POSTGRES_PASSWORD: betblocker_test
        ports:
          - 5432:5432
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
      redis:
        image: redis:7-alpine
        ports:
          - 6379:6379
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    env:
      DATABASE_URL: postgres://betblocker:betblocker_test@localhost:5432/betblocker_test
      REDIS_URL: redis://localhost:6379
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Install sqlx-cli
        run: cargo install sqlx-cli --no-default-features --features postgres
      - name: Run migrations
        run: sqlx migrate run --source migrations
      - name: Run tests
        run: cargo test --workspace

  build:
    name: Build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Build all crates
        run: cargo build --workspace
```

- [ ] **Step 3: Verify just commands work**

Run: `just ci`
Expected: Format check, clippy, and tests all pass

- [ ] **Step 4: Commit**

```bash
git add justfile .github/
git commit -m "chore: add justfile dev commands and GitHub Actions CI"
```

---

## Foundation Complete Checklist

- [ ] Cargo workspace compiles with all crate stubs
- [ ] `bb-common` has all domain enums and models with serde support
- [ ] `bb-proto` compiles protobuf definitions for all agent-API messages
- [ ] All 21 database migrations run cleanly against PostgreSQL/TimescaleDB
- [ ] Docker compose starts dev infrastructure
- [ ] `just ci` passes (format + lint + test)
- [ ] All code committed with clean history
