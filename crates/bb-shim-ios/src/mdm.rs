//! iOS MDM (Mobile Device Management) integration.
//!
//! Interfaces with iOS MDM configuration profiles for supervised device
//! management, DNS content filter deployment, and restriction enforcement.

use thiserror::Error;

/// Errors that can occur when interacting with the iOS MDM subsystem.
#[derive(Debug, Error, PartialEq)]
pub enum MdmError {
    #[error("this device is not under MDM management")]
    NotManaged,
    #[error("failed to install the MDM configuration profile")]
    ProfileInstallFailed,
    #[error("iOS MDM integration is not supported on this build")]
    Unsupported,
}

pub type Result<T> = std::result::Result<T, MdmError>;

/// Interface for iOS MDM management operations.
pub trait MdmManager {
    /// Returns `true` if the device is currently managed by an MDM server.
    fn is_managed(&self) -> bool;

    /// Install an MDM configuration profile from raw profile data.
    fn install_profile(&self, profile_data: &[u8]) -> Result<()>;

    /// Return a list of bundle identifiers for MDM-managed apps.
    fn get_managed_apps(&self) -> Result<Vec<String>>;

    /// Apply an MDM restriction to the app identified by `bundle_id`.
    fn restrict_app(&self, bundle_id: &str) -> Result<()>;
}

/// Stub implementation that returns `Err(Unsupported)` or `false` for all methods.
///
/// Used on non-iOS builds or when MDM is unavailable.
pub struct StubMdmManager;

impl MdmManager for StubMdmManager {
    fn is_managed(&self) -> bool {
        false
    }

    fn install_profile(&self, _profile_data: &[u8]) -> Result<()> {
        Err(MdmError::Unsupported)
    }

    fn get_managed_apps(&self) -> Result<Vec<String>> {
        Err(MdmError::Unsupported)
    }

    fn restrict_app(&self, _bundle_id: &str) -> Result<()> {
        Err(MdmError::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stub() -> StubMdmManager {
        StubMdmManager
    }

    #[test]
    fn is_managed_returns_false() {
        assert!(!stub().is_managed());
    }

    #[test]
    fn install_profile_returns_unsupported() {
        assert_eq!(
            stub().install_profile(&[0u8, 1u8, 2u8]),
            Err(MdmError::Unsupported)
        );
    }

    #[test]
    fn get_managed_apps_returns_unsupported() {
        assert_eq!(stub().get_managed_apps(), Err(MdmError::Unsupported));
    }

    #[test]
    fn restrict_app_returns_unsupported() {
        assert_eq!(
            stub().restrict_app("com.example.app"),
            Err(MdmError::Unsupported)
        );
    }

    #[test]
    fn error_display_not_managed() {
        let msg = MdmError::NotManaged.to_string();
        assert!(msg.contains("not under MDM management"));
    }

    #[test]
    fn error_display_profile_install_failed() {
        let msg = MdmError::ProfileInstallFailed.to_string();
        assert!(msg.contains("profile"));
    }
}
