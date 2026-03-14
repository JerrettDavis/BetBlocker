//! DNS query monitoring on Windows.
//!
//! Intercepts and monitors DNS traffic using Windows networking APIs.
//! On Windows, uses `GetAdaptersAddresses` to enumerate DNS servers
//! and `netsh` to enforce DNS settings. On non-Windows platforms,
//! provides type-compatible stubs.

use std::collections::HashMap;
use std::net::IpAddr;

use chrono::{DateTime, Utc};
use tokio::sync::watch;
use tokio::task::JoinHandle;

/// Errors from DNS monitor operations.
#[derive(Debug, thiserror::Error)]
pub enum DnsMonitorError {
    /// Underlying Win32 API error.
    #[error("Win32 error: {0}")]
    Win32Error(String),

    /// The monitoring loop encountered an error.
    #[error("monitoring failed: {0}")]
    MonitoringFailed(String),

    /// Failed to enforce DNS settings on an adapter.
    #[error("DNS enforcement failed: {0}")]
    EnforcementFailed(String),
}

/// Represents a detected DNS configuration change on a network adapter.
#[derive(Debug, Clone)]
pub struct DnsChange {
    /// The name of the network adapter that changed.
    pub adapter_name: String,
    /// The DNS servers configured before the change.
    pub old_servers: Vec<IpAddr>,
    /// The DNS servers configured after the change.
    pub new_servers: Vec<IpAddr>,
    /// When the change was detected.
    pub timestamp: DateTime<Utc>,
}

/// Monitors DNS configuration and enforces BetBlocker DNS settings.
///
/// Periodically polls adapter DNS servers and re-applies the expected
/// configuration when external changes are detected.
pub struct DnsMonitor {
    /// The DNS server address that BetBlocker enforces (e.g. `127.0.0.1`).
    pub expected_dns: IpAddr,
}

impl DnsMonitor {
    /// Create a new DNS monitor with the given expected DNS server address.
    pub fn new(expected_dns: IpAddr) -> Self {
        Self { expected_dns }
    }

    /// Retrieve the current DNS servers for every network adapter.
    ///
    /// On Windows, calls `GetAdaptersAddresses` to enumerate adapters.
    /// On non-Windows, returns an empty map.
    #[cfg(target_os = "windows")]
    pub fn get_current_dns_servers() -> Result<HashMap<String, Vec<IpAddr>>, DnsMonitorError> {
        // Use netsh to enumerate DNS servers per adapter.
        let output = std::process::Command::new("netsh")
            .args(["interface", "ip", "show", "dnsservers"])
            .output()
            .map_err(|e| DnsMonitorError::Win32Error(e.to_string()))?;

        if !output.status.success() {
            return Err(DnsMonitorError::Win32Error(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut result: HashMap<String, Vec<IpAddr>> = HashMap::new();
        let mut current_adapter: Option<String> = None;

        for line in stdout.lines() {
            let trimmed = line.trim();
            // Lines like: Configuration for interface "Ethernet"
            if let Some(rest) = trimmed.strip_prefix("Configuration for interface") {
                let name = rest.trim().trim_matches('"').trim().to_string();
                if !name.is_empty() {
                    current_adapter = Some(name);
                }
            } else if let Some(ref adapter) = current_adapter {
                // Try to parse IP addresses from the line
                // DNS servers lines contain IP addresses
                for word in trimmed.split_whitespace() {
                    if let Ok(ip) = word.parse::<IpAddr>() {
                        result.entry(adapter.clone()).or_default().push(ip);
                    }
                }
            }
        }

        Ok(result)
    }

    /// Stub for non-Windows platforms.
    #[cfg(not(target_os = "windows"))]
    pub fn get_current_dns_servers() -> Result<HashMap<String, Vec<IpAddr>>, DnsMonitorError> {
        Ok(HashMap::new())
    }

    /// Start the DNS monitoring loop.
    ///
    /// Polls DNS servers every 30 seconds, detects changes, and enforces
    /// the expected DNS configuration. Returns a `JoinHandle` to the
    /// spawned monitoring task.
    pub fn start_monitoring(
        &self,
        mut shutdown_rx: watch::Receiver<bool>,
    ) -> JoinHandle<Result<(), DnsMonitorError>> {
        let expected_dns = self.expected_dns;

        tokio::spawn(async move {
            tracing::info!(dns = %expected_dns, "DNS monitoring started");

            let mut previous_state = Self::get_current_dns_servers().unwrap_or_default();

            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let current = match Self::get_current_dns_servers() {
                            Ok(c) => c,
                            Err(e) => {
                                tracing::error!(error = %e, "Failed to query DNS servers");
                                continue;
                            }
                        };

                        // Detect changes per adapter
                        for (adapter, servers) in &current {
                            let old = previous_state.get(adapter).cloned().unwrap_or_default();
                            if *servers != old {
                                let change = DnsChange {
                                    adapter_name: adapter.clone(),
                                    old_servers: old,
                                    new_servers: servers.clone(),
                                    timestamp: Utc::now(),
                                };
                                tracing::warn!(
                                    adapter = %change.adapter_name,
                                    old = ?change.old_servers,
                                    new = ?change.new_servers,
                                    "DNS configuration changed, re-enforcing"
                                );

                                // Check if expected DNS is present
                                if !servers.contains(&expected_dns) {
                                    if let Err(e) = Self::enforce_dns(adapter, &expected_dns) {
                                        tracing::error!(
                                            adapter = %adapter,
                                            error = %e,
                                            "Failed to enforce DNS"
                                        );
                                    }
                                }
                            }
                        }

                        previous_state = current;
                    }
                    _ = shutdown_rx.changed() => {
                        tracing::info!("DNS monitoring shutting down");
                        break;
                    }
                }
            }

            Ok(())
        })
    }

    /// Enforce BetBlocker DNS settings on a specific adapter via `netsh`.
    #[cfg(target_os = "windows")]
    pub fn enforce_dns(adapter: &str, dns_server: &IpAddr) -> Result<(), DnsMonitorError> {
        let output = std::process::Command::new("netsh")
            .args([
                "interface",
                "ip",
                "set",
                "dnsservers",
                adapter,
                "static",
                &dns_server.to_string(),
                "primary",
            ])
            .output()
            .map_err(|e| DnsMonitorError::EnforcementFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DnsMonitorError::EnforcementFailed(format!(
                "netsh failed for adapter {adapter}: {stderr}"
            )));
        }

        tracing::info!(adapter = %adapter, dns = %dns_server, "DNS enforced via netsh");
        Ok(())
    }

    /// Stub for non-Windows platforms.
    #[cfg(not(target_os = "windows"))]
    pub fn enforce_dns(_adapter: &str, _dns_server: &IpAddr) -> Result<(), DnsMonitorError> {
        Err(DnsMonitorError::EnforcementFailed(
            "DNS enforcement is only available on Windows".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn dns_change_fields() {
        let change = DnsChange {
            adapter_name: "Ethernet".to_string(),
            old_servers: vec![IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))],
            new_servers: vec![IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))],
            timestamp: Utc::now(),
        };
        assert_eq!(change.adapter_name, "Ethernet");
        assert_eq!(change.old_servers.len(), 1);
        assert_eq!(change.new_servers.len(), 1);
    }

    #[test]
    fn dns_change_clone() {
        let change = DnsChange {
            adapter_name: "Wi-Fi".to_string(),
            old_servers: vec![],
            new_servers: vec![IpAddr::V4(Ipv4Addr::LOCALHOST)],
            timestamp: Utc::now(),
        };
        let cloned = change.clone();
        assert_eq!(change.adapter_name, cloned.adapter_name);
        assert_eq!(change.new_servers, cloned.new_servers);
    }

    #[test]
    fn dns_monitor_construction() {
        let monitor = DnsMonitor::new(IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_eq!(monitor.expected_dns, IpAddr::V4(Ipv4Addr::LOCALHOST));
    }

    #[test]
    fn dns_monitor_error_display() {
        let err = DnsMonitorError::Win32Error("access denied".to_string());
        assert!(err.to_string().contains("access denied"));

        let err = DnsMonitorError::MonitoringFailed("timeout".to_string());
        assert!(err.to_string().contains("timeout"));

        let err = DnsMonitorError::EnforcementFailed("netsh failed".to_string());
        assert!(err.to_string().contains("netsh failed"));
    }

    #[test]
    fn get_current_dns_servers_returns_map() {
        // On non-Windows, returns empty map; on Windows, returns actual adapters
        let result = DnsMonitor::get_current_dns_servers();
        assert!(result.is_ok());
    }
}
