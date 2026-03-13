# Phase 1 Sub-Plan 3: Agent Core

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Build the cross-platform blocking engine: plugin system, blocklist matcher, DNS resolver, HOSTS file plugin, event system, and config manager.
**Architecture:** bb-agent-core (engine) + bb-agent-plugins (trait defs + built-in plugins). Plugin registry with static dispatch. Blocklist matching via HashSet for O(1) domain lookup.
**Tech Stack:** Rust, hickory-dns, rusqlite, serde, tokio
**Depends on:** Sub-Plan 1 (Foundation)

**Reference Docs:**
- ADR-002: `docs/architecture/adrs/ADR-002-plugin-architecture-for-blocking-layers.md`
- Agent Protocol: `docs/architecture/agent-protocol.md`
- Repo Structure: `docs/architecture/repo-structure.md`
- SP1 Foundation: `docs/superpowers/plans/2026-03-12-phase1-sp1-foundation.md`

---

## File Structure

```
crates/
  bb-agent-plugins/
    Cargo.toml
    src/
      lib.rs                    # Re-exports
      traits.rs                 # BlockingPlugin, DnsBlockingPlugin, etc.
      types.rs                  # PluginConfig, PluginHealth, BlockDecision, PluginError
      registry.rs               # PluginInstance enum + PluginRegistry
      dns_resolver/
        mod.rs                  # DnsResolverPlugin
        handler.rs              # DNS query handler (hickory-dns)
      hosts_file/
        mod.rs                  # HostsFilePlugin
        platform.rs             # Platform-specific HOSTS file paths
  bb-agent-core/
    Cargo.toml
    src/
      lib.rs                    # Re-exports
      blocklist/
        mod.rs                  # Blocklist struct + matching
        cache.rs                # BlocklistCache (file persistence)
      events/
        mod.rs                  # EventEmitter
        privacy.rs              # Privacy filter
        store.rs                # SQLite event store
      config/
        mod.rs                  # AgentConfig
```

---

## Chunk 1: Plugin System

### Task 1: Plugin traits and supporting types

**Crate:** `bb-agent-plugins`
**Files:** `src/traits.rs`, `src/types.rs`

- [ ] **Step 1: Add dependencies to bb-agent-plugins/Cargo.toml**

```toml
[package]
name = "bb-agent-plugins"
version.workspace = true
edition.workspace = true

[dependencies]
bb-common = { path = "../bb-common" }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
chrono = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
hickory-server = "0.25"
hickory-resolver = "0.25"
rusqlite = { version = "0.32", features = ["bundled"] }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
tempfile = "3"
```

- [ ] **Step 2: Create plugin types in `src/types.rs`**

```rust
use std::collections::HashMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Configuration passed to a plugin during init.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Plugin-specific key-value settings.
    pub settings: HashMap<String, serde_json::Value>,
    /// Whether this plugin is enabled (can be toggled by enrollment policy).
    pub enabled: bool,
    /// Priority relative to other plugins in the same layer (lower = checked first).
    pub priority: u32,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            settings: HashMap::new(),
            enabled: true,
            priority: 100,
        }
    }
}

/// Health status returned by plugin health checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHealth {
    pub healthy: bool,
    pub message: String,
    pub checked_at: DateTime<Utc>,
    /// Optional details for diagnostics (not sent to API).
    pub details: HashMap<String, String>,
}

impl PluginHealth {
    pub fn ok() -> Self {
        Self {
            healthy: true,
            message: "OK".into(),
            checked_at: Utc::now(),
            details: HashMap::new(),
        }
    }

    pub fn degraded(message: impl Into<String>) -> Self {
        Self {
            healthy: false,
            message: message.into(),
            checked_at: Utc::now(),
            details: HashMap::new(),
        }
    }
}

/// The blocking layer a plugin belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockingLayer {
    Dns,
    App,
    Browser,
}

/// Result of a blocking check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockDecision {
    /// Domain/app is not in the blocklist -- allow through.
    Allow,
    /// Domain/app is blocked. `reason` is a human-readable string for logging.
    Block { reason: String },
    /// Plugin cannot determine -- defer to next plugin.
    Abstain,
}

impl BlockDecision {
    pub fn is_blocked(&self) -> bool {
        matches!(self, BlockDecision::Block { .. })
    }
}

impl fmt::Display for BlockDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockDecision::Allow => write!(f, "Allow"),
            BlockDecision::Block { reason } => write!(f, "Block({reason})"),
            BlockDecision::Abstain => write!(f, "Abstain"),
        }
    }
}

/// Errors returned by plugin operations.
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("Plugin initialization failed: {0}")]
    InitFailed(String),

    #[error("Activation failed: {0}")]
    ActivationFailed(String),

    #[error("Plugin is not healthy: {0}")]
    Unhealthy(String),

    #[error("OS prerequisite missing: {0}")]
    PrerequisiteMissing(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Identifier for an application (used by AppBlockingPlugin).
/// Placeholder for Phase 2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppIdentifier {
    pub package_name: Option<String>,
    pub executable_path: Option<String>,
    pub display_name: Option<String>,
}

/// Match result from app scanning. Placeholder for Phase 2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMatch {
    pub app_id: AppIdentifier,
    pub confidence: f64,
    pub reason: String,
}

/// Content blocking rules for browser extensions. Placeholder for Phase 3.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentRules {
    pub rules_json: String,
    pub generated_at: DateTime<Utc>,
}

/// Browser extension health. Placeholder for Phase 3.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionHealth {
    pub installed: bool,
    pub version: Option<String>,
    pub integrity_ok: bool,
}
```

- [ ] **Step 3: Create plugin traits in `src/traits.rs`**

These traits are from ADR-002. The `Blocklist` type referenced here is defined in Chunk 2 (Task 4).

```rust
use crate::types::{
    AppIdentifier, AppMatch, BlockDecision, ContentRules, ExtensionHealth,
    PluginConfig, PluginError, PluginHealth, BlockingLayer,
};
use crate::blocklist::Blocklist;

/// Base trait for all blocking plugins. Handles lifecycle management.
pub trait BlockingPlugin: Send + Sync + 'static {
    /// Unique identifier, e.g., "dns.resolver", "dns.hosts".
    fn id(&self) -> &str;

    /// Human-readable name for logging and status reporting.
    fn name(&self) -> &str;

    /// Which blocking layer this plugin belongs to.
    fn layer(&self) -> BlockingLayer;

    /// Initialize with configuration. Called once at agent startup.
    /// May fail if OS prerequisites are missing.
    fn init(&mut self, config: &PluginConfig) -> Result<(), PluginError>;

    /// Activate blocking. Called after init, once the blocklist is loaded.
    fn activate(&mut self, blocklist: &Blocklist) -> Result<(), PluginError>;

    /// Deactivate blocking. Called on graceful shutdown or plugin hot-reload.
    fn deactivate(&mut self) -> Result<(), PluginError>;

    /// Receive an updated blocklist after a delta sync completes.
    fn update_blocklist(&mut self, blocklist: &Blocklist) -> Result<(), PluginError>;

    /// Health check, called periodically by the watchdog.
    fn health_check(&self) -> Result<PluginHealth, PluginError>;
}

/// DNS/Network layer plugins implement this in addition to BlockingPlugin.
pub trait DnsBlockingPlugin: BlockingPlugin {
    /// Check if a domain should be blocked.
    /// Must be extremely fast (sub-microsecond for cache hit).
    fn check_domain(&self, domain: &str) -> BlockDecision;

    /// Handle a raw DNS query packet. For plugins that operate at the
    /// packet level (WFP, VpnService). Returns None if this plugin
    /// does not handle raw packets.
    fn handle_dns_query(&self, query: &[u8]) -> Option<Vec<u8>>;
}

/// Application layer plugins (Phase 2).
pub trait AppBlockingPlugin: BlockingPlugin {
    /// Check if an application should be blocked.
    fn check_app(&self, app_id: &AppIdentifier) -> BlockDecision;

    /// Scan installed applications and return matches.
    fn scan_installed(&self) -> Vec<AppMatch>;

    /// Start monitoring for new app installations.
    fn watch_installs(&mut self) -> Result<(), PluginError>;
}

/// Browser/Content layer plugins (Phase 3).
pub trait ContentBlockingPlugin: BlockingPlugin {
    /// Generate content blocking rules for browser extensions.
    fn generate_rules(&self, blocklist: &Blocklist) -> ContentRules;

    /// Check browser extension presence and integrity.
    fn check_extension_health(&self) -> ExtensionHealth;
}
```

- [ ] **Step 4: Wire up `src/lib.rs` to re-export traits and types**

Re-export `traits::*` and `types::*` as the public API of `bb-agent-plugins`. Also forward-declare the `blocklist` module (which lives in `bb-agent-core` but is re-exported through a path dependency).

---

### Task 2: PluginInstance enum and PluginRegistry

**Files:** `src/registry.rs`

- [ ] **Step 1: Create PluginInstance enum with conditional compilation**

```rust
use crate::types::{
    AppIdentifier, BlockDecision, BlockingLayer, PluginConfig, PluginError, PluginHealth,
};
use crate::traits::{BlockingPlugin, DnsBlockingPlugin};
use crate::blocklist::Blocklist;

#[cfg(feature = "dns-resolver")]
use crate::dns_resolver::DnsResolverPlugin;

#[cfg(feature = "dns-hosts")]
use crate::hosts_file::HostsFilePlugin;

/// Enum dispatch over all compiled-in plugins.
/// Each variant is conditionally compiled via feature flags.
pub enum PluginInstance {
    #[cfg(feature = "dns-resolver")]
    DnsResolver(DnsResolverPlugin),

    #[cfg(feature = "dns-hosts")]
    DnsHosts(HostsFilePlugin),
    // Phase 2+: AppProcess, BrowserExtension, etc.
}

/// Macro to dispatch BlockingPlugin methods across all PluginInstance variants.
/// Avoids manually writing match arms for every method.
macro_rules! dispatch_blocking {
    ($self:expr, $method:ident $(, $arg:expr)*) => {
        match $self {
            #[cfg(feature = "dns-resolver")]
            PluginInstance::DnsResolver(p) => p.$method($($arg),*),
            #[cfg(feature = "dns-hosts")]
            PluginInstance::DnsHosts(p) => p.$method($($arg),*),
        }
    };
}

impl PluginInstance {
    pub fn id(&self) -> &str {
        dispatch_blocking!(self, id)
    }

    pub fn name(&self) -> &str {
        dispatch_blocking!(self, name)
    }

    pub fn layer(&self) -> BlockingLayer {
        dispatch_blocking!(self, layer)
    }

    pub fn init(&mut self, config: &PluginConfig) -> Result<(), PluginError> {
        dispatch_blocking!(self, init, config)
    }

    pub fn activate(&mut self, blocklist: &Blocklist) -> Result<(), PluginError> {
        dispatch_blocking!(self, activate, blocklist)
    }

    pub fn deactivate(&mut self) -> Result<(), PluginError> {
        dispatch_blocking!(self, deactivate)
    }

    pub fn update_blocklist(&mut self, blocklist: &Blocklist) -> Result<(), PluginError> {
        dispatch_blocking!(self, update_blocklist, blocklist)
    }

    pub fn health_check(&self) -> Result<PluginHealth, PluginError> {
        dispatch_blocking!(self, health_check)
    }

    /// Returns true if this plugin is a DNS-layer plugin.
    pub fn is_dns_plugin(&self) -> bool {
        self.layer() == BlockingLayer::Dns
    }

    /// Check domain against this plugin, if it supports DNS blocking.
    /// Returns Abstain for non-DNS plugins.
    pub fn check_domain(&self, domain: &str) -> BlockDecision {
        match self {
            #[cfg(feature = "dns-resolver")]
            PluginInstance::DnsResolver(p) => p.check_domain(domain),
            #[cfg(feature = "dns-hosts")]
            PluginInstance::DnsHosts(p) => p.check_domain(domain),
        }
    }
}
```

- [ ] **Step 2: Create PluginRegistry with short-circuit domain checking**

```rust
use tracing::{info, warn, error};

pub struct PluginRegistry {
    plugins: Vec<PluginInstance>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Register a plugin instance.
    pub fn register(&mut self, plugin: PluginInstance) {
        info!(plugin_id = plugin.id(), "Registered plugin");
        self.plugins.push(plugin);
    }

    /// Build the default registry with all compiled-in plugins.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        #[cfg(feature = "dns-resolver")]
        registry.register(PluginInstance::DnsResolver(
            DnsResolverPlugin::new(),
        ));

        #[cfg(feature = "dns-hosts")]
        registry.register(PluginInstance::DnsHosts(
            HostsFilePlugin::new(),
        ));

        registry
    }

    /// Initialize all plugins. Returns errors for plugins that failed init.
    /// Failed plugins are removed from the registry.
    pub fn init_all(&mut self, config: &PluginConfig, blocklist: &Blocklist) -> Vec<PluginError> {
        let mut errors = Vec::new();
        let mut i = 0;
        while i < self.plugins.len() {
            let plugin = &mut self.plugins[i];
            let id = plugin.id().to_string();

            match plugin.init(config) {
                Ok(()) => {
                    info!(plugin_id = %id, "Plugin initialized");
                    match plugin.activate(blocklist) {
                        Ok(()) => {
                            info!(plugin_id = %id, "Plugin activated");
                            i += 1;
                        }
                        Err(e) => {
                            error!(plugin_id = %id, error = %e, "Plugin activation failed, removing");
                            errors.push(e);
                            self.plugins.remove(i);
                        }
                    }
                }
                Err(e) => {
                    error!(plugin_id = %id, error = %e, "Plugin init failed, removing");
                    errors.push(e);
                    self.plugins.remove(i);
                }
            }
        }
        errors
    }

    /// Query all DNS plugins for a domain.
    /// Short-circuits on first Block decision for performance.
    pub fn check_domain(&self, domain: &str) -> BlockDecision {
        for plugin in &self.plugins {
            if !plugin.is_dns_plugin() {
                continue;
            }
            let decision = plugin.check_domain(domain);
            if decision.is_blocked() {
                return decision;
            }
        }
        BlockDecision::Allow
    }

    /// Run health checks on all plugins. Returns (plugin_id, error) for failures.
    pub fn health_check_all(&self) -> Vec<(String, PluginError)> {
        let mut failures = Vec::new();
        for plugin in &self.plugins {
            match plugin.health_check() {
                Ok(health) if !health.healthy => {
                    failures.push((
                        plugin.id().to_string(),
                        PluginError::Unhealthy(health.message),
                    ));
                }
                Err(e) => {
                    failures.push((plugin.id().to_string(), e));
                }
                Ok(_) => {}
            }
        }
        failures
    }

    /// Push updated blocklist to all active plugins.
    pub fn update_blocklist_all(&mut self, blocklist: &Blocklist) -> Vec<PluginError> {
        let mut errors = Vec::new();
        for plugin in &mut self.plugins {
            if let Err(e) = plugin.update_blocklist(blocklist) {
                warn!(plugin_id = plugin.id(), error = %e, "Blocklist update failed");
                errors.push(e);
            }
        }
        errors
    }

    /// Gracefully deactivate all plugins (shutdown path).
    pub fn deactivate_all(&mut self) -> Vec<PluginError> {
        let mut errors = Vec::new();
        for plugin in &mut self.plugins {
            if let Err(e) = plugin.deactivate() {
                warn!(plugin_id = plugin.id(), error = %e, "Plugin deactivation failed");
                errors.push(e);
            }
        }
        errors
    }

    /// Number of active plugins.
    pub fn active_count(&self) -> usize {
        self.plugins.len()
    }
}
```

- [ ] **Step 3: Add feature flags to bb-agent-plugins/Cargo.toml**

Add the feature flag section:

```toml
[features]
default = ["dns-resolver", "dns-hosts"]
dns-resolver = []
dns-hosts = []
```

---

### Task 3: Plugin lifecycle tests

**Files:** `src/registry.rs` (test module)

- [ ] **Step 1: Create a MockPlugin for testing**

Implement `BlockingPlugin` and `DnsBlockingPlugin` on a `MockPlugin` struct that tracks calls (init_called, activate_called, deactivate_called) and returns configurable results. Store call counts in the struct fields.

- [ ] **Step 2: Test plugin lifecycle: init -> activate -> deactivate**

Assert that init is called before activate, that deactivate can be called, and that the registry correctly manages the lifecycle sequence.

- [ ] **Step 3: Test health check aggregation**

Create a registry with two mock plugins, one healthy and one unhealthy. Verify `health_check_all()` returns exactly one failure with the correct plugin ID.

- [ ] **Step 4: Test check_domain short-circuit behavior**

Register two DNS mock plugins. First plugin returns `Block`. Verify the second plugin's `check_domain` is never called (use a call counter). Verify registry returns `Block`.

- [ ] **Step 5: Test failed init removes plugin from registry**

Register a mock plugin whose `init()` returns `Err`. Call `init_all()`. Verify `active_count()` is 0 and the error is collected.

---

## Chunk 2: Blocklist Engine

### Task 4: Blocklist struct and matching

**Crate:** `bb-agent-core`
**Files:** `src/blocklist/mod.rs`

- [ ] **Step 1: Add dependencies to bb-agent-core/Cargo.toml**

```toml
[package]
name = "bb-agent-core"
version.workspace = true
edition.workspace = true

[dependencies]
bb-common = { path = "../bb-common" }
bb-agent-plugins = { path = "../bb-agent-plugins" }
serde = { workspace = true }
serde_json = { workspace = true }
toml = "0.8"
thiserror = { workspace = true }
chrono = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
rusqlite = { version = "0.32", features = ["bundled"] }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
tempfile = "3"
```

- [ ] **Step 2: Implement Blocklist with HashSet matching**

```rust
use std::collections::HashSet;

/// The in-memory blocklist used by all plugins for domain checks.
/// Exact domains go in a HashSet for O(1) lookup.
/// Wildcard patterns (e.g., *.bet365.com) are stored separately.
#[derive(Debug, Clone)]
pub struct Blocklist {
    /// Exact domain matches (lowercase, no trailing dot).
    exact: HashSet<String>,
    /// Wildcard patterns stored as suffix strings.
    /// e.g., "*.bet365.com" is stored as ".bet365.com"
    /// so we can check if a domain ends with the suffix.
    wildcard_suffixes: Vec<String>,
    /// Blocklist version from the API (for delta sync).
    pub version: i64,
}

impl Blocklist {
    pub fn new(version: i64) -> Self {
        Self {
            exact: HashSet::new(),
            wildcard_suffixes: Vec::new(),
            version,
        }
    }

    /// Load from a newline-delimited file of domains.
    /// Lines starting with `*.` are treated as wildcard patterns.
    /// Empty lines and lines starting with `#` are skipped.
    pub fn from_file(path: &std::path::Path, version: i64) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::from_str(&content, version))
    }

    /// Parse from a newline-delimited string of domains.
    pub fn from_str(content: &str, version: i64) -> Self {
        let mut blocklist = Self::new(version);
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            blocklist.add_entry(line);
        }
        blocklist
    }

    /// Add a single domain or wildcard pattern.
    pub fn add_entry(&mut self, entry: &str) {
        let entry = entry.to_lowercase();
        let entry = entry.trim_end_matches('.');

        if let Some(suffix) = entry.strip_prefix("*.") {
            // Wildcard: store as ".suffix" for endsWith matching
            self.wildcard_suffixes.push(format!(".{suffix}"));
        } else {
            self.exact.insert(entry.to_string());
        }
    }

    /// Remove a domain or wildcard pattern.
    pub fn remove_entry(&mut self, entry: &str) {
        let entry = entry.to_lowercase();
        let entry = entry.trim_end_matches('.');

        if let Some(suffix) = entry.strip_prefix("*.") {
            let needle = format!(".{suffix}");
            self.wildcard_suffixes.retain(|s| s != &needle);
        } else {
            self.exact.remove(entry);
        }
    }

    /// Check if a domain is blocked.
    /// Checks exact match first, then walks parent domains,
    /// then checks wildcard suffixes.
    pub fn is_blocked(&self, domain: &str) -> bool {
        let domain = domain.to_lowercase();
        let domain = domain.trim_end_matches('.');

        // 1. Exact match on the full domain
        if self.exact.contains(domain) {
            return true;
        }

        // 2. Walk parent domains: sub.bet365.com -> bet365.com -> com
        //    This ensures sub.bet365.com is blocked when bet365.com is in the list.
        let mut remaining = domain;
        while let Some(pos) = remaining.find('.') {
            remaining = &remaining[pos + 1..];
            if self.exact.contains(remaining) {
                return true;
            }
        }

        // 3. Wildcard suffix matching
        for suffix in &self.wildcard_suffixes {
            if domain.ends_with(suffix.as_str()) {
                return true;
            }
        }

        false
    }

    /// Number of entries (exact + wildcard).
    pub fn len(&self) -> usize {
        self.exact.len() + self.wildcard_suffixes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.exact.is_empty() && self.wildcard_suffixes.is_empty()
    }
}
```

- [ ] **Step 3: Re-export Blocklist from bb-agent-core's `lib.rs`**

Make `Blocklist` available as `bb_agent_core::blocklist::Blocklist`. Also add a path dependency from `bb-agent-plugins` to `bb-agent-core` for the `Blocklist` type, or alternatively define `Blocklist` in `bb-agent-plugins` directly to avoid a circular dependency. The simpler approach: define `Blocklist` in `bb-agent-plugins` since all plugins need it, and re-export from `bb-agent-core`.

---

### Task 5: BlocklistCache -- file persistence

**Files:** `src/blocklist/cache.rs`

- [ ] **Step 1: Implement BlocklistCache**

Create a `BlocklistCache` struct that wraps a file path. Provide methods:
- `save(blocklist: &Blocklist)` -- serialize the blocklist to a newline-delimited file with a version header line (`# version: <N>`). Write to a temp file first, then atomically rename for crash safety.
- `load() -> Result<Blocklist>` -- read the cache file, parse the version header, call `Blocklist::from_str`.
- `exists() -> bool` -- check if the cache file is present.

- [ ] **Step 2: Implement delta application**

Add a method `apply_delta(blocklist: &mut Blocklist, added: &[String], removed: &[String], new_version: i64)` that calls `add_entry` for each added domain, `remove_entry` for each removed domain, and updates the version. After applying, persist via `save()`.

- [ ] **Step 3: Test cache round-trip**

Write to a temp file, load back, and verify all domains and the version are preserved. Test that delta application correctly adds and removes entries.

---

### Task 6: Blocklist matching tests

**Files:** `src/blocklist/mod.rs` (test module)

- [ ] **Step 1: Test exact domain match**

`bet365.com` in blocklist -> `is_blocked("bet365.com")` returns true.

- [ ] **Step 2: Test subdomain match via parent domain walk**

`bet365.com` in blocklist -> `is_blocked("www.bet365.com")` and `is_blocked("sub.deep.bet365.com")` both return true.

- [ ] **Step 3: Test wildcard match**

`*.gambling-site.com` in blocklist -> `is_blocked("app.gambling-site.com")` returns true, but `is_blocked("gambling-site.com")` returns false (wildcard requires a subdomain).

- [ ] **Step 4: Test non-gambling domains pass through**

`is_blocked("google.com")`, `is_blocked("github.com")` return false.

- [ ] **Step 5: Test case insensitivity**

`Bet365.COM` in blocklist, query `bet365.com` -> blocked. Query `BET365.com` -> blocked.

- [ ] **Step 6: Test trailing dot normalization**

`bet365.com.` in blocklist, query `bet365.com` -> blocked. Query `bet365.com.` -> blocked.

- [ ] **Step 7: Test from_file loading**

Create a temp file with mixed entries (comments, blanks, exact, wildcard). Verify correct counts and matching behavior.

---

## Chunk 3: Event System + Config

### Task 7: EventEmitter with SQLite backing store

**Crate:** `bb-agent-core`
**Files:** `src/events/mod.rs`, `src/events/store.rs`

- [ ] **Step 1: Define AgentEvent struct**

Create an `AgentEvent` struct for local event representation:
- `id: Option<i64>` (set after SQLite insert)
- `event_type: bb_common::enums::EventType`
- `category: bb_common::enums::EventCategory`
- `severity: bb_common::enums::EventSeverity`
- `domain: Option<String>` (the domain that was blocked, if applicable)
- `plugin_id: String` (which plugin generated this event)
- `metadata: serde_json::Value`
- `timestamp: DateTime<Utc>`
- `reported: bool` (whether it has been sent to the API)

- [ ] **Step 2: Implement EventStore (SQLite)**

Create an `EventStore` struct wrapping a `rusqlite::Connection`. Methods:
- `new(path: &Path)` -- open/create the SQLite database, run `CREATE TABLE IF NOT EXISTS` for the events table.
- `insert(event: &AgentEvent) -> Result<i64>` -- insert and return the row ID.
- `unreported(limit: usize) -> Result<Vec<AgentEvent>>` -- fetch unreported events ordered by timestamp, limited.
- `mark_reported(ids: &[i64]) -> Result<()>` -- update `reported = true` for the given IDs.
- `prune_older_than(days: u32) -> Result<usize>` -- delete old reported events to bound storage.

- [ ] **Step 3: Implement EventEmitter**

Create an `EventEmitter` struct with:
- An in-memory buffer (`Vec<AgentEvent>`) for batching.
- A reference/owned `EventStore` for persistence.
- `emit(event: AgentEvent)` -- push to buffer.
- `flush() -> Result<()>` -- write all buffered events to SQLite, clear the buffer.
- `flush_threshold: usize` -- auto-flush when buffer exceeds this size (default 50).

The emitter should be usable from multiple threads. Wrap the buffer in `Arc<Mutex<>>` or use a `tokio::sync::mpsc` channel for lock-free event submission.

- [ ] **Step 4: Test emit and flush cycle**

Create an EventStore on a temp SQLite file. Emit several events, flush, verify they appear in `unreported()`. Mark as reported, verify `unreported()` returns empty.

---

### Task 8: Privacy filter

**Files:** `src/events/privacy.rs`

- [ ] **Step 1: Implement PrivacyFilter**

Create a `PrivacyFilter` struct that takes the enrollment's `ReportingConfig` (from `bb-common`). Provide a method `filter(event: &AgentEvent) -> Option<AgentEvent>` that:

- **Self-enrolled tier:** Strip `domain` field (replace with None). Only keep aggregated counts (event_type + count). Drop metadata entirely. If `reporting_config.domain_details` is false (default), redact the domain.
- **Partner tier:** Keep domain only if `reporting_config.domain_details` is true. Always keep event_type and category. Redact detailed metadata unless explicitly opted in.
- **Authority tier:** Keep full detail including domain and metadata. This is an audit-grade tier.

Return `None` to drop the event entirely (e.g., if `reporting_config.blocked_attempt_counts` is false and the event is a block count).

- [ ] **Step 2: Test privacy filter per tier**

Test that a block event with a domain:
- Self tier (default config): domain is stripped, metadata cleared.
- Partner tier (domain_details=true): domain is preserved.
- Authority tier: full event preserved.
- Self tier with blocked_attempt_counts=false: event is dropped (returns None).

---

### Task 9: AgentConfig

**Files:** `src/config/mod.rs`

- [ ] **Step 1: Define AgentConfig struct**

Create `AgentConfig` with serde Deserialize, loadable from TOML:
- `device_id: Option<String>` (set after registration)
- `enrollment_token: Option<String>` (used for initial registration)
- `api_url: String` (default: `"https://api.betblocker.org"`)
- `dns: DnsConfig` -- upstream servers, listen address, port
- `plugins: HashMap<String, PluginConfig>` -- per-plugin config keyed by plugin ID
- `reporting: ReportingSettings` -- flush interval, batch size
- `data_dir: PathBuf` -- where to store blocklist cache, events DB, etc.
- `log_level: String` (default: `"info"`)

`DnsConfig` sub-struct: `upstream_servers: Vec<String>`, `listen_addr: String` (default `"127.0.0.1"`), `listen_port: u16` (default `53`).

- [ ] **Step 2: Implement config loading**

`AgentConfig::load(path: &Path) -> Result<Self>` -- read the TOML file, deserialize with serde. Apply defaults for missing fields using `#[serde(default)]`.

`AgentConfig::merge_enrollment_config(&mut self, enrollment: &bb_common::models::Enrollment)` -- override reporting settings, plugin enabled/disabled flags, and protection config from the enrollment's server-side configuration.

- [ ] **Step 3: Implement config change detection**

Add a method `has_changed(&self, other: &AgentConfig) -> Vec<String>` that returns a list of field names that differ between two configs. This is used after merging enrollment config to determine what needs to be reloaded (e.g., if DNS upstream changed, restart the resolver; if a plugin was disabled, deactivate it).

- [ ] **Step 4: Test config loading from TOML**

Write a sample TOML config to a temp file, load it, verify all fields. Test that missing optional fields get defaults. Test merge with an enrollment config overrides the correct fields.

---

## Chunk 4: DNS Resolver Plugin

### Task 10: DnsResolverPlugin

**Crate:** `bb-agent-plugins`
**Files:** `src/dns_resolver/mod.rs`, `src/dns_resolver/handler.rs`

- [ ] **Step 1: Implement DnsResolverPlugin struct**

Create `DnsResolverPlugin` with fields:
- `blocklist: Option<Arc<Blocklist>>` (set on activate/update)
- `upstream_servers: Vec<SocketAddr>` (parsed from config)
- `listen_addr: SocketAddr`
- `server_handle: Option<tokio::task::JoinHandle<()>>` (the running DNS server task)
- `active: bool`

Implement `BlockingPlugin` trait: `init` parses config for upstream servers and listen address. `activate` stores the blocklist and starts the DNS server task. `deactivate` aborts the server task. `health_check` verifies the server task is still running.

Implement `DnsBlockingPlugin` trait: `check_domain` delegates to `self.blocklist.is_blocked(domain)`. `handle_dns_query` returns None (this plugin uses hickory-dns server, not raw packet handling).

- [ ] **Step 2: Implement the DNS request handler using hickory-dns**

```rust
use std::net::SocketAddr;
use std::sync::Arc;

use hickory_server::authority::MessageResponseBuilder;
use hickory_server::server::{Request, RequestHandler, ResponseHandler, ResponseInfo};
use hickory_resolver::TokioAsyncResolver;
use hickory_resolver::config::{ResolverConfig, ResolverOpts, NameServerConfig, Protocol};
use tracing::{debug, info, warn};

use crate::blocklist::Blocklist;
use crate::types::BlockDecision;

/// Handles incoming DNS requests: checks blocklist, forwards or blocks.
pub struct BlockingDnsHandler {
    blocklist: Arc<Blocklist>,
    upstream: TokioAsyncResolver,
}

impl BlockingDnsHandler {
    pub fn new(blocklist: Arc<Blocklist>, upstream_servers: &[SocketAddr]) -> Self {
        // Build resolver config pointing to upstream DNS servers
        let mut resolver_config = ResolverConfig::new();
        for addr in upstream_servers {
            resolver_config.add_name_server(NameServerConfig::new(
                *addr,
                Protocol::Udp,
            ));
        }
        let upstream = TokioAsyncResolver::tokio(resolver_config, ResolverOpts::default());

        Self {
            blocklist,
            upstream,
        }
    }

    /// Update the blocklist atomically (Arc swap).
    pub fn update_blocklist(&mut self, blocklist: Arc<Blocklist>) {
        self.blocklist = blocklist;
    }
}

#[async_trait::async_trait]
impl RequestHandler for BlockingDnsHandler {
    async fn handle_request<R: ResponseHandler>(
        &self,
        request: &Request,
        mut response_handle: R,
    ) -> ResponseInfo {
        let query = request.query();
        let domain = query.name().to_string();
        let domain = domain.trim_end_matches('.');

        if self.blocklist.is_blocked(domain) {
            debug!(domain = %domain, "Blocked DNS query");

            // Return NXDOMAIN for blocked domains
            let builder = MessageResponseBuilder::from_message_request(request);
            let response = builder.error_msg(request.header(), hickory_server::proto::op::ResponseCode::NXDomain);
            return response_handle
                .send_response(response)
                .await
                .unwrap_or_else(|e| {
                    warn!(error = %e, "Failed to send NXDOMAIN response");
                    let mut header = hickory_server::proto::op::Header::new();
                    header.set_response_code(hickory_server::proto::op::ResponseCode::ServFail);
                    header.into()
                });
        }

        // Forward non-blocked queries to upstream
        debug!(domain = %domain, "Forwarding DNS query to upstream");
        match self.upstream.lookup(query.name(), query.query_type()).await {
            Ok(lookup) => {
                let builder = MessageResponseBuilder::from_message_request(request);
                let records: Vec<_> = lookup.records().to_vec();
                let response = builder.build(
                    *request.header(),
                    records.iter(),
                    std::iter::empty(),
                    std::iter::empty(),
                    std::iter::empty(),
                );
                response_handle
                    .send_response(response)
                    .await
                    .unwrap_or_else(|e| {
                        warn!(error = %e, "Failed to send upstream response");
                        let mut header = hickory_server::proto::op::Header::new();
                        header.set_response_code(hickory_server::proto::op::ResponseCode::ServFail);
                        header.into()
                    })
            }
            Err(e) => {
                warn!(domain = %domain, error = %e, "Upstream DNS lookup failed");
                let builder = MessageResponseBuilder::from_message_request(request);
                let response = builder.error_msg(request.header(), hickory_server::proto::op::ResponseCode::ServFail);
                response_handle
                    .send_response(response)
                    .await
                    .unwrap_or_else(|e| {
                        warn!(error = %e, "Failed to send SERVFAIL response");
                        let mut header = hickory_server::proto::op::Header::new();
                        header.set_response_code(hickory_server::proto::op::ResponseCode::ServFail);
                        header.into()
                    })
            }
        }
    }
}
```

- [ ] **Step 3: Implement DNS server startup in DnsResolverPlugin::activate**

In `activate`, spawn a tokio task that:
1. Creates a `BlockingDnsHandler` with the current blocklist and upstream servers.
2. Binds a UDP socket on `listen_addr` using `hickory_server::ServerFuture`.
3. Runs the server future until the task is aborted (on deactivate).

Store the `JoinHandle` so `deactivate` can call `.abort()`.

---

### Task 11: NXDOMAIN responses and configurable upstream

**Files:** `src/dns_resolver/mod.rs`

- [ ] **Step 1: Configurable upstream DNS servers**

In `DnsResolverPlugin::init`, read `upstream_servers` from `PluginConfig.settings`. Default to `["8.8.8.8:53", "1.1.1.1:53"]` if not configured. Parse each string into a `SocketAddr`. Return `PluginError::ConfigError` if parsing fails.

- [ ] **Step 2: Configurable block response**

Add a config option `block_response` that supports `"nxdomain"` (default) or `"zero_ip"` (return 0.0.0.0 A record). Implement both response types in the handler. NXDOMAIN is the default because it causes browsers to show a clear error, while 0.0.0.0 causes a connection timeout that is slower to fail.

- [ ] **Step 3: Logging and metrics**

Add tracing spans for each DNS query with the domain name. Track counters: `queries_total`, `queries_blocked`, `queries_forwarded`, `upstream_errors`. These are in-memory atomics read by the health check and event emitter.

---

### Task 12: DNS resolver tests

**Files:** `src/dns_resolver/mod.rs` (test module)

- [ ] **Step 1: Test blocked domain returns NXDOMAIN**

Create a `BlockingDnsHandler` with a blocklist containing `bet365.com`. Send a mock DNS query for `bet365.com`. Verify the response code is NXDOMAIN.

- [ ] **Step 2: Test non-blocked domain is forwarded**

Use a mock upstream or a known-good domain. Verify the response contains valid DNS records (not NXDOMAIN).

- [ ] **Step 3: Test subdomain blocking**

Query `www.bet365.com` when `bet365.com` is in the blocklist. Verify NXDOMAIN is returned.

- [ ] **Step 4: Test upstream failure returns SERVFAIL**

Configure an unreachable upstream server (e.g., `192.0.2.1:53`). Query a non-blocked domain. Verify SERVFAIL response.

Note: DNS integration tests should bind to a high port (e.g., `127.0.0.1:15353`) to avoid requiring root/admin privileges during testing.

---

## Chunk 5: HOSTS File Plugin

### Task 13: HostsFilePlugin

**Crate:** `bb-agent-plugins`
**Files:** `src/hosts_file/mod.rs`, `src/hosts_file/platform.rs`

- [ ] **Step 1: Platform-aware HOSTS file path**

In `platform.rs`, define `pub fn hosts_file_path() -> PathBuf` using conditional compilation:
- Linux/macOS: `/etc/hosts`
- Windows: `C:\Windows\System32\drivers\etc\hosts`

Also accept an override via `PluginConfig.settings["hosts_file_path"]` for testing and non-standard setups.

- [ ] **Step 2: Implement HostsFilePlugin struct**

Fields:
- `hosts_path: PathBuf`
- `blocklist: Option<Arc<Blocklist>>`
- `active: bool`
- `entries_hash: Option<String>` (SHA-256 of BetBlocker entries for tamper detection)

Implement `BlockingPlugin`: `init` resolves the hosts path. `activate` calls `write_entries`. `deactivate` calls `remove_entries`. `update_blocklist` calls `write_entries` with the new list.

Implement `DnsBlockingPlugin`: `check_domain` delegates to blocklist (same as DNS resolver -- the HOSTS file plugin's blocking happens at the OS level, not in this check). `handle_dns_query` returns None.

- [ ] **Step 3: Implement write_entries and remove_entries**

`write_entries(blocklist: &Blocklist)`:
1. Read the current HOSTS file content.
2. Remove any existing BetBlocker section (between `# BEGIN BETBLOCKER` and `# END BETBLOCKER` markers).
3. Append a new BetBlocker section with `0.0.0.0 <domain>` for each exact domain in the blocklist. Include a timestamp comment.
4. Write back atomically (write to temp file, rename).
5. Compute and store SHA-256 hash of the BetBlocker section.

`remove_entries()`:
1. Read HOSTS file, strip the BetBlocker section, write back.

- [ ] **Step 4: Handle large blocklists**

If the blocklist exceeds 10,000 domains, log a warning. The HOSTS file approach does not scale well beyond this. Write only the highest-confidence entries (sorted by confidence descending) up to a configurable max (default 5,000). The DNS resolver plugin is the primary blocking mechanism; HOSTS is a fallback.

---

### Task 14: Tamper detection

**Files:** `src/hosts_file/mod.rs`

- [ ] **Step 1: Implement tamper detection in health_check**

In `health_check()`:
1. Read the current HOSTS file.
2. Extract the BetBlocker section.
3. Compute SHA-256 of the section.
4. Compare against the stored `entries_hash`.
5. If mismatch, return `PluginHealth::degraded("HOSTS file tampered")`.

- [ ] **Step 2: Implement auto-restore**

Add a method `check_and_restore() -> Result<bool>` that:
1. Calls `health_check()`.
2. If tampered, re-writes the BetBlocker section from the current blocklist.
3. Emits a `TamperDetected` event (via a callback or channel provided during init).
4. Returns `true` if restoration was needed.

This method should be called periodically by the agent's watchdog (every 30 seconds, configurable).

- [ ] **Step 3: Ensure atomic writes prevent partial corruption**

Verify that the write_entries method uses a temp file + rename pattern. On Windows, use `std::fs::rename` which may fail if the target is locked -- in that case, fall back to read-modify-write with a file lock.

---

### Task 15: HOSTS file plugin tests

**Files:** `src/hosts_file/mod.rs` (test module)

- [ ] **Step 1: Test write_entries on a temp file**

Create a temp file with existing content (e.g., `127.0.0.1 localhost`). Write BetBlocker entries. Verify the original content is preserved and BetBlocker entries are appended between markers.

- [ ] **Step 2: Test remove_entries cleans up**

After writing entries, call `remove_entries`. Verify the temp file only contains the original content (no BetBlocker markers or entries).

- [ ] **Step 3: Test idempotent writes**

Call `write_entries` twice with the same blocklist. Verify only one BetBlocker section exists (no duplication).

- [ ] **Step 4: Test tamper detection**

Write entries, then manually modify the BetBlocker section in the temp file. Call `health_check`. Verify it returns degraded status.

- [ ] **Step 5: Test auto-restore after tamper**

Write entries, tamper with the file, call `check_and_restore()`. Verify the file is restored to the correct state and the method returns `true`.

- [ ] **Step 6: Test with empty blocklist**

Activate with an empty blocklist. Verify the HOSTS file gets a BetBlocker section with only marker comments and no entries. Deactivate and verify markers are removed.

---

## Verification Checklist

After all chunks are complete, verify:

- [ ] `cargo build -p bb-agent-plugins --all-features` compiles cleanly
- [ ] `cargo build -p bb-agent-core` compiles cleanly
- [ ] `cargo test -p bb-agent-plugins` -- all plugin system tests pass
- [ ] `cargo test -p bb-agent-core` -- all blocklist, event, and config tests pass
- [ ] `cargo clippy -p bb-agent-plugins -p bb-agent-core -- -D warnings` -- no warnings
- [ ] Plugin registry correctly short-circuits on first Block decision
- [ ] Blocklist matches `sub.bet365.com` when `bet365.com` is in the list
- [ ] DNS resolver returns NXDOMAIN for blocked domains (manual test on high port)
- [ ] HOSTS file plugin writes and removes entries cleanly
- [ ] Events persist to SQLite and survive process restart
- [ ] Privacy filter strips domain info for self-enrolled tier
- [ ] AgentConfig loads from TOML and merges enrollment overrides
