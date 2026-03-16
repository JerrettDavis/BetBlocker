// Platform shim: many functions are cross-platform stubs.
#![allow(
    clippy::missing_errors_doc,
    clippy::doc_markdown,
    clippy::must_use_candidate
)]

//! iOS platform shim for `BetBlocker`.
//!
//! Provides iOS-specific implementations for MDM integration
//! and platform trait definitions.

pub mod content_filter;
pub mod mdm;
pub mod traits;

use content_filter::StubContentFilterManager;
use mdm::StubMdmManager;

/// Composite iOS platform that bundles all platform managers.
///
/// In production this would hold real Swift/FFI-backed implementations.
/// Currently every manager is a stub returning `Err(Unsupported)`.
pub struct IosPlatform {
    pub mdm: StubMdmManager,
    pub content_filter: StubContentFilterManager,
}

impl IosPlatform {
    /// Construct a new `IosPlatform` composed of stub implementations.
    pub fn new() -> Self {
        Self {
            mdm: StubMdmManager,
            content_filter: StubContentFilterManager,
        }
    }
}

impl Default for IosPlatform {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content_filter::ContentFilterManager;
    use crate::mdm::MdmManager;

    #[test]
    fn ios_platform_constructs() {
        let platform = IosPlatform::new();
        // Verify each stub is accessible and behaves as expected.
        assert!(!platform.mdm.is_managed());
        assert!(!platform.content_filter.is_active());
        assert!(platform.mdm.get_managed_apps().is_err());
    }

    #[test]
    fn ios_platform_default_constructs() {
        let platform = IosPlatform::default();
        assert!(!platform.mdm.is_managed());
    }
}
