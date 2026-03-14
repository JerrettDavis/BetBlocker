use crate::types::{AppIdentifier, PluginError};

/// Trait for scanning installed applications and checking running processes.
/// Platform-specific implementations will provide real scanning logic;
/// the trait defines the interface.
pub trait AppInventoryScanner: Send + Sync {
    /// Scan the system for installed applications and return their identifiers.
    fn scan_installed(&self) -> Result<Vec<AppIdentifier>, PluginError>;

    /// Check if an application identified by `app_id` is currently running.
    /// Returns `Some(pid)` if running, `None` if not.
    fn is_running(&self, app_id: &AppIdentifier) -> Result<Option<u32>, PluginError>;
}

/// A no-op scanner that returns empty results. Useful for testing
/// and as a placeholder on platforms where scanning is not yet implemented.
pub struct NoOpScanner;

impl AppInventoryScanner for NoOpScanner {
    fn scan_installed(&self) -> Result<Vec<AppIdentifier>, PluginError> {
        Ok(Vec::new())
    }

    fn is_running(&self, _app_id: &AppIdentifier) -> Result<Option<u32>, PluginError> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_scanner_returns_empty() {
        let scanner = NoOpScanner;
        let installed = scanner.scan_installed().unwrap();
        assert!(installed.is_empty());
    }

    #[test]
    fn noop_scanner_reports_not_running() {
        use bb_common::enums::Platform;
        let scanner = NoOpScanner;
        let app = AppIdentifier::empty(Platform::Windows);
        let result = scanner.is_running(&app).unwrap();
        assert!(result.is_none());
    }
}
