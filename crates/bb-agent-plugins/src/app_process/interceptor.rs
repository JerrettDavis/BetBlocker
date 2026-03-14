/// A detected running process that matches a blocked application signature.
#[derive(Debug, Clone)]
pub struct ProcessDetection {
    /// OS process ID.
    pub pid: u32,
    /// The app match that triggered this detection.
    pub app_match: crate::types::AppMatch,
    /// When this detection was recorded.
    pub detected_at: chrono::DateTime<chrono::Utc>,
    /// Whether the process has been killed.
    pub killed: bool,
}

/// Trait for monitoring running processes and detecting blocked applications.
pub trait ProcessInterceptor: Send + Sync {
    /// Start process monitoring.
    fn start(&mut self) -> Result<(), crate::types::PluginError>;

    /// Stop process monitoring.
    fn stop(&mut self) -> Result<(), crate::types::PluginError>;

    /// Poll for newly detected processes since the last call.
    /// Returns a list of detections — callers should drain this each tick.
    fn poll_detections(&mut self) -> Vec<ProcessDetection>;

    /// Attempt to kill a process by PID.
    /// Returns `true` if the kill succeeded (or the process was already gone).
    fn kill_process(&self, pid: u32) -> bool;
}

// ── NoOp implementation (cross-platform stub) ──────────────────────────────

/// A no-op interceptor that never detects any process.
/// Used on unsupported platforms and in tests.
pub struct NoOpInterceptor;

impl ProcessInterceptor for NoOpInterceptor {
    fn start(&mut self) -> Result<(), crate::types::PluginError> {
        Ok(())
    }

    fn stop(&mut self) -> Result<(), crate::types::PluginError> {
        Ok(())
    }

    fn poll_detections(&mut self) -> Vec<ProcessDetection> {
        Vec::new()
    }

    fn kill_process(&self, _pid: u32) -> bool {
        false
    }
}

/// Factory: create the best available interceptor for the current platform.
/// Currently returns a `NoOpInterceptor` on all platforms — real platform
/// implementations (WMI, sysinfo, /proc) will be wired in a later sprint.
pub fn create_interceptor() -> Box<dyn ProcessInterceptor> {
    Box::new(NoOpInterceptor)
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// A mock interceptor that returns a fixed list of detections once.
    pub struct MockInterceptor {
        pending: Vec<ProcessDetection>,
        started: bool,
    }

    impl MockInterceptor {
        pub fn new(detections: Vec<ProcessDetection>) -> Self {
            Self {
                pending: detections,
                started: false,
            }
        }

        pub fn is_started(&self) -> bool {
            self.started
        }
    }

    impl ProcessInterceptor for MockInterceptor {
        fn start(&mut self) -> Result<(), crate::types::PluginError> {
            self.started = true;
            Ok(())
        }

        fn stop(&mut self) -> Result<(), crate::types::PluginError> {
            self.started = false;
            Ok(())
        }

        fn poll_detections(&mut self) -> Vec<ProcessDetection> {
            std::mem::take(&mut self.pending)
        }

        fn kill_process(&self, _pid: u32) -> bool {
            true
        }
    }

    fn make_detection(pid: u32) -> ProcessDetection {
        use crate::types::{AppIdentifier, AppMatch, AppMatchType};
        use bb_common::enums::Platform;
        use uuid::Uuid;

        ProcessDetection {
            pid,
            app_match: AppMatch {
                app_id: AppIdentifier::empty(Platform::Windows),
                signature_id: Uuid::nil(),
                signature_name: "TestApp".to_string(),
                match_type: AppMatchType::ExactExecutable,
                confidence: 1.0,
                reason: "test".to_string(),
            },
            detected_at: chrono::Utc::now(),
            killed: false,
        }
    }

    #[test]
    fn noop_interceptor_start_stop() {
        let mut interceptor = NoOpInterceptor;
        assert!(interceptor.start().is_ok());
        assert!(interceptor.stop().is_ok());
    }

    #[test]
    fn noop_interceptor_poll_returns_empty() {
        let mut interceptor = NoOpInterceptor;
        let detections = interceptor.poll_detections();
        assert!(detections.is_empty());
    }

    #[test]
    fn noop_interceptor_kill_returns_false() {
        let interceptor = NoOpInterceptor;
        assert!(!interceptor.kill_process(1234));
    }

    #[test]
    fn create_interceptor_returns_noop() {
        let mut interceptor = create_interceptor();
        // Should start/stop without error
        assert!(interceptor.start().is_ok());
        let detections = interceptor.poll_detections();
        assert!(detections.is_empty());
        assert!(interceptor.stop().is_ok());
    }

    #[test]
    fn mock_interceptor_start_sets_flag() {
        let mut mock = MockInterceptor::new(vec![]);
        assert!(!mock.is_started());
        mock.start().unwrap();
        assert!(mock.is_started());
        mock.stop().unwrap();
        assert!(!mock.is_started());
    }

    #[test]
    fn mock_interceptor_poll_drains_detections() {
        let detections = vec![make_detection(100), make_detection(200)];
        let mut mock = MockInterceptor::new(detections);

        let first = mock.poll_detections();
        assert_eq!(first.len(), 2);
        assert_eq!(first[0].pid, 100);
        assert_eq!(first[1].pid, 200);

        // Second poll should be empty
        let second = mock.poll_detections();
        assert!(second.is_empty());
    }

    #[test]
    fn mock_interceptor_kill_returns_true() {
        let mock = MockInterceptor::new(vec![]);
        assert!(mock.kill_process(9999));
    }
}
