//! Android Device Owner API integration.
//!
//! Manages device administration policies via the Android Device Owner API,
//! enabling uninstall prevention and configuration restrictions.

use thiserror::Error;

/// Errors that can occur when interacting with the Android Device Owner API.
#[derive(Debug, Error, PartialEq)]
pub enum DeviceOwnerError {
    #[error("this application is not the device owner")]
    NotDeviceOwner,
    #[error("the requested policy is not allowed in the current device state")]
    PolicyNotAllowed,
    #[error("JNI call to Android framework failed")]
    JniFailed,
    #[error("Device Owner API is not supported on this build")]
    Unsupported,
}

pub type Result<T> = std::result::Result<T, DeviceOwnerError>;

/// Status information about the current device/profile owner state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceOwnerStatus {
    /// Whether this application is the device owner.
    pub is_device_owner: bool,
    /// Whether this application is a profile owner.
    pub is_profile_owner: bool,
    /// The Android API level of the running device.
    pub api_level: u32,
}

/// Interface for Android Device Owner management operations.
pub trait DeviceOwnerManager {
    /// Check current device/profile owner status.
    fn check_status(&self) -> Result<DeviceOwnerStatus>;

    /// Set an app restriction for the given package.
    fn set_app_restriction(&self, package: &str, restricted: bool) -> Result<()>;

    /// Block installation of the given package.
    fn block_app_install(&self, package: &str) -> Result<()>;

    /// Configure always-on VPN for the given VPN package.
    fn set_vpn_always_on(&self, package: &str) -> Result<()>;

    /// Return a list of installed packages on the device.
    fn get_installed_packages(&self) -> Result<Vec<String>>;
}

/// Stub implementation that returns `Err(Unsupported)` for all methods.
///
/// Used on non-Android builds or when the Device Owner API is unavailable.
pub struct StubDeviceOwnerManager;

impl DeviceOwnerManager for StubDeviceOwnerManager {
    fn check_status(&self) -> Result<DeviceOwnerStatus> {
        Err(DeviceOwnerError::Unsupported)
    }

    fn set_app_restriction(&self, _package: &str, _restricted: bool) -> Result<()> {
        Err(DeviceOwnerError::Unsupported)
    }

    fn block_app_install(&self, _package: &str) -> Result<()> {
        Err(DeviceOwnerError::Unsupported)
    }

    fn set_vpn_always_on(&self, _package: &str) -> Result<()> {
        Err(DeviceOwnerError::Unsupported)
    }

    fn get_installed_packages(&self) -> Result<Vec<String>> {
        Err(DeviceOwnerError::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stub() -> StubDeviceOwnerManager {
        StubDeviceOwnerManager
    }

    #[test]
    fn check_status_returns_unsupported() {
        assert_eq!(stub().check_status(), Err(DeviceOwnerError::Unsupported));
    }

    #[test]
    fn set_app_restriction_returns_unsupported() {
        assert_eq!(
            stub().set_app_restriction("com.example.app", true),
            Err(DeviceOwnerError::Unsupported)
        );
    }

    #[test]
    fn block_app_install_returns_unsupported() {
        assert_eq!(
            stub().block_app_install("com.example.app"),
            Err(DeviceOwnerError::Unsupported)
        );
    }

    #[test]
    fn set_vpn_always_on_returns_unsupported() {
        assert_eq!(
            stub().set_vpn_always_on("com.example.vpn"),
            Err(DeviceOwnerError::Unsupported)
        );
    }

    #[test]
    fn get_installed_packages_returns_unsupported() {
        assert_eq!(
            stub().get_installed_packages(),
            Err(DeviceOwnerError::Unsupported)
        );
    }

    #[test]
    fn error_display_not_device_owner() {
        let msg = DeviceOwnerError::NotDeviceOwner.to_string();
        assert!(msg.contains("not the device owner"));
    }

    #[test]
    fn error_display_jni_failed() {
        let msg = DeviceOwnerError::JniFailed.to_string();
        assert!(msg.contains("JNI"));
    }
}
