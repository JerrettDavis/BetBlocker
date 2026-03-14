use async_trait::async_trait;
use bb_common::models::bypass_detection::{VpnInfo, VpnInterfaceType};

use crate::bypass_detection::traits::{BypassDetectionError, NetworkInterfaceMonitor};

/// Linux network interface monitor that reads `/sys/class/net/` to detect
/// VPN interfaces and provides a stub `watch_interfaces` channel.
pub struct LinuxNetworkMonitor;

impl LinuxNetworkMonitor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LinuxNetworkMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Infer the VPN interface type from the interface name.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn infer_interface_type(name: &str) -> VpnInterfaceType {
    if name.starts_with("tun") || name.starts_with("utun") {
        VpnInterfaceType::Tun
    } else if name.starts_with("tap") {
        VpnInterfaceType::Tap
    } else if name.starts_with("wg") {
        VpnInterfaceType::WireGuard
    } else {
        VpnInterfaceType::Unknown
    }
}

#[cfg(target_os = "linux")]
#[async_trait]
impl NetworkInterfaceMonitor for LinuxNetworkMonitor {
    async fn detect_vpn_interfaces(&self) -> Result<Vec<VpnInfo>, BypassDetectionError> {
        use crate::bypass_detection::known_processes::VPN_INTERFACE_PREFIXES;

        let mut results = Vec::new();
        let entries = std::fs::read_dir("/sys/class/net/").map_err(BypassDetectionError::Io)?;

        for entry in entries {
            let entry = entry.map_err(BypassDetectionError::Io)?;
            let name = entry.file_name().to_string_lossy().to_string();

            let is_vpn = VPN_INTERFACE_PREFIXES
                .iter()
                .any(|prefix| name.starts_with(prefix));

            if is_vpn {
                results.push(VpnInfo {
                    interface_name: name.clone(),
                    interface_type: infer_interface_type(&name),
                    process_name: None,
                });
            }
        }

        Ok(results)
    }

    async fn watch_interfaces(
        &self,
    ) -> Result<tokio::sync::mpsc::Receiver<VpnInfo>, BypassDetectionError> {
        // Stub: real implementation would use netlink sockets.
        // Returns an open channel that will never produce items until
        // a real netlink listener is wired up.
        let (_tx, rx) = tokio::sync::mpsc::channel(16);
        Ok(rx)
    }
}

#[cfg(not(target_os = "linux"))]
#[async_trait]
impl NetworkInterfaceMonitor for LinuxNetworkMonitor {
    async fn detect_vpn_interfaces(&self) -> Result<Vec<VpnInfo>, BypassDetectionError> {
        // Non-Linux stub: no interfaces detected.
        Ok(Vec::new())
    }

    async fn watch_interfaces(
        &self,
    ) -> Result<tokio::sync::mpsc::Receiver<VpnInfo>, BypassDetectionError> {
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        Ok(rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_tun_type() {
        assert_eq!(infer_interface_type("tun0"), VpnInterfaceType::Tun);
        assert_eq!(infer_interface_type("tun1"), VpnInterfaceType::Tun);
    }

    #[test]
    fn infer_utun_type() {
        assert_eq!(infer_interface_type("utun0"), VpnInterfaceType::Tun);
    }

    #[test]
    fn infer_tap_type() {
        assert_eq!(infer_interface_type("tap0"), VpnInterfaceType::Tap);
    }

    #[test]
    fn infer_wireguard_type() {
        assert_eq!(infer_interface_type("wg0"), VpnInterfaceType::WireGuard);
        assert_eq!(infer_interface_type("wg1"), VpnInterfaceType::WireGuard);
    }

    #[test]
    fn infer_unknown_type() {
        assert_eq!(infer_interface_type("nordlynx0"), VpnInterfaceType::Unknown);
        assert_eq!(infer_interface_type("ppp0"), VpnInterfaceType::Unknown);
    }

    #[tokio::test]
    async fn stub_returns_empty_on_non_linux() {
        let monitor = LinuxNetworkMonitor::new();
        // On non-Linux this returns empty; on Linux it reads /sys/class/net/.
        let result = monitor.detect_vpn_interfaces().await;
        // Should not error on any platform.
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn watch_interfaces_returns_receiver() {
        let monitor = LinuxNetworkMonitor::new();
        let result = monitor.watch_interfaces().await;
        assert!(result.is_ok());
    }
}
