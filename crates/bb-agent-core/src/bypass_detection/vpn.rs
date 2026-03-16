use bb_common::models::bypass_detection::VpnInfo;

use super::known_processes::VPN_PROCESS_NAMES;
use super::traits::{BypassDetectionError, NetworkInterfaceMonitor, ProcessScanner};

/// Orchestrates VPN detection by combining network interface monitoring
/// with process scanning.
pub struct VpnDetector {
    interface_monitor: Box<dyn NetworkInterfaceMonitor>,
    process_scanner: Box<dyn ProcessScanner>,
}

impl VpnDetector {
    pub fn new(
        interface_monitor: Box<dyn NetworkInterfaceMonitor>,
        process_scanner: Box<dyn ProcessScanner>,
    ) -> Self {
        Self {
            interface_monitor,
            process_scanner,
        }
    }

    /// Run a one-shot detection: check interfaces and processes, merge results.
    pub async fn detect(&self) -> Result<Vec<VpnInfo>, BypassDetectionError> {
        let mut results = self.interface_monitor.detect_vpn_interfaces().await?;

        let processes = self
            .process_scanner
            .scan_for_processes(VPN_PROCESS_NAMES)
            .await?;

        // For each detected process that isn't already associated with an
        // interface result, create a standalone VpnInfo entry.
        for proc_name in processes {
            let already_linked = results
                .iter()
                .any(|v| v.process_name.as_deref() == Some(proc_name.as_str()));
            if !already_linked {
                results.push(VpnInfo {
                    interface_name: String::new(),
                    interface_type: bb_common::models::bypass_detection::VpnInterfaceType::Unknown,
                    process_name: Some(proc_name),
                });
            }
        }

        Ok(results)
    }

    /// Start watching for new VPN interfaces appearing. Delegates to the
    /// underlying `NetworkInterfaceMonitor`.
    pub async fn watch(
        &self,
    ) -> Result<tokio::sync::mpsc::Receiver<VpnInfo>, BypassDetectionError> {
        self.interface_monitor.watch_interfaces().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use bb_common::models::bypass_detection::VpnInterfaceType;

    // ── Mock implementations ────────────────────────────────────────

    struct MockInterfaceMonitor {
        interfaces: Vec<VpnInfo>,
    }

    #[async_trait]
    impl NetworkInterfaceMonitor for MockInterfaceMonitor {
        async fn detect_vpn_interfaces(&self) -> Result<Vec<VpnInfo>, BypassDetectionError> {
            Ok(self.interfaces.clone())
        }

        async fn watch_interfaces(
            &self,
        ) -> Result<tokio::sync::mpsc::Receiver<VpnInfo>, BypassDetectionError> {
            let (_tx, rx) = tokio::sync::mpsc::channel(1);
            Ok(rx)
        }
    }

    struct MockProcessScanner {
        found: Vec<String>,
    }

    #[async_trait]
    impl ProcessScanner for MockProcessScanner {
        async fn scan_for_processes(
            &self,
            _known_names: &[&str],
        ) -> Result<Vec<String>, BypassDetectionError> {
            Ok(self.found.clone())
        }
    }

    // ── Tests ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn no_vpn_detected() {
        let detector = VpnDetector::new(
            Box::new(MockInterfaceMonitor { interfaces: vec![] }),
            Box::new(MockProcessScanner { found: vec![] }),
        );
        let results = detector.detect().await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn vpn_interface_found() {
        let detector = VpnDetector::new(
            Box::new(MockInterfaceMonitor {
                interfaces: vec![VpnInfo {
                    interface_name: "tun0".to_string(),
                    interface_type: VpnInterfaceType::Tun,
                    process_name: None,
                }],
            }),
            Box::new(MockProcessScanner { found: vec![] }),
        );
        let results = detector.detect().await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].interface_name, "tun0");
    }

    #[tokio::test]
    async fn vpn_process_found() {
        let detector = VpnDetector::new(
            Box::new(MockInterfaceMonitor { interfaces: vec![] }),
            Box::new(MockProcessScanner {
                found: vec!["openvpn".to_string()],
            }),
        );
        let results = detector.detect().await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].process_name.as_deref(), Some("openvpn"));
        assert!(results[0].interface_name.is_empty());
    }

    #[tokio::test]
    async fn both_interface_and_process() {
        let detector = VpnDetector::new(
            Box::new(MockInterfaceMonitor {
                interfaces: vec![VpnInfo {
                    interface_name: "wg0".to_string(),
                    interface_type: VpnInterfaceType::WireGuard,
                    process_name: Some("wireguard-go".to_string()),
                }],
            }),
            Box::new(MockProcessScanner {
                found: vec!["wireguard-go".to_string(), "nordvpn".to_string()],
            }),
        );
        let results = detector.detect().await.unwrap();
        // wg0 (linked to wireguard-go) + nordvpn (standalone)
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].interface_name, "wg0");
        assert_eq!(results[1].process_name.as_deref(), Some("nordvpn"));
    }
}
