/// Action taken when a blocked application installation is detected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallAction {
    /// The install was blocked / rolled back.
    Blocked,
    /// The event was logged but no further action taken.
    Logged,
    /// The installer file was moved to quarantine.
    Quarantined,
}

/// A detected installation event for a blocked application.
#[derive(Debug, Clone)]
pub struct InstallDetection {
    /// Path to the installer or newly created application directory.
    pub path: std::path::PathBuf,
    /// The app match that triggered this detection.
    pub app_match: crate::types::AppMatch,
    /// When this detection was recorded.
    pub detected_at: chrono::DateTime<chrono::Utc>,
    /// Action taken (or recommended) for this detection.
    pub action: InstallAction,
}

/// Trait for watching the filesystem for new application installations.
pub trait InstallWatcher: Send + Sync {
    /// Start watching for installs.
    fn start(&mut self) -> Result<(), crate::types::PluginError>;

    /// Stop watching.
    fn stop(&mut self) -> Result<(), crate::types::PluginError>;

    /// Poll for newly detected installation events since the last call.
    fn poll_installations(&mut self) -> Vec<InstallDetection>;
}

// ── NoOp implementation ─────────────────────────────────────────────────────

/// A no-op watcher that never reports any installation events.
/// Used on all platforms until real filesystem watching is implemented.
pub struct NoOpInstallWatcher;

impl InstallWatcher for NoOpInstallWatcher {
    fn start(&mut self) -> Result<(), crate::types::PluginError> {
        Ok(())
    }

    fn stop(&mut self) -> Result<(), crate::types::PluginError> {
        Ok(())
    }

    fn poll_installations(&mut self) -> Vec<InstallDetection> {
        Vec::new()
    }
}

/// Factory: create the best available install watcher for the current platform.
/// Currently returns `NoOpInstallWatcher` on all platforms.
pub fn create_install_watcher() -> Box<dyn InstallWatcher> {
    Box::new(NoOpInstallWatcher)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// A mock watcher that returns a fixed list of detections once.
    pub struct MockInstallWatcher {
        pending: Vec<InstallDetection>,
        started: bool,
    }

    impl MockInstallWatcher {
        pub fn new(detections: Vec<InstallDetection>) -> Self {
            Self {
                pending: detections,
                started: false,
            }
        }

        pub fn is_started(&self) -> bool {
            self.started
        }
    }

    impl InstallWatcher for MockInstallWatcher {
        fn start(&mut self) -> Result<(), crate::types::PluginError> {
            self.started = true;
            Ok(())
        }

        fn stop(&mut self) -> Result<(), crate::types::PluginError> {
            self.started = false;
            Ok(())
        }

        fn poll_installations(&mut self) -> Vec<InstallDetection> {
            std::mem::take(&mut self.pending)
        }
    }

    fn make_detection(path: &str, action: InstallAction) -> InstallDetection {
        use crate::types::{AppIdentifier, AppMatch, AppMatchType};
        use bb_common::enums::Platform;
        use uuid::Uuid;

        InstallDetection {
            path: std::path::PathBuf::from(path),
            app_match: AppMatch {
                app_id: AppIdentifier::empty(Platform::Windows),
                signature_id: Uuid::nil(),
                signature_name: "TestApp".to_string(),
                match_type: AppMatchType::ExactExecutable,
                confidence: 1.0,
                reason: "test".to_string(),
            },
            detected_at: chrono::Utc::now(),
            action,
        }
    }

    #[test]
    fn noop_watcher_start_stop() {
        let mut watcher = NoOpInstallWatcher;
        assert!(watcher.start().is_ok());
        assert!(watcher.stop().is_ok());
    }

    #[test]
    fn noop_watcher_poll_returns_empty() {
        let mut watcher = NoOpInstallWatcher;
        let detections = watcher.poll_installations();
        assert!(detections.is_empty());
    }

    #[test]
    fn create_install_watcher_returns_noop() {
        let mut watcher = create_install_watcher();
        assert!(watcher.start().is_ok());
        let detections = watcher.poll_installations();
        assert!(detections.is_empty());
        assert!(watcher.stop().is_ok());
    }

    #[test]
    fn mock_watcher_start_sets_flag() {
        let mut mock = MockInstallWatcher::new(vec![]);
        assert!(!mock.is_started());
        mock.start().unwrap();
        assert!(mock.is_started());
        mock.stop().unwrap();
        assert!(!mock.is_started());
    }

    #[test]
    fn mock_watcher_poll_drains_detections() {
        let detections = vec![
            make_detection("C:\\installer_a.exe", InstallAction::Blocked),
            make_detection("C:\\installer_b.exe", InstallAction::Quarantined),
        ];
        let mut mock = MockInstallWatcher::new(detections);

        let first = mock.poll_installations();
        assert_eq!(first.len(), 2);
        assert_eq!(first[0].action, InstallAction::Blocked);
        assert_eq!(first[1].action, InstallAction::Quarantined);

        let second = mock.poll_installations();
        assert!(second.is_empty());
    }

    #[test]
    fn install_action_logged_variant() {
        let d = make_detection("/tmp/installer.pkg", InstallAction::Logged);
        assert_eq!(d.action, InstallAction::Logged);
    }
}
