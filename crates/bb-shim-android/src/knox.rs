//! Samsung Knox integration.
//!
//! Provides enhanced device management capabilities on Samsung devices
//! via the Knox SDK, including firewall rules and app restrictions.

use thiserror::Error;

/// Errors that can occur when interacting with the Samsung Knox SDK.
#[derive(Debug, Error, PartialEq)]
pub enum KnoxError {
    #[error("Samsung Knox is not available on this device")]
    NotAvailable,
    #[error("the Knox license key is invalid or expired")]
    LicenseInvalid,
    #[error("failed to apply the requested Knox policy")]
    PolicyFailed,
    #[error("Knox integration is not supported on this build")]
    Unsupported,
}

pub type Result<T> = std::result::Result<T, KnoxError>;

/// Interface for Samsung Knox management operations.
pub trait KnoxManager {
    /// Returns `true` if Knox is available on the current device.
    fn is_available(&self) -> bool;

    /// Activate a Knox license using the provided key.
    fn activate_license(&self, key: &str) -> Result<()>;

    /// Add or remove a firewall rule for the given domain.
    fn set_firewall_rule(&self, domain: &str, blocked: bool) -> Result<()>;

    /// Prevent the given package from being uninstalled.
    fn block_app_uninstall(&self, package: &str) -> Result<()>;

    /// Enable VPN lockdown so that network traffic is blocked when the VPN is down.
    fn enable_vpn_lockdown(&self) -> Result<()>;
}

/// Stub implementation that returns `Err(Unsupported)` or `false` for all methods.
///
/// Used on non-Samsung builds or when the Knox SDK is unavailable.
pub struct StubKnoxManager;

impl KnoxManager for StubKnoxManager {
    fn is_available(&self) -> bool {
        false
    }

    fn activate_license(&self, _key: &str) -> Result<()> {
        Err(KnoxError::Unsupported)
    }

    fn set_firewall_rule(&self, _domain: &str, _blocked: bool) -> Result<()> {
        Err(KnoxError::Unsupported)
    }

    fn block_app_uninstall(&self, _package: &str) -> Result<()> {
        Err(KnoxError::Unsupported)
    }

    fn enable_vpn_lockdown(&self) -> Result<()> {
        Err(KnoxError::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stub() -> StubKnoxManager {
        StubKnoxManager
    }

    #[test]
    fn is_available_returns_false() {
        assert!(!stub().is_available());
    }

    #[test]
    fn activate_license_returns_unsupported() {
        assert_eq!(
            stub().activate_license("some-key"),
            Err(KnoxError::Unsupported)
        );
    }

    #[test]
    fn set_firewall_rule_returns_unsupported() {
        assert_eq!(
            stub().set_firewall_rule("example.com", true),
            Err(KnoxError::Unsupported)
        );
    }

    #[test]
    fn block_app_uninstall_returns_unsupported() {
        assert_eq!(
            stub().block_app_uninstall("com.example.app"),
            Err(KnoxError::Unsupported)
        );
    }

    #[test]
    fn enable_vpn_lockdown_returns_unsupported() {
        assert_eq!(stub().enable_vpn_lockdown(), Err(KnoxError::Unsupported));
    }

    #[test]
    fn error_display_not_available() {
        let msg = KnoxError::NotAvailable.to_string();
        assert!(msg.contains("not available"));
    }

    #[test]
    fn error_display_license_invalid() {
        let msg = KnoxError::LicenseInvalid.to_string();
        assert!(msg.contains("license"));
    }
}
