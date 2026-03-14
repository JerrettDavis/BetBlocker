# Phase 2 Sub-Plan 7: Linux + Mobile Tamper Resistance

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Harden Linux agent with kernel-level MAC policies (AppArmor, SELinux), add experimental eBPF DNS interception, and define platform-abstracted interfaces for Android Device Owner/Knox and iOS MDM integration.
**Architecture:** `bb-shim-linux` crate for Linux MAC/eBPF. `bb-shim-android` and `bb-shim-ios` crates define trait-based interfaces stubbed for future platform agents. Deploy artifacts under `deploy/apparmor/` and `deploy/selinux/`. eBPF program in `deploy/ebpf/`.
**Tech Stack:** Rust, AppArmor, SELinux (type enforcement), eBPF/XDP (libbpf via aya), Android Device Owner API, Samsung Knox SDK, Apple MDM/Network Extension
**Depends on:** Phase 1 Linux agent (`crates/bb-agent-linux/`), existing tamper detection (`crates/bb-agent-core/src/tamper/`)

**Reference Docs:**
- Phase 2 Design (Section 2): `docs/plans/2026-03-13-phase2-design.md`
- Master Plan: `docs/superpowers/plans/2026-03-13-phase2-master-plan.md`
- Existing tamper: `crates/bb-agent-core/src/tamper/mod.rs`
- Linux agent: `crates/bb-agent-linux/src/main.rs`
- Linux installer: `deploy/linux/install.sh`
- Enums: `crates/bb-common/src/enums.rs`

---

## File Structure

```
crates/
  bb-shim-linux/
    Cargo.toml
    src/
      lib.rs                        # Re-exports
      apparmor.rs                   # AppArmor profile installation and management
      selinux.rs                    # SELinux policy module installation and management
      ebpf.rs                      # eBPF DNS interception (stretch goal)
      mac.rs                       # Unified MAC abstraction (detect + apply)
  bb-shim-android/
    Cargo.toml
    src/
      lib.rs                       # Re-exports
      device_owner.rs              # Device Owner provisioning and policy management
      knox.rs                      # Samsung Knox SDK integration
      traits.rs                    # Platform-abstracted interfaces
  bb-shim-ios/
    Cargo.toml
    src/
      lib.rs                       # Re-exports
      mdm.rs                       # MDM profile integration
      traits.rs                    # Platform-abstracted interfaces
deploy/
  apparmor/
    betblocker-agent               # AppArmor profile definition
  selinux/
    betblocker.te                  # SELinux type enforcement policy
    betblocker.fc                  # SELinux file context definitions
    betblocker.if                  # SELinux interface definitions
  ebpf/
    dns_intercept.bpf.c            # eBPF DNS interception program (stretch)
```

---

## Chunk 1: Linux MAC Abstraction + AppArmor Profile (Lines ~1-95)

### Task 1: MAC detection and abstraction layer

**Crate:** `bb-shim-linux`
**Files:** `Cargo.toml`, `src/lib.rs`, `src/mac.rs`

- [ ] **Step 1: Create `crates/bb-shim-linux/Cargo.toml`** with deps: `thiserror`, `tracing`, `tokio`, `serde`. Feature flags: `apparmor` (default), `selinux`, `ebpf`.
- [ ] **Step 2: Create `src/lib.rs`** re-exporting `mac`, `apparmor`, `selinux`, and conditionally `ebpf`.
- [ ] **Step 3: Create `src/mac.rs`** with `MacSystem` enum (`AppArmor`, `SELinux`, `None`) and `detect_mac_system()` fn that checks `/sys/module/apparmor` and `/sys/fs/selinux`. Add `MacStatus` struct reporting active system, profile/policy loaded status, enforcement mode.
- [ ] **Step 4: Define `MacProtection` trait** with methods: `install() -> Result<()>`, `verify() -> Result<MacStatus>`, `is_enforcing() -> bool`, `uninstall() -> Result<()>`. Both `AppArmorProtection` and `SELinuxProtection` implement this.
- [ ] **Step 5: Write tests** for `detect_mac_system()` (mock `/sys` paths via trait abstraction or cfg(test) overrides), `MacStatus` serialization, and `MacSystem` enum coverage.

### Task 2: AppArmor profile definition

**File:** `deploy/apparmor/betblocker-agent`

- [ ] **Step 6: Create the AppArmor profile** defining:
  - Profile name: `betblocker-agent`
  - Allow: read/write `/var/lib/betblocker/**`, read `/etc/machine-id`, read `/proc/self/exe`, network inet/inet6 stream/dgram, execute `/usr/lib/betblocker/bb-agent-linux`
  - Deny for other processes: write to `/usr/lib/betblocker/**`, write to `/var/lib/betblocker/**`, ptrace on the agent, signal (KILL/STOP/TERM) to agent (except from agent itself)
  - Include base abstractions: `abstractions/base`, `abstractions/nameservice`

```apparmor
#include <tunables/global>

profile betblocker-agent /usr/lib/betblocker/bb-agent-linux {
  #include <abstractions/base>
  #include <abstractions/nameservice>

  # Agent binary
  /usr/lib/betblocker/bb-agent-linux mr,

  # Data directory
  /var/lib/betblocker/ rw,
  /var/lib/betblocker/** rwk,

  # Log directory
  /var/log/betblocker/ rw,
  /var/log/betblocker/** rw,

  # System reads
  /etc/machine-id r,
  /proc/self/exe r,
  /proc/sys/net/** r,

  # Network access (DNS resolver + API comms)
  network inet stream,
  network inet dgram,
  network inet6 stream,
  network inet6 dgram,
  network netlink raw,

  # nftables management
  /usr/sbin/nft Ux,

  # Deny ptrace from other processes
  deny ptrace (read, trace) peer=unconfined,

  # Certificate and key material
  /var/lib/betblocker/certs/** rw,
}
```

- [ ] **Step 7: Write a companion deny profile** `deploy/apparmor/betblocker-files` that prevents non-agent processes from writing to `/usr/lib/betblocker/` and `/var/lib/betblocker/`. Note: AppArmor's file-owner rules are limited; this is best-effort and documented as such.
- [ ] **Step 8: Write tests** validating profile syntax (parse with `apparmor_parser -p` in CI where available, else validate structure via string assertions on the profile content).

### Task 3: AppArmor installation automation

**File:** `crates/bb-shim-linux/src/apparmor.rs`

- [ ] **Step 9: Create `AppArmorProtection` struct** with fields: `profile_path: PathBuf`, `profile_name: String`, `profiles_dir: PathBuf` (default `/etc/apparmor.d/`).
- [ ] **Step 10: Implement `install()`** -- copy profile to `/etc/apparmor.d/betblocker-agent`, run `apparmor_parser -r /etc/apparmor.d/betblocker-agent` to load and enforce. Handle errors: apparmor_parser not found, permission denied, parse failure.
- [ ] **Step 11: Implement `verify()`** -- check `/sys/kernel/security/apparmor/profiles` for `betblocker-agent (enforce)`. Return `MacStatus` with enforcement state.
- [ ] **Step 12: Implement `is_enforcing()`** -- parse `/sys/kernel/security/apparmor/profiles`, find our profile, check mode string.
- [ ] **Step 13: Implement `uninstall()`** -- run `apparmor_parser -R /etc/apparmor.d/betblocker-agent`, remove profile file. Only used during agent uninstall.
- [ ] **Step 14: Implement `MacProtection` trait** for `AppArmorProtection`.
- [ ] **Step 15: Add `verify_and_repair()`** method -- if profile not loaded or in complain mode, reload in enforce mode. Emit tamper event if profile was removed externally.
- [ ] **Step 16: Write unit tests** -- mock Command execution via a `CommandRunner` trait. Test install success/failure paths, verify parsing, repair logic.
- [ ] **Step 17: Update `deploy/linux/install.sh`** -- after installing binary, detect AppArmor (`aa-enabled`), copy profile, load with `apparmor_parser -r`. Add `--no-apparmor` flag to skip.

---

## Chunk 2: SELinux Policy Module (Lines ~96-175)

### Task 4: SELinux type enforcement policy

**Files:** `deploy/selinux/betblocker.te`, `deploy/selinux/betblocker.fc`, `deploy/selinux/betblocker.if`

- [ ] **Step 18: Create `betblocker.te`** (type enforcement) defining:
  - Module: `betblocker 1.0.0`
  - Types: `betblocker_t` (process domain), `betblocker_exec_t` (binary), `betblocker_var_t` (data files), `betblocker_log_t` (logs)
  - Domain transition: `init_t` -> `betblocker_t` when executing `betblocker_exec_t`
  - Allow: `betblocker_t` read/write `betblocker_var_t`, append `betblocker_log_t`, network tcp/udp, dns resolution, execute nft
  - Deny: all other domains write to `betblocker_exec_t` and `betblocker_var_t`, ptrace `betblocker_t`

- [ ] **Step 19: Create `betblocker.fc`** (file contexts):
  - `/usr/lib/betblocker(/.*)?` -> `system_u:object_r:betblocker_exec_t:s0`
  - `/var/lib/betblocker(/.*)?` -> `system_u:object_r:betblocker_var_t:s0`
  - `/var/log/betblocker(/.*)?` -> `system_u:object_r:betblocker_log_t:s0`

- [ ] **Step 20: Create `betblocker.if`** (interface definitions) with `betblocker_domtrans` macro for init system domain transition.

- [ ] **Step 21: Write validation tests** -- check `.te` file contains required type declarations and allow rules via string assertions. Full `checkmodule` validation gated behind CI feature flag.

### Task 5: SELinux installation automation

**File:** `crates/bb-shim-linux/src/selinux.rs`

- [ ] **Step 22: Create `SELinuxProtection` struct** with fields: `policy_dir: PathBuf`, `module_name: String`.
- [ ] **Step 23: Implement `install()`** -- compile policy: `checkmodule -M -m -o betblocker.mod betblocker.te`, then `semodule_package -o betblocker.pp -m betblocker.mod -f betblocker.fc`, then `semodule -i betblocker.pp`. Apply file contexts: `restorecon -R /usr/lib/betblocker /var/lib/betblocker /var/log/betblocker`.
- [ ] **Step 24: Implement `verify()`** -- run `semodule -l | grep betblocker`, check `getenforce` output. Return `MacStatus`.
- [ ] **Step 25: Implement `is_enforcing()`** -- parse `getenforce` output for "Enforcing".
- [ ] **Step 26: Implement `uninstall()`** -- `semodule -r betblocker`, restore default file contexts.
- [ ] **Step 27: Implement `MacProtection` trait** for `SELinuxProtection`.
- [ ] **Step 28: Add `verify_and_repair()`** -- if module not loaded, reinstall. If permissive, log warning (cannot force enforcing globally).
- [ ] **Step 29: Write unit tests** -- mock Command execution. Test compile/install/verify/remove paths.
- [ ] **Step 30: Update `deploy/linux/install.sh`** -- detect SELinux (`getenforce`), compile and install policy module if enforcing/permissive. Add `--no-selinux` flag.

---

## Chunk 3: Linux Agent Integration + eBPF Stretch Goal (Lines ~176-280)

### Task 6: Integrate MAC protection into Linux agent

**Files:** `crates/bb-agent-linux/src/main.rs`, `crates/bb-agent-linux/Cargo.toml`

- [ ] **Step 31: Add `bb-shim-linux` dependency** to `bb-agent-linux/Cargo.toml` with features `apparmor` and `selinux` as optional.
- [ ] **Step 32: Add MAC initialization to `run()`** -- after `ensure_directories()`, call `mac::detect_mac_system()`. Based on result, instantiate `AppArmorProtection` or `SELinuxProtection`. Call `verify()` to check current status. Log protection level.
- [ ] **Step 33: Add periodic MAC verification task** -- every 60s, call `verify_and_repair()` on the active MAC protection. On tamper detection (profile/policy removed), emit `TamperDetected` event via `EventEmitter`.
- [ ] **Step 34: Extend heartbeat metadata** -- include MAC protection status (`mac_system`, `mac_enforcing`, `mac_profile_loaded`) in heartbeat `ProtectionStatus` metadata.
- [ ] **Step 35: Write integration tests** -- test MAC detect + verify flow with mock filesystem. Test tamper detection when profile is "removed" (mock returns not-loaded).

### Task 7: eBPF DNS interception (stretch goal)

**Files:** `crates/bb-shim-linux/src/ebpf.rs`, `deploy/ebpf/dns_intercept.bpf.c`

- [ ] **Step 36: Create `deploy/ebpf/dns_intercept.bpf.c`** -- XDP/TC hook program that:
  - Parses UDP packets on port 53
  - Extracts queried domain name from DNS query
  - Looks up domain in a BPF hash map (populated by userspace agent)
  - For blocked domains: rewrites DNS response to NXDOMAIN or redirect IP
  - For allowed domains: passes through (XDP_PASS)
  - Note: This is a C BPF program compiled with clang, loaded via aya-rs

- [ ] **Step 37: Create `EbpfDnsInterceptor` struct** in `src/ebpf.rs` with aya-rs for loading the compiled BPF program. Methods: `load()`, `attach(interface)`, `update_blocklist(domains)`, `detach()`, `stats()`.
- [ ] **Step 38: Implement blocklist sync to BPF map** -- `update_blocklist()` takes a `HashSet<String>` of blocked domains, hashes each domain name, and updates the BPF hash map via aya's map API. Map key: domain hash (u64), value: action byte (0=pass, 1=block).
- [ ] **Step 39: Implement stats collection** -- read BPF per-CPU array map for counters: total queries, blocked queries, passed queries. Expose via `stats()` method.
- [ ] **Step 40: Add feature gate** -- entire eBPF module behind `#[cfg(feature = "ebpf")]`. Requires: Linux 5.10+, CAP_BPF, aya crate. Document kernel version requirements.
- [ ] **Step 41: Write unit tests** -- test domain hashing, blocklist map update logic (without actual BPF loading). Integration tests gated behind `test_ebpf` feature requiring root + compatible kernel.

---

## Chunk 4: Android Device Owner Interfaces (Lines ~281-380)

### Task 8: Android platform abstraction traits

**Crate:** `bb-shim-android`
**Files:** `Cargo.toml`, `src/lib.rs`, `src/traits.rs`

- [ ] **Step 42: Create `crates/bb-shim-android/Cargo.toml`** with deps: `thiserror`, `serde`, `tracing`. Mark as `#[cfg(target_os = "android")]` where needed but keep traits compilable on all platforms for testing.
- [ ] **Step 43: Create `src/traits.rs`** defining:
  - `DeviceAdminProvider` trait: `is_device_owner() -> bool`, `is_device_admin() -> bool`, `request_admin_activation()`, `get_admin_status() -> AdminStatus`
  - `DevicePolicyProvider` trait: `set_uninstall_blocked(pkg: &str, blocked: bool)`, `set_vpn_always_on(pkg: &str, enabled: bool)`, `add_user_restriction(restriction: &str)`, `clear_user_restriction(restriction: &str)`, `get_active_restrictions() -> Vec<String>`
  - `ProvisioningProvider` trait: `generate_qr_provisioning_data(config: &ProvisioningConfig) -> String`, `handle_provisioning_complete()`
  - `AdminStatus` enum: `NotAdmin`, `DeviceAdmin`, `DeviceOwner`, `ProfileOwner`
  - `ProvisioningConfig` struct: `admin_component: String`, `wifi_ssid: Option<String>`, `enrollment_token: String`, `api_url: String`
- [ ] **Step 44: Write tests** for trait default implementations and `ProvisioningConfig` serialization.

### Task 9: Device Owner provisioning flow

**File:** `crates/bb-shim-android/src/device_owner.rs`

- [ ] **Step 45: Create `DeviceOwnerManager` struct** implementing `DeviceAdminProvider` and `DevicePolicyProvider`. On non-Android, all methods return stub/error results.
- [ ] **Step 46: Implement QR code provisioning data generation** -- `generate_qr_provisioning_data()` produces a JSON payload per Android's `DevicePolicyManager.EXTRA_PROVISIONING_*` format:

```rust
/// QR code provisioning payload for factory-reset provisioning.
/// The user scans this QR during device setup to install BetBlocker as Device Owner.
pub fn generate_qr_provisioning_data(config: &ProvisioningConfig) -> serde_json::Value {
    serde_json::json!({
        "android.app.extra.PROVISIONING_DEVICE_ADMIN_COMPONENT_NAME": config.admin_component,
        "android.app.extra.PROVISIONING_DEVICE_ADMIN_PACKAGE_DOWNLOAD_LOCATION": config.apk_url,
        "android.app.extra.PROVISIONING_DEVICE_ADMIN_PACKAGE_CHECKSUM": config.apk_checksum,
        "android.app.extra.PROVISIONING_WIFI_SSID": config.wifi_ssid,
        "android.app.extra.PROVISIONING_LOCALE": "en_US",
        "android.app.extra.PROVISIONING_SKIP_ENCRYPTION": false,
    })
}
```

- [ ] **Step 47: Implement ADB provisioning helper** -- `generate_adb_command()` returns the `dpm set-device-owner` command string for already-provisioned devices. Document prerequisites (no accounts, factory reset preferred).
- [ ] **Step 48: Implement policy enforcement stubs** -- `set_uninstall_blocked()`, `set_vpn_always_on()`, `add_user_restriction()`. On non-Android, log and return `Err(PlatformError::NotAndroid)`. On Android, these would call into JNI/Android APIs.
- [ ] **Step 49: Define restriction constants** -- `DISALLOW_INSTALL_APPS`, `DISALLOW_UNINSTALL_APPS`, `DISALLOW_CONFIG_VPN`, `DISALLOW_DEBUGGING`, `DISALLOW_SAFE_BOOT`, `DISALLOW_FACTORY_RESET`.
- [ ] **Step 50: Write tests** for QR payload generation (validate JSON structure), ADB command generation, restriction constant values, stub error returns on non-Android.

### Task 10: Device Owner policy management

**File:** `crates/bb-shim-android/src/device_owner.rs` (continued)

- [ ] **Step 51: Implement `DeviceOwnerPolicyManager`** with methods:
  - `enforce_betblocker_policies()` -- applies all standard restrictions (prevent uninstall, VPN always-on, disable safe boot)
  - `relax_policies()` -- called during legitimate unenrollment to remove restrictions
  - `verify_policies() -> PolicyStatus` -- checks all expected restrictions are still active
- [ ] **Step 52: Define `PolicyStatus` struct** -- fields: `uninstall_blocked: bool`, `vpn_enforced: bool`, `restrictions_active: Vec<String>`, `tampered: bool`.
- [ ] **Step 53: Implement periodic policy verification** -- `verify_and_repair()` checks all policies, re-applies any that were removed, returns whether tamper was detected.
- [ ] **Step 54: Write tests** for policy enforcement, verification, and tamper detection logic.

---

## Chunk 5: Samsung Knox + iOS MDM Interfaces (Lines ~381-480)

### Task 11: Samsung Knox integration

**File:** `crates/bb-shim-android/src/knox.rs`

- [ ] **Step 55: Define `KnoxProvider` trait** with methods:
  - `is_knox_available() -> bool` -- detect Samsung Knox SDK presence
  - `get_knox_version() -> Option<String>`
  - `enable_managed_vpn(config: &KnoxVpnConfig) -> Result<()>` -- Knox Workspace managed VPN (user cannot disconnect)
  - `set_app_protection(pkg: &str, config: &AppProtection) -> Result<()>` -- prevent force-stop, clear data
  - `enable_device_policy(policy: KnoxPolicy) -> Result<()>`
- [ ] **Step 56: Define supporting types:**
  - `KnoxVpnConfig`: `vpn_package: String`, `vpn_profile: String`, `always_on: bool`, `lockdown: bool`
  - `AppProtection`: `prevent_force_stop: bool`, `prevent_clear_data: bool`, `prevent_uninstall: bool`
  - `KnoxPolicy` enum: `BlockUninstall`, `EnforceVpn`, `DisableDevSettings`, `PreventFactoryReset`
- [ ] **Step 57: Create `KnoxManager` struct** implementing `KnoxProvider`. All methods stubbed on non-Android with `PlatformError::NotAndroid`. On Android, methods document the Knox SDK API calls needed.
- [ ] **Step 58: Implement Knox detection** -- check for `com.samsung.android.knox.container` package and Knox SDK version via `KnoxEnterpriseLicenseManager`.
- [ ] **Step 59: Write tests** for Knox detection logic, VPN config serialization, policy enum coverage.

### Task 12: iOS MDM profile integration

**Crate:** `bb-shim-ios`
**Files:** `Cargo.toml`, `src/lib.rs`, `src/traits.rs`, `src/mdm.rs`

- [ ] **Step 60: Create `crates/bb-shim-ios/Cargo.toml`** with deps: `thiserror`, `serde`, `tracing`.
- [ ] **Step 61: Define `MdmProvider` trait** in `src/traits.rs`:
  - `is_mdm_managed() -> bool`
  - `get_mdm_status() -> MdmStatus`
  - `install_profile(profile: &MdmProfile) -> Result<()>`
  - `remove_profile(profile_id: &str) -> Result<()>`
  - `get_installed_profiles() -> Vec<MdmProfileInfo>`
- [ ] **Step 62: Define MDM types:**
  - `MdmStatus`: `managed: bool`, `supervised: bool`, `profile_installed: bool`, `network_extension_active: bool`
  - `MdmProfile`: `identifier: String`, `display_name: String`, `organization: String`, `payloads: Vec<MdmPayload>`
  - `MdmPayload` enum: `VpnPayload(VpnConfig)`, `DnsPayload(DnsConfig)`, `RestrictionsPayload(Restrictions)`
  - `VpnConfig`: `vpn_type: String`, `server: String`, `always_on: bool`
  - `DnsConfig`: `servers: Vec<String>`, `supplemental_match_domains: Vec<String>`
  - `Restrictions`: `allow_vpn_creation: bool`, `allow_dns_settings: bool`
- [ ] **Step 63: Create `MdmManager` struct** in `src/mdm.rs` implementing `MdmProvider`. All methods return `PlatformError::NotIos` on non-iOS. Include detailed doc comments describing the MDM infrastructure requirements:
  - Apple Push Notification Service (APNS) certificate
  - MDM server endpoint (can be self-hosted or via MDM partner)
  - Profile signing certificate (Apple Developer Program)
  - Network Extension entitlement
- [ ] **Step 64: Implement MDM profile plist generation** -- `generate_profile_plist()` produces the XML plist payload for a DNS proxy / VPN configuration profile. This is cross-platform testable since it is pure data generation.
- [ ] **Step 65: Document supervised mode flow** -- add doc comments on `MdmManager` explaining Apple Configurator and Apple Business Manager provisioning for supervised devices. No code needed; this is operational guidance.
- [ ] **Step 66: Write tests** for profile plist generation (validate XML structure), MdmStatus serialization, payload enum coverage, stub error returns.

---

## Chunk 6: Common Enums, Heartbeat Extension, Final Integration (Lines ~481-500)

### Task 13: Common enum and heartbeat updates

**Files:** `crates/bb-common/src/enums.rs`, `crates/bb-agent-core/src/tamper/mod.rs`

- [ ] **Step 67: Add `KernelProtectionType` enum** to `enums.rs`: `AppArmor`, `SELinux`, `Ebpf`, `DeviceOwner`, `Knox`, `Mdm`, `None`.
- [ ] **Step 68: Add `KernelProtectionStatus` enum** to `enums.rs`: `NotAvailable`, `Installed`, `Enforcing`, `Degraded`, `TamperDetected`.
- [ ] **Step 69: Update `tamper/mod.rs`** -- re-export MAC/kernel protection types. Add `KernelProtection` struct combining `KernelProtectionType` and `KernelProtectionStatus` for heartbeat reporting.
- [ ] **Step 70: Write tests** for new enum serialization/deserialization round-trips.

### Task 14: End-to-end verification

- [ ] **Step 71: Create integration test** in `crates/bb-shim-linux/tests/integration.rs` -- test MAC detection, AppArmor profile parsing, SELinux policy validation. Gate behind `test_mac` feature requiring appropriate system.
- [ ] **Step 72: Update CI pipeline** -- add Linux CI job that runs on Ubuntu (for AppArmor tests) and optionally Fedora container (for SELinux tests). eBPF tests require kernel 5.10+ and are optional.
- [ ] **Step 73: Update `deploy/linux/install.sh`** -- integrate MAC profile/policy installation with detection and graceful fallback. Print MAC protection status at end of install.

---

## Definition of Done

- [ ] AppArmor profile installs and enforces on Ubuntu/Debian, protecting agent files and process
- [ ] SELinux policy module compiles and installs on RHEL/Fedora, protecting agent files and process
- [ ] Linux agent detects MAC system, loads appropriate protection, and monitors for tampering
- [ ] eBPF DNS interceptor loads and blocks domains at kernel level (stretch goal, feature-gated)
- [ ] Android Device Owner interfaces defined with QR/ADB provisioning, policy management, and Knox traits
- [ ] iOS MDM interfaces defined with profile generation, MDM status, and Network Extension management
- [ ] All new enums and protection status types integrated into heartbeat reporting
- [ ] All code has tests; `cargo test` passes; CI is green
