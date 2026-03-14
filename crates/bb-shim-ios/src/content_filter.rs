//! iOS content filter interface.
//!
//! Abstracts over the iOS Network Extension / NEContentFilterManager API
//! for DNS-layer and content-level blocking of gambling-related domains.

use thiserror::Error;

/// Errors that can occur when interacting with the iOS content filter subsystem.
#[derive(Debug, Error, PartialEq)]
pub enum ContentFilterError {
    #[error("the content filter extension is not authorized; user permission is required")]
    NotAuthorized,
    #[error("failed to apply the content filter configuration")]
    FilterFailed,
    #[error("iOS content filter is not supported on this build")]
    Unsupported,
}

pub type Result<T> = std::result::Result<T, ContentFilterError>;

/// Interface for the iOS Network Extension content filter.
pub trait ContentFilterManager {
    /// Request user authorization to activate the content filter extension.
    fn request_authorization(&self) -> Result<()>;

    /// Start the content filter using the provided domain blocklist.
    fn start_filter(&self, blocklist: Vec<String>) -> Result<()>;

    /// Stop the active content filter.
    fn stop_filter(&self) -> Result<()>;

    /// Replace the active blocklist with a new set of domains.
    fn update_blocklist(&self, domains: Vec<String>) -> Result<()>;

    /// Returns `true` if the content filter is currently active.
    fn is_active(&self) -> bool;
}

/// Stub implementation that returns `Err(Unsupported)` or `false` for all methods.
///
/// Used on non-iOS builds or when the content filter extension is unavailable.
pub struct StubContentFilterManager;

impl ContentFilterManager for StubContentFilterManager {
    fn request_authorization(&self) -> Result<()> {
        Err(ContentFilterError::Unsupported)
    }

    fn start_filter(&self, _blocklist: Vec<String>) -> Result<()> {
        Err(ContentFilterError::Unsupported)
    }

    fn stop_filter(&self) -> Result<()> {
        Err(ContentFilterError::Unsupported)
    }

    fn update_blocklist(&self, _domains: Vec<String>) -> Result<()> {
        Err(ContentFilterError::Unsupported)
    }

    fn is_active(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stub() -> StubContentFilterManager {
        StubContentFilterManager
    }

    #[test]
    fn request_authorization_returns_unsupported() {
        assert_eq!(
            stub().request_authorization(),
            Err(ContentFilterError::Unsupported)
        );
    }

    #[test]
    fn start_filter_returns_unsupported() {
        assert_eq!(
            stub().start_filter(vec!["gambling.example".to_string()]),
            Err(ContentFilterError::Unsupported)
        );
    }

    #[test]
    fn stop_filter_returns_unsupported() {
        assert_eq!(stub().stop_filter(), Err(ContentFilterError::Unsupported));
    }

    #[test]
    fn update_blocklist_returns_unsupported() {
        assert_eq!(
            stub().update_blocklist(vec!["bet.example".to_string()]),
            Err(ContentFilterError::Unsupported)
        );
    }

    #[test]
    fn is_active_returns_false() {
        assert!(!stub().is_active());
    }

    #[test]
    fn error_display_not_authorized() {
        let msg = ContentFilterError::NotAuthorized.to_string();
        assert!(msg.contains("not authorized"));
    }

    #[test]
    fn error_display_filter_failed() {
        let msg = ContentFilterError::FilterFailed.to_string();
        assert!(msg.contains("filter"));
    }
}
