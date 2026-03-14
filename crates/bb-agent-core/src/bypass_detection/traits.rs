use async_trait::async_trait;
use bb_common::models::bypass_detection::{ProxyInfo, VpnInfo};

#[derive(Debug, thiserror::Error)]
pub enum BypassDetectionError {
    #[error("platform not supported")]
    PlatformNotSupported,
    #[error("permission denied")]
    PermissionDenied,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

#[async_trait]
pub trait NetworkInterfaceMonitor: Send + Sync {
    async fn detect_vpn_interfaces(&self) -> Result<Vec<VpnInfo>, BypassDetectionError>;
    async fn watch_interfaces(
        &self,
    ) -> Result<tokio::sync::mpsc::Receiver<VpnInfo>, BypassDetectionError>;
}

#[async_trait]
pub trait ProxyConfigMonitor: Send + Sync {
    async fn detect_proxy_config(&self) -> Result<Option<ProxyInfo>, BypassDetectionError>;
}

#[async_trait]
pub trait ProcessScanner: Send + Sync {
    async fn scan_for_processes(
        &self,
        known_names: &[&str],
    ) -> Result<Vec<String>, BypassDetectionError>;
}
