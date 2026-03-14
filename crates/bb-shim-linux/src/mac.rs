//! Linux Mandatory Access Control (MAC) abstraction.
//!
//! Provides a unified interface over AppArmor and SELinux for
//! confining the BetBlocker agent and protecting its resources.

use serde::{Deserialize, Serialize};

/// Which MAC system is active on the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MacSystem {
    AppArmor,
    SELinux,
    None,
}

/// Current status of MAC protection for BetBlocker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacStatus {
    pub system: MacSystem,
    pub profile_loaded: bool,
    pub enforcing: bool,
    pub profile_name: Option<String>,
}

/// Errors that can occur during MAC operations.
#[derive(Debug, thiserror::Error)]
pub enum MacError {
    #[error("command execution failed: {0}")]
    CommandFailed(String),
    #[error("permission denied")]
    PermissionDenied,
    #[error("not supported on this system")]
    NotSupported,
    #[error("profile not found")]
    ProfileNotFound,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Detect which MAC system is active on the current host.
///
/// On Linux, checks for the presence of `/sys/module/apparmor` and
/// `/sys/fs/selinux` to determine which system is in use.
/// On non-Linux platforms, always returns [`MacSystem::None`].
pub fn detect_mac_system() -> MacSystem {
    #[cfg(target_os = "linux")]
    {
        if std::path::Path::new("/sys/module/apparmor").exists() {
            return MacSystem::AppArmor;
        }
        if std::path::Path::new("/sys/fs/selinux").exists() {
            return MacSystem::SELinux;
        }
    }
    MacSystem::None
}

/// Trait for MAC protection implementations (AppArmor, SELinux).
pub trait MacProtection: Send + Sync {
    /// Install the MAC profile/policy on the system.
    fn install(&self) -> Result<(), MacError>;

    /// Verify the current MAC status for BetBlocker.
    fn verify(&self) -> Result<MacStatus, MacError>;

    /// Check whether the profile is in enforcing mode.
    fn is_enforcing(&self) -> bool;

    /// Remove the MAC profile/policy from the system.
    fn uninstall(&self) -> Result<(), MacError>;

    /// Verify the profile and repair it if it is not loaded or not enforcing.
    fn verify_and_repair(&self) -> Result<MacStatus, MacError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_system_equality() {
        assert_eq!(MacSystem::AppArmor, MacSystem::AppArmor);
        assert_eq!(MacSystem::SELinux, MacSystem::SELinux);
        assert_eq!(MacSystem::None, MacSystem::None);
        assert_ne!(MacSystem::AppArmor, MacSystem::SELinux);
        assert_ne!(MacSystem::AppArmor, MacSystem::None);
    }

    #[test]
    fn mac_system_clone() {
        let system = MacSystem::AppArmor;
        let cloned = system;
        assert_eq!(system, cloned);
    }

    #[test]
    fn mac_system_debug() {
        let dbg = format!("{:?}", MacSystem::AppArmor);
        assert_eq!(dbg, "AppArmor");
    }

    #[test]
    fn mac_status_serialization_roundtrip() {
        let status = MacStatus {
            system: MacSystem::AppArmor,
            profile_loaded: true,
            enforcing: true,
            profile_name: Some("betblocker-agent".to_string()),
        };
        let json = serde_json::to_string(&status).expect("serialize");
        let deserialized: MacStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.system, MacSystem::AppArmor);
        assert!(deserialized.profile_loaded);
        assert!(deserialized.enforcing);
        assert_eq!(
            deserialized.profile_name.as_deref(),
            Some("betblocker-agent")
        );
    }

    #[test]
    fn mac_status_serialization_none_profile() {
        let status = MacStatus {
            system: MacSystem::None,
            profile_loaded: false,
            enforcing: false,
            profile_name: None,
        };
        let json = serde_json::to_string(&status).expect("serialize");
        let deserialized: MacStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.system, MacSystem::None);
        assert!(!deserialized.profile_loaded);
        assert!(!deserialized.enforcing);
        assert!(deserialized.profile_name.is_none());
    }

    #[test]
    fn detect_mac_system_returns_none_on_non_linux() {
        // On Windows/macOS (where tests run), there is no /sys/module/apparmor
        // or /sys/fs/selinux, so this should return None.
        #[cfg(not(target_os = "linux"))]
        {
            assert_eq!(detect_mac_system(), MacSystem::None);
        }
    }

    #[test]
    fn mac_error_display() {
        let err = MacError::CommandFailed("apparmor_parser failed".to_string());
        assert!(err.to_string().contains("apparmor_parser failed"));

        let err = MacError::PermissionDenied;
        assert_eq!(err.to_string(), "permission denied");

        let err = MacError::NotSupported;
        assert_eq!(err.to_string(), "not supported on this system");

        let err = MacError::ProfileNotFound;
        assert_eq!(err.to_string(), "profile not found");
    }
}
