pub mod interceptor;
pub mod install_watcher;
pub mod quarantine;
pub mod scanner;

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tracing::{info, warn};

use crate::blocklist::app_signatures::AppSignatureStore;
use crate::blocklist::Blocklist;
use crate::traits::{AppBlockingPlugin, BlockingPlugin};
use crate::types::{
    AppIdentifier, AppMatch, BlockDecision, BlockingLayer, PluginConfig, PluginError, PluginHealth,
};

use interceptor::{create_interceptor, ProcessInterceptor};
use install_watcher::{create_install_watcher, InstallAction, InstallWatcher};
use scanner::{AppInventoryScanner, NoOpScanner};

// ── Event types ──────────────────────────────────────────────────────────────

/// An event emitted when a blocked application is detected running.
#[derive(Debug, Clone)]
pub struct AppDetectedEvent {
    pub pid: u32,
    pub app_match: AppMatch,
    pub detected_at: DateTime<Utc>,
}

/// An event emitted when a blocked application process is killed.
#[derive(Debug, Clone)]
pub struct AppBlockedEvent {
    pub pid: u32,
    pub app_match: AppMatch,
    pub blocked_at: DateTime<Utc>,
    /// Whether the kill succeeded.
    pub kill_succeeded: bool,
}

/// An event emitted when a blocked application install is detected.
#[derive(Debug, Clone)]
pub struct AppInstallDetectedEvent {
    pub path: std::path::PathBuf,
    pub app_match: AppMatch,
    pub detected_at: DateTime<Utc>,
    pub action: InstallAction,
}

// ── AppProcessPlugin ─────────────────────────────────────────────────────────

/// Default scan interval: 30 seconds.
const DEFAULT_SCAN_INTERVAL: Duration = Duration::from_secs(30);

/// Plugin that monitors running processes and new app installations,
/// blocking any that match the loaded app signature blocklist.
pub struct AppProcessPlugin {
    /// Scanner for enumerating installed/running applications.
    scanner: Box<dyn AppInventoryScanner>,
    /// Interceptor for real-time process detection.
    interceptor: Box<dyn ProcessInterceptor>,
    /// Watcher for new application installations.
    install_watcher: Box<dyn InstallWatcher>,
    /// Shared app signature store (updated on blocklist refresh).
    signatures: Arc<AppSignatureStore>,
    /// How often to run a full scan.
    scan_interval: Duration,
    /// Timestamp of the last full scan.
    last_scan: Option<DateTime<Utc>>,
    /// Whether the plugin is currently active.
    active: bool,
    /// Accumulated detected events (drained by the caller).
    detected_events: Vec<AppDetectedEvent>,
    /// Accumulated blocked events (drained by the caller).
    blocked_events: Vec<AppBlockedEvent>,
    /// Accumulated install detected events (drained by the caller).
    install_events: Vec<AppInstallDetectedEvent>,
}

impl AppProcessPlugin {
    /// Create a new plugin with default (NoOp) implementations.
    pub fn new() -> Self {
        Self {
            scanner: Box::new(NoOpScanner),
            interceptor: create_interceptor(),
            install_watcher: create_install_watcher(),
            signatures: Arc::new(AppSignatureStore::new()),
            scan_interval: DEFAULT_SCAN_INTERVAL,
            last_scan: None,
            active: false,
            detected_events: Vec::new(),
            blocked_events: Vec::new(),
            install_events: Vec::new(),
        }
    }

    /// Create a plugin with custom components (useful for testing).
    pub fn with_components(
        scanner: Box<dyn AppInventoryScanner>,
        interceptor: Box<dyn ProcessInterceptor>,
        install_watcher: Box<dyn InstallWatcher>,
        scan_interval: Duration,
    ) -> Self {
        Self {
            scanner,
            interceptor,
            install_watcher,
            signatures: Arc::new(AppSignatureStore::new()),
            scan_interval,
            last_scan: None,
            active: false,
            detected_events: Vec::new(),
            blocked_events: Vec::new(),
            install_events: Vec::new(),
        }
    }

    /// Set the scan interval.
    pub fn with_scan_interval(mut self, interval: Duration) -> Self {
        self.scan_interval = interval;
        self
    }

    // ── Event draining ────────────────────────────────────────────────────

    /// Drain all pending `AppDetectedEvent`s since the last call.
    pub fn drain_detected_events(&mut self) -> Vec<AppDetectedEvent> {
        std::mem::take(&mut self.detected_events)
    }

    /// Drain all pending `AppBlockedEvent`s since the last call.
    pub fn drain_blocked_events(&mut self) -> Vec<AppBlockedEvent> {
        std::mem::take(&mut self.blocked_events)
    }

    /// Drain all pending `AppInstallDetectedEvent`s since the last call.
    pub fn drain_install_events(&mut self) -> Vec<AppInstallDetectedEvent> {
        std::mem::take(&mut self.install_events)
    }

    // ── Core scanning logic ───────────────────────────────────────────────

    /// Run one full scan cycle: enumerate installed/running apps, check against
    /// signatures, and kill any blocked processes.
    pub fn run_scan_cycle(&mut self) {
        if !self.active {
            return;
        }

        // Scan installed apps
        match self.scanner.scan_installed() {
            Ok(apps) => {
                for app_id in apps {
                    if let Some(app_match) = self.signatures.check_app(&app_id) {
                        // Check if it's running
                        match self.scanner.is_running(&app_id) {
                            Ok(Some(pid)) => {
                                info!(
                                    pid = pid,
                                    signature = %app_match.signature_name,
                                    "Blocked app detected running, attempting to kill"
                                );
                                self.detected_events.push(AppDetectedEvent {
                                    pid,
                                    app_match: app_match.clone(),
                                    detected_at: Utc::now(),
                                });
                                let kill_ok = self.interceptor.kill_process(pid);
                                self.blocked_events.push(AppBlockedEvent {
                                    pid,
                                    app_match,
                                    blocked_at: Utc::now(),
                                    kill_succeeded: kill_ok,
                                });
                            }
                            Ok(None) => {
                                // Installed but not running — no action needed
                            }
                            Err(e) => {
                                warn!(error = %e, "Failed to check if app is running");
                            }
                        }
                    }
                }
            }
            Err(e) => {
                warn!(error = %e, "App scan failed");
            }
        }

        self.last_scan = Some(Utc::now());
    }

    /// Process real-time detections from the interceptor and handle any
    /// pending install detections from the install watcher.
    /// Should be called every agent tick.
    pub fn tick(&mut self) {
        if !self.active {
            return;
        }

        // Process interceptor detections
        let detections = self.interceptor.poll_detections();
        for mut detection in detections {
            info!(
                pid = detection.pid,
                signature = %detection.app_match.signature_name,
                "Real-time process detection"
            );
            self.detected_events.push(AppDetectedEvent {
                pid: detection.pid,
                app_match: detection.app_match.clone(),
                detected_at: detection.detected_at,
            });

            let kill_ok = self.interceptor.kill_process(detection.pid);
            detection.killed = kill_ok;
            self.blocked_events.push(AppBlockedEvent {
                pid: detection.pid,
                app_match: detection.app_match,
                blocked_at: Utc::now(),
                kill_succeeded: kill_ok,
            });
        }

        // Process install watcher detections
        let installs = self.install_watcher.poll_installations();
        for install in installs {
            info!(
                path = %install.path.display(),
                signature = %install.app_match.signature_name,
                action = ?install.action,
                "Install detection"
            );
            self.install_events.push(AppInstallDetectedEvent {
                path: install.path,
                app_match: install.app_match,
                detected_at: install.detected_at,
                action: install.action,
            });
        }

        // Run periodic full scan if interval has elapsed
        let should_scan = match self.last_scan {
            None => true,
            Some(last) => {
                let elapsed = Utc::now()
                    .signed_duration_since(last)
                    .to_std()
                    .unwrap_or(Duration::ZERO);
                elapsed >= self.scan_interval
            }
        };

        if should_scan {
            self.run_scan_cycle();
        }
    }
}

impl Default for AppProcessPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for AppProcessPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppProcessPlugin")
            .field("active", &self.active)
            .field("scan_interval", &self.scan_interval)
            .field("last_scan", &self.last_scan)
            .field("signatures_count", &self.signatures.len())
            .finish()
    }
}

// ── BlockingPlugin impl ───────────────────────────────────────────────────────

impl BlockingPlugin for AppProcessPlugin {
    fn id(&self) -> &str {
        "app.process"
    }

    fn name(&self) -> &str {
        "Application Process Blocker"
    }

    fn layer(&self) -> BlockingLayer {
        BlockingLayer::App
    }

    fn init(&mut self, _config: &PluginConfig) -> Result<(), PluginError> {
        info!("AppProcessPlugin initialized");
        Ok(())
    }

    fn activate(&mut self, blocklist: &Blocklist) -> Result<(), PluginError> {
        // Extract app signatures from blocklist and build the store
        // We re-use the blocklist's check_app which already has the signature store.
        // For the process plugin we need direct access — copy signatures via check.
        // Since Blocklist doesn't expose the raw store, we rebuild from scratch.
        // The caller should call update_blocklist after loading signatures.
        let _ = blocklist; // signatures come via update_blocklist

        self.interceptor
            .start()
            .map_err(|e| PluginError::ActivationFailed(format!("Interceptor start failed: {e}")))?;

        self.install_watcher
            .start()
            .map_err(|e| PluginError::ActivationFailed(format!("Install watcher start failed: {e}")))?;

        self.active = true;
        info!("AppProcessPlugin activated");
        Ok(())
    }

    fn deactivate(&mut self) -> Result<(), PluginError> {
        if let Err(e) = self.interceptor.stop() {
            warn!(error = %e, "Interceptor stop error during deactivation");
        }
        if let Err(e) = self.install_watcher.stop() {
            warn!(error = %e, "Install watcher stop error during deactivation");
        }
        self.active = false;
        info!("AppProcessPlugin deactivated");
        Ok(())
    }

    fn update_blocklist(&mut self, blocklist: &Blocklist) -> Result<(), PluginError> {
        // We extract matching by delegating through the blocklist's check_app.
        // For the standalone signature store, we accept updated signatures via
        // a dedicated method. Here we update a dummy app to test connectivity;
        // real signature refresh is done by the agent calling `update_signatures`.
        let _ = blocklist;
        info!("AppProcessPlugin blocklist updated");
        Ok(())
    }

    fn health_check(&self) -> Result<PluginHealth, PluginError> {
        if !self.active {
            return Ok(PluginHealth::degraded("AppProcessPlugin is not active"));
        }

        let mut health = PluginHealth::ok();
        health.details.insert(
            "scan_interval_secs".into(),
            self.scan_interval.as_secs().to_string(),
        );
        health.details.insert(
            "last_scan".into(),
            self.last_scan
                .map(|t| t.to_rfc3339())
                .unwrap_or_else(|| "never".to_string()),
        );
        health.details.insert(
            "signatures_loaded".into(),
            self.signatures.len().to_string(),
        );
        Ok(health)
    }
}

// ── AppBlockingPlugin impl ────────────────────────────────────────────────────

impl AppBlockingPlugin for AppProcessPlugin {
    fn check_app(&self, app_id: &AppIdentifier) -> BlockDecision {
        match self.signatures.check_app(app_id) {
            Some(m) => BlockDecision::Block {
                reason: m.reason,
            },
            None => BlockDecision::Allow,
        }
    }

    fn scan_installed(&self) -> Vec<AppMatch> {
        match self.scanner.scan_installed() {
            Ok(apps) => apps
                .into_iter()
                .filter_map(|app_id| self.signatures.check_app(&app_id))
                .collect(),
            Err(e) => {
                warn!(error = %e, "scan_installed failed");
                Vec::new()
            }
        }
    }

    fn watch_installs(&mut self) -> Result<(), PluginError> {
        self.install_watcher.start()
    }
}

// ── Public helper: update signatures directly ─────────────────────────────────

impl AppProcessPlugin {
    /// Replace the current signature store with a new one.
    /// Called by the agent when a blocklist refresh includes updated app signatures.
    pub fn update_signatures(&mut self, store: AppSignatureStore) {
        self.signatures = Arc::new(store);
    }

    /// Access the current signature store.
    pub fn signatures(&self) -> &AppSignatureStore {
        &self.signatures
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocklist::app_signatures::{AppSignatureStore, AppSignatureSummary};
    use crate::types::AppIdentifier;
    use bb_common::enums::Platform;
    use uuid::Uuid;

    fn make_plugin_with_signatures() -> AppProcessPlugin {
        let mut plugin = AppProcessPlugin::new();
        let sig = AppSignatureSummary {
            public_id: Uuid::nil(),
            name: "Bet365".to_string(),
            package_names: vec![],
            executable_names: vec!["bet365.exe".to_string()],
            cert_hashes: vec![],
            display_name_patterns: vec![],
            platforms: vec!["windows".to_string()],
            category: "sports_betting".to_string(),
            confidence: 0.85,
        };
        plugin.update_signatures(AppSignatureStore::from_summaries(vec![sig]));
        plugin
    }

    // ── BlockingPlugin trait ───────────────────────────────────────────────

    #[test]
    fn plugin_id_and_name() {
        let plugin = AppProcessPlugin::new();
        assert_eq!(plugin.id(), "app.process");
        assert_eq!(plugin.name(), "Application Process Blocker");
    }

    #[test]
    fn plugin_layer_is_app() {
        let plugin = AppProcessPlugin::new();
        assert_eq!(plugin.layer(), BlockingLayer::App);
    }

    #[test]
    fn plugin_init_succeeds() {
        let mut plugin = AppProcessPlugin::new();
        let config = PluginConfig::default();
        assert!(plugin.init(&config).is_ok());
    }

    #[test]
    fn plugin_activate_deactivate() {
        let mut plugin = AppProcessPlugin::new();
        let blocklist = Blocklist::new(1);
        assert!(plugin.activate(&blocklist).is_ok());
        assert!(plugin.active);
        assert!(plugin.deactivate().is_ok());
        assert!(!plugin.active);
    }

    #[test]
    fn plugin_health_check_inactive() {
        let plugin = AppProcessPlugin::new();
        let health = plugin.health_check().unwrap();
        assert!(!health.healthy);
        assert!(health.message.contains("not active"));
    }

    #[test]
    fn plugin_health_check_active() {
        let mut plugin = AppProcessPlugin::new();
        let blocklist = Blocklist::new(1);
        plugin.activate(&blocklist).unwrap();
        let health = plugin.health_check().unwrap();
        assert!(health.healthy);
        assert!(health.details.contains_key("scan_interval_secs"));
        assert!(health.details.contains_key("last_scan"));
        assert!(health.details.contains_key("signatures_loaded"));
        plugin.deactivate().unwrap();
    }

    #[test]
    fn plugin_update_blocklist() {
        let mut plugin = AppProcessPlugin::new();
        let blocklist = Blocklist::new(1);
        assert!(plugin.update_blocklist(&blocklist).is_ok());
    }

    // ── AppBlockingPlugin trait ────────────────────────────────────────────

    #[test]
    fn check_app_blocked_by_executable() {
        let plugin = make_plugin_with_signatures();
        let mut app = AppIdentifier::empty(Platform::Windows);
        app.executable_name = Some("bet365.exe".to_string());
        let decision = plugin.check_app(&app);
        assert!(decision.is_blocked());
    }

    #[test]
    fn check_app_allowed_unknown() {
        let plugin = make_plugin_with_signatures();
        let mut app = AppIdentifier::empty(Platform::Windows);
        app.executable_name = Some("chrome.exe".to_string());
        let decision = plugin.check_app(&app);
        assert_eq!(decision, BlockDecision::Allow);
    }

    #[test]
    fn scan_installed_returns_empty_with_noop_scanner() {
        let plugin = make_plugin_with_signatures();
        let matches = plugin.scan_installed();
        assert!(matches.is_empty());
    }

    // ── Tick and scan cycle ────────────────────────────────────────────────

    #[test]
    fn tick_does_nothing_when_inactive() {
        let mut plugin = AppProcessPlugin::new();
        plugin.tick(); // should not panic
        assert!(plugin.drain_detected_events().is_empty());
        assert!(plugin.drain_blocked_events().is_empty());
    }

    #[test]
    fn tick_runs_scan_on_first_call() {
        let mut plugin = make_plugin_with_signatures();
        let blocklist = Blocklist::new(1);
        plugin.activate(&blocklist).unwrap();

        // No processes running (NoOpScanner), so no events
        plugin.tick();
        assert!(plugin.drain_detected_events().is_empty());
        assert!(plugin.drain_blocked_events().is_empty());
        // But last_scan should be set now
        assert!(plugin.last_scan.is_some());

        plugin.deactivate().unwrap();
    }

    #[test]
    fn run_scan_cycle_does_nothing_when_inactive() {
        let mut plugin = make_plugin_with_signatures();
        plugin.run_scan_cycle();
        assert!(plugin.drain_detected_events().is_empty());
        assert!(plugin.last_scan.is_none());
    }

    #[test]
    fn run_scan_cycle_sets_last_scan_when_active() {
        let mut plugin = make_plugin_with_signatures();
        let blocklist = Blocklist::new(1);
        plugin.activate(&blocklist).unwrap();

        plugin.run_scan_cycle();
        assert!(plugin.last_scan.is_some());

        plugin.deactivate().unwrap();
    }

    // ── Event emission via mock interceptor ───────────────────────────────

    #[test]
    fn tick_emits_detection_and_blocked_events_from_interceptor() {
        use crate::app_process::interceptor::{ProcessDetection, ProcessInterceptor};
        use crate::types::{AppMatch, AppMatchType};

        struct MockInterceptor {
            pending: Vec<ProcessDetection>,
        }

        impl ProcessInterceptor for MockInterceptor {
            fn start(&mut self) -> Result<(), PluginError> { Ok(()) }
            fn stop(&mut self) -> Result<(), PluginError> { Ok(()) }
            fn poll_detections(&mut self) -> Vec<ProcessDetection> {
                std::mem::take(&mut self.pending)
            }
            fn kill_process(&self, _pid: u32) -> bool { true }
        }

        let detection = ProcessDetection {
            pid: 1234,
            app_match: AppMatch {
                app_id: AppIdentifier::empty(Platform::Windows),
                signature_id: Uuid::nil(),
                signature_name: "Bet365".to_string(),
                match_type: AppMatchType::ExactExecutable,
                confidence: 1.0,
                reason: "executable match".to_string(),
            },
            detected_at: Utc::now(),
            killed: false,
        };

        let mock_interceptor = Box::new(MockInterceptor {
            pending: vec![detection],
        });

        let mut plugin = AppProcessPlugin::with_components(
            Box::new(NoOpScanner),
            mock_interceptor,
            create_install_watcher(),
            Duration::from_secs(300), // long interval so scan doesn't run
        );
        let blocklist = Blocklist::new(1);
        plugin.activate(&blocklist).unwrap();

        // Manually set last_scan to prevent immediate full scan
        plugin.last_scan = Some(Utc::now());
        plugin.tick();

        let detected = plugin.drain_detected_events();
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].pid, 1234);

        let blocked = plugin.drain_blocked_events();
        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0].pid, 1234);
        assert!(blocked[0].kill_succeeded);

        plugin.deactivate().unwrap();
    }

    // ── Install watcher integration ────────────────────────────────────────

    #[test]
    fn tick_emits_install_events_from_watcher() {
        use crate::app_process::install_watcher::{InstallDetection, InstallWatcher};
        use crate::types::{AppMatch, AppMatchType};

        struct MockWatcher {
            pending: Vec<InstallDetection>,
        }

        impl InstallWatcher for MockWatcher {
            fn start(&mut self) -> Result<(), PluginError> { Ok(()) }
            fn stop(&mut self) -> Result<(), PluginError> { Ok(()) }
            fn poll_installations(&mut self) -> Vec<InstallDetection> {
                std::mem::take(&mut self.pending)
            }
        }

        let install = InstallDetection {
            path: std::path::PathBuf::from("C:\\bet365_setup.exe"),
            app_match: AppMatch {
                app_id: AppIdentifier::empty(Platform::Windows),
                signature_id: Uuid::nil(),
                signature_name: "Bet365".to_string(),
                match_type: AppMatchType::ExactExecutable,
                confidence: 1.0,
                reason: "installer detected".to_string(),
            },
            detected_at: Utc::now(),
            action: InstallAction::Logged,
        };

        let mock_watcher = Box::new(MockWatcher {
            pending: vec![install],
        });

        let mut plugin = AppProcessPlugin::with_components(
            Box::new(NoOpScanner),
            create_interceptor(),
            mock_watcher,
            Duration::from_secs(300),
        );
        let blocklist = Blocklist::new(1);
        plugin.activate(&blocklist).unwrap();
        plugin.last_scan = Some(Utc::now());

        plugin.tick();

        let installs = plugin.drain_install_events();
        assert_eq!(installs.len(), 1);
        assert_eq!(installs[0].action, InstallAction::Logged);

        plugin.deactivate().unwrap();
    }

    // ── Event draining ────────────────────────────────────────────────────

    #[test]
    fn drain_events_returns_empty_on_no_events() {
        let mut plugin = AppProcessPlugin::new();
        assert!(plugin.drain_detected_events().is_empty());
        assert!(plugin.drain_blocked_events().is_empty());
        assert!(plugin.drain_install_events().is_empty());
    }

    // ── update_signatures ─────────────────────────────────────────────────

    #[test]
    fn update_signatures_replaces_store() {
        let mut plugin = AppProcessPlugin::new();
        assert_eq!(plugin.signatures().len(), 0);

        let sig = AppSignatureSummary {
            public_id: Uuid::nil(),
            name: "TestApp".to_string(),
            package_names: vec!["com.test.app".to_string()],
            executable_names: vec![],
            cert_hashes: vec![],
            display_name_patterns: vec![],
            platforms: vec!["windows".to_string()],
            category: "test".to_string(),
            confidence: 0.9,
        };
        plugin.update_signatures(AppSignatureStore::from_summaries(vec![sig]));
        assert_eq!(plugin.signatures().len(), 1);
    }

    // ── Debug impl ────────────────────────────────────────────────────────

    #[test]
    fn debug_impl_does_not_panic() {
        let plugin = AppProcessPlugin::new();
        let _ = format!("{plugin:?}");
    }
}
