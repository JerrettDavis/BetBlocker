# ADR-002: Plugin Architecture for Blocking Layers

## Status
Proposed

## Date
2026-03-12

## Context

BetBlocker's blocking engine operates across three layers (DNS/Network, Application, Browser/Content), each of which behaves differently on each of the five supported platforms. The combinations create a matrix:

| Layer | Windows | macOS | Linux | Android | iOS |
|-------|---------|-------|-------|---------|-----|
| DNS | WFP + local resolver | Network Extension | iptables + local resolver | VpnService | NEDNSProxyProvider |
| App | Process monitoring + WMI | Launch Services + Endpoint Security | procfs + inotify | PackageManager + UsageStats | Not possible (sandboxed) |
| Browser | Extension + named pipe | Extension + XPC | Extension + Unix socket | Extension + bound service | Content blocker JSON rules |

Each cell in this matrix has different capabilities, different OS APIs, and different levels of blocking effectiveness. The architecture must:

1. Allow each layer to be developed and shipped independently (Phase 1 ships DNS only; Phase 2 adds App; Phase 3 adds Browser).
2. Allow platform-specific implementations of the same logical layer without conditional compilation spaghetti in the core engine.
3. Support runtime discovery of available plugins (e.g., the App blocking plugin is not present in Phase 1 builds).
4. Maintain a uniform interface for the core engine to query "is this domain/app/content blocked?" regardless of which plugins are active.
5. Support future extensibility (community plugins in Phase 4, e.g., a Tor exit node blocker or a custom DNS provider).

## Decision

**Use a Rust trait-based plugin system with static dispatch at compile time and a registry pattern for runtime lifecycle management.**

### Plugin Trait Hierarchy

```rust
/// Every plugin implements this base trait for lifecycle management.
pub trait BlockingPlugin: Send + Sync + 'static {
    /// Unique identifier, e.g., "dns.wfp", "app.process_monitor", "browser.extension"
    fn id(&self) -> &str;

    /// Human-readable name for logging and status reporting
    fn name(&self) -> &str;

    /// Which blocking layer this plugin belongs to
    fn layer(&self) -> BlockingLayer;

    /// Initialize the plugin with its configuration.
    /// Called once at agent startup. May fail if OS prerequisites are missing
    /// (e.g., WFP driver not installed).
    fn init(&mut self, config: &PluginConfig) -> Result<(), PluginError>;

    /// Activate blocking. Called after init, and after blocklist is loaded.
    fn activate(&mut self, blocklist: &Blocklist) -> Result<(), PluginError>;

    /// Deactivate blocking. Called on graceful shutdown or plugin hot-reload.
    fn deactivate(&mut self) -> Result<(), PluginError>;

    /// Receive an updated blocklist. Called when a delta sync completes.
    fn update_blocklist(&mut self, blocklist: &Blocklist) -> Result<(), PluginError>;

    /// Health check. Called periodically by the watchdog.
    /// Returns Err if the plugin has been tampered with or is non-functional.
    fn health_check(&self) -> Result<PluginHealth, PluginError>;
}

/// DNS/Network layer plugins implement this additional trait.
pub trait DnsBlockingPlugin: BlockingPlugin {
    /// Check if a domain should be blocked.
    /// Must be extremely fast (sub-microsecond for cache hit).
    fn check_domain(&self, domain: &str) -> BlockDecision;

    /// Handle a raw DNS query packet. For plugins that operate at the
    /// packet level (WFP, VpnService).
    fn handle_dns_query(&self, query: &[u8]) -> Option<Vec<u8>>;
}

/// Application layer plugins implement this additional trait.
pub trait AppBlockingPlugin: BlockingPlugin {
    /// Check if an application identifier should be blocked.
    fn check_app(&self, app_id: &AppIdentifier) -> BlockDecision;

    /// Scan installed applications and return matches.
    fn scan_installed(&self) -> Vec<AppMatch>;

    /// Start monitoring for new app installations.
    fn watch_installs(&mut self) -> Result<(), PluginError>;
}

/// Browser/Content layer plugins implement this additional trait.
pub trait ContentBlockingPlugin: BlockingPlugin {
    /// Generate content blocking rules in the format expected by browser extensions.
    fn generate_rules(&self, blocklist: &Blocklist) -> ContentRules;

    /// Check browser extension presence and integrity.
    fn check_extension_health(&self) -> ExtensionHealth;
}
```

### Static Dispatch, Not Dynamic

Plugins are compiled into the agent binary using feature flags, not loaded as dynamic libraries at runtime.

```toml
# Cargo.toml for bb-agent
[features]
default = ["dns-resolver", "dns-hosts"]
dns-resolver = ["bb-plugin-dns-resolver"]
dns-wfp = ["bb-shim-windows/wfp"]
dns-network-ext = ["bb-shim-macos/network-extension"]
dns-vpnservice = ["bb-shim-android/vpnservice"]
dns-hosts = ["bb-plugin-dns-hosts"]
app-process = ["bb-plugin-app-process"]
app-device-admin = ["bb-shim-android/device-admin"]
browser-extension = ["bb-plugin-browser-extension"]
content-blocker-ios = ["bb-shim-ios/content-blocker"]
```

**Rationale for static over dynamic dispatch:**

- Dynamic loading (`dlopen` / `LoadLibrary`) is a tamper resistance liability. An attacker could replace a plugin `.so`/`.dll` with a no-op implementation. Static compilation means the plugin code is embedded in the signed binary.
- Static dispatch via monomorphization eliminates vtable overhead in the DNS hot path. `check_domain()` is called for every DNS query; virtual dispatch adds ~2-5ns per call, which matters at thousands of queries per second.
- Rust's `dyn Trait` requires heap allocation and loses type information. For the plugin registry (which manages a small, known set of plugins), `enum` dispatch is simpler and faster.
- The plugin set is known at compile time per platform. There is no user-installable plugin mechanism in Phases 1-3. Phase 4 community plugins will use a sandboxed WASM runtime (separate ADR when that phase is planned).

### Plugin Registry

The registry is an `enum`-based dispatch layer that avoids trait objects:

```rust
/// Enum over all compiled-in plugins. Variants are conditionally compiled.
pub enum PluginInstance {
    #[cfg(feature = "dns-resolver")]
    DnsResolver(DnsResolverPlugin),

    #[cfg(feature = "dns-wfp")]
    DnsWfp(WfpPlugin),

    #[cfg(feature = "dns-hosts")]
    DnsHosts(HostsFilePlugin),

    #[cfg(feature = "app-process")]
    AppProcess(ProcessMonitorPlugin),

    // ... etc
}

pub struct PluginRegistry {
    plugins: Vec<PluginInstance>,
}

impl PluginRegistry {
    /// Initialize all compiled-in plugins. Skip any that fail init
    /// (log warning, continue with remaining plugins).
    pub fn init_all(&mut self, config: &AgentConfig, blocklist: &Blocklist) -> Vec<PluginError> { ... }

    /// Query all DNS plugins for a domain. Returns Block if ANY plugin blocks.
    /// Short-circuits on first Block decision for performance.
    pub fn check_domain(&self, domain: &str) -> BlockDecision { ... }

    /// Query all App plugins for an application. Same short-circuit logic.
    pub fn check_app(&self, app_id: &AppIdentifier) -> BlockDecision { ... }

    /// Run health checks on all plugins. Returns failures.
    pub fn health_check_all(&self) -> Vec<(String, PluginError)> { ... }

    /// Push updated blocklist to all active plugins.
    pub fn update_blocklist_all(&mut self, blocklist: &Blocklist) -> Vec<PluginError> { ... }
}
```

### Plugin Lifecycle

```
                    +--------+
                    |  Built |  (compiled into binary via feature flag)
                    +---+----+
                        |
                   init(config)
                        |
                   +----v-----+
              +--->| Inactive |  (initialized but not blocking)
              |    +----+-----+
              |         |
              |    activate(blocklist)
              |         |
              |    +----v----+
              |    |  Active |  (blocking, receiving queries)
              |    +----+----+
              |         |
              |    deactivate()
              |         |
              +---------+
```

- **Built**: The plugin code is compiled in. At startup, the registry enumerates all compiled-in plugins.
- **Inactive**: `init()` has been called. The plugin has validated that its OS prerequisites exist (e.g., WFP driver is installed, VpnService permission is granted). If `init()` fails, the plugin stays in "not available" state and is excluded from the registry.
- **Active**: `activate()` has been called with the current blocklist. The plugin is intercepting queries/processes/content. `update_blocklist()` can be called while active.
- **Deactivate**: Called on graceful shutdown, or if the plugin needs to be hot-reloaded (e.g., after a blocklist format change that requires re-initialization).

### Platform Shim Design

Each platform shim crate provides one or more plugin implementations behind a C-ABI bridge where necessary:

```
bb-shim-windows/
  src/
    wfp.rs          # WFP callout driver interaction via IOCTL
    minifilter.rs   # File protection driver interaction
    service.rs      # Windows Service lifecycle (SCM registration)
    lib.rs          # Conditional compilation: only builds on windows

bb-shim-macos/
  src/
    network_ext.rs  # Network Extension provider (calls Swift bridge)
    system_ext.rs   # System Extension + Endpoint Security
    launchd.rs      # launchd daemon lifecycle
    bridge/
      swift/        # Minimal Swift code for APIs not accessible via C
    lib.rs

bb-shim-android/
  src/
    vpnservice.rs   # VpnService via JNI
    device_admin.rs  # Device Administrator/Owner via JNI
    jni_bridge.rs    # JNI helper utilities
    lib.rs
```

The key constraint: **shim crates contain zero business logic.** They translate between OS APIs and Rust types. The blocking decision is always made by `bb-core`; the shim just provides the interception mechanism.

### Configuration Per Plugin

Each plugin receives a `PluginConfig` struct that is deserialized from the agent's configuration file:

```rust
pub struct PluginConfig {
    /// Plugin-specific key-value configuration
    pub settings: HashMap<String, serde_json::Value>,

    /// Whether this plugin is enabled (can be disabled by enrollment policy)
    pub enabled: bool,

    /// Priority relative to other plugins in the same layer
    /// (lower number = checked first)
    pub priority: u32,
}
```

Plugin configuration is part of the enrollment policy. The API can push configuration changes that enable/disable plugins or adjust their settings (e.g., setting the DNS resolver's upstream server, or enabling/disabling the HOSTS file fallback).

## Alternatives Considered

### Dynamic Plugin Loading (dlopen/LoadLibrary)

**Pros:** True runtime extensibility, smaller base binary, community plugins without recompilation.

**Rejected because:**
- Fatal for tamper resistance. A replaced `.so`/`.dll` could silently disable blocking. Binary signature verification of individual plugins adds complexity and still has TOCTOU issues.
- Rust's ABI is unstable. Dynamic plugins would need to use `extern "C"` interfaces, losing type safety at the boundary.
- The plugin set is known at compile time for Phases 1-3. The flexibility is not needed and the security cost is too high.
- When Phase 4 community plugins arrive, WASM sandboxing (wasmtime) provides safe dynamic loading with capability-based security, which is a strictly better solution than native dynamic loading.

### Trait Objects (dyn Trait)

**Pros:** Simpler code than enum dispatch, more idiomatic for polymorphism.

**Rejected for the hot path (check_domain) because:**
- Virtual dispatch overhead (~2-5ns per call) on every DNS query. With 1000+ queries/second, this adds measurable latency.
- Trait objects require heap allocation (`Box<dyn BlockingPlugin>`) and lose concrete type information, making debugging harder.

**Accepted for non-hot-path operations** (health checks, lifecycle management) where the overhead is irrelevant. The registry may use `dyn BlockingPlugin` internally for operations that run infrequently. The key is that `check_domain()` goes through the enum dispatch path.

### WASM Plugin Sandbox (Now)

**Pros:** Safe dynamic loading, capability-based security, language-agnostic plugins.

**Deferred to Phase 4 because:**
- WASM runtime (wasmtime) adds 5-10 MB to the binary. Premature for Phase 1.
- The blocking plugins need deep OS access (WFP, Network Extension) that cannot be sandboxed in WASM.
- WASM is appropriate for content analysis plugins (heuristic classifiers, pattern matchers) that don't need OS access, which aligns with Phase 3-4 functionality.

## Consequences

### What becomes easier

- **Phased delivery.** Phase 1 ships with `dns-resolver` and `dns-hosts` features only. Phase 2 adds `app-process`. Phase 3 adds `browser-extension`. Each phase is a feature flag change, not an architectural change.
- **Platform isolation.** A bug in the WFP shim cannot affect the macOS build. Conditional compilation ensures platform code is never present in the wrong binary.
- **Testing.** Each plugin can be tested independently with a mock `Blocklist` and mock OS APIs. The registry can be tested with any subset of plugins.
- **Performance.** Static dispatch means the compiler can inline `check_domain()` calls. The DNS hot path has zero overhead from the plugin system.

### What becomes harder

- **Adding a new plugin requires recompilation.** There is no "drop a file and restart" mechanism. This is an intentional trade-off for tamper resistance.
- **Enum dispatch boilerplate.** Every new plugin variant requires updating the `PluginInstance` enum and its match arms. Mitigation: a procedural macro can generate the dispatch code from the enum variants.
- **Feature flag combinatorial explosion.** With many plugins and platforms, the feature flag matrix grows. CI must test meaningful combinations, not all 2^N possibilities. Mitigation: define platform profiles (`windows-full`, `macos-full`, `android-full`) that bundle the correct features.

## Implementation Notes

### Phase 1 Plugins

| Plugin | Feature Flag | Platforms | Description |
|--------|-------------|-----------|-------------|
| `DnsResolverPlugin` | `dns-resolver` | All | Local DNS resolver, intercepts queries, checks blocklist, forwards non-blocked to upstream |
| `HostsFilePlugin` | `dns-hosts` | Windows, macOS, Linux | Writes blocked domains to HOSTS file as redundant fallback. Survives agent crashes. |
| `WfpPlugin` | `dns-wfp` | Windows | WFP callout driver for DNS interception. Ensures apps with hardcoded DNS cannot bypass. |
| `NetworkExtPlugin` | `dns-network-ext` | macOS | Network Extension DNS proxy provider. Required for macOS DNS interception. |
| `VpnServicePlugin` | `dns-vpnservice` | Android | Local VPN that routes DNS through agent. Standard Android approach. |
| `NEDnsProxyPlugin` | `dns-ios-proxy` | iOS | NEDNSProxyProvider for iOS DNS interception. |
| `NftablesPlugin` | `dns-nftables` | Linux | nftables rules to redirect DNS to local resolver. |

### Plugin Communication

Plugins do not communicate directly with each other. All coordination goes through the core engine:

- DNS plugin blocks a domain -> core engine records the event -> event reporter sends to API
- App plugin detects a gambling app -> core engine records the event -> DNS plugin is not involved
- Browser extension reports a heuristic match -> agent receives via IPC -> core engine evaluates -> may send federated report

This star topology (core engine at center) prevents coupling between plugins and ensures the core engine is the single source of truth for blocking decisions.

### Future: WASM Plugin Sandbox (Phase 4)

When community plugins are introduced, they will run in a wasmtime sandbox with explicit capability grants:

- Content analysis plugins: granted read access to page content, no network access, no filesystem access
- Custom DNS rules: granted blocklist read access, no write access
- All WASM plugins: memory-limited, CPU-time-limited, no access to enrollment credentials or cryptographic material

This will be a separate ADR when Phase 4 planning begins.
