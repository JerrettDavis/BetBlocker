//! Windows MSI installer integration helpers.
//!
//! Provides helpers called from the WiX custom-action DLL or a standalone
//! installer binary to set up the service, directories, and firewall rules
//! during installation and to undo those changes on uninstallation.

use std::path::{Path, PathBuf};

use crate::service::{ServiceConfig, ServiceError};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during installation or uninstallation.
#[derive(Debug, thiserror::Error)]
pub enum InstallerError {
    /// A Windows Service operation failed.
    #[error("service error: {0}")]
    Service(#[from] ServiceError),

    /// An IO error occurred (directory creation, etc.).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// A firewall / netsh operation failed.
    #[error("firewall error: {0}")]
    Firewall(String),

    /// Registry operation failed (reading version info, etc.).
    #[error("registry error: {0}")]
    Registry(String),

    /// The service is already installed (idempotency guard).
    #[error("service is already installed")]
    AlreadyInstalled,

    /// The service is not installed.
    #[error("service is not installed")]
    NotInstalled,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Internal name used to register the service with the SCM.
pub const SERVICE_NAME: &str = "BetBlockerAgent";

/// Data directory created during installation.
pub const DATA_DIR: &str = r"C:\ProgramData\BetBlocker";

/// Registry key that holds the installed version.
///
/// `HKLM\SOFTWARE\BetBlocker`
#[cfg(target_os = "windows")]
const REGISTRY_KEY: &str = r"SOFTWARE\BetBlocker";

/// Registry value name for the installed version string.
#[cfg(target_os = "windows")]
const VERSION_VALUE: &str = "Version";

// ---------------------------------------------------------------------------
// post_install
// ---------------------------------------------------------------------------

/// Perform all post-installation steps.
///
/// 1. Create `C:\ProgramData\BetBlocker\` subdirectories.
/// 2. Apply restrictive ACLs so that only SYSTEM / Administrators can write.
/// 3. Register the Windows Service via the SCM.
/// 4. Configure automatic restart failure actions.
/// 5. Start the service.
///
/// `binary_path` is the full path to the installed agent binary.
pub fn post_install(binary_path: &Path) -> Result<(), InstallerError> {
    tracing::info!(binary = %binary_path.display(), "Running post-install steps");

    // 1 & 2 — Create directories with ACLs
    let subdirs: &[&str] = &[
        DATA_DIR,
        r"C:\ProgramData\BetBlocker\certs",
        r"C:\ProgramData\BetBlocker\logs",
        r"C:\ProgramData\BetBlocker\plugins",
    ];

    for dir in subdirs {
        std::fs::create_dir_all(dir)?;
        let path = PathBuf::from(dir);
        let _ = crate::acl::set_restrictive_directory_acl(&path);
    }

    tracing::info!("Data directories created");

    // 3 — Register service
    let config = ServiceConfig::new(
        SERVICE_NAME,
        "BetBlocker Agent",
        "BetBlocker gambling site blocking service",
        binary_path,
    );

    crate::service::register_service(&config)?;
    tracing::info!("Service registered with SCM");

    // 4 — Failure actions (restart 0 s / 5 s / 30 s)
    crate::service::set_failure_actions(SERVICE_NAME)?;
    tracing::info!("Failure actions configured");

    // 5 — Start the service (best-effort; may require reboot on some systems)
    start_service()?;
    tracing::info!("Service started");

    Ok(())
}

// ---------------------------------------------------------------------------
// pre_uninstall
// ---------------------------------------------------------------------------

/// Perform all pre-uninstallation steps.
///
/// 1. Stop the service if it is running.
/// 2. Unregister (delete) the service from the SCM.
/// 3. Remove the BetBlocker DNS-redirect firewall rules.
pub fn pre_uninstall() -> Result<(), InstallerError> {
    tracing::info!("Running pre-uninstall steps");

    // 1 & 2 — Stop + delete service
    crate::service::unregister_service(SERVICE_NAME)?;
    tracing::info!("Service stopped and deleted from SCM");

    // 3 — Remove firewall rules
    remove_firewall_rules()?;
    tracing::info!("Firewall rules removed");

    Ok(())
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Return `true` if the BetBlocker service is currently installed in the SCM.
#[must_use]
pub fn is_installed() -> bool {
    crate::service::is_service_installed(SERVICE_NAME)
}

/// Return the installed version string from the registry, if available.
///
/// Reads `HKLM\SOFTWARE\BetBlocker\Version`.
/// Returns `None` if the key does not exist or cannot be read.
#[cfg(target_os = "windows")]
#[must_use]
pub fn get_installed_version() -> Option<String> {
    use std::process::Command;

    let output = Command::new("reg")
        .args([
            "query",
            &format!("HKLM\\{REGISTRY_KEY}"),
            "/v",
            VERSION_VALUE,
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(VERSION_VALUE) {
            let fields: Vec<&str> = trimmed.split_whitespace().collect();
            return fields.last().map(|s| s.to_string());
        }
    }

    None
}

/// Non-Windows stub — always returns `None`.
#[cfg(not(target_os = "windows"))]
#[must_use]
pub fn get_installed_version() -> Option<String> {
    None
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Start the BetBlocker service via `sc.exe`.
fn start_service() -> Result<(), InstallerError> {
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("sc")
            .args(["start", SERVICE_NAME])
            .output()
            .map_err(|e| InstallerError::Io(e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::warn!(stderr = %stderr, "sc start returned non-zero (service may already be running)");
        }
    }
    Ok(())
}

/// Remove BetBlocker DNS-redirect firewall rules via `netsh`.
fn remove_firewall_rules() -> Result<(), InstallerError> {
    #[cfg(target_os = "windows")]
    {
        const RULE_PREFIX: &str = "BetBlocker-DNS-Redirect";
        let rule_names = [
            format!("{RULE_PREFIX}-UDP"),
            format!("{RULE_PREFIX}-TCP"),
            format!("{RULE_PREFIX}-Block"),
        ];

        for rule_name in &rule_names {
            let output = std::process::Command::new("netsh")
                .args([
                    "advfirewall",
                    "firewall",
                    "delete",
                    "rule",
                    &format!("name={rule_name}"),
                ])
                .output()
                .map_err(|e| InstallerError::Firewall(e.to_string()))?;

            if !output.status.success() {
                tracing::debug!(rule = %rule_name, "Firewall rule not found or already deleted");
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installer_error_display_service() {
        let err = InstallerError::Service(ServiceError::AlreadyRunning);
        assert!(err.to_string().contains("service"));
    }

    #[test]
    fn installer_error_display_firewall() {
        let err = InstallerError::Firewall("netsh failed".to_string());
        assert!(err.to_string().contains("netsh failed"));
    }

    #[test]
    fn installer_error_display_already_installed() {
        let err = InstallerError::AlreadyInstalled;
        assert!(err.to_string().contains("already installed"));
    }

    #[test]
    fn installer_error_display_not_installed() {
        let err = InstallerError::NotInstalled;
        assert!(err.to_string().contains("not installed"));
    }

    #[test]
    fn installer_error_display_registry() {
        let err = InstallerError::Registry("key not found".to_string());
        assert!(err.to_string().contains("key not found"));
    }

    #[test]
    fn is_installed_returns_bool_without_panic() {
        // On CI / non-Windows this is always false; just ensure no panic.
        let _installed = is_installed();
    }

    #[test]
    fn get_installed_version_returns_option_without_panic() {
        // On non-Windows always None.
        let _ver = get_installed_version();
    }
}
