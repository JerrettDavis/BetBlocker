//! DNS query monitoring on macOS.
//!
//! Monitors and intercepts DNS resolution using macOS networking APIs.
//! On macOS, uses SCDynamicStore to observe DNS configuration changes.
//! On non-macOS platforms, provides type-compatible stubs.

use tokio::sync::watch;

/// Errors from DNS monitor operations.
#[derive(Debug, thiserror::Error)]
pub enum DnsMonitorError {
    #[error("failed to create SCDynamicStore session")]
    SessionCreationFailed,

    #[error("failed to read DNS configuration: {0}")]
    ReadFailed(String),

    #[error("failed to enforce DNS settings: {0}")]
    EnforceFailed(String),

    #[error("monitor task error: {0}")]
    TaskError(String),
}

/// Monitors macOS DNS configuration and enforces BetBlocker DNS settings.
///
/// On macOS, uses the `SCDynamicStore` API (via the SystemConfiguration
/// framework) to watch for DNS server changes and re-apply BetBlocker's
/// DNS configuration when external changes are detected.
pub struct DnsMonitor {
    /// The DNS server addresses BetBlocker enforces (e.g. 127.0.0.1).
    pub enforced_servers: Vec<String>,
}

impl DnsMonitor {
    /// Create a new DNS monitor.
    ///
    /// `enforced_servers` are the DNS server addresses that BetBlocker
    /// will enforce (typically `["127.0.0.1"]` for local resolver).
    pub fn new(enforced_servers: Vec<String>) -> Self {
        Self { enforced_servers }
    }

    /// Start the DNS monitor loop.
    ///
    /// Watches for DNS configuration changes and re-enforces settings.
    /// Runs until the shutdown signal is received.
    #[cfg(target_os = "macos")]
    pub async fn start(
        &self,
        mut shutdown_rx: watch::Receiver<bool>,
    ) -> Result<(), DnsMonitorError> {
        tracing::info!(servers = ?self.enforced_servers, "DNS monitor starting");

        // Initial enforcement
        self.enforce_dns()?;

        // Poll loop: check DNS settings periodically
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let current = self.current_dns_servers();
                    if current != self.enforced_servers {
                        tracing::warn!(
                            current = ?current,
                            expected = ?self.enforced_servers,
                            "DNS configuration changed externally, re-enforcing"
                        );
                        if let Err(e) = self.enforce_dns() {
                            tracing::error!(error = %e, "Failed to re-enforce DNS");
                        }
                    }
                }
                _ = shutdown_rx.changed() => {
                    tracing::info!("DNS monitor shutting down");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Stub start for non-macOS platforms.
    #[cfg(not(target_os = "macos"))]
    pub async fn start(
        &self,
        mut shutdown_rx: watch::Receiver<bool>,
    ) -> Result<(), DnsMonitorError> {
        tracing::info!("DNS monitor is a no-op on non-macOS");
        // Wait for shutdown signal to keep the API consistent
        let _ = shutdown_rx.changed().await;
        Ok(())
    }

    /// Get the currently configured DNS servers.
    ///
    /// On macOS, reads from SCDynamicStore. On other platforms, returns
    /// an empty list.
    #[cfg(target_os = "macos")]
    pub fn current_dns_servers(&self) -> Vec<String> {
        // Use scutil to read DNS configuration
        let output = std::process::Command::new("scutil")
            .args(["--dns"])
            .output();

        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let mut servers = Vec::new();
                for line in stdout.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("nameserver[") {
                        // Format: "nameserver[0] : 1.2.3.4"
                        if let Some(addr) = trimmed.split(':').nth(1) {
                            let addr = addr.trim().to_string();
                            if !addr.is_empty() && !servers.contains(&addr) {
                                servers.push(addr);
                            }
                        }
                    }
                }
                servers
            }
            _ => Vec::new(),
        }
    }

    /// Stub for non-macOS platforms.
    #[cfg(not(target_os = "macos"))]
    pub fn current_dns_servers(&self) -> Vec<String> {
        Vec::new()
    }

    /// Enforce BetBlocker DNS settings on all network services.
    ///
    /// Sets the DNS servers for all active network interfaces to the
    /// configured enforced servers.
    #[cfg(target_os = "macos")]
    pub fn enforce_dns(&self) -> Result<(), DnsMonitorError> {
        // List all network services
        let output = std::process::Command::new("networksetup")
            .args(["-listallnetworkservices"])
            .output()
            .map_err(|e| DnsMonitorError::EnforceFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(DnsMonitorError::EnforceFailed(
                "Failed to list network services".to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().skip(1) {
            // Skip the header line
            let service = line.trim();
            if service.is_empty() || service.starts_with('*') {
                continue;
            }

            let mut args = vec!["-setdnsservers", service];
            for server in &self.enforced_servers {
                args.push(server);
            }

            let result = std::process::Command::new("networksetup")
                .args(&args)
                .output();

            match result {
                Ok(o) if !o.status.success() => {
                    tracing::debug!(
                        service = service,
                        "Failed to set DNS for network service (may be inactive)"
                    );
                }
                Err(e) => {
                    tracing::debug!(
                        service = service,
                        error = %e,
                        "Failed to set DNS for network service"
                    );
                }
                _ => {
                    tracing::debug!(service = service, "DNS enforced for network service");
                }
            }
        }

        tracing::info!(servers = ?self.enforced_servers, "DNS settings enforced");
        Ok(())
    }

    /// Stub for non-macOS platforms.
    #[cfg(not(target_os = "macos"))]
    pub fn enforce_dns(&self) -> Result<(), DnsMonitorError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_dns_monitor() {
        let monitor = DnsMonitor::new(vec!["127.0.0.1".to_string()]);
        assert_eq!(monitor.enforced_servers, vec!["127.0.0.1"]);
    }

    #[test]
    fn test_current_dns_servers_stub() {
        #[cfg(not(target_os = "macos"))]
        {
            let monitor = DnsMonitor::new(vec!["127.0.0.1".to_string()]);
            let servers = monitor.current_dns_servers();
            assert!(servers.is_empty(), "stub should return empty list");
        }
    }

    #[test]
    fn test_enforce_dns_stub() {
        #[cfg(not(target_os = "macos"))]
        {
            let monitor = DnsMonitor::new(vec!["127.0.0.1".to_string()]);
            assert!(monitor.enforce_dns().is_ok(), "stub enforce should succeed");
        }
    }
}
