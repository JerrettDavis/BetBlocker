use std::process::Command;

/// Manages nftables rules that redirect all DNS traffic (UDP/TCP port 53)
/// to the local BetBlocker DNS resolver.
///
/// Creates a dedicated `betblocker` table in the `inet` family with
/// an output chain that redirects DNS queries to `127.0.0.1:{port}`.
/// The agent's own DNS queries are excluded via UID match to prevent loops.
pub struct NftablesManager {
    /// Port the local DNS resolver listens on.
    resolver_port: u16,
    /// UID of the betblocker agent process (for loop prevention).
    agent_uid: u32,
    /// Whether rules are currently installed.
    rules_installed: bool,
}

/// Errors from nftables operations.
#[derive(Debug, thiserror::Error)]
pub enum NftablesError {
    #[error("nft command failed: {0}")]
    CommandFailed(String),

    #[error("nft not found -- is nftables installed?")]
    NftNotFound,

    #[error("Insufficient privileges for nftables management")]
    InsufficientPrivileges,

    #[allow(dead_code)]
    #[error("Rules tampered with: {0}")]
    RulesTampered(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl NftablesManager {
    pub fn new(resolver_port: u16, agent_uid: u32) -> Self {
        Self {
            resolver_port,
            agent_uid,
            rules_installed: false,
        }
    }

    /// Generate the nftables ruleset for DNS redirection.
    ///
    /// The ruleset creates a `betblocker` table with:
    /// - An output chain that intercepts all outgoing DNS packets.
    /// - A rule excluding packets from the agent's own UID (loop prevention).
    /// - Redirect rules for both UDP and TCP port 53.
    pub fn generate_ruleset(&self) -> String {
        format!(
            r#"table inet betblocker {{
    chain output {{
        type nat hook output priority -100; policy accept;

        # Skip agent's own DNS queries to prevent redirect loops
        meta skuid {uid} accept

        # Redirect all outgoing DNS (UDP and TCP) to local resolver
        udp dport 53 redirect to :{port}
        tcp dport 53 redirect to :{port}
    }}
}}"#,
            uid = self.agent_uid,
            port = self.resolver_port,
        )
    }

    /// Install the nftables DNS redirect rules.
    ///
    /// Creates the `betblocker` table with output chain rules.
    /// Idempotent: removes existing table first if present.
    pub fn install_rules(&mut self) -> Result<(), NftablesError> {
        // Remove existing rules first (idempotent)
        let _ = self.remove_rules();

        let ruleset = self.generate_ruleset();

        let output = Command::new("nft")
            .args(["-f", "-"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(ref mut stdin) = child.stdin {
                    stdin.write_all(ruleset.as_bytes())?;
                }
                child.wait_with_output()
            })
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    NftablesError::NftNotFound
                } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                    NftablesError::InsufficientPrivileges
                } else {
                    NftablesError::Io(e)
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("Permission denied") || stderr.contains("Operation not permitted") {
                return Err(NftablesError::InsufficientPrivileges);
            }
            return Err(NftablesError::CommandFailed(stderr.to_string()));
        }

        self.rules_installed = true;
        tracing::info!(
            port = self.resolver_port,
            "nftables DNS redirect rules installed"
        );
        Ok(())
    }

    /// Remove the betblocker nftables table and all its rules.
    pub fn remove_rules(&mut self) -> Result<(), NftablesError> {
        let output = Command::new("nft")
            .args(["delete", "table", "inet", "betblocker"])
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    NftablesError::NftNotFound
                } else {
                    NftablesError::Io(e)
                }
            })?;

        // Ignore "no such table" errors (already removed)
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("No such file or directory") && !stderr.contains("does not exist") {
                return Err(NftablesError::CommandFailed(stderr.to_string()));
            }
        }

        self.rules_installed = false;
        tracing::info!("nftables DNS redirect rules removed");
        Ok(())
    }

    /// Verify that the DNS redirect rules are still in place.
    ///
    /// Called periodically (every 30s) to detect external rule removal.
    /// Returns true if rules are intact, false if they were removed.
    pub fn verify_rules(&self) -> Result<bool, NftablesError> {
        let output = Command::new("nft")
            .args(["list", "table", "inet", "betblocker"])
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    NftablesError::NftNotFound
                } else {
                    NftablesError::Io(e)
                }
            })?;

        if !output.status.success() {
            return Ok(false);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Check that our redirect rule is present
        let has_redirect =
            stdout.contains("redirect to") && stdout.contains(&format!(":{}", self.resolver_port));

        Ok(has_redirect)
    }

    /// Verify and re-install rules if they were removed externally.
    ///
    /// Returns true if rules were re-installed (tamper detected).
    pub fn verify_and_repair(&mut self) -> Result<bool, NftablesError> {
        match self.verify_rules() {
            Ok(true) => Ok(false), // Rules intact
            Ok(false) => {
                tracing::warn!(
                    "nftables rules removed externally -- re-installing (Level 1 tamper event)"
                );
                self.install_rules()?;
                Ok(true) // Tamper detected and repaired
            }
            Err(NftablesError::NftNotFound) => {
                tracing::error!("nft binary not found -- cannot verify rules");
                Err(NftablesError::NftNotFound)
            }
            Err(e) => Err(e),
        }
    }

    /// Whether rules are currently believed to be installed.
    #[allow(dead_code)]
    pub fn is_installed(&self) -> bool {
        self.rules_installed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_ruleset_contains_redirect() {
        let mgr = NftablesManager::new(5353, 0);
        let ruleset = mgr.generate_ruleset();

        assert!(ruleset.contains("table inet betblocker"));
        assert!(ruleset.contains("redirect to :5353"));
        assert!(ruleset.contains("meta skuid 0"));
        assert!(ruleset.contains("udp dport 53"));
        assert!(ruleset.contains("tcp dport 53"));
    }

    #[test]
    fn test_generate_ruleset_custom_uid() {
        let mgr = NftablesManager::new(5353, 999);
        let ruleset = mgr.generate_ruleset();
        assert!(ruleset.contains("meta skuid 999"));
    }

    #[test]
    fn test_generate_ruleset_custom_port() {
        let mgr = NftablesManager::new(8053, 0);
        let ruleset = mgr.generate_ruleset();
        assert!(ruleset.contains("redirect to :8053"));
    }

    #[test]
    fn test_not_installed_initially() {
        let mgr = NftablesManager::new(5353, 0);
        assert!(!mgr.is_installed());
    }

    // Integration tests requiring root are gated behind a feature flag.
    // Run with: cargo test --features test_nftables -- nftables
    #[cfg(feature = "test_nftables")]
    mod integration {
        use super::*;

        #[test]
        fn test_install_and_remove_rules() {
            let mut mgr = NftablesManager::new(5353, 0);
            mgr.install_rules().expect("install");
            assert!(mgr.is_installed());
            assert!(mgr.verify_rules().expect("verify"));

            mgr.remove_rules().expect("remove");
            assert!(!mgr.is_installed());
        }
    }
}
