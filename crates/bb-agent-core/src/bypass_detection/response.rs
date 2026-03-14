use bb_common::models::bypass_detection::BypassDetectionResult;
use serde::{Deserialize, Serialize};

use super::traits::BypassDetectionError;

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

// ---------------------------------------------------------------------------
// KernelNetworkControl trait
// ---------------------------------------------------------------------------

/// Platform-level interface for network manipulation.
///
/// Implementations are responsible for interacting with OS-specific APIs to
/// block or restrict network interfaces.  Use [`StubKernelNetworkControl`] on
/// platforms that don't yet have a real implementation.
pub trait KernelNetworkControl: Send + Sync {
    /// Block the VPN/bypass network interface.
    fn block_vpn_interface(&self) -> Result<(), BypassDetectionError>;

    /// Enter full network lockdown mode (block all non-essential traffic).
    fn lockdown_network(&self) -> Result<(), BypassDetectionError>;
}

/// Stub implementation that returns [`BypassDetectionError::PlatformNotSupported`]
/// for all operations.  Used on platforms where kernel network control is not
/// yet implemented.
pub struct StubKernelNetworkControl;

impl KernelNetworkControl for StubKernelNetworkControl {
    fn block_vpn_interface(&self) -> Result<(), BypassDetectionError> {
        Err(BypassDetectionError::PlatformNotSupported)
    }

    fn lockdown_network(&self) -> Result<(), BypassDetectionError> {
        Err(BypassDetectionError::PlatformNotSupported)
    }
}

// ---------------------------------------------------------------------------
// BypassResponseHandler
// ---------------------------------------------------------------------------

/// Handles bypass detection results according to the configured response mode.
pub struct BypassResponseHandler {
    mode: BypassResponseMode,
    /// Optional kernel-level network controller.  When `None`, `Block` and
    /// `Lockdown` modes fall back to `EmitAlert`.
    kernel_control: Option<Box<dyn KernelNetworkControl>>,
}

impl BypassResponseHandler {
    /// Create a handler without kernel network control.
    pub fn new(mode: BypassResponseMode) -> Self {
        Self {
            mode,
            kernel_control: None,
        }
    }

    /// Create a handler with an explicit kernel network controller.
    pub fn with_kernel_control(
        mode: BypassResponseMode,
        kernel_control: Box<dyn KernelNetworkControl>,
    ) -> Self {
        Self {
            mode,
            kernel_control: Some(kernel_control),
        }
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
                if !anything_detected {
                    return BypassAction::None;
                }
                match &self.kernel_control {
                    Some(kc) => {
                        match kc.block_vpn_interface() {
                            Ok(()) => BypassAction::BlockNetwork,
                            Err(BypassDetectionError::PlatformNotSupported) => {
                                tracing::warn!(
                                    "Block mode requested but platform not supported; falling back to alert"
                                );
                                BypassAction::EmitAlert
                            }
                            Err(e) => {
                                tracing::error!(error = %e, "kernel block_vpn_interface failed");
                                BypassAction::EmitAlert
                            }
                        }
                    }
                    None => {
                        tracing::warn!(
                            "Block mode requested but no kernel controller configured; falling back to alert"
                        );
                        BypassAction::EmitAlert
                    }
                }
            }
            BypassResponseMode::Lockdown => {
                if !anything_detected {
                    return BypassAction::None;
                }
                match &self.kernel_control {
                    Some(kc) => {
                        match kc.lockdown_network() {
                            Ok(()) => BypassAction::EnterLockdown,
                            Err(BypassDetectionError::PlatformNotSupported) => {
                                tracing::warn!(
                                    "Lockdown mode requested but platform not supported; falling back to alert"
                                );
                                BypassAction::EmitAlert
                            }
                            Err(e) => {
                                tracing::error!(error = %e, "kernel lockdown_network failed");
                                BypassAction::EmitAlert
                            }
                        }
                    }
                    None => {
                        tracing::warn!(
                            "Lockdown mode requested but no kernel controller configured; falling back to alert"
                        );
                        BypassAction::EmitAlert
                    }
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

    // ── Block mode without kernel controller (fallback to alert) ────

    #[test]
    fn block_mode_nothing_detected() {
        let handler = BypassResponseHandler::new(BypassResponseMode::Block);
        assert_eq!(handler.handle_detection(&empty_result()), BypassAction::None);
    }

    #[test]
    fn block_mode_no_kernel_falls_back_to_alert() {
        let handler = BypassResponseHandler::new(BypassResponseMode::Block);
        assert_eq!(
            handler.handle_detection(&proxy_result()),
            BypassAction::EmitAlert,
            "no kernel controller: should fall back to EmitAlert"
        );
    }

    // ── Block mode with stub kernel controller (PlatformNotSupported) ─

    #[test]
    fn block_mode_stub_kernel_falls_back_to_alert() {
        let handler = BypassResponseHandler::with_kernel_control(
            BypassResponseMode::Block,
            Box::new(StubKernelNetworkControl),
        );
        assert_eq!(
            handler.handle_detection(&vpn_result()),
            BypassAction::EmitAlert,
            "stub kernel returns PlatformNotSupported: should fall back to EmitAlert"
        );
    }

    // ── Block mode with working kernel controller ────────────────────

    struct WorkingKernel;
    impl KernelNetworkControl for WorkingKernel {
        fn block_vpn_interface(&self) -> Result<(), BypassDetectionError> {
            Ok(())
        }
        fn lockdown_network(&self) -> Result<(), BypassDetectionError> {
            Ok(())
        }
    }

    #[test]
    fn block_mode_working_kernel_returns_block_network() {
        let handler = BypassResponseHandler::with_kernel_control(
            BypassResponseMode::Block,
            Box::new(WorkingKernel),
        );
        assert_eq!(
            handler.handle_detection(&vpn_result()),
            BypassAction::BlockNetwork
        );
    }

    // ── Lockdown mode without kernel controller ──────────────────────

    #[test]
    fn lockdown_mode_nothing_detected() {
        let handler = BypassResponseHandler::new(BypassResponseMode::Lockdown);
        assert_eq!(handler.handle_detection(&empty_result()), BypassAction::None);
    }

    #[test]
    fn lockdown_mode_no_kernel_falls_back_to_alert() {
        let handler = BypassResponseHandler::new(BypassResponseMode::Lockdown);
        assert_eq!(
            handler.handle_detection(&vpn_result()),
            BypassAction::EmitAlert,
            "no kernel controller: lockdown should fall back to EmitAlert"
        );
    }

    // ── Lockdown mode with stub kernel ───────────────────────────────

    #[test]
    fn lockdown_mode_stub_kernel_falls_back_to_alert() {
        let handler = BypassResponseHandler::with_kernel_control(
            BypassResponseMode::Lockdown,
            Box::new(StubKernelNetworkControl),
        );
        assert_eq!(
            handler.handle_detection(&vpn_result()),
            BypassAction::EmitAlert
        );
    }

    // ── Lockdown mode with working kernel ────────────────────────────

    #[test]
    fn lockdown_mode_working_kernel_returns_enter_lockdown() {
        let handler = BypassResponseHandler::with_kernel_control(
            BypassResponseMode::Lockdown,
            Box::new(WorkingKernel),
        );
        assert_eq!(
            handler.handle_detection(&vpn_result()),
            BypassAction::EnterLockdown
        );
    }

    // ── StubKernelNetworkControl ─────────────────────────────────────

    #[test]
    fn stub_kernel_block_returns_platform_not_supported() {
        let stub = StubKernelNetworkControl;
        assert!(matches!(
            stub.block_vpn_interface(),
            Err(BypassDetectionError::PlatformNotSupported)
        ));
    }

    #[test]
    fn stub_kernel_lockdown_returns_platform_not_supported() {
        let stub = StubKernelNetworkControl;
        assert!(matches!(
            stub.lockdown_network(),
            Err(BypassDetectionError::PlatformNotSupported)
        ));
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
