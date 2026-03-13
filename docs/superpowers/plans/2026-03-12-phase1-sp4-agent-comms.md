# Phase 1 Sub-Plan 4: Agent Communication + Linux Platform

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Build agent-API communication (mTLS, heartbeat, blocklist sync, event reporting), tamper resistance (watchdog, integrity checks), and the Linux platform shim.
**Architecture:** ApiClient with rustls mTLS, protobuf serialization, retry/circuit-breaker. Watchdog as mutual supervision. Linux shim via systemd + nftables.
**Tech Stack:** Rust, rustls, reqwest, prost, tokio, systemd
**Depends on:** Sub-Plans 1 (Foundation), 3 (Agent Core)

**Reference Docs:** `docs/architecture/agent-protocol.md`, `docs/architecture/adrs/ADR-005-tamper-resistance-architecture.md`, `docs/architecture/api-spec.md`, `docs/architecture/database-schema.md`

---

## File Structure

New/modified files in `crates/bb-agent-core/src/`: `api_client.rs`, `registration.rs`, `heartbeat.rs`, `blocklist_sync.rs`, `event_reporter.rs`, `watchdog.rs`, `integrity.rs`, `certificate.rs`. In `crates/bb-agent-linux/src/`: `main.rs`, `nftables.rs`, `platform.rs`. In `deploy/linux/`: `betblocker-agent.service`, `install.sh`. In `tests/integration/`: `api_registration_test.rs`, `blocklist_sync_test.rs`.

---

## Chunk 1: API Client + Registration

### Task 1: ApiClient struct with mTLS and retry

**Files:**
- Create: `crates/bb-agent-core/src/api_client.rs`
- Modify: `crates/bb-agent-core/src/lib.rs` (add module)
- Modify: `crates/bb-agent-core/Cargo.toml` (add reqwest, rustls, prost, backoff deps)

- [ ] **Step 1: Add dependencies to bb-agent-core Cargo.toml**

Add `reqwest` (with `rustls-tls` feature, no default features), `rustls`, `prost`, `backoff` (with `tokio` feature), `tokio-rustls`, and `webpki-roots`.

- [ ] **Step 2: Create ApiClient struct and constructor**

```rust
use std::sync::Arc;
use std::time::Duration;

use reqwest::{Certificate, Client, Identity};
use tokio::sync::RwLock;

/// Central HTTP client for all agent-to-API communication.
/// Uses mTLS with certificate pinning and protobuf serialization.
pub struct ApiClient {
    /// Base URL of the BetBlocker API (e.g., "https://api.betblocker.org")
    base_url: String,
    /// reqwest client configured with mTLS identity and pinned CA
    client: Client,
    /// Device ID assigned during registration (None before registration)
    device_id: RwLock<Option<String>>,
    /// Retry configuration
    retry_config: RetryConfig,
}

pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(300),
            backoff_multiplier: 2.0,
        }
    }
}

impl ApiClient {
    /// Create a new ApiClient with mTLS.
    /// `device_identity` is None before initial registration (uses enrollment-only TLS).
    /// After registration, reconstruct with the device certificate.
    pub fn new(
        base_url: String,
        ca_cert_pem: &[u8],
        device_identity: Option<&[u8]>,   // PKCS#12 identity
        retry_config: RetryConfig,
    ) -> Result<Self, ApiClientError> {
        let ca_cert = Certificate::from_pem(ca_cert_pem)
            .map_err(ApiClientError::CertificateError)?;

        let mut builder = Client::builder()
            .use_rustls_tls()
            .tls_built_in_root_certs(false)     // Reject system CAs — pin only ours
            .add_root_certificate(ca_cert)
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10));

        if let Some(identity_bytes) = device_identity {
            let identity = Identity::from_pem(identity_bytes)
                .map_err(ApiClientError::IdentityError)?;
            builder = builder.identity(identity);
        }

        let client = builder.build().map_err(ApiClientError::HttpClientError)?;

        Ok(Self {
            base_url,
            client,
            device_id: RwLock::new(None),
            retry_config,
        })
    }
}
```

- [ ] **Step 3: Implement retry with exponential backoff**

Add `request_with_retry` using `backoff` crate. Retry on 5xx and 429 (respect `Retry-After`). Fail immediately on other 4xx.

- [ ] **Step 4: Implement protobuf request/response helpers**

Add `post_proto<Req: prost::Message, Resp: prost::Message>(&self, path, req) -> Result<Resp>` — serialize to protobuf, set `Content-Type: application/protobuf`, send, deserialize response.

- [ ] **Step 5: Add circuit breaker state**

`CircuitState` enum (Closed, Open, HalfOpen). Open after 3 consecutive failures for 60s. HalfOpen allows one probe.

- [ ] **Step 6: Unit tests**

Test retry with `wiremock`, circuit breaker transitions, protobuf round-trip.

- [ ] **Step 7: Commit**

```bash
git add crates/bb-agent-core/src/api_client.rs crates/bb-agent-core/src/lib.rs crates/bb-agent-core/Cargo.toml
git commit -m "feat(agent): add ApiClient with mTLS, certificate pinning, and retry"
```

### Task 2: Device registration flow

**Files:**
- Create: `crates/bb-agent-core/src/registration.rs`

- [ ] **Step 1: Implement `register_device` function**

Accept enrollment token. Generate Ed25519 keypair via `ring`. Collect device fingerprint (OS, version, `/etc/machine-id`, hostname). Build `DeviceRegistrationRequest` protobuf, send via `ApiClient::post_proto`.

- [ ] **Step 2: Process registration response**

Parse `DeviceRegistrationResponse`. Store certificate and CA chain via `CertificateStore` (Task 3). Set `ApiClient.device_id`. Reconstruct `ApiClient` with mTLS identity for subsequent calls.

- [ ] **Step 3: Implement re-registration for expired certificates**

If certificate expired (offline >90 days), send `POST /api/v1/devices/re-register` with device_id, hardware_id, and expired-key signature. On 410 Gone, enter safe mode.

- [ ] **Step 4: Unit tests**

Test registration happy path, re-registration flow, and invalid/expired token handling with mock API.

- [ ] **Step 5: Commit**

```bash
git add crates/bb-agent-core/src/registration.rs
git commit -m "feat(agent): implement device registration and re-registration flows"
```

### Task 3: Certificate storage and rotation

**Files:**
- Create: `crates/bb-agent-core/src/certificate.rs`

- [ ] **Step 1: Define CertificateStore trait**

Methods: `store_identity(cert_pem, key_pem)`, `load_identity()`, `store_ca_chain(pem)`, `load_ca_chain()`, `certificate_expires_at()`.

- [ ] **Step 2: Implement LinuxCertificateStore**

Store in `/var/lib/betblocker/certs/` (0600 permissions). Encrypt private key at rest via HKDF from `/etc/machine-id` + random salt. Fall back to TPM2 via `tpm2-tss` if available.

- [ ] **Step 3: Implement certificate rotation**

On startup and each heartbeat, check if cert is within 30 days of expiry. If so, generate new keypair, call `POST /api/v1/devices/{id}/rotate-certificate`, verify with test heartbeat, delete old key.

- [ ] **Step 4: Unit tests**

Test store/load round-trip, expiry detection, and rotation trigger at 60 days.

- [ ] **Step 5: Commit**

```bash
git add crates/bb-agent-core/src/certificate.rs
git commit -m "feat(agent): certificate storage with encryption-at-rest and rotation"
```

---

## Chunk 2: Heartbeat + Sync

### Task 4: HeartbeatSender

**Files:**
- Create: `crates/bb-agent-core/src/heartbeat.rs`

- [ ] **Step 1: Implement HeartbeatSender struct and run loop**

```rust
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use tokio::time::{interval, MissedTickBehavior};

use crate::api_client::ApiClient;
use crate::plugin::PluginRegistry;

/// Sends periodic heartbeats to the API and processes server commands.
pub struct HeartbeatSender {
    api_client: Arc<ApiClient>,
    plugin_registry: Arc<PluginRegistry>,
    device_id: String,
    /// Current interval, adjustable by server via next_heartbeat_seconds
    current_interval: Duration,
    /// Tier-based bounds: server cannot push interval outside these
    min_interval: Duration,
    max_interval: Duration,
    /// Monotonically increasing counter for replay detection
    sequence_number: u64,
}

impl HeartbeatSender {
    /// Run the heartbeat loop until shutdown signal is received.
    pub async fn run(&mut self, mut shutdown: watch::Receiver<bool>) {
        let mut ticker = interval(self.current_interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    match self.send_heartbeat().await {
                        Ok(response) => {
                            self.process_response(response);
                            // Server may adjust our interval
                            if response.next_heartbeat_seconds > 0 {
                                let requested = Duration::from_secs(
                                    response.next_heartbeat_seconds
                                );
                                let clamped = requested
                                    .clamp(self.min_interval, self.max_interval);
                                if clamped != self.current_interval {
                                    self.current_interval = clamped;
                                    ticker = interval(clamped);
                                    ticker.set_missed_tick_behavior(
                                        MissedTickBehavior::Delay,
                                    );
                                }
                            }
                            self.sequence_number += 1;
                        }
                        Err(e) => {
                            tracing::warn!("Heartbeat failed: {e}");
                            // Queue for offline batch send (max 1000)
                            self.queue_offline_heartbeat();
                        }
                    }
                }
                _ = shutdown.changed() => {
                    tracing::info!("Heartbeat sender shutting down");
                    break;
                }
            }
        }
    }
}
```

- [ ] **Step 2: Implement `send_heartbeat`**

Collect protection status from `PluginRegistry`. Compute integrity hash (SHA-256 of binary + config). Build `HeartbeatRequest` protobuf, send via `ApiClient::post_proto`.

- [ ] **Step 3: Implement `process_response`**

Handle `HeartbeatAck` variants: OK, CLOCK_DRIFT_WARNING (log), INTEGRITY_MISMATCH (re-check), VERSION_OUTDATED (flag), ENROLLMENT_SUSPENDED (suspend mode). Handle `force_blocklist_sync` and `config_update` fields.

- [ ] **Step 4: Implement offline heartbeat batching**

On failure, queue heartbeats in-memory (VecDeque, max 1000). On reconnection, drain via `POST /api/v1/devices/{id}/heartbeat-batch`.

- [ ] **Step 5: Unit tests**

Test tier intervals, server-adjusted interval clamping, offline queue fill/drain, shutdown signal.

- [ ] **Step 6: Commit**

```bash
git add crates/bb-agent-core/src/heartbeat.rs
git commit -m "feat(agent): heartbeat sender with tier intervals and offline batching"
```

---

### Task 5: BlocklistSyncer

**Files:**
- Create: `crates/bb-agent-core/src/blocklist_sync.rs`

- [ ] **Step 1: Implement BlocklistSyncer struct**

Holds current version (`u64`), API client, blocklist store reference, and server Ed25519 signing public key.

- [ ] **Step 2: Implement delta sync**

Send `BlocklistSyncRequest` with current version. Decompress response payload (zstd). Verify Ed25519 signature over `SHA-256(to_version || delta_payload)`. Apply added/removed/modified entries. Update version.

- [ ] **Step 3: Implement full sync fallback**

If delta fails (version gap, corruption), request full sync with version=0. Replace entire local blocklist. Log fallback.

- [ ] **Step 4: Notify plugins after sync**

Signal PluginRegistry so DNS resolver and HOSTS file plugins reload blocklist data.

- [ ] **Step 5: Unit tests**

Test delta apply (add, remove, modify entries). Test signature verification (valid and invalid). Test full sync fallback trigger. Test zstd decompression.

- [ ] **Step 6: Commit**

```bash
git add crates/bb-agent-core/src/blocklist_sync.rs
git commit -m "feat(agent): blocklist syncer with delta sync and signature verification"
```

---

### Task 6: EventReporter

**Files:**
- Create: `crates/bb-agent-core/src/event_reporter.rs`

- [ ] **Step 1: Implement EventReporter struct**

Reads from local SQLite event store (Sub-Plan 3). Holds enrollment tier for privacy filter level.

- [ ] **Step 2: Implement privacy filter**

Self tier: counts only. Partner tier: aggregated by default, detailed with consent. Authority tier: full detail. Filter applied before serialization.

- [ ] **Step 3: Implement batched send**

Query up to 100 unreported events, apply privacy filter, serialize to `EventBatch` protobuf, send, mark reported on success.

- [ ] **Step 4: Implement periodic run loop**

Run every 5 minutes (configurable), same shutdown-aware `tokio::select!` pattern as HeartbeatSender.

- [ ] **Step 5: Unit tests**

Test privacy filter for each tier. Test batch size limiting. Test retry-on-failure leaves events unmarked.

- [ ] **Step 6: Commit**

```bash
git add crates/bb-agent-core/src/event_reporter.rs
git commit -m "feat(agent): event reporter with privacy filtering and batched upload"
```

---

## Chunk 3: Tamper Resistance

### Task 7: Watchdog

**Files:**
- Create: `crates/bb-agent-core/src/watchdog.rs`

- [ ] **Step 1: Design internal watchdog as a tokio task**

Phase 1: in-process watchdog (tokio task, not separate binary). Monitors health via `tokio::sync::mpsc` channel. Agent sends ping every 5s. After 3 missed pings (15s), log tamper event and trigger recovery.

- [ ] **Step 2: Implement WatchdogMonitor struct**

Fields: `health_rx` (mpsc), `last_ping` timestamp, `recovery_callback`. Run loop ticks every 5s, checks for received pings.

- [ ] **Step 3: Implement agent-side ping sender**

Spawn task sending `WatchdogPing { timestamp, binary_hash, blocklist_version }` every 5s. Watchdog validates binary hash.

- [ ] **Step 4: Implement recovery action**

On missed pings: log Level 2 tamper event, restart failed subsystem. After 3 failed restarts, send high-priority tamper alert via API.

- [ ] **Step 5: Document Phase 2 evolution**

Code comment: Phase 2 promotes watchdog to separate binary (`bb-watchdog`) with Unix domain socket IPC per ADR-005.

- [ ] **Step 6: Unit tests**

Test that missed pings trigger recovery callback. Test that valid pings keep the watchdog happy. Test binary hash mismatch detection.

- [ ] **Step 7: Commit**

```bash
git add crates/bb-agent-core/src/watchdog.rs
git commit -m "feat(agent): in-process watchdog monitor with health ping protocol"
```

---

### Task 8: Binary integrity checker

**Files:**
- Create: `crates/bb-agent-core/src/integrity.rs`

- [ ] **Step 1: Implement binary self-hash**

Read agent binary from `/proc/self/exe`, compute SHA-256 via `ring::digest`, store in memory.

- [ ] **Step 2: Implement expected hash comparison**

Compare against expected hash from enrollment config. On mismatch, log Level 2 tamper event, enter degraded mode.

- [ ] **Step 3: Implement periodic re-check**

Every 30 minutes, re-hash binary from disk. If different from startup hash, log tamper event and alert.

- [ ] **Step 4: Implement config integrity check**

Verify enrollment config Ed25519 signature on every load. On failure, enter safe mode (seed blocklist only, tamper alert).

- [ ] **Step 5: Unit tests**

Test hash computation is deterministic. Test mismatch detection with a tampered binary path. Test config signature validation.

- [ ] **Step 6: Commit**

```bash
git add crates/bb-agent-core/src/integrity.rs
git commit -m "feat(agent): binary and config integrity checking with tamper alerts"
```

---

### Task 9: Config integrity and encrypted storage

**Files:**
- Modify: `crates/bb-agent-core/src/integrity.rs` (add config encryption)

- [ ] **Step 1: Implement encrypted config storage**

AES-256-GCM encryption, key derived via HKDF from `/etc/machine-id` + random salt. Store at `/var/lib/betblocker/config.enc`.

- [ ] **Step 2: Implement config load with integrity check**

Read, decrypt, verify Ed25519 signature. On failure, enter safe mode.

- [ ] **Step 3: Implement config restoration**

On corruption, restore from backup at `config.enc.bak` (written on every successful update). If backup also corrupted, enter safe mode and request re-enrollment.

- [ ] **Step 4: Unit tests**

Test encrypt/decrypt round-trip. Test tamper detection (modify encrypted bytes, verify failure). Test backup restoration.

- [ ] **Step 5: Commit**

```bash
git add crates/bb-agent-core/src/integrity.rs
git commit -m "feat(agent): encrypted config storage with tamper detection and restoration"
```

---

## Chunk 4: Linux Platform

### Task 10: systemd service unit file

**Files:**
- Create: `deploy/linux/betblocker-agent.service`

- [ ] **Step 1: Create the systemd unit file**

```ini
[Unit]
Description=BetBlocker Agent — gambling site blocking service
Documentation=https://betblocker.org/docs
After=network-online.target nss-lookup.target
Wants=network-online.target

[Service]
Type=notify
ExecStart=/usr/lib/betblocker/bb-agent-linux
ExecReload=/bin/kill -HUP $MAINPID

# Restart policy: always restart, escalating delay
Restart=always
RestartSec=5
StartLimitIntervalSec=300
StartLimitBurst=10

# Security hardening
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
PrivateDevices=true
NoNewPrivileges=true
ReadWritePaths=/var/lib/betblocker /var/log/betblocker
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
RestrictSUIDSGID=true
MemoryDenyWriteExecute=true

# Resource limits
MemoryMax=128M
CPUQuota=10%

# Capabilities: only what we need for DNS redirect via nftables
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=betblocker-agent

[Install]
WantedBy=multi-user.target
```

- [ ] **Step 2: Commit**

```bash
git add deploy/linux/betblocker-agent.service
git commit -m "feat(linux): add systemd service unit with security hardening"
```

---

### Task 11: nftables DNS redirect rules

**Files:**
- Create: `crates/bb-agent-linux/src/nftables.rs`

- [ ] **Step 1: Implement NftablesManager struct**

Manages nftables rules redirecting DNS (UDP/TCP 53) to local resolver. Stores resolver listen port (default: 5353).

- [ ] **Step 2: Implement `install_rules`**

Create `betblocker` nftables table with output chain redirecting DNS to `127.0.0.1:{port}`. Exclude agent's own queries (UID match) to avoid loops.

- [ ] **Step 3: Implement `remove_rules`**

`nft delete table inet betblocker` on deactivation or uninstall.

- [ ] **Step 4: Implement rule verification**

Check rules every 30s. Re-install if removed externally, log Level 1 tamper event.

- [ ] **Step 5: Unit tests**

Test nft command string generation. Root-required integration tests gated behind `#[cfg(test_nftables)]`.

- [ ] **Step 6: Commit**

```bash
git add crates/bb-agent-linux/src/nftables.rs
git commit -m "feat(linux): nftables DNS redirect rules with tamper monitoring"
```

---

### Task 12: Agent binary entrypoint

**Files:**
- Modify: `crates/bb-agent-linux/src/main.rs`

- [ ] **Step 1: Implement main function with tracing**

Init `tracing-subscriber` with journald. Parse CLI args (`--config-dir`, `--enroll <token>`). Load/create `/var/lib/betblocker/`.

- [ ] **Step 2: Implement signal handling**

SIGTERM (shutdown) and SIGHUP (reload) via `tokio::signal::unix`. Shutdown broadcasts via `watch::Sender<bool>`.

- [ ] **Step 3: Implement startup orchestration**

Sequence: verify config integrity, init ApiClient, register if needed, start PluginRegistry, install nftables, spawn HeartbeatSender/BlocklistSyncer/EventReporter/Watchdog, call `sd_notify` ready.

- [ ] **Step 4: Implement graceful shutdown**

Send final heartbeat, deactivate plugins (reverse order), remove nftables rules, flush events, exit 0.

- [ ] **Step 5: Unit tests**

Test startup orchestration with mocked subsystems. Test graceful shutdown ordering.

- [ ] **Step 6: Commit**

```bash
git add crates/bb-agent-linux/src/main.rs
git commit -m "feat(linux): agent entrypoint with signal handling and graceful shutdown"
```

---

### Task 13: Installation script

**Files:**
- Create: `deploy/linux/install.sh`

- [ ] **Step 1: Write install.sh**

Check root, copy binary to `/usr/lib/betblocker/` (root:root 755, `chattr +i`), create data/log dirs, install systemd unit, daemon-reload, enable+start, verify healthy within 10s.

- [ ] **Step 2: Write uninstall.sh**

Stop, disable, remove unit, `chattr -i`, delete binary and data dirs. Requires `--confirm` flag.

- [ ] **Step 3: Test on a clean Ubuntu 22.04 VM**

Manual verification. Document expected output in script header comment.

- [ ] **Step 4: Commit**

```bash
git add deploy/linux/install.sh deploy/linux/uninstall.sh
git commit -m "feat(linux): install and uninstall scripts for systemd deployment"
```

---

## Chunk 5: Integration Tests

### Task 14: Registration + heartbeat integration test

**Files:**
- Create: `tests/integration/api_registration_test.rs`

- [ ] **Step 1: Create test harness**

`sqlx::PgPool` with test DB. Spawn real `bb-api` on random port with test CA certificate.

- [ ] **Step 2: Test full registration flow**

Create enrollment, generate token, register device. Assert: device_id returned, certificate valid, device in DB.

- [ ] **Step 3: Test heartbeat round-trip**

Reconstruct `ApiClient` with mTLS cert. Send heartbeat. Assert: 200, recorded in DB, valid ack.

- [ ] **Step 4: Test re-registration after expiry**

Expire cert in DB. Heartbeat fails. Re-register. Assert: new cert issued, heartbeat succeeds.

- [ ] **Step 5: Commit**

```bash
git add tests/integration/api_registration_test.rs
git commit -m "test: integration tests for device registration and heartbeat"
```

---

### Task 15: Blocklist sync integration test

**Files:**
- Create: `tests/integration/blocklist_sync_test.rs`

- [ ] **Step 1: Seed test blocklist**

Insert 100 entries at version 1. Generate Ed25519 signing keypair for test API.

- [ ] **Step 2: Test initial full sync**

Sync with version=0. Assert: 100 entries received, signature valid, local version=1.

- [ ] **Step 3: Test delta sync**

Add 5, remove 2 on API (version 2). Sync from version=1. Assert: delta applied, local version=2, 103 entries.

- [ ] **Step 4: Test invalid signature rejection**

Tamper with payload post-signing. Assert: rejected, blocklist unchanged, error logged.

- [ ] **Step 5: Commit**

```bash
git add tests/integration/blocklist_sync_test.rs
git commit -m "test: integration tests for blocklist delta and full sync"
```

---

## Summary

| Chunk | Tasks | Key Deliverables |
|-------|-------|-----------------|
| 1: API Client + Registration | 1-3 | `ApiClient` with mTLS, device registration, cert storage/rotation |
| 2: Heartbeat + Sync | 4-6 | Heartbeat loop, blocklist delta sync, event reporter |
| 3: Tamper Resistance | 7-9 | In-process watchdog, binary/config integrity checks |
| 4: Linux Platform | 10-13 | systemd unit, nftables rules, entrypoint, install script |
| 5: Integration Tests | 14-15 | End-to-end registration, heartbeat, and blocklist sync tests |

**Total steps:** ~70 | **Order:** Chunks 1-3 sequential. Chunk 4 starts after Chunk 1. Chunk 5 requires Chunks 1-2 + API from Sub-Plan 2.
