use bb_common::enums::{EventCategory, EventSeverity, EventType};
use bb_common::models::bypass_detection::{ProxyInfo, TorInfo};
use chrono::Utc;

use crate::events::AgentEvent;

/// Create an `AgentEvent` for a VPN detection.
pub fn create_vpn_detected_event(
    detection_type: &str,
    details: serde_json::Value,
) -> AgentEvent {
    AgentEvent {
        id: None,
        event_type: EventType::VpnDetected,
        category: EventCategory::System,
        severity: EventSeverity::Warning,
        domain: None,
        plugin_id: "bypass_detection".to_string(),
        metadata: serde_json::json!({
            "detection_type": detection_type,
            "details": details,
        }),
        timestamp: Utc::now(),
        reported: false,
    }
}

/// Create an `AgentEvent` for a proxy detection.
pub fn create_proxy_detected_event(proxy_info: &ProxyInfo) -> AgentEvent {
    AgentEvent {
        id: None,
        event_type: EventType::BypassAttempt,
        category: EventCategory::System,
        severity: EventSeverity::Warning,
        domain: None,
        plugin_id: "bypass_detection".to_string(),
        metadata: serde_json::json!({
            "detection_type": "proxy",
            "proxy_type": proxy_info.proxy_type,
            "address": proxy_info.address,
            "source": proxy_info.source,
        }),
        timestamp: Utc::now(),
        reported: false,
    }
}

/// Create an `AgentEvent` for a Tor detection.
pub fn create_tor_detected_event(tor_info: &TorInfo) -> AgentEvent {
    AgentEvent {
        id: None,
        event_type: EventType::BypassAttempt,
        category: EventCategory::System,
        severity: EventSeverity::Critical,
        domain: None,
        plugin_id: "bypass_detection".to_string(),
        metadata: serde_json::json!({
            "detection_type": "tor",
            "process_detected": tor_info.process_detected,
            "exit_node_match": tor_info.exit_node_match,
        }),
        timestamp: Utc::now(),
        reported: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bb_common::models::bypass_detection::{ProxySource, ProxyType};

    #[test]
    fn vpn_detected_event_structure() {
        let event = create_vpn_detected_event(
            "interface",
            serde_json::json!({"interface_name": "tun0"}),
        );

        assert_eq!(event.event_type, EventType::VpnDetected);
        assert_eq!(event.category, EventCategory::System);
        assert_eq!(event.severity, EventSeverity::Warning);
        assert_eq!(event.plugin_id, "bypass_detection");
        assert!(event.domain.is_none());
        assert!(!event.reported);
        assert_eq!(event.metadata["detection_type"], "interface");
        assert_eq!(event.metadata["details"]["interface_name"], "tun0");
    }

    #[test]
    fn proxy_detected_event_structure() {
        let proxy_info = ProxyInfo {
            proxy_type: ProxyType::Socks5,
            address: "127.0.0.1:1080".to_string(),
            source: ProxySource::EnvironmentVariable,
        };
        let event = create_proxy_detected_event(&proxy_info);

        assert_eq!(event.event_type, EventType::BypassAttempt);
        assert_eq!(event.category, EventCategory::System);
        assert_eq!(event.severity, EventSeverity::Warning);
        assert_eq!(event.metadata["detection_type"], "proxy");
        assert_eq!(event.metadata["address"], "127.0.0.1:1080");
        assert_eq!(event.metadata["proxy_type"], "socks5");
    }

    #[test]
    fn tor_detected_event_structure() {
        let tor_info = TorInfo {
            process_detected: true,
            exit_node_match: false,
        };
        let event = create_tor_detected_event(&tor_info);

        assert_eq!(event.event_type, EventType::BypassAttempt);
        assert_eq!(event.category, EventCategory::System);
        assert_eq!(event.severity, EventSeverity::Critical);
        assert_eq!(event.metadata["detection_type"], "tor");
        assert_eq!(event.metadata["process_detected"], true);
        assert_eq!(event.metadata["exit_node_match"], false);
    }

    #[test]
    fn events_are_not_reported_by_default() {
        let vpn = create_vpn_detected_event("test", serde_json::json!({}));
        let proxy = create_proxy_detected_event(&ProxyInfo {
            proxy_type: ProxyType::Http,
            address: "localhost:8080".to_string(),
            source: ProxySource::SystemSettings,
        });
        let tor = create_tor_detected_event(&TorInfo {
            process_detected: false,
            exit_node_match: true,
        });

        assert!(!vpn.reported);
        assert!(!proxy.reported);
        assert!(!tor.reported);
    }
}
