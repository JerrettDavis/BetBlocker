/// Integration tests for the AppProcessPlugin covering end-to-end scenarios:
/// scan-to-block, interceptor kill flow, install watcher quarantine, blocklist
/// update propagation, and config validation.
///
/// All tests are gated behind the `app-process` feature flag.
#[cfg(feature = "app-process")]
mod app_process_integration {
    use std::time::Duration;

    use bb_agent_plugins::app_process::interceptor::{ProcessDetection, ProcessInterceptor};
    use bb_agent_plugins::app_process::install_watcher::{
        InstallAction, InstallDetection, InstallWatcher,
    };
    use bb_agent_plugins::app_process::scanner::AppInventoryScanner;
    use bb_agent_plugins::app_process::{
        AppProcessPlugin, DEFAULT_SCAN_INTERVAL_SECS, MAX_SCAN_INTERVAL_SECS,
        MIN_SCAN_INTERVAL_SECS,
    };
    use bb_agent_plugins::blocklist::app_signatures::{AppSignatureStore, AppSignatureSummary};
    use bb_agent_plugins::blocklist::Blocklist;
    use bb_agent_plugins::traits::{AppBlockingPlugin, BlockingPlugin};
    use bb_agent_plugins::types::{AppIdentifier, AppMatch, AppMatchType, PluginConfig, PluginError};
    use bb_common::enums::Platform;
    use chrono::Utc;
    use uuid::Uuid;

    // ── Helper factories ─────────────────────────────────────────────────────

    fn gambling_signature() -> AppSignatureSummary {
        AppSignatureSummary {
            public_id: Uuid::from_u128(1),
            name: "Bet365".to_string(),
            package_names: vec!["com.bet365.sportsbook".to_string()],
            executable_names: vec!["bet365.exe".to_string()],
            cert_hashes: vec![],
            display_name_patterns: vec!["bet365".to_string()],
            platforms: vec!["windows".to_string()],
            category: "sports_betting".to_string(),
            confidence: 0.85,
        }
    }

    fn pokerstars_signature() -> AppSignatureSummary {
        AppSignatureSummary {
            public_id: Uuid::from_u128(2),
            name: "PokerStars".to_string(),
            package_names: vec!["com.pokerstars.app".to_string()],
            executable_names: vec!["pokerstars.exe".to_string()],
            cert_hashes: vec![],
            display_name_patterns: vec!["pokerstars".to_string()],
            platforms: vec!["windows".to_string()],
            category: "poker".to_string(),
            confidence: 0.85,
        }
    }

    fn store_with_gambling() -> AppSignatureStore {
        AppSignatureStore::from_summaries(vec![gambling_signature()])
    }

    fn app_id_with_exe(exe: &str) -> AppIdentifier {
        let mut id = AppIdentifier::empty(Platform::Windows);
        id.executable_name = Some(exe.to_string());
        id
    }

    fn make_process_detection(pid: u32, exe: &str) -> ProcessDetection {
        ProcessDetection {
            pid,
            app_match: AppMatch {
                app_id: app_id_with_exe(exe),
                signature_id: Uuid::nil(),
                signature_name: "Bet365".to_string(),
                match_type: AppMatchType::ExactExecutable,
                confidence: 1.0,
                reason: "executable match".to_string(),
            },
            detected_at: Utc::now(),
            killed: false,
        }
    }

    fn make_install_detection(path: &str, action: InstallAction) -> InstallDetection {
        InstallDetection {
            path: std::path::PathBuf::from(path),
            app_match: AppMatch {
                app_id: app_id_with_exe("bet365_setup.exe"),
                signature_id: Uuid::nil(),
                signature_name: "Bet365".to_string(),
                match_type: AppMatchType::ExactExecutable,
                confidence: 1.0,
                reason: "installer detected".to_string(),
            },
            detected_at: Utc::now(),
            action,
        }
    }

    // ── Mock scanner ─────────────────────────────────────────────────────────

    /// Scanner that returns a preconfigured list of installed apps, one of which
    /// is "running" at the given PID.
    struct MockScanner {
        installed: Vec<AppIdentifier>,
        /// Maps executable_name -> pid for apps that should appear running.
        running: std::collections::HashMap<String, u32>,
    }

    impl MockScanner {
        fn new(installed: Vec<AppIdentifier>, running: Vec<(String, u32)>) -> Self {
            Self {
                installed,
                running: running.into_iter().collect(),
            }
        }
    }

    impl AppInventoryScanner for MockScanner {
        fn scan_installed(&self) -> Result<Vec<AppIdentifier>, PluginError> {
            Ok(self.installed.clone())
        }

        fn is_running(&self, app_id: &AppIdentifier) -> Result<Option<u32>, PluginError> {
            if let Some(exe) = &app_id.executable_name {
                if let Some(&pid) = self.running.get(exe.as_str()) {
                    return Ok(Some(pid));
                }
            }
            Ok(None)
        }
    }

    // ── Mock interceptor ─────────────────────────────────────────────────────

    struct MockInterceptor {
        pending: Vec<ProcessDetection>,
        killed_pids: std::sync::Arc<std::sync::Mutex<Vec<u32>>>,
    }

    impl MockInterceptor {
        fn new(pending: Vec<ProcessDetection>) -> (Self, std::sync::Arc<std::sync::Mutex<Vec<u32>>>) {
            let killed = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            let s = Self {
                pending,
                killed_pids: std::sync::Arc::clone(&killed),
            };
            (s, killed)
        }
    }

    impl ProcessInterceptor for MockInterceptor {
        fn start(&mut self) -> Result<(), PluginError> {
            Ok(())
        }
        fn stop(&mut self) -> Result<(), PluginError> {
            Ok(())
        }
        fn poll_detections(&mut self) -> Vec<ProcessDetection> {
            std::mem::take(&mut self.pending)
        }
        fn kill_process(&self, pid: u32) -> bool {
            self.killed_pids.lock().unwrap().push(pid);
            true
        }
    }

    // ── Mock install watcher ─────────────────────────────────────────────────

    struct MockInstallWatcher {
        pending: Vec<InstallDetection>,
    }

    impl MockInstallWatcher {
        fn new(pending: Vec<InstallDetection>) -> Self {
            Self { pending }
        }
    }

    impl InstallWatcher for MockInstallWatcher {
        fn start(&mut self) -> Result<(), PluginError> {
            Ok(())
        }
        fn stop(&mut self) -> Result<(), PluginError> {
            Ok(())
        }
        fn poll_installations(&mut self) -> Vec<InstallDetection> {
            std::mem::take(&mut self.pending)
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // SP3 Task 24: Full integration test — scan to block
    // ─────────────────────────────────────────────────────────────────────────

    /// T24-1: Create AppProcessPlugin, load blocklist with app signatures,
    /// inject mock scanner that returns a known gambling app, verify
    /// `run_scan_cycle()` returns the match.
    #[test]
    fn scan_cycle_detects_blocked_app_that_is_running() {
        // Set up scanner: bet365.exe is installed AND running at PID 4242
        let bet365 = app_id_with_exe("bet365.exe");
        let scanner = MockScanner::new(
            vec![bet365.clone()],
            vec![("bet365.exe".to_string(), 4242)],
        );
        let (interceptor, _killed) = MockInterceptor::new(vec![]);
        let watcher = MockInstallWatcher::new(vec![]);

        let mut plugin = AppProcessPlugin::with_components(
            Box::new(scanner),
            Box::new(interceptor),
            Box::new(watcher),
            Duration::from_secs(300),
        );
        plugin.update_signatures(store_with_gambling());

        let blocklist = Blocklist::new(1);
        plugin.activate(&blocklist).unwrap();
        // Prevent tick from triggering scan prematurely
        plugin.set_last_scan(Some(Utc::now()));

        // Run one explicit scan cycle
        plugin.run_scan_cycle();

        let detected = plugin.drain_detected_events();
        assert_eq!(detected.len(), 1, "expected one detection");
        assert_eq!(detected[0].pid, 4242);
        assert_eq!(detected[0].app_match.signature_name, "Bet365");

        let blocked = plugin.drain_blocked_events();
        assert_eq!(blocked.len(), 1, "expected one blocked event");
        assert_eq!(blocked[0].pid, 4242);
        assert!(blocked[0].kill_succeeded, "kill should have succeeded");

        plugin.deactivate().unwrap();
    }

    /// T24-1b: Scanner returns a known gambling app that is NOT running —
    /// no detection or blocked events should be emitted.
    #[test]
    fn scan_cycle_no_events_when_app_installed_but_not_running() {
        let bet365 = app_id_with_exe("bet365.exe");
        // Not in the running map → `is_running` returns None
        let scanner = MockScanner::new(vec![bet365], vec![]);
        let (interceptor, _killed) = MockInterceptor::new(vec![]);
        let watcher = MockInstallWatcher::new(vec![]);

        let mut plugin = AppProcessPlugin::with_components(
            Box::new(scanner),
            Box::new(interceptor),
            Box::new(watcher),
            Duration::from_secs(300),
        );
        plugin.update_signatures(store_with_gambling());

        let blocklist = Blocklist::new(1);
        plugin.activate(&blocklist).unwrap();
        plugin.set_last_scan(Some(Utc::now()));

        plugin.run_scan_cycle();

        assert!(plugin.drain_detected_events().is_empty());
        assert!(plugin.drain_blocked_events().is_empty());

        plugin.deactivate().unwrap();
    }

    /// T24-2: Mock interceptor detects a process matching a signature —
    /// verify `tick()` triggers kill and emits events.
    #[test]
    fn tick_interceptor_detection_triggers_kill_and_events() {
        let detection = make_process_detection(9999, "bet365.exe");
        let (interceptor, killed_pids) = MockInterceptor::new(vec![detection]);
        let watcher = MockInstallWatcher::new(vec![]);

        let mut plugin = AppProcessPlugin::with_components(
            Box::new(bb_agent_plugins::app_process::scanner::NoOpScanner),
            Box::new(interceptor),
            Box::new(watcher),
            Duration::from_secs(300),
        );
        plugin.update_signatures(store_with_gambling());

        let blocklist = Blocklist::new(1);
        plugin.activate(&blocklist).unwrap();
        // Prevent scan from running so only interceptor events are processed
        plugin.set_last_scan(Some(Utc::now()));

        plugin.tick();

        // Detection event emitted
        let detected = plugin.drain_detected_events();
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].pid, 9999);

        // Blocked event emitted with kill_succeeded = true
        let blocked = plugin.drain_blocked_events();
        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0].pid, 9999);
        assert!(blocked[0].kill_succeeded);

        // The mock interceptor actually recorded the kill
        let kills = killed_pids.lock().unwrap();
        assert!(kills.contains(&9999), "PID 9999 should have been killed");

        plugin.deactivate().unwrap();
    }

    /// T24-3: Mock install watcher detects a new file — verify quarantine action
    /// and event emitted.
    #[test]
    fn tick_install_watcher_detection_emits_install_event() {
        let install = make_install_detection("C:\\Users\\test\\bet365_setup.exe", InstallAction::Quarantined);
        let (interceptor, _) = MockInterceptor::new(vec![]);
        let watcher = MockInstallWatcher::new(vec![install]);

        let mut plugin = AppProcessPlugin::with_components(
            Box::new(bb_agent_plugins::app_process::scanner::NoOpScanner),
            Box::new(interceptor),
            Box::new(watcher),
            Duration::from_secs(300),
        );
        plugin.update_signatures(store_with_gambling());

        let blocklist = Blocklist::new(1);
        plugin.activate(&blocklist).unwrap();
        plugin.set_last_scan(Some(Utc::now()));

        plugin.tick();

        let installs = plugin.drain_install_events();
        assert_eq!(installs.len(), 1, "expected one install event");
        assert_eq!(installs[0].action, InstallAction::Quarantined);
        assert_eq!(
            installs[0].path,
            std::path::PathBuf::from("C:\\Users\\test\\bet365_setup.exe")
        );
        assert_eq!(installs[0].app_match.signature_name, "Bet365");

        plugin.deactivate().unwrap();
    }

    /// T24-3b: Blocked install action is reported correctly.
    #[test]
    fn tick_install_watcher_blocked_action_emits_event() {
        let install = make_install_detection("/tmp/pokerstars_setup.run", InstallAction::Blocked);
        let (interceptor, _) = MockInterceptor::new(vec![]);
        let watcher = MockInstallWatcher::new(vec![install]);

        let mut plugin = AppProcessPlugin::with_components(
            Box::new(bb_agent_plugins::app_process::scanner::NoOpScanner),
            Box::new(interceptor),
            Box::new(watcher),
            Duration::from_secs(300),
        );

        let blocklist = Blocklist::new(1);
        plugin.activate(&blocklist).unwrap();
        plugin.set_last_scan(Some(Utc::now()));

        plugin.tick();

        let installs = plugin.drain_install_events();
        assert_eq!(installs.len(), 1);
        assert_eq!(installs[0].action, InstallAction::Blocked);

        plugin.deactivate().unwrap();
    }

    /// T24-4: Blocklist update propagates new signatures — after calling
    /// `update_signatures` with a new store, a previously allowed app is now blocked.
    #[test]
    fn blocklist_update_propagates_new_signatures() {
        // Initially no signatures loaded
        let mut plugin = AppProcessPlugin::new();
        let blocklist = Blocklist::new(1);
        plugin.activate(&blocklist).unwrap();

        let app = app_id_with_exe("pokerstars.exe");

        // Before update: app is allowed
        assert!(
            !plugin.check_app(&app).is_blocked(),
            "should be allowed before signature update"
        );

        // After update: PokerStars signature added
        let new_store = AppSignatureStore::from_summaries(vec![pokerstars_signature()]);
        plugin.update_signatures(new_store);

        // Now the app should be blocked
        let decision = plugin.check_app(&app);
        assert!(decision.is_blocked(), "should be blocked after signature update");

        plugin.deactivate().unwrap();
    }

    // ─────────────────────────────────────────────────────────────────────────
    // SP3 Task 25: Blocklist sync integration
    // ─────────────────────────────────────────────────────────────────────────

    /// T25-1: update_blocklist with new signatures changes behavior —
    /// a scan cycle that previously found no match now detects one.
    #[test]
    fn update_blocklist_with_new_signatures_changes_scan_result() {
        // Scanner: pokerstars.exe is installed and running at PID 777
        let ps_app = app_id_with_exe("pokerstars.exe");
        let scanner = MockScanner::new(
            vec![ps_app.clone()],
            vec![("pokerstars.exe".to_string(), 777)],
        );
        let (interceptor, _) = MockInterceptor::new(vec![]);
        let watcher = MockInstallWatcher::new(vec![]);

        let mut plugin = AppProcessPlugin::with_components(
            Box::new(scanner),
            Box::new(interceptor),
            Box::new(watcher),
            Duration::from_secs(300),
        );
        // Start with only gambling signature (no pokerstars)
        plugin.update_signatures(store_with_gambling());

        let blocklist = Blocklist::new(1);
        plugin.activate(&blocklist).unwrap();
        plugin.set_last_scan(Some(Utc::now()));

        // First scan: pokerstars not in signatures → no events
        plugin.run_scan_cycle();
        assert!(
            plugin.drain_detected_events().is_empty(),
            "should not detect pokerstars before signature is loaded"
        );

        // Now add PokerStars signature
        let updated_store =
            AppSignatureStore::from_summaries(vec![gambling_signature(), pokerstars_signature()]);
        plugin.update_signatures(updated_store);

        // Reset last_scan so the next scan cycle runs
        plugin.set_last_scan(None);
        plugin.run_scan_cycle();

        let detected = plugin.drain_detected_events();
        assert_eq!(detected.len(), 1, "should detect pokerstars after signature update");
        assert_eq!(detected[0].app_match.signature_name, "PokerStars");

        plugin.deactivate().unwrap();
    }

    /// T25-2: Removing a signature allows a previously-blocked app.
    #[test]
    fn removing_signature_allows_previously_blocked_app() {
        let bet365 = app_id_with_exe("bet365.exe");
        let scanner = MockScanner::new(
            vec![bet365.clone()],
            vec![("bet365.exe".to_string(), 1234)],
        );
        let (interceptor, _) = MockInterceptor::new(vec![]);
        let watcher = MockInstallWatcher::new(vec![]);

        let mut plugin = AppProcessPlugin::with_components(
            Box::new(scanner),
            Box::new(interceptor),
            Box::new(watcher),
            Duration::from_secs(300),
        );
        // Initially: bet365 signature present → blocked
        plugin.update_signatures(store_with_gambling());

        let blocklist = Blocklist::new(1);
        plugin.activate(&blocklist).unwrap();
        plugin.set_last_scan(Some(Utc::now()));

        plugin.run_scan_cycle();
        assert_eq!(plugin.drain_detected_events().len(), 1, "bet365 should be detected initially");

        // Now remove the bet365 signature (replace with empty store)
        plugin.update_signatures(AppSignatureStore::new());

        // The app should no longer be blocked by check_app
        assert!(
            !plugin.check_app(&bet365).is_blocked(),
            "bet365 should be allowed after signature removal"
        );

        // Run another scan cycle — no events because signature is gone
        plugin.set_last_scan(None);
        plugin.run_scan_cycle();
        assert!(
            plugin.drain_detected_events().is_empty(),
            "no detection after signature removed"
        );

        plugin.deactivate().unwrap();
    }

    // ─────────────────────────────────────────────────────────────────────────
    // SP3 Task 26: Config validation
    // ─────────────────────────────────────────────────────────────────────────

    /// T26-1: Config constants have expected values.
    #[test]
    fn config_constants_have_expected_values() {
        assert_eq!(DEFAULT_SCAN_INTERVAL_SECS, 900);
        assert_eq!(MIN_SCAN_INTERVAL_SECS, 60);
        assert_eq!(MAX_SCAN_INTERVAL_SECS, 86400);
    }

    /// T26-2: validate_scan_interval accepts valid values.
    #[test]
    fn validate_scan_interval_accepts_valid_values() {
        // Boundary values
        assert!(AppProcessPlugin::validate_scan_interval(MIN_SCAN_INTERVAL_SECS).is_ok());
        assert!(AppProcessPlugin::validate_scan_interval(DEFAULT_SCAN_INTERVAL_SECS).is_ok());
        assert!(AppProcessPlugin::validate_scan_interval(MAX_SCAN_INTERVAL_SECS).is_ok());

        // Mid-range value
        assert!(AppProcessPlugin::validate_scan_interval(3600).is_ok());
    }

    /// T26-3: Config validation rejects below-minimum value.
    #[test]
    fn validate_scan_interval_rejects_below_minimum() {
        let result = AppProcessPlugin::validate_scan_interval(MIN_SCAN_INTERVAL_SECS - 1);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("minimum"),
            "error should mention minimum: {msg}"
        );
    }

    /// T26-4: Config validation rejects above-maximum value.
    #[test]
    fn validate_scan_interval_rejects_above_maximum() {
        let result = AppProcessPlugin::validate_scan_interval(MAX_SCAN_INTERVAL_SECS + 1);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("maximum"),
            "error should mention maximum: {msg}"
        );
    }

    /// T26-5: zero is rejected.
    #[test]
    fn validate_scan_interval_rejects_zero() {
        let result = AppProcessPlugin::validate_scan_interval(0);
        assert!(result.is_err());
    }

    /// T26-6: init() reads and validates scan_interval_secs from PluginConfig.
    #[test]
    fn init_applies_valid_scan_interval_from_config() {
        let mut plugin = AppProcessPlugin::new();
        let mut config = PluginConfig::default();
        config
            .settings
            .insert("scan_interval_secs".to_string(), serde_json::json!(300u64));
        assert!(plugin.init(&config).is_ok());
        // Internal interval should have been updated to 300 seconds
        // We verify indirectly: health_check after activate shows the value.
        let blocklist = Blocklist::new(1);
        plugin.activate(&blocklist).unwrap();
        let health = plugin.health_check().unwrap();
        assert_eq!(
            health.details.get("scan_interval_secs").map(|s| s.as_str()),
            Some("300")
        );
        plugin.deactivate().unwrap();
    }

    /// T26-7: init() rejects out-of-range scan_interval_secs from PluginConfig.
    #[test]
    fn init_rejects_out_of_range_scan_interval_from_config() {
        let mut plugin = AppProcessPlugin::new();
        let mut config = PluginConfig::default();
        config
            .settings
            .insert("scan_interval_secs".to_string(), serde_json::json!(10u64)); // below 60
        let result = plugin.init(&config);
        assert!(result.is_err(), "init should fail for interval below minimum");
    }

    /// T26-8: init() without scan_interval_secs uses the default.
    #[test]
    fn init_uses_default_interval_when_not_configured() {
        let mut plugin = AppProcessPlugin::new();
        let config = PluginConfig::default();
        assert!(plugin.init(&config).is_ok());
        let blocklist = Blocklist::new(1);
        plugin.activate(&blocklist).unwrap();
        let health = plugin.health_check().unwrap();
        assert_eq!(
            health.details.get("scan_interval_secs").map(|s| s.as_str()),
            Some(DEFAULT_SCAN_INTERVAL_SECS.to_string().as_str())
        );
        plugin.deactivate().unwrap();
    }

    // ─────────────────────────────────────────────────────────────────────────
    // SP7 Task 6 — MAC init integration (mocked, cross-platform)
    // ─────────────────────────────────────────────────────────────────────────
    //
    // The real MAC verification lives in bb-shim-linux and is tested there.
    // Here we add a lightweight integration smoke-test that verifies the
    // detect_mac_system function is callable and returns a sensible result
    // even when running on non-Linux CI.

    #[test]
    fn mac_detect_returns_none_on_non_linux_host() {
        use bb_shim_linux::mac::{detect_mac_system, MacSystem};
        let system = detect_mac_system();
        // On Windows / macOS (where CI typically runs) there is no AppArmor or
        // SELinux sysfs, so we must get None.  On Linux CI the answer will
        // depend on the host, so we just assert the call does not panic.
        #[cfg(not(target_os = "linux"))]
        assert_eq!(system, MacSystem::None);
        #[cfg(target_os = "linux")]
        {
            // Any valid MacSystem variant is acceptable on Linux.
            let _ = system;
        }
    }
}
