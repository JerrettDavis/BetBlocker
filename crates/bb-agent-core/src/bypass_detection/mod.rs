pub mod events;
pub mod known_processes;
pub mod linux;
pub mod proxy;
pub mod response;
pub mod tor;
pub mod traits;
pub mod vpn;

use bb_common::models::bypass_detection::BypassDetectionResult;
use chrono::Utc;

use self::proxy::ProxyDetector;
use self::response::{BypassAction, BypassResponseHandler};
use self::tor::TorDetector;
use self::traits::BypassDetectionError;
use self::vpn::VpnDetector;

/// Top-level orchestrator that runs all bypass detectors and produces a
/// combined `BypassDetectionResult`, then determines the appropriate response.
pub struct BypassDetector {
    vpn_detector: VpnDetector,
    proxy_detector: ProxyDetector,
    tor_detector: TorDetector,
    response_handler: BypassResponseHandler,
}

impl BypassDetector {
    pub fn new(
        vpn_detector: VpnDetector,
        proxy_detector: ProxyDetector,
        tor_detector: TorDetector,
        response_handler: BypassResponseHandler,
    ) -> Self {
        Self {
            vpn_detector,
            proxy_detector,
            tor_detector,
            response_handler,
        }
    }

    /// Factory for Linux: wires up the Linux-specific implementations.
    #[cfg(target_os = "linux")]
    pub fn create_default_linux() -> Self {
        use std::sync::Arc;

        use tokio::sync::RwLock;

        use self::linux::netlink_monitor::LinuxNetworkMonitor;
        use self::linux::process_scanner::LinuxProcessScanner;
        use self::linux::proxy_monitor::LinuxProxyMonitor;
        use self::response::BypassResponseMode;

        let vpn_detector = VpnDetector::new(
            Box::new(LinuxNetworkMonitor::new()),
            Box::new(LinuxProcessScanner::new()),
        );
        let proxy_detector = ProxyDetector::new(Box::new(LinuxProxyMonitor::new()));
        let tor_detector = TorDetector::new(
            Box::new(LinuxProcessScanner::new()),
            Arc::new(RwLock::new(None)),
        );
        let response_handler = BypassResponseHandler::new(BypassResponseMode::Alert);

        Self::new(vpn_detector, proxy_detector, tor_detector, response_handler)
    }

    /// Run a single detection cycle: execute all detectors, combine results,
    /// and determine the response action.
    pub async fn run_detection_cycle(
        &self,
    ) -> Result<(BypassDetectionResult, BypassAction), BypassDetectionError> {
        // Run VPN detection. Take first result if any.
        let vpn_results = self.vpn_detector.detect().await?;
        let vpn = vpn_results.into_iter().next();

        // Run proxy detection.
        let proxy = self.proxy_detector.detect().await?;

        // Run Tor detection.
        let tor_info = self.tor_detector.detect().await?;
        let tor = if tor_info.process_detected || tor_info.exit_node_match {
            Some(tor_info)
        } else {
            None
        };

        let result = BypassDetectionResult {
            vpn,
            proxy,
            tor,
            detected_at: Utc::now(),
        };

        let action = self.response_handler.handle_detection(&result);

        Ok((result, action))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use async_trait::async_trait;
    use bb_common::models::bypass_detection::{
        ProxyInfo, ProxySource, ProxyType, VpnInfo, VpnInterfaceType,
    };
    use response::BypassResponseMode;
    use tokio::sync::RwLock;

    // ── Mock implementations ────────────────────────────────────────

    struct MockInterfaceMonitor {
        interfaces: Vec<VpnInfo>,
    }

    #[async_trait]
    impl traits::NetworkInterfaceMonitor for MockInterfaceMonitor {
        async fn detect_vpn_interfaces(
            &self,
        ) -> Result<Vec<VpnInfo>, traits::BypassDetectionError> {
            Ok(self.interfaces.clone())
        }

        async fn watch_interfaces(
            &self,
        ) -> Result<tokio::sync::mpsc::Receiver<VpnInfo>, traits::BypassDetectionError> {
            let (_tx, rx) = tokio::sync::mpsc::channel(1);
            Ok(rx)
        }
    }

    struct MockProcessScanner {
        found: Vec<String>,
    }

    #[async_trait]
    impl traits::ProcessScanner for MockProcessScanner {
        async fn scan_for_processes(
            &self,
            _known_names: &[&str],
        ) -> Result<Vec<String>, traits::BypassDetectionError> {
            Ok(self.found.clone())
        }
    }

    struct MockProxyMonitor {
        result: Option<ProxyInfo>,
    }

    #[async_trait]
    impl traits::ProxyConfigMonitor for MockProxyMonitor {
        async fn detect_proxy_config(
            &self,
        ) -> Result<Option<ProxyInfo>, traits::BypassDetectionError> {
            Ok(self.result.clone())
        }
    }

    fn build_detector(
        interfaces: Vec<VpnInfo>,
        vpn_processes: Vec<String>,
        proxy: Option<ProxyInfo>,
        tor_processes: Vec<String>,
        mode: BypassResponseMode,
    ) -> BypassDetector {
        let vpn_detector = VpnDetector::new(
            Box::new(MockInterfaceMonitor { interfaces }),
            Box::new(MockProcessScanner {
                found: vpn_processes,
            }),
        );
        let proxy_detector = ProxyDetector::new(Box::new(MockProxyMonitor { result: proxy }));
        let tor_detector = TorDetector::new(
            Box::new(MockProcessScanner {
                found: tor_processes,
            }),
            Arc::new(RwLock::new(None)),
        );
        let response_handler = BypassResponseHandler::new(mode);

        BypassDetector::new(vpn_detector, proxy_detector, tor_detector, response_handler)
    }

    // ── Tests ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn all_clear() {
        let detector = build_detector(vec![], vec![], None, vec![], BypassResponseMode::Alert);

        let (result, action) = detector.run_detection_cycle().await.unwrap();
        assert!(result.vpn.is_none());
        assert!(result.proxy.is_none());
        assert!(result.tor.is_none());
        assert_eq!(action, BypassAction::None);
    }

    #[tokio::test]
    async fn vpn_detected() {
        let detector = build_detector(
            vec![VpnInfo {
                interface_name: "tun0".to_string(),
                interface_type: VpnInterfaceType::Tun,
                process_name: None,
            }],
            vec![],
            None,
            vec![],
            BypassResponseMode::Alert,
        );

        let (result, action) = detector.run_detection_cycle().await.unwrap();
        assert!(result.vpn.is_some());
        assert_eq!(result.vpn.as_ref().unwrap().interface_name, "tun0");
        assert!(result.proxy.is_none());
        assert!(result.tor.is_none());
        assert_eq!(action, BypassAction::EmitAlert);
    }

    #[tokio::test]
    async fn proxy_detected() {
        let detector = build_detector(
            vec![],
            vec![],
            Some(ProxyInfo {
                proxy_type: ProxyType::Socks5,
                address: "127.0.0.1:1080".to_string(),
                source: ProxySource::EnvironmentVariable,
            }),
            vec![],
            BypassResponseMode::Block,
        );

        let (result, action) = detector.run_detection_cycle().await.unwrap();
        assert!(result.vpn.is_none());
        assert!(result.proxy.is_some());
        // No kernel controller configured, so Block mode falls back to EmitAlert
        assert_eq!(action, BypassAction::EmitAlert);
    }

    #[tokio::test]
    async fn tor_detected() {
        let detector = build_detector(
            vec![],
            vec![],
            None,
            vec!["tor".to_string()],
            BypassResponseMode::Lockdown,
        );

        let (result, action) = detector.run_detection_cycle().await.unwrap();
        assert!(result.tor.is_some());
        assert!(result.tor.as_ref().unwrap().process_detected);
        // No kernel controller configured, so Lockdown mode falls back to EmitAlert
        assert_eq!(action, BypassAction::EmitAlert);
    }

    #[tokio::test]
    async fn combined_detection() {
        let detector = build_detector(
            vec![VpnInfo {
                interface_name: "wg0".to_string(),
                interface_type: VpnInterfaceType::WireGuard,
                process_name: None,
            }],
            vec![],
            Some(ProxyInfo {
                proxy_type: ProxyType::Http,
                address: "proxy:8080".to_string(),
                source: ProxySource::SystemSettings,
            }),
            vec!["tor".to_string()],
            BypassResponseMode::Alert,
        );

        let (result, action) = detector.run_detection_cycle().await.unwrap();
        assert!(result.vpn.is_some());
        assert!(result.proxy.is_some());
        assert!(result.tor.is_some());
        assert_eq!(action, BypassAction::EmitAlert);
    }

    #[tokio::test]
    async fn log_mode_returns_log_only() {
        let detector = build_detector(
            vec![VpnInfo {
                interface_name: "tun0".to_string(),
                interface_type: VpnInterfaceType::Tun,
                process_name: None,
            }],
            vec![],
            None,
            vec![],
            BypassResponseMode::Log,
        );

        let (_result, action) = detector.run_detection_cycle().await.unwrap();
        assert_eq!(action, BypassAction::LogOnly);
    }
}
