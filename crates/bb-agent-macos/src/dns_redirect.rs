//! macOS DNS redirection via pf (packet filter).
//!
//! Manages DNS traffic interception on macOS using pf rules that
//! redirect outbound DNS queries (port 53) to the local BetBlocker
//! DNS resolver. Uses a dedicated anchor `com.betblocker` to isolate
//! rules from the system pf configuration.

#[cfg(target_os = "macos")]
use std::process::Command;

/// Errors from pf operations.
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum PfError {
    #[error("pfctl command failed: {0}")]
    CommandFailed(String),

    #[error("pfctl not found")]
    PfctlNotFound,

    #[error("insufficient privileges for pf management")]
    InsufficientPrivileges,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Manages pf (packet filter) rules for DNS redirection on macOS.
///
/// Creates an anchor `com.betblocker` with rdr (redirect) rules that
/// intercept DNS traffic and send it to the local resolver.
#[allow(dead_code)]
pub struct PfManager {
    /// Port the local DNS resolver listens on.
    resolver_port: u16,
    /// Whether rules are currently installed.
    rules_installed: bool,
}

impl PfManager {
    /// Create a new `PfManager`.
    pub fn new(resolver_port: u16) -> Self {
        Self {
            resolver_port,
            rules_installed: false,
        }
    }

    /// Generate the pf anchor rules for DNS redirection.
    ///
    /// Produces rdr rules that redirect UDP and TCP port 53 traffic
    /// to the local resolver on 127.0.0.1.
    #[allow(dead_code)]
    pub fn generate_rules(&self) -> String {
        format!(
            concat!(
                "# BetBlocker DNS redirect rules\n",
                "rdr pass on lo0 proto udp from any to any port 53 -> 127.0.0.1 port {port}\n",
                "rdr pass on lo0 proto tcp from any to any port 53 -> 127.0.0.1 port {port}\n",
                "rdr pass on en0 proto udp from any to any port 53 -> 127.0.0.1 port {port}\n",
                "rdr pass on en0 proto tcp from any to any port 53 -> 127.0.0.1 port {port}\n",
                "rdr pass on en1 proto udp from any to any port 53 -> 127.0.0.1 port {port}\n",
                "rdr pass on en1 proto tcp from any to any port 53 -> 127.0.0.1 port {port}\n",
            ),
            port = self.resolver_port,
        )
    }

    /// Install pf DNS redirect rules.
    ///
    /// Loads rules into the `com.betblocker` anchor and enables pf if needed.
    #[cfg(target_os = "macos")]
    pub fn install_rules(&mut self) -> Result<(), PfError> {
        let rules = self.generate_rules();

        // Load rules into the anchor via pfctl -a com.betblocker -f -
        let output = Command::new("pfctl")
            .args(["-a", "com.betblocker", "-f", "-"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(ref mut stdin) = child.stdin {
                    stdin.write_all(rules.as_bytes())?;
                }
                child.wait_with_output()
            })
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    PfError::PfctlNotFound
                } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                    PfError::InsufficientPrivileges
                } else {
                    PfError::Io(e)
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("Permission denied") {
                return Err(PfError::InsufficientPrivileges);
            }
            return Err(PfError::CommandFailed(stderr.to_string()));
        }

        // Enable pf if not already enabled
        let _ = Command::new("pfctl").args(["-e"]).output();

        self.rules_installed = true;
        tracing::info!(port = self.resolver_port, "pf DNS redirect rules installed");
        Ok(())
    }

    /// Stub for non-macOS platforms.
    #[cfg(not(target_os = "macos"))]
    pub fn install_rules(&mut self) -> Result<(), PfError> {
        tracing::warn!("pf install_rules is a no-op on non-macOS");
        self.rules_installed = true;
        Ok(())
    }

    /// Verify that DNS redirect rules are intact and repair if needed.
    ///
    /// Returns `Ok(true)` if rules were repaired (tamper detected),
    /// `Ok(false)` if rules are intact.
    #[cfg(target_os = "macos")]
    pub fn verify_and_repair(&mut self) -> Result<bool, PfError> {
        let output = Command::new("pfctl")
            .args(["-a", "com.betblocker", "-sr"])
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    PfError::PfctlNotFound
                } else {
                    PfError::Io(e)
                }
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let has_redirect =
            stdout.contains("rdr") && stdout.contains(&format!("port {}", self.resolver_port));

        if has_redirect {
            Ok(false) // Rules intact
        } else {
            tracing::warn!("pf rules removed externally -- re-installing");
            self.install_rules()?;
            Ok(true) // Tamper detected and repaired
        }
    }

    /// Stub for non-macOS platforms.
    #[cfg(not(target_os = "macos"))]
    pub fn verify_and_repair(&mut self) -> Result<bool, PfError> {
        Ok(false)
    }

    /// Remove the BetBlocker pf anchor rules.
    #[allow(dead_code)]
    #[cfg(target_os = "macos")]
    pub fn remove_rules(&mut self) -> Result<(), PfError> {
        // Flush the anchor rules
        let output = Command::new("pfctl")
            .args(["-a", "com.betblocker", "-F", "all"])
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    PfError::PfctlNotFound
                } else {
                    PfError::Io(e)
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Ignore "anchor not found" errors
            if !stderr.contains("No such") && !stderr.contains("does not exist") {
                return Err(PfError::CommandFailed(stderr.to_string()));
            }
        }

        self.rules_installed = false;
        tracing::info!("pf DNS redirect rules removed");
        Ok(())
    }

    /// Stub for non-macOS platforms.
    #[allow(dead_code)]
    #[cfg(not(target_os = "macos"))]
    pub fn remove_rules(&mut self) -> Result<(), PfError> {
        self.rules_installed = false;
        Ok(())
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
    fn test_generate_rules_contains_redirect() {
        let mgr = PfManager::new(5353);
        let rules = mgr.generate_rules();

        assert!(rules.contains("rdr pass"), "should contain rdr pass rules");
        assert!(rules.contains("port 53"), "should redirect port 53");
        assert!(
            rules.contains("port 5353"),
            "should redirect to resolver port"
        );
        assert!(rules.contains("127.0.0.1"), "should redirect to localhost");
    }

    #[test]
    fn test_generate_rules_both_protocols() {
        let mgr = PfManager::new(5353);
        let rules = mgr.generate_rules();

        assert!(rules.contains("proto udp"), "should include UDP rules");
        assert!(rules.contains("proto tcp"), "should include TCP rules");
    }

    #[test]
    fn test_generate_rules_custom_port() {
        let mgr = PfManager::new(8053);
        let rules = mgr.generate_rules();

        assert!(rules.contains("port 8053"), "should use custom port");
    }

    #[test]
    fn test_generate_rules_multiple_interfaces() {
        let mgr = PfManager::new(5353);
        let rules = mgr.generate_rules();

        assert!(rules.contains("on lo0"), "should include loopback");
        assert!(rules.contains("on en0"), "should include en0 (wired)");
        assert!(rules.contains("on en1"), "should include en1 (wifi)");
    }

    #[test]
    fn test_not_installed_initially() {
        let mgr = PfManager::new(5353);
        assert!(!mgr.is_installed());
    }

    #[test]
    fn test_verify_and_repair_stub() {
        #[cfg(not(target_os = "macos"))]
        {
            let mut mgr = PfManager::new(5353);
            let result = mgr.verify_and_repair();
            assert!(result.is_ok());
            assert!(
                !result.expect("should be ok"),
                "stub should report no tamper"
            );
        }
    }
}
