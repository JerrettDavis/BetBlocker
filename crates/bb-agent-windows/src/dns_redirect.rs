//! Windows DNS redirection.
//!
//! Manages DNS traffic interception on Windows using Windows Firewall
//! rules to redirect port 53 traffic to the local BetBlocker resolver.

/// Errors from DNS redirect operations.
#[derive(Debug, thiserror::Error)]
pub enum DnsRedirectError {
    /// Failed to install firewall rules.
    #[error("failed to install firewall rules: {0}")]
    InstallFailed(String),

    /// Failed to remove firewall rules.
    #[error("failed to remove firewall rules: {0}")]
    RemoveFailed(String),

    /// Failed to verify firewall rules.
    #[error("failed to verify firewall rules: {0}")]
    VerifyFailed(String),

    /// IO error from command execution.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Rule name prefix used for BetBlocker DNS redirect firewall rules.
const RULE_NAME_PREFIX: &str = "BetBlocker-DNS-Redirect";

/// Manages Windows Firewall rules for DNS redirection.
///
/// Installs `netsh advfirewall` rules that intercept outbound port 53
/// traffic and redirect it to the local BetBlocker DNS resolver.
pub struct WindowsDnsRedirect {
    /// Port the local BetBlocker resolver listens on.
    pub resolver_port: u16,
    /// Whether firewall rules are currently installed.
    pub rules_installed: bool,
}

impl WindowsDnsRedirect {
    /// Create a new `WindowsDnsRedirect` targeting the given resolver port.
    pub fn new(resolver_port: u16) -> Self {
        Self {
            resolver_port,
            rules_installed: false,
        }
    }

    /// Build the netsh command arguments for the UDP redirect rule.
    fn udp_rule_args(&self, action: &str) -> Vec<String> {
        let rule_name = format!("{RULE_NAME_PREFIX}-UDP");
        match action {
            "add" => vec![
                "advfirewall".to_string(),
                "firewall".to_string(),
                "add".to_string(),
                "rule".to_string(),
                format!("name={rule_name}"),
                "dir=out".to_string(),
                "action=allow".to_string(),
                "protocol=UDP".to_string(),
                "remoteport=53".to_string(),
                format!("localport={}", self.resolver_port),
                "enable=yes".to_string(),
            ],
            "delete" => vec![
                "advfirewall".to_string(),
                "firewall".to_string(),
                "delete".to_string(),
                "rule".to_string(),
                format!("name={rule_name}"),
            ],
            _ => vec![],
        }
    }

    /// Build the netsh command arguments for the TCP redirect rule.
    fn tcp_rule_args(&self, action: &str) -> Vec<String> {
        let rule_name = format!("{RULE_NAME_PREFIX}-TCP");
        match action {
            "add" => vec![
                "advfirewall".to_string(),
                "firewall".to_string(),
                "add".to_string(),
                "rule".to_string(),
                format!("name={rule_name}"),
                "dir=out".to_string(),
                "action=allow".to_string(),
                "protocol=TCP".to_string(),
                "remoteport=53".to_string(),
                format!("localport={}", self.resolver_port),
                "enable=yes".to_string(),
            ],
            "delete" => vec![
                "advfirewall".to_string(),
                "firewall".to_string(),
                "delete".to_string(),
                "rule".to_string(),
                format!("name={rule_name}"),
            ],
            _ => vec![],
        }
    }

    /// Build the netsh command arguments for the block rule (blocks
    /// non-BetBlocker DNS traffic on port 53).
    fn block_rule_args(&self, action: &str) -> Vec<String> {
        let rule_name = format!("{RULE_NAME_PREFIX}-Block");
        match action {
            "add" => vec![
                "advfirewall".to_string(),
                "firewall".to_string(),
                "add".to_string(),
                "rule".to_string(),
                format!("name={rule_name}"),
                "dir=out".to_string(),
                "action=block".to_string(),
                "protocol=UDP".to_string(),
                "remoteport=53".to_string(),
                "enable=yes".to_string(),
            ],
            "delete" => vec![
                "advfirewall".to_string(),
                "firewall".to_string(),
                "delete".to_string(),
                "rule".to_string(),
                format!("name={rule_name}"),
            ],
            _ => vec![],
        }
    }

    /// Install Windows Firewall rules to redirect port 53 traffic.
    ///
    /// Creates rules that:
    /// 1. Allow outbound UDP/TCP on port 53 from the local resolver port
    /// 2. Block other outbound DNS traffic on port 53
    #[cfg(target_os = "windows")]
    pub fn install_rules(&mut self) -> Result<(), DnsRedirectError> {
        // Remove any existing rules first to ensure idempotency
        let _ = self.remove_rules();

        let rules = [
            self.udp_rule_args("add"),
            self.tcp_rule_args("add"),
            self.block_rule_args("add"),
        ];

        for args in &rules {
            let output = std::process::Command::new("netsh")
                .args(args)
                .output()
                .map_err(|e| DnsRedirectError::InstallFailed(e.to_string()))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(DnsRedirectError::InstallFailed(format!(
                    "netsh failed: {stderr}"
                )));
            }
        }

        self.rules_installed = true;
        tracing::info!(
            resolver_port = self.resolver_port,
            "DNS redirect firewall rules installed"
        );
        Ok(())
    }

    /// Stub for non-Windows platforms.
    #[cfg(not(target_os = "windows"))]
    pub fn install_rules(&mut self) -> Result<(), DnsRedirectError> {
        self.rules_installed = true;
        Ok(())
    }

    /// Remove all BetBlocker DNS redirect firewall rules.
    #[cfg(target_os = "windows")]
    pub fn remove_rules(&mut self) -> Result<(), DnsRedirectError> {
        let rules = [
            self.udp_rule_args("delete"),
            self.tcp_rule_args("delete"),
            self.block_rule_args("delete"),
        ];

        for args in &rules {
            let output = std::process::Command::new("netsh")
                .args(args)
                .output()
                .map_err(|e| DnsRedirectError::RemoveFailed(e.to_string()))?;

            // Ignore errors from deleting non-existent rules
            if !output.status.success() {
                tracing::debug!(
                    args = ?args,
                    "Rule deletion returned non-zero (may not exist)"
                );
            }
        }

        self.rules_installed = false;
        tracing::info!("DNS redirect firewall rules removed");
        Ok(())
    }

    /// Stub for non-Windows platforms.
    #[cfg(not(target_os = "windows"))]
    pub fn remove_rules(&mut self) -> Result<(), DnsRedirectError> {
        self.rules_installed = false;
        Ok(())
    }

    /// Verify firewall rules are in place and reinstall if needed.
    ///
    /// Returns `Ok(true)` if rules were already present, `Ok(false)` if
    /// they had to be reinstalled.
    #[cfg(target_os = "windows")]
    pub fn verify_and_repair(&mut self) -> Result<bool, DnsRedirectError> {
        let output = std::process::Command::new("netsh")
            .args([
                "advfirewall",
                "firewall",
                "show",
                "rule",
                &format!("name={RULE_NAME_PREFIX}-UDP"),
            ])
            .output()
            .map_err(|e| DnsRedirectError::VerifyFailed(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if output.status.success() && stdout.contains(RULE_NAME_PREFIX) {
            // Rules exist
            self.rules_installed = true;
            Ok(true)
        } else {
            // Rules missing, reinstall
            tracing::warn!("DNS redirect rules missing, reinstalling");
            self.install_rules()?;
            Ok(false)
        }
    }

    /// Stub for non-Windows platforms.
    #[cfg(not(target_os = "windows"))]
    pub fn verify_and_repair(&mut self) -> Result<bool, DnsRedirectError> {
        if self.rules_installed {
            Ok(true)
        } else {
            self.install_rules()?;
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_redirect() {
        let redirect = WindowsDnsRedirect::new(5353);
        assert_eq!(redirect.resolver_port, 5353);
        assert!(!redirect.rules_installed);
    }

    #[test]
    fn udp_rule_args_add_contains_port() {
        let redirect = WindowsDnsRedirect::new(5353);
        let args = redirect.udp_rule_args("add");
        assert!(args.iter().any(|a| a.contains("UDP")));
        assert!(args.iter().any(|a| a.contains("53")));
        assert!(args.iter().any(|a| a.contains("5353")));
        assert!(args.iter().any(|a| a.contains(RULE_NAME_PREFIX)));
    }

    #[test]
    fn tcp_rule_args_add_contains_port() {
        let redirect = WindowsDnsRedirect::new(5353);
        let args = redirect.tcp_rule_args("add");
        assert!(args.iter().any(|a| a.contains("TCP")));
        assert!(args.iter().any(|a| a.contains("53")));
    }

    #[test]
    fn block_rule_args_add_contains_block() {
        let redirect = WindowsDnsRedirect::new(5353);
        let args = redirect.block_rule_args("add");
        assert!(args.iter().any(|a| a.contains("block")));
        assert!(args.iter().any(|a| a.contains("53")));
    }

    #[test]
    fn delete_rule_args_contain_rule_name() {
        let redirect = WindowsDnsRedirect::new(5353);
        let args = redirect.udp_rule_args("delete");
        assert!(args.iter().any(|a| a.contains(RULE_NAME_PREFIX)));
        assert!(args.iter().any(|a| a == "delete"));
    }

    #[test]
    fn idempotent_install_and_remove() {
        // On Windows, netsh requires admin; test the non-Windows stub path only
        #[cfg(not(target_os = "windows"))]
        {
            let mut redirect = WindowsDnsRedirect::new(5353);

            // First install
            assert!(redirect.install_rules().is_ok());
            assert!(redirect.rules_installed);

            // Second install (idempotent)
            assert!(redirect.install_rules().is_ok());
            assert!(redirect.rules_installed);

            // Remove
            assert!(redirect.remove_rules().is_ok());
            assert!(!redirect.rules_installed);

            // Remove again (idempotent)
            assert!(redirect.remove_rules().is_ok());
            assert!(!redirect.rules_installed);
        }
    }

    #[test]
    fn verify_and_repair_when_not_installed() {
        // On Windows, netsh requires admin; test the non-Windows stub path only
        #[cfg(not(target_os = "windows"))]
        {
            let mut redirect = WindowsDnsRedirect::new(5353);
            assert!(!redirect.rules_installed);

            let result = redirect.verify_and_repair();
            assert!(result.is_ok());
            assert!(!result.unwrap()); // false = had to reinstall
        }
    }

    #[test]
    fn error_display() {
        let err = DnsRedirectError::InstallFailed("access denied".to_string());
        assert!(err.to_string().contains("access denied"));

        let err = DnsRedirectError::RemoveFailed("not found".to_string());
        assert!(err.to_string().contains("not found"));

        let err = DnsRedirectError::VerifyFailed("timeout".to_string());
        assert!(err.to_string().contains("timeout"));
    }
}
