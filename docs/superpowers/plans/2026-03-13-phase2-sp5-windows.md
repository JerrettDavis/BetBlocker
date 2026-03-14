# Phase 2 Sub-Plan 5: Windows Platform

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Complete the Windows platform shim and agent binary, delivering a fully functional Windows agent at parity with the Linux agent: Windows Service lifecycle, DNS monitoring, file/registry ACL protection, TPM/DPAPI key storage, MSI installer, auto-update, and the `bb-agent-windows` binary crate. Wave 2 adds WFP callout driver and kernel minifilter stubs.

**Architecture:** `bb-shim-windows` provides Win32 platform abstractions. `bb-agent-windows` is the binary crate (mirrors `bb-agent-linux`). Kernel drivers (WFP, minifilter) are C projects with Rust IOCTL interfaces, deferred to Wave 2.

**Tech Stack:** Rust, `windows-service` crate, `windows` crate (Win32 bindings), WDK (C drivers), WiX/cargo-wix (MSI), DPAPI/TPM via `windows` crate

**Depends on:** Phase 1 agent core (`bb-agent-core`, `bb-agent-plugins`, `bb-common`)

**Reference Docs:**
- Phase 2 Design (section 2, 8): `docs/plans/2026-03-13-phase2-design.md`
- Master Plan: `docs/superpowers/plans/2026-03-13-phase2-master-plan.md`
- Linux agent reference: `crates/bb-agent-linux/src/main.rs`, `platform.rs`, `nftables.rs`
- Agent core: `crates/bb-agent-core/src/`

---

## File Structure

```
crates/
  bb-shim-windows/
    Cargo.toml
    src/
      lib.rs                  # Re-exports, feature gates
      service.rs              # Windows Service lifecycle (SCM, control handler)
      dns_monitor.rs          # DNS config monitoring (NotifyIpInterfaceChange, GetAdaptersInfo)
      acl.rs                  # File/registry DACL management
      keystore.rs             # TPM/DPAPI key storage for device certs
      installer.rs            # MSI install/uninstall helpers
      updater.rs              # Auto-update mechanism
      wfp.rs                  # WFP callout driver IOCTL interface (Wave 2)
      minifilter.rs           # Kernel minifilter IOCTL interface (Wave 2)
    driver/                   # WFP callout driver (C, WDK) -- Wave 2
      betblocker_wfp.c
      betblocker_wfp.inf
    minifilter/               # Filesystem minifilter (C, WDK) -- Wave 2
      betblocker_mf.c
      betblocker_mf.inf
  bb-agent-windows/
    Cargo.toml
    src/
      main.rs                 # Agent entrypoint + Windows Service dispatch
      platform.rs             # Windows PlatformBridge (machine ID, dirs, service notify)
      dns_redirect.rs         # WFP user-mode DNS redirect (netsh/WFP API)
deploy/
  windows/
    betblocker.wxs            # WiX installer definition
    install.ps1               # PowerShell install helper
```

---

## Chunk 1: bb-shim-windows Crate Setup + Windows Service Lifecycle (~130 lines)

### Task 1: Crate scaffolding

**Crate:** `bb-shim-windows`
**Files:** `Cargo.toml`, `src/lib.rs`

- [ ] **Step 1:** Create `crates/bb-shim-windows/Cargo.toml` with dependencies: `windows-service = "0.7"`, `windows = { version = "0.58", features = ["Win32_Security", "Win32_System_Registry", "Win32_NetworkManagement_IpHelper", "Win32_System_Services"] }`, `thiserror`, `tracing`, `tokio`. Gate kernel driver modules behind `feature = "kernel-drivers"`.
- [ ] **Step 2:** Create `src/lib.rs` re-exporting `service`, `dns_monitor`, `acl`, `keystore`, `installer`, `updater`. Conditionally export `wfp` and `minifilter` behind `kernel-drivers` feature.
- [ ] **Step 3:** Add `bb-shim-windows` to workspace `Cargo.toml` members. Add `cfg(target_os = "windows")` conditional compilation note in lib.rs.

### Task 2: Windows Service lifecycle

**File:** `src/service.rs`

- [ ] **Step 4:** Define `ServiceError` enum with variants: `RegistrationFailed`, `ControlHandlerFailed`, `StatusUpdateFailed`, `AlreadyRunning`, `Win32(windows::core::Error)`.
- [ ] **Step 5:** Write `register_service()` function that calls `OpenSCManagerW` + `CreateServiceW` with: `SERVICE_WIN32_OWN_PROCESS`, `SERVICE_AUTO_START`, `LocalSystem` account, binary path from `std::env::current_exe()`.
- [ ] **Step 6:** Write `set_failure_actions()` that configures `SERVICE_FAILURE_ACTIONS` via `ChangeServiceConfig2W`: restart after 0s, 5s, 30s for first/second/subsequent failures. Reset period = 86400s.
- [ ] **Step 7:** Write `unregister_service()` that opens and deletes the service via `DeleteService`.
- [ ] **Step 8:** Write `ServiceControlHandler` struct implementing the control handler callback via `RegisterServiceCtrlHandlerExW`. Handle `SERVICE_CONTROL_STOP`, `SERVICE_CONTROL_SHUTDOWN`, `SERVICE_CONTROL_INTERROGATE`, and custom control code 128 (config reload). Store a `tokio::sync::watch::Sender<bool>` for shutdown signaling.
- [ ] **Step 9:** Write `run_as_service(entry: impl FnOnce(watch::Receiver<bool>))` that calls `StartServiceCtrlDispatcherW` with a service main function. The service main registers the control handler, sets status to `SERVICE_RUNNING`, calls the entry closure, then sets `SERVICE_STOPPED` on return.
- [ ] **Step 10:** Write `set_service_status()` helper that wraps `SetServiceStatus` calls with correct `SERVICE_STATUS` struct population (current state, accepted controls, exit code, checkpoint, wait hint).
- [ ] **Step 11:** Write unit tests: test `ServiceError` display strings, test that `ServiceControlHandler` default state is not-shutdown. Write integration test (gated behind `#[cfg(test_windows_service)]`) that registers/unregisters a test service.

### Task 3: Service lifecycle tests

- [ ] **Step 12:** Write mock SCM test using a `MockServiceContext` trait to abstract Win32 calls. Test control handler dispatches STOP correctly (sends true on shutdown channel).
- [ ] **Step 13:** Test failure actions configuration struct generation independently of Win32 calls.

---

## Chunk 2: DNS Monitoring + DNS Redirect (~120 lines)

### Task 4: DNS configuration monitoring

**Crate:** `bb-shim-windows`
**File:** `src/dns_monitor.rs`

- [ ] **Step 14:** Define `DnsMonitorError` enum: `Win32Error`, `MonitoringFailed`, `EnforcementFailed`.
- [ ] **Step 15:** Define `DnsChange` struct: `{ adapter_name: String, old_servers: Vec<IpAddr>, new_servers: Vec<IpAddr>, timestamp: chrono::DateTime<Utc> }`.
- [ ] **Step 16:** Write `DnsMonitor` struct with `new(expected_dns: IpAddr) -> Self`. Holds the expected DNS server address (agent's resolver) and a callback channel for changes.
- [ ] **Step 17:** Write `get_current_dns_servers() -> Result<HashMap<String, Vec<IpAddr>>>` using `GetAdaptersAddresses` (or `GetAdaptersInfo`) to enumerate all adapters and their DNS server lists.
- [ ] **Step 18:** Write `start_monitoring(shutdown: watch::Receiver<bool>) -> JoinHandle` that:
  - Calls `NotifyIpInterfaceChange` to register a callback for interface changes
  - On callback, re-reads DNS servers via `get_current_dns_servers()`
  - Compares against expected, emits `DnsChange` events on mismatch
  - Falls back to polling `get_current_dns_servers()` every 30s if callback registration fails
- [ ] **Step 19:** Write `enforce_dns(adapter: &str, dns_server: IpAddr) -> Result<()>` that sets the adapter's DNS back to the agent's resolver via `netsh interface ip set dns`. This is the remediation action.
- [ ] **Step 20:** Write tests: test `get_current_dns_servers()` returns non-empty on Windows CI. Test `DnsChange` serialization. Test enforcement logic with mock command executor.

### Task 5: DNS redirect via Windows Firewall / netsh

**Crate:** `bb-agent-windows`
**File:** `src/dns_redirect.rs`

- [ ] **Step 21:** Write `WindowsDnsRedirect` struct (analogous to `NftablesManager` in Linux). Fields: `resolver_port: u16`, `rules_installed: bool`.
- [ ] **Step 22:** Write `install_rules()` that creates Windows Firewall rules via `netsh advfirewall` to redirect DNS (port 53) traffic to the local resolver. Uses `netsh` commands as subprocess (same pattern as nftables).
- [ ] **Step 23:** Write `remove_rules()` and `verify_and_repair()` mirroring the `NftablesManager` pattern.
- [ ] **Step 24:** Write tests: test rule generation strings, test idempotent install/remove. Integration test gated behind `#[cfg(test_windows_firewall)]`.

---

## Chunk 3: File/Registry ACLs + TPM/DPAPI Key Storage (~120 lines)

### Task 6: File and registry ACL management

**Crate:** `bb-shim-windows`
**File:** `src/acl.rs`

- [ ] **Step 25:** Define `AclError` enum: `Win32Error`, `InvalidPath`, `AccessDenied`, `SecurityDescriptorFailed`.
- [ ] **Step 26:** Write `set_restrictive_file_acl(path: &Path) -> Result<()>` that:
  - Builds a DACL granting SYSTEM full control, Administrators read+execute, Users read+execute
  - Adds an explicit DENY ACE for DELETE and WRITE_DAC for non-SYSTEM principals
  - Applies via `SetNamedSecurityInfoW` with `SE_FILE_OBJECT`
- [ ] **Step 27:** Write `set_restrictive_directory_acl(dir: &Path) -> Result<()>` that applies the same DACL recursively with `CONTAINER_INHERIT_ACE | OBJECT_INHERIT_ACE` flags.
- [ ] **Step 28:** Write `protect_registry_key(hive: HKEY, subkey: &str) -> Result<()>` that sets a restrictive DACL on a registry key via `RegSetKeySecurity`. Grant SYSTEM full control, deny write for others.
- [ ] **Step 29:** Write `verify_acl(path: &Path) -> Result<bool>` that reads current DACL via `GetNamedSecurityInfoW` and compares against expected. Returns false if ACL has been weakened.
- [ ] **Step 30:** Write tests: test DACL construction produces valid security descriptor bytes. Test `verify_acl` detects weakened permissions. Integration test (gated) on a temp directory.

### Task 7: TPM/DPAPI key storage

**Crate:** `bb-shim-windows`
**File:** `src/keystore.rs`

- [ ] **Step 31:** Define `KeystoreError` enum: `TpmNotAvailable`, `DpapiError`, `SerializationError`, `Win32Error`.
- [ ] **Step 32:** Define `WindowsKeystore` struct implementing the `CertificateStore` trait pattern from `bb-agent-core::comms::certificate`. Fields: `data_dir: PathBuf`, `use_tpm: bool`.
- [ ] **Step 33:** Write `store_key(name: &str, key_bytes: &[u8]) -> Result<()>` that:
  - Tries TPM first via `NCryptCreatePersistedKey` + `NCryptFinalizeKey` with `NCRYPT_MACHINE_KEY_FLAG`
  - Falls back to DPAPI via `CryptProtectData` with `CRYPTPROTECT_LOCAL_MACHINE` flag, writing to `data_dir/{name}.dpapi`
- [ ] **Step 34:** Write `load_key(name: &str) -> Result<Vec<u8>>` with matching retrieval logic (TPM `NCryptOpenKey` + `NCryptExportKey`, or DPAPI `CryptUnprotectData`).
- [ ] **Step 35:** Write `delete_key(name: &str) -> Result<()>` for cleanup.
- [ ] **Step 36:** Write `has_tpm() -> bool` helper that checks TPM availability via `NCryptOpenStorageProvider` with `MS_PLATFORM_CRYPTO_PROVIDER`.
- [ ] **Step 37:** Write tests: test DPAPI round-trip (store + load) on Windows CI. Test TPM detection (should return true/false without panic). Test fallback from TPM to DPAPI when TPM unavailable.

---

## Chunk 4: bb-agent-windows Binary Crate (~130 lines)

### Task 8: Crate scaffolding

**Crate:** `bb-agent-windows`
**Files:** `Cargo.toml`, `src/main.rs`, `src/platform.rs`

- [ ] **Step 38:** Create `crates/bb-agent-windows/Cargo.toml` with dependencies: `bb-common`, `bb-proto`, `bb-agent-core`, `bb-agent-plugins`, `bb-shim-windows`, `tokio`, `tracing`, `tracing-subscriber` (with `tracing-etw` or Windows Event Log feature), `clap`, `thiserror`, `serde_json`, `chrono`, `windows-service`.
- [ ] **Step 39:** Add to workspace members in root `Cargo.toml`.

### Task 9: Platform bridge

**File:** `src/platform.rs`

- [ ] **Step 40:** Write `read_machine_id() -> String` that reads `HKLM\SOFTWARE\Microsoft\Cryptography\MachineGuid` via `RegGetValueW`. Fallback to hostname.
- [ ] **Step 41:** Write `ensure_directories() -> Result<()>` creating `C:\ProgramData\BetBlocker\`, `C:\ProgramData\BetBlocker\certs\`, `C:\ProgramData\BetBlocker\logs\`. Apply restrictive ACLs via `bb_shim_windows::acl::set_restrictive_directory_acl()`.
- [ ] **Step 42:** Write `service_notify_ready()`, `service_notify_stopping()`, `service_notify_status(status: &str)` that update service status via `bb_shim_windows::service::set_service_status()`. These mirror the `sd_notify_*` functions from the Linux platform bridge.
- [ ] **Step 43:** Write tests for `read_machine_id()` (returns non-empty string), `ensure_directories()` (creates dirs in temp location for test).

### Task 10: Agent main entrypoint

**File:** `src/main.rs`

- [ ] **Step 44:** Define `Cli` struct with `clap::Parser`: `--config-dir` (default `C:\ProgramData\BetBlocker`), `--enroll <token>`, `--config <path>`, `--install-service` flag, `--uninstall-service` flag.
- [ ] **Step 45:** Write `main()` that:
  - Parses CLI args
  - If `--install-service`: call `bb_shim_windows::service::register_service()` + `set_failure_actions()`, then exit
  - If `--uninstall-service`: call `bb_shim_windows::service::unregister_service()`, then exit
  - Otherwise: call `bb_shim_windows::service::run_as_service(run)` to enter SCM dispatch
- [ ] **Step 46:** Write `run(shutdown_rx: watch::Receiver<bool>)` mirroring `bb-agent-linux/src/main.rs::run()`:
  - Phase 1 Setup: `ensure_directories()`, load config, init event store, init cert store (using `WindowsKeystore`), binary integrity check
  - Phase 2 Registration: same flow as Linux (enrollment or existing device ID)
  - Phase 3 Subsystems: plugin registry, DNS redirect (`WindowsDnsRedirect`), watchdog, heartbeat, event reporter, binary integrity loop, DNS redirect verify loop, DNS monitor
  - Phase 4 Wait: await shutdown signal, cleanup, deactivate plugins
- [ ] **Step 47:** Write `setup_tracing()` that initializes tracing with Windows Event Log output (via `tracing-subscriber` with a custom layer) in addition to stderr.
- [ ] **Step 48:** Write integration test (gated) that verifies agent starts in non-service mode (for development), loads config, and shuts down cleanly on Ctrl+C.

---

## Chunk 5: MSI Installer + Auto-Update (~110 lines)

### Task 11: MSI installer helpers

**Crate:** `bb-shim-windows`
**File:** `src/installer.rs`

- [ ] **Step 49:** Define `InstallerError` enum: `WixNotFound`, `BuildFailed`, `ServiceRegistrationFailed`, `AclSetupFailed`.
- [ ] **Step 50:** Write `post_install() -> Result<()>` for MSI custom action: create data directories, set ACLs, register the service, set failure actions, start the service. This is called by the MSI after file copy.
- [ ] **Step 51:** Write `pre_uninstall() -> Result<()>`: stop the service, unregister the service, remove firewall rules. Leave data directory for potential re-install.
- [ ] **Step 52:** Write `is_installed() -> bool` checking if the BetBlocker service exists via `OpenServiceW`.
- [ ] **Step 53:** Write `get_installed_version() -> Option<String>` reading version from registry key `HKLM\SOFTWARE\BetBlocker\Version`.

### Task 12: WiX installer definition

**File:** `deploy/windows/betblocker.wxs`

- [ ] **Step 54:** Create WiX XML defining: Product (BetBlocker, upgrade GUID), Directory structure (`ProgramFiles\BetBlocker`), Component (agent binary, config template), ServiceInstall + ServiceControl elements, custom actions calling `post_install()` / `pre_uninstall()`.
- [ ] **Step 55:** Add `cargo-wix` configuration to `bb-agent-windows/Cargo.toml` or create a build script that invokes `cargo wix` with the `.wxs` file.

### Task 13: Auto-update mechanism

**Crate:** `bb-shim-windows`
**File:** `src/updater.rs`

- [ ] **Step 56:** Define `UpdateError` enum: `DownloadFailed`, `VerificationFailed`, `InstallFailed`, `RollbackFailed`.
- [ ] **Step 57:** Define `UpdateInfo` struct: `{ version: String, download_url: String, sha256: String, release_notes: String }`.
- [ ] **Step 58:** Write `check_for_update(api_client: &ApiClient, current_version: &str) -> Result<Option<UpdateInfo>>` that queries the API `/v1/agent/updates?platform=windows&current={version}`.
- [ ] **Step 59:** Write `download_and_verify(info: &UpdateInfo, dest: &Path) -> Result<()>` that downloads the MSI, verifies SHA-256 hash, and verifies Authenticode signature via `WinVerifyTrust`.
- [ ] **Step 60:** Write `apply_update(msi_path: &Path) -> Result<()>` that:
  - Spawns `msiexec /i <path> /qn /norestart REINSTALL=ALL REINSTALLMODE=vomus` as a detached process
  - The MSI upgrade handles service stop/start via ServiceControl elements
  - Agent exits after spawning msiexec; the new version starts via service auto-restart
- [ ] **Step 61:** Write `UpdateScheduler` that runs `check_for_update()` periodically (default every 6 hours), respects a configurable update window, and applies updates during low-activity periods.
- [ ] **Step 62:** Write tests: test version comparison logic, test `UpdateInfo` deserialization, test SHA-256 verification against known hash. Mock API client for `check_for_update` test.

---

## Chunk 6: WFP Callout Driver + Minifilter Design (Wave 2, Deferred) (~100 lines)

> **Note:** These tasks are Wave 2 -- they depend on the Wave 1 service lifecycle being complete. The C driver code requires WDK and WHQL signing. This chunk covers the Rust IOCTL interface and C driver stubs only.

### Task 14: WFP IOCTL interface (Rust side)

**Crate:** `bb-shim-windows`
**File:** `src/wfp.rs` (behind `kernel-drivers` feature)

- [ ] **Step 63:** Define IOCTL codes as constants: `IOCTL_WFP_ADD_BLOCKED_DOMAIN`, `IOCTL_WFP_REMOVE_BLOCKED_DOMAIN`, `IOCTL_WFP_CLEAR_BLOCKLIST`, `IOCTL_WFP_GET_STATS`, `IOCTL_WFP_SET_DNS_REDIRECT`. Use `CTL_CODE` macro pattern (DeviceType=FILE_DEVICE_UNKNOWN, Function=0x800+, Method=METHOD_BUFFERED, Access=FILE_ANY_ACCESS).
- [ ] **Step 64:** Define `WfpDriverClient` struct holding a device handle (`HANDLE`) to `\\.\BetBlockerWfp`.
- [ ] **Step 65:** Write `WfpDriverClient::open() -> Result<Self>` using `CreateFileW` to open the device.
- [ ] **Step 66:** Write `send_ioctl<T: Pod>(code: u32, input: &T) -> Result<Vec<u8>>` generic helper wrapping `DeviceIoControl`.
- [ ] **Step 67:** Write typed methods: `add_blocked_domain(domain: &str)`, `remove_blocked_domain(domain: &str)`, `clear_blocklist()`, `get_stats() -> WfpStats`, `set_dns_redirect(port: u16)`.
- [ ] **Step 68:** Write tests with a mock device handle. Test IOCTL code values match expected bit patterns. Test serialization of domain strings for IOCTL buffers.

### Task 15: WFP callout driver stub (C)

**File:** `crates/bb-shim-windows/driver/betblocker_wfp.c`

- [ ] **Step 69:** Write `DriverEntry` + `DriverUnload` stubs. Register WFP callout via `FwpmCalloutAdd0`. Register classify function that inspects DNS packets.
- [ ] **Step 70:** Write device object creation (`IoCreateDevice`) and IOCTL dispatch (`IRP_MJ_DEVICE_CONTROL`) skeleton. Parse IOCTL codes, dispatch to handler stubs.
- [ ] **Step 71:** Write INF file (`betblocker_wfp.inf`) for driver installation. Document WDK build commands and WHQL submission process in comments.

### Task 16: Minifilter IOCTL interface (Rust side)

**Crate:** `bb-shim-windows`
**File:** `src/minifilter.rs` (behind `kernel-drivers` feature)

- [ ] **Step 72:** Define IOCTL codes: `IOCTL_MF_GET_STATUS`, `IOCTL_MF_ADD_PROTECTED_PATH`, `IOCTL_MF_REMOVE_PROTECTED_PATH`, `IOCTL_MF_SET_UPDATE_TOKEN` (allows the updater to temporarily bypass protection).
- [ ] **Step 73:** Write `MinifilterClient` struct with `open()`, `add_protected_path(path: &Path)`, `remove_protected_path(path: &Path)`, `get_status() -> MinifilterStatus`, `set_update_token(token: &[u8; 32])`.
- [ ] **Step 74:** Write tests with mock device handle. Test path serialization (wide strings for Win32).

### Task 17: Minifilter driver stub (C)

**File:** `crates/bb-shim-windows/minifilter/betblocker_mf.c`

- [ ] **Step 75:** Write `DriverEntry` registering minifilter via `FltRegisterFilter`. Define pre-operation callbacks for `IRP_MJ_CREATE`, `IRP_MJ_SET_INFORMATION` (rename/delete), `IRP_MJ_WRITE`.
- [ ] **Step 76:** Write path-matching logic stub: compare target file path against protected paths list. Deny access if path is protected and caller is not the agent (check by process token or update token).
- [ ] **Step 77:** Write INF file (`betblocker_mf.inf`). Document altitude allocation request process (Microsoft requires a unique altitude for minifilters).

---

## Definition of Done

- [ ] `bb-shim-windows` compiles on Windows with all Wave 1 modules (`service`, `dns_monitor`, `acl`, `keystore`, `installer`, `updater`)
- [ ] `bb-agent-windows` runs as a Windows Service: registers with SCM, handles stop/shutdown, auto-restarts on failure
- [ ] DNS monitoring detects and remediates DNS configuration changes
- [ ] Agent directory and registry keys are protected with restrictive DACLs
- [ ] Device certificates stored via DPAPI (with TPM upgrade path)
- [ ] MSI installer installs, registers, and starts the service
- [ ] Auto-update checks API, downloads, verifies, and applies MSI updates
- [ ] All code has unit tests; integration tests gated behind feature flags
- [ ] Wave 2: WFP and minifilter IOCTL interfaces compile; C driver stubs build with WDK
- [ ] CI: Windows runner builds `bb-agent-windows` and runs unit tests
