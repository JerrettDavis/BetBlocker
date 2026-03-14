//! Android VPN service interface.
//!
//! Provides an abstraction over Android's `VpnService` API for establishing
//! a local VPN tunnel used to intercept and filter DNS traffic.

use thiserror::Error;

/// Errors that can occur when interacting with the Android VPN service.
#[derive(Debug, Error, PartialEq)]
pub enum VpnServiceError {
    #[error("the VPN service has not been prepared; user consent is required")]
    NotPrepared,
    #[error("the VPN tunnel is already running")]
    AlreadyRunning,
    #[error("failed to establish or maintain the VPN tunnel")]
    TunnelFailed,
    #[error("Android VPN service is not supported on this build")]
    Unsupported,
}

pub type Result<T> = std::result::Result<T, VpnServiceError>;

/// Interface for the Android VPN service used to intercept DNS queries.
pub trait AndroidVpnService {
    /// Prepare the VPN service, obtaining user consent if required.
    fn prepare(&self) -> Result<()>;

    /// Start the VPN tunnel using the provided DNS server addresses.
    fn start_tunnel(&self, dns_servers: Vec<String>) -> Result<()>;

    /// Stop the running VPN tunnel.
    fn stop_tunnel(&self) -> Result<()>;

    /// Returns `true` if the tunnel is currently active.
    fn is_running(&self) -> bool;

    /// Update the DNS blocklist enforced by the tunnel.
    fn set_dns_blocklist(&self, domains: Vec<String>) -> Result<()>;
}

/// Stub implementation that returns `Err(Unsupported)` or `false` for all methods.
///
/// Used on non-Android builds or when the VPN service is unavailable.
pub struct StubAndroidVpnService;

impl AndroidVpnService for StubAndroidVpnService {
    fn prepare(&self) -> Result<()> {
        Err(VpnServiceError::Unsupported)
    }

    fn start_tunnel(&self, _dns_servers: Vec<String>) -> Result<()> {
        Err(VpnServiceError::Unsupported)
    }

    fn stop_tunnel(&self) -> Result<()> {
        Err(VpnServiceError::Unsupported)
    }

    fn is_running(&self) -> bool {
        false
    }

    fn set_dns_blocklist(&self, _domains: Vec<String>) -> Result<()> {
        Err(VpnServiceError::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stub() -> StubAndroidVpnService {
        StubAndroidVpnService
    }

    #[test]
    fn prepare_returns_unsupported() {
        assert_eq!(stub().prepare(), Err(VpnServiceError::Unsupported));
    }

    #[test]
    fn start_tunnel_returns_unsupported() {
        assert_eq!(
            stub().start_tunnel(vec!["1.1.1.1".to_string()]),
            Err(VpnServiceError::Unsupported)
        );
    }

    #[test]
    fn stop_tunnel_returns_unsupported() {
        assert_eq!(stub().stop_tunnel(), Err(VpnServiceError::Unsupported));
    }

    #[test]
    fn is_running_returns_false() {
        assert!(!stub().is_running());
    }

    #[test]
    fn set_dns_blocklist_returns_unsupported() {
        assert_eq!(
            stub().set_dns_blocklist(vec!["gambling.example".to_string()]),
            Err(VpnServiceError::Unsupported)
        );
    }

    #[test]
    fn error_display_not_prepared() {
        let msg = VpnServiceError::NotPrepared.to_string();
        assert!(msg.contains("not been prepared"));
    }

    #[test]
    fn error_display_tunnel_failed() {
        let msg = VpnServiceError::TunnelFailed.to_string();
        assert!(msg.contains("tunnel"));
    }
}
