//! macOS platform integration for the BetBlocker agent.
//!
//! Provides macOS-specific initialization, launchd service management,
//! and platform utilities.
//!
//! The `sd_notify_*` functions exist for API parity with the Linux agent
//! but are no-ops on macOS (launchd uses KeepAlive instead of notify).

/// Notify that the service is ready.
///
/// No-op on macOS: launchd does not use the sd_notify protocol.
/// Present for API parity with the Linux agent.
pub fn sd_notify_ready() {
    // macOS launchd uses KeepAlive/RunAtLoad instead of sd_notify.
}

/// Notify that the service is stopping.
///
/// No-op on macOS.
pub fn sd_notify_stopping() {
    // No-op on macOS.
}

/// Notify the current service status.
///
/// No-op on macOS.
pub fn sd_notify_status(_status: &str) {
    // No-op on macOS.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sd_notify_ready_noop() {
        sd_notify_ready(); // Should not panic
    }

    #[test]
    fn test_sd_notify_stopping_noop() {
        sd_notify_stopping(); // Should not panic
    }

    #[test]
    fn test_sd_notify_status_noop() {
        sd_notify_status("test status"); // Should not panic
    }
}
