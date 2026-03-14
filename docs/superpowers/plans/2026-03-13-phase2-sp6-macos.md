# Phase 2, Sub-Plan 6: macOS Platform

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver a fully functional macOS agent at parity with `bb-agent-linux`: launchd daemon lifecycle, DNS interception via Network Extension, Keychain-based key storage, DNS configuration monitoring, pkg installer with notarization, and (Wave 2) System Extension + Endpoint Security for tamper resistance.

**Parent plan:** `docs/superpowers/plans/2026-03-13-phase2-master-plan.md`
**Design doc:** `docs/plans/2026-03-13-phase2-design.md` (sections 2 and 8)
**Reference impl:** `crates/bb-agent-linux/` (mirrors this structure)
**Depends on:** Phase 1 agent core (`bb-agent-core`, `bb-agent-plugins`, `bb-common`)

**Task IDs:** P2-MAC-1 through P2-MAC-4, P2-TAMPER-3

**New crates/files created by this plan:**
| Location | Description |
|----------|-------------|
| `crates/bb-shim-macos/` | macOS platform shim crate (launchd, keychain, DNS monitor, NE, ES) |
| `crates/bb-agent-macos/` | macOS agent binary crate (entrypoint, mirrors bb-agent-linux) |
| `crates/bb-shim-macos/bridge/swift/` | Swift package for Network Extension + System Extension |
| `deploy/macos/` | launchd plist, pkg scripts, notarization config |

---

## Chunk 1: Crate Scaffolding + launchd Lifecycle (P2-MAC-1)

### 1.1 Crate setup

- [ ] Create `crates/bb-shim-macos/Cargo.toml` with dependencies: `tokio`, `tracing`, `thiserror`, `security-framework` (Keychain), `core-foundation` (SCDynamicStore). Gate crate with `cfg(target_os = "macos")`.
- [ ] Create `crates/bb-shim-macos/src/lib.rs` exporting modules: `launchd`, `keychain`, `dns_monitor`, `network_ext`, `platform`.
- [ ] Create `crates/bb-agent-macos/Cargo.toml` mirroring `bb-agent-linux/Cargo.toml` structure, depending on `bb-agent-core`, `bb-agent-plugins`, `bb-common`, `bb-shim-macos`.
- [ ] Create `crates/bb-agent-macos/src/main.rs` as a stub `fn main()` with `#[tokio::main]`.
- [ ] Add both crates to workspace `Cargo.toml` members list (conditionally or behind cfg).
- [ ] Verify `cargo check -p bb-shim-macos -p bb-agent-macos` passes (on macOS CI or with `--target` cross-check).

### 1.2 Platform bridge (`platform.rs`)

- [ ] Create `crates/bb-shim-macos/src/platform.rs` with:
  - `read_machine_id() -> String` -- read hardware UUID via `IOPlatformExpertDevice` IOKit call (fallback: `sysctl kern.uuid`).
  - `ensure_directories()` -- create `/Library/Application Support/BetBlocker/`, `/Library/Application Support/BetBlocker/certs/`, `/var/log/betblocker/` with mode 0o700.
  - `current_uid() -> u32` -- wraps `libc::getuid()`.
- [ ] Write unit tests: `test_read_machine_id_returns_nonempty`, `test_current_uid`.

### 1.3 launchd integration

- [ ] Create `crates/bb-shim-macos/src/launchd.rs` with:
  - `LaunchdPlist` struct holding plist path, label, program path.
  - `LaunchdPlist::generate() -> String` -- renders XML plist with `KeepAlive=true`, `RunAtLoad=true`, `AbandonProcessGroup=true`, `StandardOutPath`, `StandardErrorPath`, `Label=com.betblocker.agent`.
  - `LaunchdPlist::install()` -- writes to `/Library/LaunchDaemons/com.betblocker.agent.plist`, sets ownership root:wheel, mode 0o644, runs `launchctl bootstrap system <path>`.
  - `LaunchdPlist::uninstall()` -- `launchctl bootout system/com.betblocker.agent`.
  - `LaunchdPlist::is_loaded() -> bool` -- checks via `launchctl print system/com.betblocker.agent`.
- [ ] Create `deploy/macos/com.betblocker.agent.plist` -- static reference plist file.
- [ ] Write tests: `test_plist_generation_valid_xml`, `test_plist_contains_keepalive`, `test_plist_contains_label`.

### 1.4 Signal handling (macOS-specific)

- [ ] In `bb-agent-macos/src/main.rs`, implement signal handling matching `bb-agent-linux`: SIGTERM, SIGINT via `tokio::signal::unix`, SIGHUP for config reload. No `sd_notify` -- launchd uses the process lifecycle directly.
- [ ] Create `crates/bb-agent-macos/src/platform.rs` with no-op stubs for `sd_notify_ready()`, `sd_notify_stopping()`, `sd_notify_status()` (launchd doesn't use this protocol; keeps API parity with Linux agent).

---

## Chunk 2: Agent Entrypoint + DNS Config Monitoring (P2-MAC-1, partial P2-MAC-3)

### 2.1 Agent binary (`bb-agent-macos`)

- [ ] Create `crates/bb-agent-macos/src/main.rs` mirroring `bb-agent-linux/src/main.rs` structure:
  - `Cli` struct with `--config-dir` (default `/Library/Application Support/BetBlocker`), `--enroll`, `--config`.
  - `run(cli)` async function following the same 4-phase pattern: Setup, Registration, Initialize subsystems, Wait for shutdown.
  - Use `bb-agent-core` for: `AgentConfig`, `EventStore`, `EventEmitter`, `CertificateStore` (or macOS Keychain variant), `ApiClient`, `RegistrationService`, `HeartbeatSender`, `EventReporter`, `BinaryIntegrity`, `WatchdogMonitor`.
  - Plugin registry init via `bb_agent_plugins::PluginRegistry::with_defaults()`.
  - Emit `AgentStarted` event with `"platform": "macos"`.
  - Graceful shutdown: deactivate plugins, flush events, await tasks with 5s timeout.
- [ ] Create `crates/bb-agent-macos/src/dns_redirect.rs` -- macOS equivalent of `nftables.rs`:
  - `PfManager` struct wrapping `pfctl` commands (macOS's packet filter).
  - `install_rules()` -- adds DNS redirect rules via `pfctl` to redirect port 53 to local resolver. Uses anchor `com.betblocker`.
  - `verify_and_repair() -> Result<bool>` -- checks if rules are intact, reinstalls if removed.
  - `remove_rules()` -- cleanup on shutdown.
- [ ] Write tests: `test_pf_rule_generation`, `test_verify_detects_missing_rules` (mock command execution).

### 2.2 DNS configuration monitoring via SCDynamicStore

- [ ] Create `crates/bb-shim-macos/src/dns_monitor.rs`:
  - `DnsMonitor` struct holding an `SCDynamicStore` session.
  - `DnsMonitor::new()` -- creates store, registers notification keys for `State:/Network/Global/DNS` and `State:/Network/Service/.*/DNS`.
  - `DnsMonitor::start(shutdown_rx) -> JoinHandle` -- spawns a thread running `CFRunLoop` to receive SCDynamicStore callbacks. On DNS change, logs the event and re-applies DNS configuration if it was tampered with.
  - `DnsMonitor::current_dns_servers() -> Vec<IpAddr>` -- reads current DNS config from SCDynamicStore.
  - `DnsMonitor::enforce_dns(servers: &[IpAddr])` -- writes DNS config back via `scutil` or `networksetup` CLI.
- [ ] DNS monitor uses `core-foundation` and `system-configuration` crates for SCDynamicStore FFI.
- [ ] Write tests: `test_current_dns_servers_returns_list`, `test_dns_monitor_creation`.

### 2.3 Integration wiring

- [ ] Wire `DnsMonitor` into `bb-agent-macos/src/main.rs` run loop -- start monitoring after plugin init, stop on shutdown.
- [ ] Wire `PfManager` into run loop with periodic verify (30s interval), matching nftables pattern.

---

## Chunk 3: Keychain Integration + Certificate Store (P2-MAC-3)

### 3.1 Keychain key storage

- [ ] Create `crates/bb-shim-macos/src/keychain.rs`:
  - `KeychainCertificateStore` struct implementing `bb_agent_core::comms::certificate::CertificateStore` trait.
  - `store_identity(pem: &[u8])` -- stores client cert+key in System Keychain with:
    - `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly` access control
    - `kSecAttrIsExtractable = false` (non-exportable)
    - Label: `com.betblocker.agent.identity`
    - Application tag for lookup
  - `load_identity() -> Option<Vec<u8>>` -- retrieves from Keychain by label.
  - `store_ca_chain(pem: &[u8])` -- stores CA cert chain with label `com.betblocker.ca`.
  - `load_ca_chain() -> Option<Vec<u8>>` -- retrieves CA chain.
  - `delete_all()` -- removes all BetBlocker keychain items (for uninstall).
- [ ] Use `security-framework` crate for Keychain operations (wraps Security.framework).
- [ ] Write tests: `test_store_and_load_identity_roundtrip`, `test_store_and_load_ca_roundtrip`, `test_delete_all_cleans_up`. Tests use a temporary keychain created with `SecKeychainCreate` to avoid polluting the system keychain.

### 3.2 File permissions hardening

- [ ] Create `crates/bb-shim-macos/src/file_protect.rs`:
  - `set_agent_file_permissions(path: &Path)` -- sets owner root:wheel, mode 0o600 for config/data, 0o755 for binaries.
  - `set_immutable_flag(path: &Path)` -- sets `SF_IMMUTABLE` (system immutable flag) via `chflags`. Only root can clear this.
  - `verify_permissions(path: &Path) -> bool` -- checks owner/mode are correct.
- [ ] Write tests: `test_permission_setting` (requires root or skip in CI), `test_verify_permissions`.

### 3.3 Swap CertificateStore implementation

- [ ] In `bb-agent-macos/src/main.rs`, use `KeychainCertificateStore` instead of `FileCertificateStore` when on macOS. Both implement the same trait, so the agent core code is unchanged.
- [ ] Add feature flag `keychain-store` to `bb-shim-macos` to allow fallback to file store in development.

---

## Chunk 4: Network Extension with XPC Bridge (P2-MAC-2)

### 4.1 Swift package setup

- [ ] Create `crates/bb-shim-macos/bridge/swift/Package.swift` -- Swift package defining:
  - `BetBlockerNetworkExtension` target (NEDNSProxyProvider subclass)
  - `BetBlockerXPCBridge` target (XPC client/server protocol)
- [ ] Create `crates/bb-shim-macos/bridge/swift/Sources/BetBlockerNetworkExtension/DNSProxyProvider.swift`:
  - Subclass `NEDNSProxyProvider`.
  - `startProxy(options:completionHandler:)` -- establishes XPC connection to main agent, calls completion.
  - `stopProxy(reason:completionHandler:)` -- tears down XPC, calls completion.
  - `handleNewFlow(_ flow: NEAppProxyFlow) -> Bool` -- forwards DNS queries to agent via XPC, returns blocked/allowed response.
- [ ] Create entitlements file `crates/bb-shim-macos/bridge/swift/BetBlockerNetworkExtension.entitlements` with `com.apple.developer.networking.networkextension` array containing `dns-proxy`.

### 4.2 XPC protocol

- [ ] Create `crates/bb-shim-macos/bridge/swift/Sources/BetBlockerXPCBridge/XPCProtocol.swift`:
  - `@objc protocol BetBlockerXPCProtocol`: `checkDomain(_ domain: String, reply: @escaping (Bool) -> Void)` -- returns true if blocked.
  - `getBlocklist(reply: @escaping (Data) -> Void)` -- returns serialized blocklist for NE-local caching.
  - `reportEvent(_ eventData: Data)` -- fire-and-forget event from NE to agent.
- [ ] Create `crates/bb-shim-macos/bridge/swift/Sources/BetBlockerXPCBridge/XPCServer.swift`:
  - `XPCServer` class that creates `NSXPCListener` with Mach service name `com.betblocker.agent.xpc`.
  - Accepts connections, validates code signing requirement (only BetBlocker-signed clients).
  - Delegates `checkDomain` calls to a Rust callback via C FFI.
- [ ] Create `crates/bb-shim-macos/bridge/swift/Sources/BetBlockerXPCBridge/XPCClient.swift`:
  - `XPCClient` class used by the Network Extension to connect to the agent's XPC service.
  - Auto-reconnect on connection interruption/invalidation.

### 4.3 Rust-side XPC integration

- [ ] Create `crates/bb-shim-macos/src/network_ext.rs`:
  - `NetworkExtensionManager` struct managing NE activation/deactivation.
  - `activate()` -- calls `NEDNSProxyManager.shared().loadFromPreferences()`, enables, saves.
  - `deactivate()` -- disables and saves preferences.
  - `is_active() -> bool` -- checks `NEDNSProxyManager` connection status.
- [ ] Create `crates/bb-shim-macos/src/xpc.rs`:
  - `XpcServer` struct wrapping the Swift XPC server via C FFI.
  - `start(blocklist_checker: Arc<dyn Fn(&str) -> bool>)` -- starts XPC listener, routes `checkDomain` calls to the provided closure.
  - `stop()` -- shuts down listener.
  - C FFI bridge functions: `bb_xpc_start()`, `bb_xpc_stop()`, `bb_xpc_check_domain(domain: *const c_char) -> bool`.
- [ ] Create `crates/bb-shim-macos/src/ffi.rs` -- C-compatible FFI functions that the Swift XPC server calls into:

```rust
#[no_mangle]
pub extern "C" fn bb_check_domain(domain: *const c_char) -> bool {
    // Convert C string, call into plugin registry
}

#[no_mangle]
pub extern "C" fn bb_xpc_report_event(data: *const u8, len: usize) {
    // Deserialize and emit event
}
```

- [ ] Write tests: `test_network_ext_manager_lifecycle`, `test_xpc_server_start_stop`. Network Extension activation tests require entitlements and are integration-only.

### 4.4 Build integration

- [ ] Create `crates/bb-shim-macos/build.rs` -- build script that:
  - Compiles the Swift package via `swift build` if on macOS.
  - Generates C header from Swift `@objc` exports.
  - Links the resulting `.dylib` or `.a`.
- [ ] Add `cc` and `swift-bridge` or manual FFI to `Cargo.toml` build-dependencies.

---

## Chunk 5: pkg Installer + Notarization Pipeline (P2-MAC-4)

### 5.1 Installer scripts

- [ ] Create `deploy/macos/scripts/preinstall.sh`:
  - Check macOS version (minimum 12.0 Monterey for Network Extension support).
  - Stop existing agent if running: `launchctl bootout system/com.betblocker.agent 2>/dev/null || true`.
- [ ] Create `deploy/macos/scripts/postinstall.sh`:
  - Copy binary to `/Library/Application Support/BetBlocker/bb-agent-macos`.
  - Set permissions (root:wheel, 0o755 for binary, 0o700 for data dir).
  - Install launchd plist to `/Library/LaunchDaemons/`.
  - Bootstrap: `launchctl bootstrap system /Library/LaunchDaemons/com.betblocker.agent.plist`.
  - Activate Network Extension via `bb-agent-macos --activate-ne` (one-shot mode).
- [ ] Create `deploy/macos/distribution.xml` -- pkg distribution descriptor with title, welcome, license, min-os-version, install-location.

### 5.2 pkg build script

- [ ] Create `deploy/macos/build-pkg.sh`:
  - Build the agent: `cargo build --release -p bb-agent-macos --target aarch64-apple-darwin` and `--target x86_64-apple-darwin`.
  - Create universal binary: `lipo -create -output bb-agent-macos <arm64> <x86_64>`.
  - Build Swift Network Extension: `swift build -c release`.
  - Assemble payload directory structure.
  - Build component pkg: `pkgbuild --root <payload> --scripts <scripts> --identifier com.betblocker.agent --version $VERSION component.pkg`.
  - Build distribution pkg: `productbuild --distribution distribution.xml --package-path . BetBlocker-$VERSION.pkg`.
  - Sign: `productsign --sign "Developer ID Installer: ..." BetBlocker-$VERSION.pkg BetBlocker-$VERSION-signed.pkg`.

### 5.3 Notarization

- [ ] Create `deploy/macos/notarize.sh`:
  - Submit for notarization: `xcrun notarytool submit BetBlocker-$VERSION-signed.pkg --apple-id $APPLE_ID --team-id $TEAM_ID --password $APP_PASSWORD --wait`.
  - Staple: `xcrun stapler staple BetBlocker-$VERSION-signed.pkg`.
  - Verify: `spctl --assess --type install BetBlocker-$VERSION-signed.pkg`.
- [ ] Create `deploy/macos/uninstall.sh`:
  - Bootout launchd job.
  - Deactivate Network Extension.
  - Remove files from `/Library/Application Support/BetBlocker/`.
  - Remove plist from `/Library/LaunchDaemons/`.
  - Remove Keychain items: `security delete-generic-password -l com.betblocker.*` or via agent `--uninstall` flag.

### 5.4 CI integration

- [ ] Add macOS build job to CI config (GitHub Actions `macos-14` runner with Xcode).
- [ ] Build universal binary + Network Extension in CI.
- [ ] Run `cargo test -p bb-shim-macos -p bb-agent-macos` on macOS runner.
- [ ] Notarization step gated on tagged releases only (requires secrets).

---

## Chunk 6: System Extension + Endpoint Security (P2-TAMPER-3, Wave 2)

> **Prerequisite:** Chunks 1-4 complete. Requires `com.apple.developer.endpoint-security.client` and `com.apple.developer.system-extension.install` entitlements.

### 6.1 System Extension lifecycle

- [ ] Create `crates/bb-shim-macos/src/system_ext.rs`:
  - `SystemExtManager` struct implementing `OSSystemExtensionRequestDelegate`.
  - `install()` -- submits `OSSystemExtensionRequest.activationRequest(forExtensionWithIdentifier:)`.
  - `uninstall()` -- submits deactivation request.
  - `status() -> SystemExtStatus` -- returns `NotInstalled | Pending | Active | RequiresApproval`.
  - Delegate callbacks: `request(_:didFinishWithResult:)`, `request(_:didFailWithError:)`, `requestNeedsUserApproval(_:)`.
- [ ] Implement via Swift bridge (System Extension API is Swift/ObjC only).
- [ ] Create `crates/bb-shim-macos/bridge/swift/Sources/BetBlockerSystemExtension/SystemExtensionDelegate.swift`.
- [ ] Write tests: `test_system_ext_status_not_installed` (safe to run without entitlements).

### 6.2 Endpoint Security client

- [ ] Create `crates/bb-shim-macos/src/endpoint_security.rs`:
  - `EndpointSecurityClient` struct wrapping an `es_client_t`.
  - `new(event_handler: impl Fn(EsEvent))` -- calls `es_new_client()`, subscribes to events:
    - `ES_EVENT_TYPE_AUTH_UNLINK` -- block deletion of agent files.
    - `ES_EVENT_TYPE_AUTH_RENAME` -- block renaming agent files.
    - `ES_EVENT_TYPE_AUTH_SIGNAL` -- block SIGKILL to agent PID.
    - `ES_EVENT_TYPE_NOTIFY_EXEC` -- observe process launches (for app blocking).
  - `handle_event(msg: *const es_message_t)` -- dispatch to per-event-type handlers.
  - For AUTH events on protected paths: return `ES_AUTH_RESULT_DENY`. For agent's own operations: return `ES_AUTH_RESULT_ALLOW`.
  - `protected_paths() -> Vec<PathBuf>` -- returns paths to protect: agent binary, config dir, plist, Keychain items.
- [ ] Endpoint Security is a C API -- use raw FFI bindings (no crate exists). Define types in `crates/bb-shim-macos/src/es_sys.rs`:

```rust
// Minimal ES FFI bindings
extern "C" {
    fn es_new_client(client: *mut *mut es_client_t, handler: es_handler_block_t) -> es_new_client_result_t;
    fn es_subscribe(client: *mut es_client_t, events: *const es_event_type_t, count: u32) -> es_return_t;
    fn es_respond_auth_result(client: *mut es_client_t, msg: *const es_message_t, result: es_auth_result_t, cache: bool) -> es_return_t;
    fn es_delete_client(client: *mut es_client_t) -> es_return_t;
}
```

- [ ] Write tests: `test_protected_paths_includes_agent_binary`, `test_protected_paths_includes_config`. ES client creation tests are integration-only (require entitlement + root).

### 6.3 Tamper detection integration

- [ ] Wire `EndpointSecurityClient` into agent run loop (Wave 2 feature flag `tamper-es`).
- [ ] On AUTH deny events, emit `AgentEvent::tamper_detected("endpoint_security", detail)`.
- [ ] On NOTIFY_EXEC events, forward to `AppProcessPlugin` for app blocking (when app blocking is enabled).
- [ ] Add `ProtectionStatus` extension to heartbeat: report ES client active, System Extension status.

---

## Chunk 7: Swift Bridge + Testing Strategy

### 7.1 Swift bridge layer

- [ ] Create `crates/bb-shim-macos/src/swift_bridge.rs` -- unified Rust interface to all Swift components:
  - `fn swift_network_ext_activate() -> Result<()>` -- calls Swift NE activation.
  - `fn swift_network_ext_deactivate() -> Result<()>`.
  - `fn swift_system_ext_install() -> Result<()>` -- calls Swift SysExt install.
  - `fn swift_system_ext_status() -> SystemExtStatus`.
  - `fn swift_xpc_start(check_fn: extern "C" fn(*const c_char) -> bool)`.
  - `fn swift_xpc_stop()`.
- [ ] In `crates/bb-shim-macos/build.rs`, add Swift compilation and linking:
  - Compile Swift sources via `swiftc` with `-emit-library` and `-emit-objc-header`.
  - Link against `NetworkExtension.framework`, `SystemExtensions.framework`, `EndpointSecurity.framework`, `Security.framework`.
  - Output `include/bb_swift_bridge.h` for Rust FFI consumption.
- [ ] Define C header `crates/bb-shim-macos/bridge/include/bb_swift_bridge.h` with all exported function signatures.

### 7.2 Testing strategy

- [ ] **Unit tests (run everywhere):** Plist generation, permission checking logic, FFI type conversions, config parsing, platform bridge (machine ID, directory creation). These use mocks/stubs for system calls.
- [ ] **Integration tests (macOS CI only):** Keychain roundtrip (with temp keychain), DNS monitor creation, pfctl rule generation. Gate with `#[cfg(target_os = "macos")]`.
- [ ] **Manual/entitlement tests (dev machine only):** Network Extension activation, System Extension installation, Endpoint Security client creation. Document in `deploy/macos/TESTING.md` (created only as part of this task).
- [ ] Create `crates/bb-shim-macos/tests/integration_keychain.rs` -- integration test using temporary keychain.
- [ ] Create `crates/bb-shim-macos/tests/integration_dns_monitor.rs` -- integration test for SCDynamicStore creation.
- [ ] Create `crates/bb-agent-macos/tests/integration_agent.rs` -- smoke test: agent starts, emits AgentStarted event, shuts down on SIGTERM.

### 7.3 Feature flags summary

- [ ] Define feature flags in `crates/bb-shim-macos/Cargo.toml`:
  - `network-extension` (default) -- enables NE + XPC code.
  - `keychain-store` (default) -- enables Keychain certificate store.
  - `tamper-es` -- enables Endpoint Security + System Extension (Wave 2, off by default).
  - `pf-redirect` (default) -- enables pfctl DNS redirect.
- [ ] Define feature flags in `crates/bb-agent-macos/Cargo.toml`:
  - `full` -- enables all shim features.
  - Default: `network-extension`, `keychain-store`, `pf-redirect`.

---

## Definition of Done

- [ ] `cargo build -p bb-agent-macos` produces a working binary on macOS.
- [ ] `cargo test -p bb-shim-macos -p bb-agent-macos` passes on macOS CI runner.
- [ ] Agent starts as launchd daemon, handles SIGTERM gracefully.
- [ ] DNS queries are intercepted via Network Extension (with entitlements) or pfctl fallback.
- [ ] Certificates stored in Keychain with non-exportable flag.
- [ ] DNS configuration changes detected via SCDynamicStore and re-enforced.
- [ ] `deploy/macos/build-pkg.sh` produces a signed, notarized .pkg installer.
- [ ] System Extension + Endpoint Security client functional with Wave 2 feature flag.
- [ ] All new code has unit tests; integration tests run on macOS CI.
