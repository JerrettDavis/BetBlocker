use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VpnInterfaceType {
    Tun,
    Tap,
    WireGuard,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyType {
    Http,
    Https,
    Socks4,
    Socks5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxySource {
    SystemSettings,
    EnvironmentVariable,
    BrowserConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpnInfo {
    pub interface_name: String,
    pub interface_type: VpnInterfaceType,
    pub process_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyInfo {
    pub proxy_type: ProxyType,
    pub address: String,
    pub source: ProxySource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorInfo {
    pub process_detected: bool,
    pub exit_node_match: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BypassDetectionResult {
    pub vpn: Option<VpnInfo>,
    pub proxy: Option<ProxyInfo>,
    pub tor: Option<TorInfo>,
    pub detected_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vpn_interface_type_roundtrips() {
        for variant in [VpnInterfaceType::Tun, VpnInterfaceType::Tap, VpnInterfaceType::WireGuard, VpnInterfaceType::Unknown] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: VpnInterfaceType = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn proxy_type_roundtrips() {
        for variant in [ProxyType::Http, ProxyType::Https, ProxyType::Socks4, ProxyType::Socks5] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: ProxyType = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn proxy_source_roundtrips() {
        for variant in [ProxySource::SystemSettings, ProxySource::EnvironmentVariable, ProxySource::BrowserConfig] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: ProxySource = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn bypass_detection_result_roundtrips() {
        let result = BypassDetectionResult {
            vpn: Some(VpnInfo {
                interface_name: "tun0".to_string(),
                interface_type: VpnInterfaceType::Tun,
                process_name: Some("openvpn".to_string()),
            }),
            proxy: Some(ProxyInfo {
                proxy_type: ProxyType::Socks5,
                address: "127.0.0.1:1080".to_string(),
                source: ProxySource::SystemSettings,
            }),
            tor: Some(TorInfo {
                process_detected: true,
                exit_node_match: false,
            }),
            detected_at: Utc::now(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: BypassDetectionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result.vpn.as_ref().unwrap().interface_name, back.vpn.as_ref().unwrap().interface_name);
        assert_eq!(result.proxy.as_ref().unwrap().proxy_type, back.proxy.as_ref().unwrap().proxy_type);
        assert_eq!(result.tor.as_ref().unwrap().process_detected, back.tor.as_ref().unwrap().process_detected);
    }

    #[test]
    fn bypass_detection_result_none_fields() {
        let result = BypassDetectionResult {
            vpn: None,
            proxy: None,
            tor: None,
            detected_at: Utc::now(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: BypassDetectionResult = serde_json::from_str(&json).unwrap();
        assert!(back.vpn.is_none());
        assert!(back.proxy.is_none());
        assert!(back.tor.is_none());
    }
}
