use bb_common::models::bypass_detection::BypassDetectionResult;
use serde::{Deserialize, Serialize};

/// Configures how the system responds to bypass detections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BypassResponseMode {
    /// Only log detections, take no action.
    Log,
    /// Emit an alert event to the API/partner.
    Alert,
    /// Block network traffic through the bypass path.
    Block,
    /// Enter full lockdown mode (block + restrict).
    Lockdown,
}

/// Action to take in response to a bypass detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BypassAction {
    /// No action needed (nothing detected or mode is Log with no detection).
    None,
    /// Log the detection only.
    LogOnly,
    /// Emit an alert event.
    EmitAlert,
    /// Block network traffic through the bypass path.
    BlockNetwork,
    /// Enter full lockdown mode.
    EnterLockdown,
}

/// Handles bypass detection results according to the configured response mode.
pub struct BypassResponseHandler {
    mode: BypassResponseMode,
}

impl BypassResponseHandler {
    pub fn new(mode: BypassResponseMode) -> Self {
        Self { mode }
    }

    pub fn mode(&self) -> BypassResponseMode {
        self.mode
    }

    /// Determine the appropriate action for the given detection result.
    pub fn handle_detection(&self, result: &BypassDetectionResult) -> BypassAction {
        let anything_detected = result.vpn.is_some()
            || result.proxy.is_some()
            || result.tor.as_ref().is_some_and(|t| t.process_detected || t.exit_node_match);

        match self.mode {
            BypassResponseMode::Log => {
                if anything_detected {
                    BypassAction::LogOnly
                } else {
                    BypassAction::None
                }
            }
            BypassResponseMode::Alert => {
                if anything_detected {
                    BypassAction::EmitAlert
                } else {
                    BypassAction::None
                }
            }
            BypassResponseMode::Block => {
                if anything_detected {
                    BypassAction::BlockNetwork
                } else {
                    BypassAction::None
                }
            }
            BypassResponseMode::Lockdown => {
                if anything_detected {
                    BypassAction::EnterLockdown
                } else {
                    BypassAction::None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bb_common::models::bypass_detection::{
        ProxyInfo, ProxySource, ProxyType, TorInfo, VpnInfo, VpnInterfaceType,
    };
    use chrono::Utc;

    fn empty_result() -> BypassDetectionResult {
        BypassDetectionResult {
            vpn: None,
            proxy: None,
            tor: None,
            detected_at: Utc::now(),
        }
    }

    fn vpn_result() -> BypassDetectionResult {
        BypassDetectionResult {
            vpn: Some(VpnInfo {
                interface_name: "tun0".to_string(),
                interface_type: VpnInterfaceType::Tun,
                process_name: None,
            }),
            proxy: None,
            tor: None,
            detected_at: Utc::now(),
        }
    }

    fn proxy_result() -> BypassDetectionResult {
        BypassDetectionResult {
            vpn: None,
            proxy: Some(ProxyInfo {
                proxy_type: ProxyType::Http,
                address: "localhost:8080".to_string(),
                source: ProxySource::EnvironmentVariable,
            }),
            tor: None,
            detected_at: Utc::now(),
        }
    }

    fn tor_result() -> BypassDetectionResult {
        BypassDetectionResult {
            vpn: None,
            proxy: None,
            tor: Some(TorInfo {
                process_detected: true,
                exit_node_match: false,
            }),
            detected_at: Utc::now(),
        }
    }

    // ── Log mode ────────────────────────────────────────────────────

    #[test]
    fn log_mode_nothing_detected() {
        let handler = BypassResponseHandler::new(BypassResponseMode::Log);
        assert_eq!(handler.handle_detection(&empty_result()), BypassAction::None);
    }

    #[test]
    fn log_mode_vpn_detected() {
        let handler = BypassResponseHandler::new(BypassResponseMode::Log);
        assert_eq!(
            handler.handle_detection(&vpn_result()),
            BypassAction::LogOnly
        );
    }

    #[test]
    fn log_mode_proxy_detected() {
        let handler = BypassResponseHandler::new(BypassResponseMode::Log);
        assert_eq!(
            handler.handle_detection(&proxy_result()),
            BypassAction::LogOnly
        );
    }

    // ── Alert mode ──────────────────────────────────────────────────

    #[test]
    fn alert_mode_nothing_detected() {
        let handler = BypassResponseHandler::new(BypassResponseMode::Alert);
        assert_eq!(handler.handle_detection(&empty_result()), BypassAction::None);
    }

    #[test]
    fn alert_mode_vpn_detected() {
        let handler = BypassResponseHandler::new(BypassResponseMode::Alert);
        assert_eq!(
            handler.handle_detection(&vpn_result()),
            BypassAction::EmitAlert
        );
    }

    #[test]
    fn alert_mode_tor_detected() {
        let handler = BypassResponseHandler::new(BypassResponseMode::Alert);
        assert_eq!(
            handler.handle_detection(&tor_result()),
            BypassAction::EmitAlert
        );
    }

    // ── Block mode ──────────────────────────────────────────────────

    #[test]
    fn block_mode_nothing_detected() {
        let handler = BypassResponseHandler::new(BypassResponseMode::Block);
        assert_eq!(handler.handle_detection(&empty_result()), BypassAction::None);
    }

    #[test]
    fn block_mode_proxy_detected() {
        let handler = BypassResponseHandler::new(BypassResponseMode::Block);
        assert_eq!(
            handler.handle_detection(&proxy_result()),
            BypassAction::BlockNetwork
        );
    }

    // ── Lockdown mode ───────────────────────────────────────────────

    #[test]
    fn lockdown_mode_nothing_detected() {
        let handler = BypassResponseHandler::new(BypassResponseMode::Lockdown);
        assert_eq!(handler.handle_detection(&empty_result()), BypassAction::None);
    }

    #[test]
    fn lockdown_mode_vpn_detected() {
        let handler = BypassResponseHandler::new(BypassResponseMode::Lockdown);
        assert_eq!(
            handler.handle_detection(&vpn_result()),
            BypassAction::EnterLockdown
        );
    }

    // ── Tor with no actual detection ────────────────────────────────

    #[test]
    fn tor_no_process_no_exit_node_is_not_detection() {
        let handler = BypassResponseHandler::new(BypassResponseMode::Alert);
        let result = BypassDetectionResult {
            vpn: None,
            proxy: None,
            tor: Some(TorInfo {
                process_detected: false,
                exit_node_match: false,
            }),
            detected_at: Utc::now(),
        };
        assert_eq!(handler.handle_detection(&result), BypassAction::None);
    }

    // ── Response mode accessor ──────────────────────────────────────

    #[test]
    fn mode_accessor() {
        let handler = BypassResponseHandler::new(BypassResponseMode::Lockdown);
        assert_eq!(handler.mode(), BypassResponseMode::Lockdown);
    }
}
