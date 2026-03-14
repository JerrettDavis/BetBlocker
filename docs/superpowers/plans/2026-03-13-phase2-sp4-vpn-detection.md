# Phase 2 Sub-Plan 4: VPN/Proxy/Tor Detection

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Detect VPN tunnels, system proxy configurations, and Tor usage that could bypass DNS blocking. Implement Log and Alert response modes. Stub Block/Lockdown modes behind kernel tamper resistance gates.

**Architecture:** New `bypass_detection` module in `bb-agent-core` with platform-abstracted traits. Linux implementations provided; Windows/macOS implementations are stubs (filled in by SP5/SP6). Detection results flow through the existing `EventEmitter` as `VpnDetected` events. Response logic reads `VpnDetectionMode` from `ProtectionConfig`.

**Tech Stack:** Rust, tokio, netlink (Linux), sysinfo, reqwest (Tor exit node fetch)

**Depends on:** Phase 1 complete (event system, comms, config). Block/Lockdown modes depend on SP5/SP6/SP7.

**Reference Docs:**
- Phase 2 Design (Section 7): `docs/plans/2026-03-13-phase2-design.md`
- Master Plan: `docs/superpowers/plans/2026-03-13-phase2-master-plan.md`
- Enums: `crates/bb-common/src/enums.rs` (`VpnDetectionMode`, `TamperResponse`)
- Events: `crates/bb-agent-core/src/events/mod.rs`
- Reporter: `crates/bb-agent-core/src/comms/reporter.rs`

---

## File Structure

```
crates/
  bb-agent-core/src/
    bypass_detection/
      mod.rs                # Module root, BypassDetector orchestrator
      traits.rs             # Platform-abstracted traits: NetworkMonitor, ProxyMonitor, ProcessScanner
      vpn.rs                # VPN tunnel detection (interface + process)
      proxy.rs              # System proxy configuration monitoring
      tor.rs                # Tor process detection + exit node list
      response.rs           # Response logic per VpnDetectionMode
      known_processes.rs    # Static lists of known VPN/Tor process names
  bb-common/src/
    models/
      tor_exit_nodes.rs     # TorExitNodeList model
      bypass_detection.rs   # BypassDetectionResult, VpnInfo, ProxyInfo, TorInfo
  bb-shim-linux/src/
    netlink_monitor.rs      # Linux netlink-based interface monitor
    proxy_monitor.rs        # Linux proxy config reader (env vars, GNOME/KDE)
  bb-api/src/
    routes/tor_exits.rs     # GET /api/v1/tor-exits endpoint
  bb-worker/src/
    tor_exits.rs            # Background job: fetch Tor exit node list from Tor Project
```

---

## Chunk 1: Platform Traits + Detection Models (Tasks 1-3)

### Task 1: Bypass detection models in bb-common

**Crate:** `bb-common`
**File:** `src/models/bypass_detection.rs`

- [ ] **Step 1:** Create `bypass_detection.rs` with detection result types:
  - `BypassDetectionResult { vpn: Option<VpnInfo>, proxy: Option<ProxyInfo>, tor: Option<TorInfo>, detected_at: DateTime<Utc> }`
  - `VpnInfo { interface_name: String, interface_type: VpnInterfaceType, process_name: Option<String> }`
  - `VpnInterfaceType` enum: `Tun, Tap, WireGuard, Unknown`
  - `ProxyInfo { proxy_type: ProxyType, address: String, source: ProxySource }`
  - `ProxyType` enum: `Http, Https, Socks4, Socks5`
  - `ProxySource` enum: `SystemSettings, EnvironmentVariable, BrowserConfig`
  - `TorInfo { process_detected: bool, exit_node_match: bool }`
- [ ] **Step 2:** Add `pub mod bypass_detection;` to `src/models/mod.rs`
- [ ] **Step 3:** Write unit tests for serialization roundtrip of all model types

### Task 2: Tor exit node list model

**Crate:** `bb-common`
**File:** `src/models/tor_exit_nodes.rs`

- [ ] **Step 1:** Create `TorExitNodeList { nodes: HashSet<IpAddr>, fetched_at: DateTime<Utc>, expires_at: DateTime<Utc> }` with `contains(ip)` method
- [ ] **Step 2:** Implement `TorExitNodeList::parse_from_csv(data: &str) -> Result<Self>` (Tor Project bulk exit list format: one IP per line, comment lines start with `#`)
- [ ] **Step 3:** Add `pub mod tor_exit_nodes;` to `src/models/mod.rs`
- [ ] **Step 4:** Write tests: parse valid list, skip comments, handle empty input, `contains` returns correct results

### Task 3: Platform-abstracted detection traits

**Crate:** `bb-agent-core`
**File:** `src/bypass_detection/traits.rs`

- [ ] **Step 1:** Create the `bypass_detection` module directory and `mod.rs` (initially just `pub mod traits;`)
- [ ] **Step 2:** Define traits in `traits.rs`:

```rust
#[async_trait]
pub trait NetworkInterfaceMonitor: Send + Sync {
    /// List current network interfaces that look like VPN tunnels.
    async fn detect_vpn_interfaces(&self) -> Result<Vec<VpnInfo>, BypassDetectionError>;
    /// Subscribe to interface change notifications (returns a receiver).
    async fn watch_interfaces(&self) -> Result<tokio::sync::mpsc::Receiver<VpnInfo>, BypassDetectionError>;
}

#[async_trait]
pub trait ProxyConfigMonitor: Send + Sync {
    /// Read current system proxy configuration.
    async fn detect_proxy_config(&self) -> Result<Option<ProxyInfo>, BypassDetectionError>;
}

#[async_trait]
pub trait ProcessScanner: Send + Sync {
    /// Scan running processes for known VPN/Tor process names.
    async fn scan_for_processes(&self, known_names: &[&str]) -> Result<Vec<String>, BypassDetectionError>;
}
```

- [ ] **Step 3:** Define `BypassDetectionError` enum with variants: `PlatformNotSupported`, `PermissionDenied`, `IoError(std::io::Error)`, `Other(String)`
- [ ] **Step 4:** Add `pub mod bypass_detection;` to `crates/bb-agent-core/src/lib.rs`

---

## Chunk 2: Known Process Lists + VPN Detection (Tasks 4-5)

### Task 4: Known VPN/Tor process name lists

**Crate:** `bb-agent-core`
**File:** `src/bypass_detection/known_processes.rs`

- [ ] **Step 1:** Define `pub const VPN_PROCESS_NAMES: &[&str]` containing known VPN client process names: `openvpn`, `wireguard-go`, `wg-quick`, `nordvpn`, `nordlynx`, `expressvpn`, `expressvpnd`, `mullvad-daemon`, `mullvad-vpn`, `surfshark`, `pia-daemon`, `cyberghost`, `protonvpn`, `windscribe`, `hotspotshield`
- [ ] **Step 2:** Define `pub const VPN_SERVICE_NAMES: &[&str]` for systemd/launchd/SCM service names: `openvpn`, `wg-quick@`, `nordvpnd`, `mullvad-daemon`, `expressvpn`
- [ ] **Step 3:** Define `pub const TOR_PROCESS_NAMES: &[&str]`: `tor`, `tor-browser`, `torbrowser`, `obfs4proxy`, `snowflake-client`
- [ ] **Step 4:** Define `pub const VPN_INTERFACE_PREFIXES: &[&str]`: `tun`, `tap`, `wg`, `utun`, `gpd`, `ppp`, `nordlynx`, `proton`, `mullvad`
- [ ] **Step 5:** Write a test verifying all lists are non-empty and contain no duplicates

### Task 5: VPN detection logic (platform-independent)

**Crate:** `bb-agent-core`
**File:** `src/bypass_detection/vpn.rs`

- [ ] **Step 1:** Create `VpnDetector` struct holding a `Box<dyn NetworkInterfaceMonitor>` and `Box<dyn ProcessScanner>`
- [ ] **Step 2:** Implement `VpnDetector::detect(&self) -> Result<Vec<VpnInfo>>` that calls `detect_vpn_interfaces()` and merges with process scan results (match process name to interface if possible)
- [ ] **Step 3:** Implement `VpnDetector::watch(&self) -> Result<Receiver<VpnInfo>>` that delegates to the monitor's `watch_interfaces()`
- [ ] **Step 4:** Write tests with mock implementations of `NetworkInterfaceMonitor` and `ProcessScanner` -- test: no VPN found, VPN interface found, VPN process found, both found (dedup)
- [ ] **Step 5:** Add `pub mod vpn;` and `pub mod known_processes;` to `bypass_detection/mod.rs`

---

## Chunk 3: Proxy + Tor Detection (Tasks 6-7)

### Task 6: System proxy configuration monitoring

**Crate:** `bb-agent-core`
**File:** `src/bypass_detection/proxy.rs`

- [ ] **Step 1:** Create `ProxyDetector` struct holding `Box<dyn ProxyConfigMonitor>`
- [ ] **Step 2:** Implement `ProxyDetector::detect(&self) -> Result<Option<ProxyInfo>>` that delegates to the monitor
- [ ] **Step 3:** Write tests with mock `ProxyConfigMonitor`: no proxy, HTTP proxy set, SOCKS proxy set
- [ ] **Step 4:** Add `pub mod proxy;` to `bypass_detection/mod.rs`

### Task 7: Tor process detection and exit node checking

**Crate:** `bb-agent-core`
**File:** `src/bypass_detection/tor.rs`

- [ ] **Step 1:** Create `TorDetector` struct holding `Box<dyn ProcessScanner>` and `Arc<RwLock<Option<TorExitNodeList>>>`
- [ ] **Step 2:** Implement `TorDetector::detect(&self) -> Result<TorInfo>` that scans for Tor process names from `TOR_PROCESS_NAMES`
- [ ] **Step 3:** Implement `TorDetector::update_exit_nodes(&self, list: TorExitNodeList)` to refresh the cached list
- [ ] **Step 4:** Implement `TorDetector::is_exit_node(&self, ip: IpAddr) -> bool` to check against cached list
- [ ] **Step 5:** Write tests: no Tor found, Tor process detected, exit node match, stale list handling
- [ ] **Step 6:** Add `pub mod tor;` to `bypass_detection/mod.rs`

---

## Chunk 4: Linux Platform Implementations (Tasks 8-10)

### Task 8: Linux netlink network interface monitor

**Crate:** `bb-agent-core` (behind `#[cfg(target_os = "linux")]`)
**File:** `src/bypass_detection/linux/netlink_monitor.rs`

- [ ] **Step 1:** Create `bypass_detection/linux/` directory with `mod.rs`
- [ ] **Step 2:** Implement `LinuxNetworkMonitor` struct implementing `NetworkInterfaceMonitor`
- [ ] **Step 3:** `detect_vpn_interfaces()`: read `/sys/class/net/` entries, filter by `VPN_INTERFACE_PREFIXES`, return `VpnInfo` for each match with interface type inferred from prefix (`tun*` -> Tun, `tap*` -> Tap, `wg*` -> WireGuard)
- [ ] **Step 4:** `watch_interfaces()`: open a netlink socket (`RTMGRP_LINK`), spawn a task that reads `RTM_NEWLINK` messages and sends matching interface names through the channel
- [ ] **Step 5:** Write integration test (gated behind `#[cfg(target_os = "linux")]`) that calls `detect_vpn_interfaces()` and verifies it returns an empty or valid list

### Task 9: Linux process scanner

**File:** `src/bypass_detection/linux/process_scanner.rs`

- [ ] **Step 1:** Implement `LinuxProcessScanner` struct implementing `ProcessScanner`
- [ ] **Step 2:** `scan_for_processes()`: iterate `/proc/*/comm` files, compare contents against `known_names` (case-insensitive), return matching process names. Use `tokio::fs::read_dir` for async iteration.
- [ ] **Step 3:** Write test with a mock `/proc` directory or use the `sysinfo` crate as fallback. Add `sysinfo` as optional dependency behind `process-scan` feature flag.
- [ ] **Step 4:** Add fallback impl using `sysinfo::System::processes()` for non-Linux platforms (behind `#[cfg(not(target_os = "linux"))]`)

### Task 10: Linux proxy config monitor

**File:** `src/bypass_detection/linux/proxy_monitor.rs`

- [ ] **Step 1:** Implement `LinuxProxyMonitor` struct implementing `ProxyConfigMonitor`
- [ ] **Step 2:** `detect_proxy_config()`: check env vars `http_proxy`, `https_proxy`, `all_proxy`, `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY` in order. Parse URL to extract proxy type and address.
- [ ] **Step 3:** Optionally check GNOME proxy settings via `gsettings` command or dconf read (spawn `gsettings get org.gnome.system.proxy mode`)
- [ ] **Step 4:** Write tests: no env vars set returns None, `http_proxy=http://proxy:8080` returns correct ProxyInfo, SOCKS URL parsed correctly

---

## Chunk 5: VpnDetected Event + Response Logic (Tasks 11-13)

### Task 11: VpnDetected event constructor

**Crate:** `bb-agent-core`
**File:** `src/events/mod.rs`

- [ ] **Step 1:** Add `AgentEvent::vpn_detected()` constructor:

```rust
pub fn vpn_detected(detection_type: &str, details: serde_json::Value) -> Self {
    Self {
        id: None,
        event_type: EventType::VpnDetected,
        category: EventCategory::Tamper,
        severity: EventSeverity::Warning,
        domain: None,
        plugin_id: "bypass_detection".to_string(),
        metadata: serde_json::json!({
            "detection_type": detection_type,
            "details": details,
        }),
        timestamp: Utc::now(),
        reported: false,
    }
}
```

- [ ] **Step 2:** Write test verifying the event has correct `event_type`, `category`, `severity`, and metadata structure

### Task 12: Response logic per VpnDetectionMode

**Crate:** `bb-agent-core`
**File:** `src/bypass_detection/response.rs`

- [ ] **Step 1:** Create `BypassResponseHandler` struct holding `VpnDetectionMode`, `EventEmitterHandle`, and optionally an `Arc<ApiClient>` (for Alert mode notifications)
- [ ] **Step 2:** Implement `handle_detection(&self, result: &BypassDetectionResult) -> Result<()>`:
  - `Disabled`: return immediately, no action
  - `Log`: emit `VpnDetected` event via `EventEmitterHandle`, log at `warn!` level
  - `Alert`: same as Log + call API to trigger partner/authority notification (`POST /api/v1/devices/{id}/alerts`)
  - `Block`: log a warning that Block mode requires kernel protections, fall back to Alert behavior
  - `Lockdown`: log a warning that Lockdown mode requires kernel protections, fall back to Alert behavior
- [ ] **Step 3:** Write tests for each mode using mock event emitter: Disabled emits nothing, Log emits event, Alert emits event (API call tested separately), Block/Lockdown fall back to Alert with warning log

### Task 13: BypassDetector orchestrator

**Crate:** `bb-agent-core`
**File:** `src/bypass_detection/mod.rs`

- [ ] **Step 1:** Create `BypassDetector` struct composing `VpnDetector`, `ProxyDetector`, `TorDetector`, and `BypassResponseHandler`
- [ ] **Step 2:** Implement `BypassDetector::run_scan(&self) -> Result<BypassDetectionResult>` that runs all three detectors and merges results
- [ ] **Step 3:** Implement `BypassDetector::run_periodic(interval: Duration, shutdown: watch::Receiver<bool>)` loop: scan on interval, pass results to `BypassResponseHandler` if any detection is positive
- [ ] **Step 4:** Implement `BypassDetector::run_realtime(shutdown: watch::Receiver<bool>)` that uses `VpnDetector::watch()` for immediate notifications on interface changes, combined with periodic proxy/Tor scans
- [ ] **Step 5:** Constructor `BypassDetector::new(mode: VpnDetectionMode, ...)` that returns `None` if mode is `Disabled`
- [ ] **Step 6:** Write tests: orchestrator with all-clear returns empty result, orchestrator with VPN detected invokes response handler, Disabled mode short-circuits

---

## Chunk 6: Tor Exit Node API + Worker (Tasks 14-16)

### Task 14: Tor exit node list API endpoint

**Crate:** `bb-api`
**File:** `src/routes/tor_exits.rs`

- [ ] **Step 1:** Add `GET /api/v1/tor-exits` endpoint returning the current Tor exit node IP list as JSON: `{ "nodes": ["1.2.3.4", ...], "fetched_at": "...", "expires_at": "..." }`
- [ ] **Step 2:** Store the list in a Redis cache key `tor:exit_nodes` with 24h TTL. Endpoint reads from Redis; returns 503 if cache is empty.
- [ ] **Step 3:** Add route registration in `src/routes/mod.rs`
- [ ] **Step 4:** Write integration test: seed Redis, call endpoint, verify response structure. Test empty cache returns 503.

### Task 15: Tor exit node refresh worker job

**Crate:** `bb-worker`
**File:** `src/tor_exits.rs`

- [ ] **Step 1:** Create `TorExitNodeRefreshJob` struct implementing the worker's job trait
- [ ] **Step 2:** `execute()`: fetch `https://check.torproject.org/torbulkexitlist` via `reqwest`, parse IPs (one per line, skip blanks/comments), store in Redis as serialized `TorExitNodeList`
- [ ] **Step 3:** Schedule to run every 6 hours (configurable). On failure, retry with exponential backoff (1m, 5m, 30m).
- [ ] **Step 4:** Add job registration in `bb-worker`'s job scheduler
- [ ] **Step 5:** Write test with mock HTTP server returning sample exit node data, verify Redis is populated correctly

### Task 16: Agent-side Tor exit node sync

**Crate:** `bb-agent-core`
**File:** `src/bypass_detection/tor.rs` (extend)

- [ ] **Step 1:** Add `TorDetector::sync_exit_nodes(api_client: &ApiClient) -> Result<()>` that fetches from `GET /api/v1/tor-exits` and calls `update_exit_nodes()`
- [ ] **Step 2:** Integrate into `BypassDetector::run_periodic()`: sync exit nodes on startup and every 6 hours
- [ ] **Step 3:** Cache the exit node list to disk (alongside blocklist cache) for offline operation
- [ ] **Step 4:** Write test: mock API returns exit nodes, TorDetector is updated, `is_exit_node()` reflects new data

---

## Chunk 7: Integration + Block/Lockdown Stubs (Tasks 17-19)

### Task 17: Heartbeat integration

**Crate:** `bb-agent-core`
**File:** `src/comms/heartbeat.rs` (extend)

- [ ] **Step 1:** Add `vpn_detected: bool` and `proxy_detected: bool` fields to the heartbeat payload (extend `ProtectionStatus` if not already present)
- [ ] **Step 2:** Before each heartbeat, run a quick `BypassDetector::run_scan()` and include results
- [ ] **Step 3:** Write test verifying heartbeat payload includes VPN/proxy status fields

### Task 18: Block and Lockdown mode stubs

**Crate:** `bb-agent-core`
**File:** `src/bypass_detection/response.rs` (extend)

- [ ] **Step 1:** Define `KernelNetworkControl` trait with methods `block_vpn_interface(name: &str) -> Result<()>` and `lockdown_network() -> Result<()>`
- [ ] **Step 2:** Implement `StubKernelNetworkControl` that returns `Err(BypassDetectionError::PlatformNotSupported)` with a descriptive message about requiring kernel protections
- [ ] **Step 3:** Gate `BypassResponseHandler` Block/Lockdown arms behind `Option<Box<dyn KernelNetworkControl>>` -- if `None`, log warning and fall back to Alert
- [ ] **Step 4:** Write test confirming Block mode without kernel control falls back to Alert behavior

### Task 19: Feature flag and Cargo.toml wiring

**Crate:** `bb-agent-core`
**File:** `Cargo.toml`

- [ ] **Step 1:** Add `bypass-detection` feature flag to `bb-agent-core/Cargo.toml` gating the `bypass_detection` module
- [ ] **Step 2:** Add dependencies: `sysinfo` (optional, behind `process-scan`), `netlink-packet-route` + `netlink-sys` (optional, behind `linux-netlink`, target `cfg(target_os = "linux")`)
- [ ] **Step 3:** Gate `pub mod bypass_detection;` in `lib.rs` behind `#[cfg(feature = "bypass-detection")]`
- [ ] **Step 4:** Enable `bypass-detection` feature in `bb-agent-linux/Cargo.toml`
- [ ] **Step 5:** Verify `cargo check --features bypass-detection` passes. Verify `cargo check` without the feature also passes (module excluded).

---

## Definition of Done

- [ ] All three detection types (VPN, proxy, Tor) have platform-abstracted traits with Linux implementations
- [ ] `BypassDetector` orchestrator runs periodic and realtime detection scans
- [ ] `VpnDetected` events are emitted and flow through the existing event pipeline
- [ ] Log mode: events recorded silently
- [ ] Alert mode: events recorded + API notification triggered
- [ ] Block/Lockdown modes: stub that falls back to Alert with warning
- [ ] Tor exit node list served by API, refreshed by worker, synced by agent
- [ ] Heartbeat payload includes VPN/proxy detection status
- [ ] All code behind `bypass-detection` feature flag
- [ ] All code has unit tests; mocks used for platform-specific interfaces
- [ ] `cargo test --features bypass-detection` passes
