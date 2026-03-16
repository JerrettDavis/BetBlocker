// Platform shim: many functions are cross-platform stubs.
#![allow(
    clippy::missing_errors_doc,
    clippy::doc_markdown,
    clippy::must_use_candidate
)]

//! Android platform shim for `BetBlocker`.
//!
//! Provides Android-specific implementations for device owner APIs,
//! Samsung Knox integration, and platform trait definitions.

pub mod device_owner;
pub mod knox;
pub mod traits;
pub mod vpn_service;

use device_owner::StubDeviceOwnerManager;
use knox::StubKnoxManager;
use vpn_service::StubAndroidVpnService;

/// Composite Android platform that bundles all platform managers.
///
/// In production this would hold real JNI-backed implementations.
/// Currently every manager is a stub returning `Err(Unsupported)`.
pub struct AndroidPlatform {
    pub device_owner: StubDeviceOwnerManager,
    pub knox: StubKnoxManager,
    pub vpn_service: StubAndroidVpnService,
}

impl AndroidPlatform {
    /// Construct a new `AndroidPlatform` composed of stub implementations.
    pub fn new() -> Self {
        Self {
            device_owner: StubDeviceOwnerManager,
            knox: StubKnoxManager,
            vpn_service: StubAndroidVpnService,
        }
    }
}

impl Default for AndroidPlatform {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_owner::DeviceOwnerManager;
    use crate::knox::KnoxManager;
    use crate::vpn_service::AndroidVpnService;

    #[test]
    fn android_platform_constructs() {
        let platform = AndroidPlatform::new();
        // Verify each stub is accessible and behaves as expected.
        assert!(!platform.knox.is_available());
        assert!(!platform.vpn_service.is_running());
        assert!(platform.device_owner.check_status().is_err());
    }

    #[test]
    fn android_platform_default_constructs() {
        let platform = AndroidPlatform::default();
        assert!(!platform.knox.is_available());
    }
}
