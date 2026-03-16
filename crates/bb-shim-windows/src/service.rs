//! Windows Service lifecycle management.
//!
//! Provides SCM registration, control handler, and service status management.

use std::fmt;
use std::path::PathBuf;

/// Errors that can occur during Windows Service operations.
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    /// Failed to register the service with SCM.
    #[error("service registration failed: {0}")]
    RegistrationFailed(String),

    /// Failed to set the service control handler.
    #[error("control handler setup failed: {0}")]
    ControlHandlerFailed(String),

    /// Failed to update the service status.
    #[error("status update failed: {0}")]
    StatusUpdateFailed(String),

    /// The service is already running.
    #[error("service is already running")]
    AlreadyRunning,

    /// Underlying Win32 error.
    #[error("Win32 error: {0}")]
    Win32(String),
}

/// Configuration for registering a Windows Service.
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// Internal service name (used by SCM).
    pub service_name: String,
    /// Display name shown in services.msc.
    pub display_name: String,
    /// Human-readable service description.
    pub description: String,
    /// Path to the service binary.
    pub binary_path: PathBuf,
}

impl ServiceConfig {
    /// Create a new `ServiceConfig`.
    #[must_use]
    pub fn new(
        service_name: impl Into<String>,
        display_name: impl Into<String>,
        description: impl Into<String>,
        binary_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            service_name: service_name.into(),
            display_name: display_name.into(),
            description: description.into(),
            binary_path: binary_path.into(),
        }
    }
}

impl fmt::Display for ServiceConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ServiceConfig {{ name: {}, display: {} }}",
            self.service_name, self.display_name
        )
    }
}

/// Handles service control events and provides shutdown signaling.
///
/// Uses a `tokio::sync::watch` channel so that multiple tasks can
/// observe the shutdown signal without polling.
#[derive(Debug)]
pub struct ServiceControlHandler {
    shutdown_tx: tokio::sync::watch::Sender<bool>,
    shutdown_rx: tokio::sync::watch::Receiver<bool>,
}

impl ServiceControlHandler {
    /// Create a new handler with an idle (non-shutdown) state.
    #[must_use]
    pub fn new() -> Self {
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        Self {
            shutdown_tx,
            shutdown_rx,
        }
    }

    /// Signal all watchers that the service should shut down.
    pub fn signal_shutdown(&self) {
        let _ignore = self.shutdown_tx.send(true);
        tracing::info!("service shutdown signalled");
    }

    /// Returns `true` if shutdown has been signalled.
    #[must_use]
    pub fn is_shutdown_signalled(&self) -> bool {
        *self.shutdown_rx.borrow()
    }

    /// Obtain a new receiver that can be used to await shutdown.
    #[must_use]
    pub fn subscribe(&self) -> tokio::sync::watch::Receiver<bool> {
        self.shutdown_rx.clone()
    }

    /// Wait asynchronously until shutdown is signalled.
    pub async fn wait_for_shutdown(&mut self) {
        // If already signalled, return immediately.
        if *self.shutdown_rx.borrow() {
            return;
        }
        // Wait for a change that sets the value to true.
        while self.shutdown_rx.changed().await.is_ok() {
            if *self.shutdown_rx.borrow() {
                return;
            }
        }
    }
}

impl Default for ServiceControlHandler {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Windows-only SCM integration
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
mod scm {
    use super::{ServiceConfig, ServiceError};
    use std::ffi::OsString;
    use std::time::Duration;
    use tracing::info;
    use windows_service::service::{
        ServiceAccess, ServiceAction, ServiceActionType, ServiceErrorControl,
        ServiceFailureActions, ServiceFailureResetPeriod, ServiceInfo, ServiceStartType,
        ServiceState, ServiceType,
    };
    use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

    /// Register (create) a Windows Service via the SCM.
    pub fn register_service(config: &ServiceConfig) -> Result<(), ServiceError> {
        let manager =
            ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CREATE_SERVICE)
                .map_err(|e| ServiceError::RegistrationFailed(e.to_string()))?;

        let service_info = ServiceInfo {
            name: OsString::from(&config.service_name),
            display_name: OsString::from(&config.display_name),
            service_type: ServiceType::OWN_PROCESS,
            start_type: ServiceStartType::AutoStart,
            error_control: ServiceErrorControl::Normal,
            executable_path: config.binary_path.clone(),
            launch_arguments: vec![],
            dependencies: vec![],
            account_name: None, // LocalSystem
            account_password: None,
        };

        let service = manager
            .create_service(
                &service_info,
                ServiceAccess::CHANGE_CONFIG | ServiceAccess::START,
            )
            .map_err(|e| ServiceError::RegistrationFailed(e.to_string()))?;

        // Set the description.
        service
            .set_description(&config.description)
            .map_err(|e| ServiceError::RegistrationFailed(e.to_string()))?;

        info!(
            service_name = %config.service_name,
            "service registered with SCM"
        );

        Ok(())
    }

    /// Unregister (stop + delete) a Windows Service.
    pub fn unregister_service(service_name: &str) -> Result<(), ServiceError> {
        let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
            .map_err(|e| ServiceError::Win32(e.to_string()))?;

        let service = manager
            .open_service(
                service_name,
                ServiceAccess::STOP | ServiceAccess::DELETE | ServiceAccess::QUERY_STATUS,
            )
            .map_err(|e| ServiceError::Win32(e.to_string()))?;

        // Best-effort stop before deletion.
        let status = service
            .query_status()
            .map_err(|e| ServiceError::Win32(e.to_string()))?;

        if status.current_state != ServiceState::Stopped {
            let _ = service.stop();
            info!(service_name, "sent stop signal before deletion");
        }

        service
            .delete()
            .map_err(|e| ServiceError::Win32(e.to_string()))?;

        info!(service_name, "service deleted from SCM");
        Ok(())
    }

    /// Configure automatic restart failure actions: restart after 0 s, 5 s, 30 s.
    pub fn set_failure_actions(service_name: &str) -> Result<(), ServiceError> {
        let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
            .map_err(|e| ServiceError::Win32(e.to_string()))?;

        let service = manager
            .open_service(service_name, ServiceAccess::CHANGE_CONFIG)
            .map_err(|e| ServiceError::Win32(e.to_string()))?;

        let actions = vec![
            ServiceAction {
                action_type: ServiceActionType::Restart,
                delay: Duration::from_secs(0),
            },
            ServiceAction {
                action_type: ServiceActionType::Restart,
                delay: Duration::from_secs(5),
            },
            ServiceAction {
                action_type: ServiceActionType::Restart,
                delay: Duration::from_secs(30),
            },
        ];

        let failure_actions = ServiceFailureActions {
            reset_period: ServiceFailureResetPeriod::After(Duration::from_secs(86_400)),
            reboot_msg: None,
            command: None,
            actions: Some(actions),
        };

        service
            .update_failure_actions(failure_actions)
            .map_err(|e| ServiceError::Win32(e.to_string()))?;

        info!(
            service_name,
            "failure actions configured (restart 0s/5s/30s)"
        );
        Ok(())
    }

    /// Check whether a service is installed (exists) in the SCM.
    pub fn is_service_installed(service_name: &str) -> bool {
        let Ok(manager) =
            ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        else {
            return false;
        };

        manager
            .open_service(service_name, ServiceAccess::QUERY_STATUS)
            .is_ok()
    }

    /// Check whether a service is currently running.
    pub fn is_service_running(service_name: &str) -> bool {
        let Ok(manager) =
            ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        else {
            return false;
        };

        let Ok(service) = manager.open_service(service_name, ServiceAccess::QUERY_STATUS) else {
            return false;
        };

        service
            .query_status()
            .map(|s| s.current_state == ServiceState::Running)
            .unwrap_or(false)
    }
}

// Re-export the SCM functions on Windows.
#[cfg(target_os = "windows")]
pub use scm::{
    is_service_installed, is_service_running, register_service, set_failure_actions,
    unregister_service,
};

// ---------------------------------------------------------------------------
// Stub implementations for non-Windows platforms (compile-only)
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "windows"))]
pub fn register_service(_config: &ServiceConfig) -> Result<(), ServiceError> {
    Err(ServiceError::Win32(
        "Windows Service APIs are not available on this platform".into(),
    ))
}

#[cfg(not(target_os = "windows"))]
pub fn unregister_service(_service_name: &str) -> Result<(), ServiceError> {
    Err(ServiceError::Win32(
        "Windows Service APIs are not available on this platform".into(),
    ))
}

#[cfg(not(target_os = "windows"))]
pub fn set_failure_actions(_service_name: &str) -> Result<(), ServiceError> {
    Err(ServiceError::Win32(
        "Windows Service APIs are not available on this platform".into(),
    ))
}

#[cfg(not(target_os = "windows"))]
#[must_use]
pub fn is_service_installed(_service_name: &str) -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
#[must_use]
pub fn is_service_running(_service_name: &str) -> bool {
    false
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_error_display_registration_failed() {
        let err = ServiceError::RegistrationFailed("access denied".into());
        assert_eq!(
            err.to_string(),
            "service registration failed: access denied"
        );
    }

    #[test]
    fn service_error_display_control_handler_failed() {
        let err = ServiceError::ControlHandlerFailed("handler error".into());
        assert_eq!(
            err.to_string(),
            "control handler setup failed: handler error"
        );
    }

    #[test]
    fn service_error_display_status_update_failed() {
        let err = ServiceError::StatusUpdateFailed("timeout".into());
        assert_eq!(err.to_string(), "status update failed: timeout");
    }

    #[test]
    fn service_error_display_already_running() {
        let err = ServiceError::AlreadyRunning;
        assert_eq!(err.to_string(), "service is already running");
    }

    #[test]
    fn service_error_display_win32() {
        let err = ServiceError::Win32("error code 5".into());
        assert_eq!(err.to_string(), "Win32 error: error code 5");
    }

    #[test]
    fn service_config_new_and_fields() {
        let config = ServiceConfig::new(
            "BetBlockerSvc",
            "BetBlocker Service",
            "Blocks gambling websites",
            "C:\\Program Files\\BetBlocker\\service.exe",
        );
        assert_eq!(config.service_name, "BetBlockerSvc");
        assert_eq!(config.display_name, "BetBlocker Service");
        assert_eq!(config.description, "Blocks gambling websites");
        assert_eq!(
            config.binary_path,
            PathBuf::from("C:\\Program Files\\BetBlocker\\service.exe")
        );
    }

    #[test]
    fn service_config_display() {
        let config = ServiceConfig::new("svc", "My Service", "desc", "/usr/bin/svc");
        let displayed = config.to_string();
        assert!(displayed.contains("svc"));
        assert!(displayed.contains("My Service"));
    }

    #[test]
    fn service_config_clone() {
        let config = ServiceConfig::new("svc", "Svc", "desc", "/bin/svc");
        let cloned = config.clone();
        assert_eq!(config.service_name, cloned.service_name);
    }

    #[test]
    fn control_handler_initial_state_is_not_shutdown() {
        let handler = ServiceControlHandler::new();
        assert!(!handler.is_shutdown_signalled());
    }

    #[test]
    fn control_handler_signal_shutdown() {
        let handler = ServiceControlHandler::new();
        handler.signal_shutdown();
        assert!(handler.is_shutdown_signalled());
    }

    #[test]
    fn control_handler_subscribe_receives_shutdown() {
        let handler = ServiceControlHandler::new();
        let rx = handler.subscribe();
        assert!(!*rx.borrow());

        handler.signal_shutdown();
        assert!(*rx.borrow());
    }

    #[test]
    fn control_handler_default_trait() {
        let handler = ServiceControlHandler::default();
        assert!(!handler.is_shutdown_signalled());
    }

    #[tokio::test]
    async fn control_handler_wait_for_shutdown() {
        let mut handler = ServiceControlHandler::new();

        // Signal from a separate reference before awaiting.
        let tx = handler.shutdown_tx.clone();
        let _ignore = tx.send(true);

        // Should return immediately since shutdown is already signalled.
        handler.wait_for_shutdown().await;
        assert!(handler.is_shutdown_signalled());
    }

    #[tokio::test]
    async fn control_handler_wait_for_shutdown_async() {
        let handler = ServiceControlHandler::new();
        let mut rx = handler.subscribe();

        // Spawn a task that signals shutdown after a tiny delay.
        let tx = handler.shutdown_tx.clone();
        tokio::spawn(async move {
            tokio::task::yield_now().await;
            let _ignore = tx.send(true);
        });

        // Wait for the value to become true.
        while !*rx.borrow_and_update() {
            if rx.changed().await.is_err() {
                break;
            }
        }
        assert!(*rx.borrow());
    }
}
