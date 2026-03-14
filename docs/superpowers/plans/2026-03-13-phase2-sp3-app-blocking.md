# Phase 2 Sub-Plan 3: Application Blocking (Layer 2)

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Detect, block, and prevent installation of gambling applications across desktop platforms (Windows, macOS, Linux). Includes app signature model, matching engine, process monitoring, install prevention, and admin UI.

**Architecture:** Extends `bb-agent-plugins` with `AppProcessPlugin` implementing `AppBlockingPlugin` trait. App signatures stored in PostgreSQL, synced to agents via blocklist delta protocol. Platform-specific scanners, process interceptors, and install watchers behind a cross-platform trait abstraction.

**Tech Stack:** Rust, sysinfo, notify (fs watcher), Win32 `CreateToolhelp32Snapshot`/`WMI`, macOS `NSWorkspace`/`FSEvents`, Linux `/proc` + inotify, PostgreSQL, TypeScript/Next.js (admin UI)

**Depends on:** Phase 1 complete (plugin system, blocklist sync, event pipeline)

**Reference Docs:**
- Phase 2 Design (Section 1): `docs/plans/2026-03-13-phase2-design.md`
- Master Plan: `docs/superpowers/plans/2026-03-13-phase2-master-plan.md`
- Existing traits: `crates/bb-agent-plugins/src/traits.rs`
- Existing types: `crates/bb-agent-plugins/src/types.rs`
- Registry: `crates/bb-agent-plugins/src/registry.rs`
- Blocklist engine: `crates/bb-agent-plugins/src/blocklist/mod.rs`
- Common enums: `crates/bb-common/src/enums.rs`
- Common models: `crates/bb-common/src/models/`
- API routes: `crates/bb-api/src/routes/`
- API services: `crates/bb-api/src/services/`

**Scope exclusions:** Android `AppDeviceAdminPlugin` is covered in SP7 (Mobile/Tamper). iOS relies on MDM + DNS blocking. This plan covers desktop only.

---

## File Structure

```
crates/
  bb-common/src/
    models/
      app_signature.rs              # AppSignature model
      mod.rs                        # (update: add app_signature)
    enums.rs                        # (update: add AppSignatureStatus)
  bb-agent-plugins/
    Cargo.toml                      # (update: add app-process feature + deps)
    src/
      lib.rs                        # (update: re-export app_process)
      types.rs                      # (update: flesh out AppIdentifier, AppMatch)
      registry.rs                   # (update: add AppProcess variant + check_app)
      blocklist/
        mod.rs                      # (update: add app_signatures field)
        app_signatures.rs           # App signature matching engine
      app_process/
        mod.rs                      # AppProcessPlugin struct + BlockingPlugin/AppBlockingPlugin impl
        scanner.rs                  # AppInventoryScanner trait + per-platform impls
        interceptor.rs              # ProcessInterceptor trait + per-platform impls
        install_watcher.rs          # InstallWatcher trait + per-platform impls
  bb-api/src/
    routes/
      admin_app_signatures.rs       # CRUD endpoints for app signatures
      mod.rs                        # (update: mount app signature routes)
    services/
      app_signature_service.rs      # Business logic
      mod.rs                        # (update: add module)
packages/
  bb-web/src/app/admin/
    app-signatures/
      page.tsx                      # App signature list page
      [id]/page.tsx                 # Edit page
      new/page.tsx                  # Create page
      components/
        AppSignatureForm.tsx         # Shared form component
        AppSignatureTable.tsx        # Table with search/filter
migrations/
  YYYYMMDDHHMMSS_create_app_signatures.sql
  YYYYMMDDHHMMSS_seed_app_signatures.sql
```

---

## Chunk 1: AppSignature Model, Database, and API (Tasks 1-5)

### Task 1: AppSignature model in bb-common

**Files:** `crates/bb-common/src/models/app_signature.rs`, `crates/bb-common/src/models/mod.rs`, `crates/bb-common/src/enums.rs`

- [ ] **Step 1:** Add `AppSignatureStatus` enum to `crates/bb-common/src/enums.rs`: variants `Active`, `Inactive`, `PendingReview`
- [ ] **Step 2:** Add `AppSignaturePlatform` enum to `crates/bb-common/src/enums.rs`: variants `Windows`, `Macos`, `Linux`, `Android`, `Ios`, `All`
- [ ] **Step 3:** Create `crates/bb-common/src/models/app_signature.rs` with the `AppSignature` struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSignature {
    pub id: i64,
    pub public_id: Uuid,
    pub name: String,                           // Human-readable name, e.g. "Bet365"
    pub package_names: Vec<String>,             // e.g. ["com.bet365.app"]
    pub executable_names: Vec<String>,          // e.g. ["bet365.exe", "Bet365.app"]
    pub cert_hashes: Vec<String>,              // SHA-256 of code-signing certs
    pub display_name_patterns: Vec<String>,    // Fuzzy match patterns (lowercase)
    pub platforms: Vec<AppSignaturePlatform>,
    pub category: GamblingCategory,
    pub status: AppSignatureStatus,
    pub confidence: f64,                       // 0.0-1.0, used for fuzzy match threshold
    pub source: BlocklistSource,
    pub evidence_url: Option<String>,
    pub tags: Vec<String>,
    pub blocklist_version_added: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

- [ ] **Step 4:** Add `AppSignatureSummary` struct (for delta sync -- omits metadata fields, only matching-relevant fields)
- [ ] **Step 5:** Update `crates/bb-common/src/models/mod.rs` to add `pub mod app_signature;` and re-export `AppSignature`
- [ ] **Step 6:** Write unit tests: serialize/deserialize roundtrip, default values

### Task 2: Database migration

**File:** `migrations/YYYYMMDDHHMMSS_create_app_signatures.sql`

- [ ] **Step 1:** Create migration with `app_signatures` table: columns match `AppSignature` fields. Use `TEXT[]` for array fields (package_names, executable_names, cert_hashes, display_name_patterns, platforms, tags). Add GIN indexes on `package_names` and `executable_names` for fast lookups.
- [ ] **Step 2:** Add `blocklist_version_added` FK to `blocklist_versions(version_number)`
- [ ] **Step 3:** Add index on `(status, platforms)` for filtered queries
- [ ] **Step 4:** Run migration locally, verify schema

### Task 3: API CRUD endpoints for app signatures

**Files:** `crates/bb-api/src/routes/admin_app_signatures.rs`, `crates/bb-api/src/services/app_signature_service.rs`

- [ ] **Step 1:** Create `crates/bb-api/src/services/app_signature_service.rs` with functions: `create_signature`, `get_signature`, `list_signatures`, `update_signature`, `delete_signature`, `list_active_for_platforms`. Follow pattern from `blocklist_service.rs`.
- [ ] **Step 2:** Create `crates/bb-api/src/routes/admin_app_signatures.rs` with request/response types:
  - `CreateAppSignatureRequest` (name, package_names, executable_names, cert_hashes, display_name_patterns, platforms, category, evidence_url, tags)
  - `UpdateAppSignatureRequest` (all optional)
  - `AppSignatureFilters` (search, category, platform, status)
- [ ] **Step 3:** Implement route handlers: `POST /admin/app-signatures`, `GET /admin/app-signatures` (paginated + filtered), `GET /admin/app-signatures/:id`, `PUT /admin/app-signatures/:id`, `DELETE /admin/app-signatures/:id`. All require `RequireAdmin` extractor.
- [ ] **Step 4:** Update `crates/bb-api/src/routes/mod.rs` to mount app signature routes under admin router
- [ ] **Step 5:** Update `crates/bb-api/src/services/mod.rs` to add `pub mod app_signature_service;`
- [ ] **Step 6:** Write integration tests: CRUD happy path, validation (name required, at least one identifier), auth (non-admin rejected), pagination, filtering

### Task 4: Extend blocklist delta sync for app signatures

**Files:** `crates/bb-common/src/models/blocklist.rs`, `crates/bb-api/src/services/blocklist_service.rs`

- [ ] **Step 1:** Add `AppSignatureDeltaEntry` to `crates/bb-common/src/models/blocklist.rs` (subset of AppSignature fields needed for matching)
- [ ] **Step 2:** Add `app_added: Vec<AppSignatureDeltaEntry>` and `app_removed: Vec<Uuid>` fields to `BlocklistDelta`
- [ ] **Step 3:** Update `blocklist_service` delta generation to include app signature changes since `from_version`
- [ ] **Step 4:** Write tests: delta includes newly added/removed app signatures, empty delta when no changes

### Task 5: Seed data for common gambling apps

**File:** `migrations/YYYYMMDDHHMMSS_seed_app_signatures.sql`

- [ ] **Step 1:** Create seed migration inserting signatures for top gambling apps. Include at minimum:
  - Bet365 (package: `com.bet365.app`, exe: `bet365.exe`, `Bet365.app`)
  - PokerStars (package: `com.pokerstars.app`, exe: `PokerStarsUpdate.exe`)
  - DraftKings (package: `com.draftkings.sportsbook`)
  - FanDuel (package: `com.fanduel.sportsbook`)
  - BetMGM (package: `com.betmgm.casino`)
  - William Hill (package: `com.williamhill.app`)
  - Paddy Power (package: `com.paddypower.sportsbook`)
  - 888poker, PartyPoker, Unibet, Betfair, Ladbrokes, Coral, SkyBet, Bwin
- [ ] **Step 2:** Each entry should include display name patterns for fuzzy matching (e.g., `["bet365", "bet 365"]`)
- [ ] **Step 3:** Set `status = 'active'`, `source = 'curated'`, `confidence = 1.0`

---

## Chunk 2: App Signature Matching Engine (Tasks 6-7)

### Task 6: Matching engine core

**File:** `crates/bb-agent-plugins/src/blocklist/app_signatures.rs`

- [ ] **Step 1:** Create `AppSignatureStore` struct holding `Vec<AppSignatureSummary>` loaded from blocklist sync. Include `HashMap<String, Vec<usize>>` indexes for O(1) package name and executable name lookups.
- [ ] **Step 2:** Implement `AppSignatureStore::from_summaries(sigs: Vec<AppSignatureSummary>) -> Self` that builds the indexes
- [ ] **Step 3:** Implement `check_package_name(&self, name: &str) -> Option<AppMatch>` -- exact match (case-insensitive) against `package_names`
- [ ] **Step 4:** Implement `check_executable(&self, exe_name: &str) -> Option<AppMatch>` -- exact match (case-insensitive) against `executable_names`
- [ ] **Step 5:** Implement `check_cert_hash(&self, hash: &str) -> Option<AppMatch>` -- exact match against `cert_hashes`
- [ ] **Step 6:** Implement `check_display_name(&self, display_name: &str) -> Option<AppMatch>` -- fuzzy match using normalized Levenshtein distance against `display_name_patterns`. Return match only if similarity >= signature's `confidence` threshold (default 0.85).
- [ ] **Step 7:** Implement `check_app(&self, app_id: &AppIdentifier) -> Option<AppMatch>` that tries all checks in order: cert_hash > package_name > executable > display_name. Returns first match found with match reason.
- [ ] **Step 8:** Write tests: exact matches for each field, fuzzy display name matching (e.g., "Bet 365 Sportsbook" matches pattern "bet365"), no false positive for "beta365tool", case insensitivity, empty store returns None

### Task 7: Integrate AppSignatureStore into Blocklist

**Files:** `crates/bb-agent-plugins/src/blocklist/mod.rs`

- [ ] **Step 1:** Add `pub mod app_signatures;` to blocklist mod.rs
- [ ] **Step 2:** Add `app_sigs: AppSignatureStore` field to `Blocklist` struct. Initialize as empty in `Blocklist::new()`.
- [ ] **Step 3:** Add `Blocklist::update_app_signatures(&mut self, sigs: Vec<AppSignatureSummary>)` method
- [ ] **Step 4:** Add `Blocklist::check_app(&self, app_id: &AppIdentifier) -> Option<AppMatch>` delegating to `app_sigs`
- [ ] **Step 5:** Write tests: blocklist with app signatures blocks matching apps, update replaces old signatures

---

## Chunk 3: Flesh Out Types and App Inventory Scanner (Tasks 8-10)

### Task 8: Update AppIdentifier and AppMatch types

**File:** `crates/bb-agent-plugins/src/types.rs`

- [ ] **Step 1:** Expand `AppIdentifier` to include all matchable fields:

```rust
pub struct AppIdentifier {
    pub package_name: Option<String>,
    pub executable_path: Option<String>,
    pub executable_name: Option<String>,
    pub display_name: Option<String>,
    pub cert_hash: Option<String>,
    pub pid: Option<u32>,
    pub platform: Platform,
}
```

- [ ] **Step 2:** Expand `AppMatch` to include signature reference:

```rust
pub struct AppMatch {
    pub app_id: AppIdentifier,
    pub signature_id: Uuid,
    pub signature_name: String,
    pub match_type: AppMatchType,  // ExactPackage, ExactExecutable, CertHash, FuzzyDisplayName
    pub confidence: f64,
    pub reason: String,
}
```

- [ ] **Step 3:** Add `AppMatchType` enum: `ExactPackage`, `ExactExecutable`, `CertHash`, `FuzzyDisplayName`
- [ ] **Step 4:** Write tests for Display impl, serialization roundtrip

### Task 9: Cross-platform AppInventoryScanner trait

**File:** `crates/bb-agent-plugins/src/app_process/scanner.rs`

- [ ] **Step 1:** Define the trait:

```rust
pub trait AppInventoryScanner: Send + Sync {
    /// Enumerate all installed applications on this system.
    fn scan_installed(&self) -> Result<Vec<AppIdentifier>, PluginError>;
    /// Check if a specific app is currently running. Returns PID if found.
    fn is_running(&self, app_id: &AppIdentifier) -> Result<Option<u32>, PluginError>;
}
```

- [ ] **Step 2:** Implement `WindowsScanner` (behind `#[cfg(target_os = "windows")]`):
  - `scan_installed`: Read `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall` and `HKCU\...` registry keys. Extract DisplayName, InstallLocation, Publisher. Also scan `%ProgramFiles%`, `%ProgramFiles(x86)%`, `%LocalAppData%\Programs` for `.exe` files.
  - `is_running`: Use `CreateToolhelp32Snapshot` + `Process32First`/`Process32Next` to enumerate processes, match by executable name.
- [ ] **Step 3:** Implement `MacosScanner` (behind `#[cfg(target_os = "macos")]`):
  - `scan_installed`: Use `system_profiler SPApplicationsDataType -json` or scan `/Applications`, `~/Applications`, `/usr/local/bin` for `.app` bundles. Parse `Info.plist` for `CFBundleIdentifier` (package name) and `CFBundleDisplayName`.
  - `is_running`: Use `sysinfo` crate or shell out to `pgrep`.
- [ ] **Step 4:** Implement `LinuxScanner` (behind `#[cfg(target_os = "linux")]`):
  - `scan_installed`: Parse `dpkg --list` output, check `/usr/share/applications/*.desktop` files for Exec= paths, scan snap/flatpak via their CLI tools.
  - `is_running`: Read `/proc/*/comm` and `/proc/*/exe` symlinks.
- [ ] **Step 5:** Add `pub fn create_scanner() -> Box<dyn AppInventoryScanner>` factory function that returns the platform-appropriate impl.
- [ ] **Step 6:** Write tests using mock scanner: verify scan results feed into matching engine correctly. Platform-specific tests behind `#[cfg(test)]` + `#[cfg(target_os = ...)]` for live system scanning.

### Task 10: Code signing certificate extraction

**File:** `crates/bb-agent-plugins/src/app_process/scanner.rs` (extend platform impls)

- [ ] **Step 1:** Windows: Use `WinVerifyTrust` API to extract Authenticode certificate, compute SHA-256 hash of the signing certificate's public key
- [ ] **Step 2:** macOS: Use `codesign --display --verbose=4` to extract certificate chain, hash the leaf cert
- [ ] **Step 3:** Linux: Check for GPG signatures on `.deb`/`.rpm` packages (lower priority -- most Linux gambling apps are rare)
- [ ] **Step 4:** Add `extract_cert_hash(exe_path: &Path) -> Option<String>` to each platform scanner
- [ ] **Step 5:** Write tests: known signed binary returns expected hash, unsigned binary returns None

---

## Chunk 4: AppProcessPlugin and Launch Interception (Tasks 11-14)

### Task 11: AppProcessPlugin struct and BlockingPlugin impl

**File:** `crates/bb-agent-plugins/src/app_process/mod.rs`

- [ ] **Step 1:** Create module file `crates/bb-agent-plugins/src/app_process/mod.rs` with `pub mod scanner; pub mod interceptor; pub mod install_watcher;`
- [ ] **Step 2:** Define `AppProcessPlugin` struct:

```rust
pub struct AppProcessPlugin {
    scanner: Box<dyn AppInventoryScanner>,
    interceptor: Option<Box<dyn ProcessInterceptor>>,
    install_watcher: Option<Box<dyn InstallWatcher>>,
    scan_interval: Duration,       // default 15 min
    last_scan: Option<Instant>,
    active: bool,
}
```

- [ ] **Step 3:** Implement `BlockingPlugin` for `AppProcessPlugin`:
  - `id()` -> `"app.process"`, `name()` -> `"Application Process Blocker"`, `layer()` -> `BlockingLayer::App`
  - `init()`: create platform scanner, validate config settings (`scan_interval_secs` from PluginConfig)
  - `activate()`: run initial scan, start interceptor and install watcher
  - `deactivate()`: stop interceptor and install watcher
  - `update_blocklist()`: update the scanner's signature store from `blocklist.app_sigs`
  - `health_check()`: verify scanner, interceptor, and watcher are alive
- [ ] **Step 4:** Implement `AppBlockingPlugin` for `AppProcessPlugin`:
  - `check_app()`: delegate to blocklist's `check_app()`
  - `scan_installed()`: delegate to scanner, filter through matching engine
  - `watch_installs()`: start the install watcher

### Task 12: ProcessInterceptor trait and platform impls

**File:** `crates/bb-agent-plugins/src/app_process/interceptor.rs`

- [ ] **Step 1:** Define trait:

```rust
pub trait ProcessInterceptor: Send + Sync {
    /// Start monitoring for new process creation events.
    fn start(&mut self, sigs: Arc<AppSignatureStore>) -> Result<(), PluginError>;
    /// Stop monitoring.
    fn stop(&mut self) -> Result<(), PluginError>;
    /// Poll for detected gambling processes since last poll. Non-blocking.
    fn poll_detections(&mut self) -> Vec<ProcessDetection>;
    /// Kill a process by PID.
    fn kill_process(&self, pid: u32) -> Result<(), PluginError>;
}

pub struct ProcessDetection {
    pub pid: u32,
    pub app_match: AppMatch,
    pub detected_at: DateTime<Utc>,
    pub killed: bool,
}
```

- [ ] **Step 2:** Implement `WindowsProcessInterceptor`:
  - Use WMI `Win32_ProcessStartTrace` event subscription (via `wmi` crate) to get notified of new process creation. On each event, resolve the executable path, check against signature store, kill if matched via `TerminateProcess`.
  - Fallback: periodic polling with `CreateToolhelp32Snapshot` every 2 seconds.
- [ ] **Step 3:** Implement `MacosProcessInterceptor`:
  - Use `NSWorkspace.didLaunchApplicationNotification` via objc bridge, or poll `sysinfo::System::refresh_processes()` every 2 seconds.
  - Kill via `kill(pid, SIGTERM)` then `SIGKILL` after 1s grace.
- [ ] **Step 4:** Implement `LinuxProcessInterceptor`:
  - Use `netlink` connector (`PROC_EVENT_FORK`, `PROC_EVENT_EXEC`) for real-time process events. Fallback to polling `/proc` every 2 seconds.
  - Kill via `kill(pid, SIGTERM)` then `SIGKILL`.
- [ ] **Step 5:** Add `pub fn create_interceptor() -> Box<dyn ProcessInterceptor>` factory function.
- [ ] **Step 6:** Write tests: mock interceptor detects and kills a process, detection list accumulates correctly, stop prevents further detections.

### Task 13: Event emission for app blocking

**File:** `crates/bb-agent-plugins/src/app_process/mod.rs` (extend AppProcessPlugin)

- [ ] **Step 1:** Add `AppDetected` and `AppBlocked` to `EventType` enum in `crates/bb-common/src/enums.rs`
- [ ] **Step 2:** In `AppProcessPlugin`, after scan detects a match: emit `EventType::AppDetected` with app details (signature name, match type, confidence)
- [ ] **Step 3:** After successful process kill: emit `EventType::Block` with `EventCategory::App` and details (PID, executable path, signature name)
- [ ] **Step 4:** Write tests: verify events emitted on detection and kill

### Task 14: Periodic scan loop

**File:** `crates/bb-agent-plugins/src/app_process/mod.rs`

- [ ] **Step 1:** Add `async fn run_scan_cycle(&mut self) -> Vec<AppMatch>` method that: calls `scanner.scan_installed()`, filters through matching engine, kills any running matched apps via interceptor, emits events for each detection.
- [ ] **Step 2:** Add `async fn tick(&mut self)` method called by the agent's main loop: checks if `scan_interval` has elapsed since `last_scan`, runs scan cycle if so. Also calls `interceptor.poll_detections()` to process any real-time detections.
- [ ] **Step 3:** Write tests: scan cycle detects and kills matched apps, respects scan interval, real-time detections processed on every tick.

---

## Chunk 5: Install Prevention Monitors (Tasks 15-17)

### Task 15: InstallWatcher trait and platform impls

**File:** `crates/bb-agent-plugins/src/app_process/install_watcher.rs`

- [ ] **Step 1:** Define trait:

```rust
pub trait InstallWatcher: Send + Sync {
    /// Start watching for new application installations.
    fn start(&mut self, sigs: Arc<AppSignatureStore>) -> Result<(), PluginError>;
    /// Stop watching.
    fn stop(&mut self) -> Result<(), PluginError>;
    /// Poll for detected installations since last poll. Non-blocking.
    fn poll_installations(&mut self) -> Vec<InstallDetection>;
}

pub struct InstallDetection {
    pub path: PathBuf,
    pub app_match: Option<AppMatch>,
    pub detected_at: DateTime<Utc>,
    pub action_taken: InstallAction,
}

pub enum InstallAction {
    Blocked,    // File deleted or install prevented
    Logged,     // Could not block, logged for review
    Quarantined, // Moved to quarantine directory
}
```

- [ ] **Step 2:** Implement `WindowsInstallWatcher`:
  - Watch directories: `%ProgramFiles%`, `%ProgramFiles(x86)%`, `%LocalAppData%\Programs`, `%AppData%`, `%TEMP%` (for installer staging)
  - Use `ReadDirectoryChangesW` via the `notify` crate with `RecursiveMode::Recursive`
  - On new `.exe`/`.msi` file: extract cert hash and display name, check against sigs, delete if matched
- [ ] **Step 3:** Implement `MacosInstallWatcher`:
  - Watch `/Applications`, `~/Applications`, `~/Downloads` (for `.dmg`/`.pkg` files)
  - Use FSEvents via `notify` crate
  - On new `.app` bundle: parse `Info.plist`, check bundle ID and display name against sigs, move to quarantine if matched
- [ ] **Step 4:** Implement `LinuxInstallWatcher`:
  - Watch `/usr/bin`, `/usr/local/bin`, `/snap`, `/var/lib/flatpak`, `~/.local/share/applications`
  - Use inotify via `notify` crate
  - Also monitor D-Bus `org.freedesktop.PackageKit` signals for package install events
  - On match: log event (deletion risky on Linux -- package manager state corruption)
- [ ] **Step 5:** Add `pub fn create_install_watcher() -> Box<dyn InstallWatcher>` factory function.
- [ ] **Step 6:** Write tests: mock watcher detects new file in watched directory, matched file triggers block action, unmatched file ignored.

### Task 16: Quarantine directory management

**File:** `crates/bb-agent-plugins/src/app_process/install_watcher.rs` (extend)

- [ ] **Step 1:** Define quarantine directory path per platform: Windows `%ProgramData%\BetBlocker\quarantine`, macOS `/Library/Application Support/BetBlocker/quarantine`, Linux `/var/lib/betblocker/quarantine`
- [ ] **Step 2:** Implement `quarantine_file(src: &Path) -> Result<PathBuf, PluginError>`: move file to quarantine dir with timestamp prefix, set restrictive permissions (owner-only read, no execute)
- [ ] **Step 3:** Implement `list_quarantined() -> Vec<QuarantinedApp>` and `delete_quarantined(path: &Path)` for admin cleanup
- [ ] **Step 4:** Write tests: file moved to quarantine, permissions set correctly, listing returns quarantined files

### Task 17: Install watcher integration with AppProcessPlugin

**File:** `crates/bb-agent-plugins/src/app_process/mod.rs`

- [ ] **Step 1:** In `activate()`, start install watcher if `watch_installs` config is enabled (default: true)
- [ ] **Step 2:** In `tick()`, call `install_watcher.poll_installations()` and emit events for each detection
- [ ] **Step 3:** Emit `EventType::AppDetected` with details including `InstallAction` taken
- [ ] **Step 4:** Write tests: install watcher detections processed during tick, events emitted

---

## Chunk 6: Plugin Registry Integration and Feature Flag (Tasks 18-20)

### Task 18: Add AppProcess variant to PluginInstance

**File:** `crates/bb-agent-plugins/src/registry.rs`

- [ ] **Step 1:** Add conditional import at top of registry.rs:

```rust
#[cfg(feature = "app-process")]
use crate::app_process::AppProcessPlugin;
```

- [ ] **Step 2:** Add variant to `PluginInstance` enum:

```rust
#[cfg(feature = "app-process")]
AppProcess(AppProcessPlugin),
```

- [ ] **Step 3:** Add `AppProcess` arm to both `dispatch_blocking!` and `dispatch_blocking_mut!` macros
- [ ] **Step 4:** Add `check_app()` method to `PluginInstance` (returns `BlockDecision::Abstain` for non-App plugins):

```rust
pub fn check_app(&self, app_id: &AppIdentifier) -> BlockDecision {
    match self {
        #[cfg(feature = "app-process")]
        PluginInstance::AppProcess(p) => p.check_app(app_id),
        _ => BlockDecision::Abstain,
    }
}
```

- [ ] **Step 5:** Add `is_app_plugin(&self) -> bool` helper method
- [ ] **Step 6:** Write test: `check_app` returns Abstain for DNS plugins, Block for AppProcess plugin with matching signature

### Task 19: Add check_app to PluginRegistry

**File:** `crates/bb-agent-plugins/src/registry.rs`

- [ ] **Step 1:** Add `check_app(&self, app_id: &AppIdentifier) -> BlockDecision` to `PluginRegistry`, following the `check_domain` pattern: iterate App-layer plugins, short-circuit on first Block.
- [ ] **Step 2:** Add `scan_all_apps(&self) -> Vec<AppMatch>` to `PluginRegistry`: collect scan results from all App-layer plugins.
- [ ] **Step 3:** Update `with_defaults()` to register `AppProcessPlugin` when `app-process` feature is enabled.
- [ ] **Step 4:** Write tests: registry with app plugin blocks matching apps, registry without app plugin allows all apps.

### Task 20: Feature flag and Cargo.toml updates

**Files:** `crates/bb-agent-plugins/Cargo.toml`, `crates/bb-agent-plugins/src/lib.rs`

- [ ] **Step 1:** Add feature flag to `Cargo.toml`:

```toml
[features]
default = ["dns-resolver", "dns-hosts"]
dns-resolver = []
dns-hosts = []
app-process = ["dep:sysinfo", "dep:notify", "dep:strsim"]

[dependencies]
sysinfo = { version = "0.32", optional = true }
notify = { version = "7", optional = true }
strsim = { version = "0.11", optional = true }   # For Levenshtein distance
```

- [ ] **Step 2:** Add conditional module declaration in `lib.rs`:

```rust
#[cfg(feature = "app-process")]
pub mod app_process;
```

- [ ] **Step 3:** Verify `cargo check --features app-process` compiles
- [ ] **Step 4:** Verify `cargo check` (default features, no app-process) still compiles -- ensure no unconditional references to app_process module

---

## Chunk 7: Admin UI for App Signature Management (Tasks 21-23)

### Task 21: App signature list page

**File:** `packages/bb-web/src/app/admin/app-signatures/page.tsx`

- [ ] **Step 1:** Create page component with data table showing: name, platforms (as badges), category, status, package_names (truncated), executable_names (truncated), created_at
- [ ] **Step 2:** Add search bar filtering by name, package name, or executable name (server-side via API `search` param)
- [ ] **Step 3:** Add filter dropdowns for category, platform, status
- [ ] **Step 4:** Add pagination controls (reuse existing `Pagination` component)
- [ ] **Step 5:** Add "New Signature" button linking to create page
- [ ] **Step 6:** Add row actions: Edit, Delete (with confirmation dialog)

### Task 22: App signature create/edit form

**Files:** `packages/bb-web/src/app/admin/app-signatures/new/page.tsx`, `packages/bb-web/src/app/admin/app-signatures/[id]/page.tsx`, `packages/bb-web/src/app/admin/app-signatures/components/AppSignatureForm.tsx`

- [ ] **Step 1:** Create `AppSignatureForm` component with fields: name (text), platforms (multi-select checkboxes), category (dropdown), status (dropdown), confidence (slider 0-1), evidence_url (url input), tags (tag input)
- [ ] **Step 2:** Add array fields with add/remove buttons: package_names, executable_names, cert_hashes, display_name_patterns. Each shows a list of text inputs with a "+" button to add more.
- [ ] **Step 3:** Add form validation: name required, at least one of package_names/executable_names/cert_hashes/display_name_patterns must be non-empty
- [ ] **Step 4:** Create page wraps form in create mode (POST to API)
- [ ] **Step 5:** Edit page loads existing signature, wraps form in edit mode (PUT to API)
- [ ] **Step 6:** Add success/error toast notifications on submit

### Task 23: Navigation and sidebar update

**File:** `packages/bb-web/src/app/admin/` (layout or sidebar component)

- [ ] **Step 1:** Add "App Signatures" link to admin sidebar navigation, under a "Blocking" section alongside existing "Blocklist" link
- [ ] **Step 2:** Add breadcrumbs to app signature pages
- [ ] **Step 3:** Write Playwright e2e test: navigate to app signatures page, create a new signature, verify it appears in the table, edit it, delete it

---

## Chunk 8: End-to-End Integration and Polish (Tasks 24-26)

### Task 24: Full integration test -- scan to block

**File:** `crates/bb-agent-plugins/tests/app_process_integration.rs` (new test file)

- [ ] **Step 1:** Write integration test (behind `#[cfg(feature = "app-process")]`): create `AppProcessPlugin`, load blocklist with app signatures, inject mock scanner that returns a known gambling app, verify `scan_installed()` returns the match
- [ ] **Step 2:** Write integration test: mock interceptor detects a process matching a signature, verify `tick()` triggers kill and emits events
- [ ] **Step 3:** Write integration test: mock install watcher detects new file, verify it is quarantined and event emitted
- [ ] **Step 4:** Write integration test: blocklist update propagates new signatures to scanner/interceptor/watcher

### Task 25: Blocklist sync integration

**File:** `crates/bb-agent-plugins/src/app_process/mod.rs` (extend)

- [ ] **Step 1:** In `update_blocklist()`, extract `AppSignatureSummary` list from updated blocklist, rebuild `AppSignatureStore`, push to interceptor and install watcher via `Arc::swap`
- [ ] **Step 2:** Write test: update_blocklist with new signatures makes previously-allowed app now blocked
- [ ] **Step 3:** Write test: update_blocklist removing a signature makes previously-blocked app now allowed

### Task 26: Documentation and config

- [ ] **Step 1:** Add `app-process` plugin configuration to agent config schema: `scan_interval_secs` (default 900), `watch_installs` (default true), `kill_on_detect` (default true), `quarantine_enabled` (default true)
- [ ] **Step 2:** Add config validation: `scan_interval_secs` minimum 60, maximum 86400
- [ ] **Step 3:** Update agent startup to conditionally create and register `AppProcessPlugin` based on feature flag and config

---

## Definition of Done

- [ ] `AppSignature` model exists in `bb-common` with all fields
- [ ] Database migration creates `app_signatures` table with indexes
- [ ] API CRUD endpoints functional with admin auth
- [ ] Seed data for 15+ common gambling apps loaded
- [ ] `AppSignatureStore` matching engine handles exact + fuzzy matching
- [ ] `AppInventoryScanner` implementations for Windows, macOS, Linux
- [ ] `ProcessInterceptor` implementations detect and kill gambling app processes
- [ ] `InstallWatcher` implementations detect and block/quarantine new installations
- [ ] `PluginInstance::AppProcess` variant with `app-process` feature flag
- [ ] `PluginRegistry::check_app()` method works end-to-end
- [ ] Admin UI for app signature CRUD with search, filter, pagination
- [ ] Events emitted for app detection, blocking, and install prevention
- [ ] All unit and integration tests pass
- [ ] `cargo check` passes with and without `app-process` feature
- [ ] Blocklist delta sync includes app signature changes
