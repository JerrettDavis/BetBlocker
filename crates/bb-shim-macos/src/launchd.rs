//! macOS launchd service management.
//!
//! Handles LaunchDaemon plist generation, loading/unloading, and
//! service lifecycle via `launchctl`.

use std::path::PathBuf;

/// Errors from launchd operations.
#[derive(Debug, thiserror::Error)]
pub enum LaunchdError {
    #[error("launchctl command failed: {0}")]
    CommandFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("plist file not found: {0}")]
    PlistNotFound(PathBuf),
}

/// Represents a launchd plist configuration for managing a LaunchDaemon.
pub struct LaunchdPlist {
    /// Path where the plist file will be written (e.g. /Library/LaunchDaemons/com.betblocker.agent.plist).
    pub plist_path: PathBuf,
    /// The launchd label (e.g. com.betblocker.agent).
    pub label: String,
    /// Path to the program binary.
    pub program_path: PathBuf,
}

impl LaunchdPlist {
    /// Create a new `LaunchdPlist` with default BetBlocker agent settings.
    pub fn new_agent() -> Self {
        Self {
            plist_path: PathBuf::from("/Library/LaunchDaemons/com.betblocker.agent.plist"),
            label: "com.betblocker.agent".to_string(),
            program_path: PathBuf::from("/usr/local/bin/bb-agent-macos"),
        }
    }

    /// Create a `LaunchdPlist` with custom paths.
    pub fn new(
        plist_path: impl Into<PathBuf>,
        label: impl Into<String>,
        program_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            plist_path: plist_path.into(),
            label: label.into(),
            program_path: program_path.into(),
        }
    }

    /// Generate the XML plist content.
    ///
    /// Produces a valid launchd plist with:
    /// - `KeepAlive` = true (auto-restart on crash)
    /// - `RunAtLoad` = true (start on boot)
    /// - Standard output/error logging to /var/log/betblocker/
    pub fn generate(&self) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>Program</key>
    <string>{program}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{program}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/var/log/betblocker/agent.out.log</string>
    <key>StandardErrorPath</key>
    <string>/var/log/betblocker/agent.err.log</string>
</dict>
</plist>
"#,
            label = self.label,
            program = self.program_path.display(),
        )
    }

    /// Install the LaunchDaemon: write the plist and bootstrap via launchctl.
    #[cfg(target_os = "macos")]
    pub fn install(&self) -> Result<(), LaunchdError> {
        // Write plist file
        std::fs::write(&self.plist_path, self.generate())?;

        // Set restrictive permissions on the plist
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o644);
            std::fs::set_permissions(&self.plist_path, perms)?;
        }

        // Bootstrap the daemon into the system domain
        let output = std::process::Command::new("launchctl")
            .args(["bootstrap", "system", &self.plist_path.to_string_lossy()])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Ignore "already bootstrapped" errors
            if !stderr.contains("already bootstrapped")
                && !stderr.contains("service already loaded")
            {
                return Err(LaunchdError::CommandFailed(stderr.to_string()));
            }
        }

        tracing::info!(label = %self.label, "LaunchDaemon installed and bootstrapped");
        Ok(())
    }

    /// Stub install for non-macOS platforms.
    #[cfg(not(target_os = "macos"))]
    pub fn install(&self) -> Result<(), LaunchdError> {
        tracing::warn!("launchd install is a no-op on non-macOS");
        Ok(())
    }

    /// Uninstall the LaunchDaemon: bootout via launchctl and remove plist.
    #[cfg(target_os = "macos")]
    pub fn uninstall(&self) -> Result<(), LaunchdError> {
        // Bootout the daemon from the system domain
        let output = std::process::Command::new("launchctl")
            .args(["bootout", &format!("system/{}", self.label)])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Ignore "not found" errors (already unloaded)
            if !stderr.contains("Could not find specified service")
                && !stderr.contains("No such process")
            {
                return Err(LaunchdError::CommandFailed(stderr.to_string()));
            }
        }

        // Remove the plist file
        if self.plist_path.exists() {
            std::fs::remove_file(&self.plist_path)?;
        }

        tracing::info!(label = %self.label, "LaunchDaemon uninstalled");
        Ok(())
    }

    /// Stub uninstall for non-macOS platforms.
    #[cfg(not(target_os = "macos"))]
    pub fn uninstall(&self) -> Result<(), LaunchdError> {
        tracing::warn!("launchd uninstall is a no-op on non-macOS");
        Ok(())
    }

    /// Check whether the daemon is currently loaded.
    #[cfg(target_os = "macos")]
    pub fn is_loaded(&self) -> bool {
        let output = std::process::Command::new("launchctl")
            .args(["print", &format!("system/{}", self.label)])
            .output();

        match output {
            Ok(o) => o.status.success(),
            Err(_) => false,
        }
    }

    /// Stub for non-macOS platforms.
    #[cfg(not(target_os = "macos"))]
    pub fn is_loaded(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_produces_valid_xml_header() {
        let plist = LaunchdPlist::new_agent();
        let xml = plist.generate();

        assert!(
            xml.starts_with("<?xml version=\"1.0\""),
            "should start with XML declaration"
        );
        assert!(xml.contains("<!DOCTYPE plist"), "should contain DOCTYPE");
        assert!(
            xml.contains("<plist version=\"1.0\">"),
            "should contain plist root"
        );
        assert!(xml.contains("</plist>"), "should close plist root");
    }

    #[test]
    fn test_generate_contains_keep_alive() {
        let plist = LaunchdPlist::new_agent();
        let xml = plist.generate();

        assert!(
            xml.contains("<key>KeepAlive</key>"),
            "should contain KeepAlive key"
        );
        assert!(xml.contains("<true/>"), "should have KeepAlive set to true");
    }

    #[test]
    fn test_generate_contains_run_at_load() {
        let plist = LaunchdPlist::new_agent();
        let xml = plist.generate();

        assert!(
            xml.contains("<key>RunAtLoad</key>"),
            "should contain RunAtLoad key"
        );
    }

    #[test]
    fn test_generate_contains_label() {
        let plist = LaunchdPlist::new_agent();
        let xml = plist.generate();

        assert!(xml.contains("<key>Label</key>"), "should contain Label key");
        assert!(
            xml.contains("<string>com.betblocker.agent</string>"),
            "should contain the agent label value"
        );
    }

    #[test]
    fn test_generate_contains_program_path() {
        let plist = LaunchdPlist::new_agent();
        let xml = plist.generate();

        assert!(
            xml.contains("/usr/local/bin/bb-agent-macos"),
            "should contain the program path"
        );
    }

    #[test]
    fn test_generate_contains_log_paths() {
        let plist = LaunchdPlist::new_agent();
        let xml = plist.generate();

        assert!(xml.contains("/var/log/betblocker/agent.out.log"));
        assert!(xml.contains("/var/log/betblocker/agent.err.log"));
    }

    #[test]
    fn test_custom_plist() {
        let plist = LaunchdPlist::new(
            "/tmp/test.plist",
            "com.test.service",
            "/usr/local/bin/test-svc",
        );
        let xml = plist.generate();

        assert!(xml.contains("com.test.service"));
        assert!(xml.contains("/usr/local/bin/test-svc"));
    }

    #[test]
    fn test_is_loaded_returns_false_on_non_macos() {
        #[cfg(not(target_os = "macos"))]
        {
            let plist = LaunchdPlist::new_agent();
            assert!(!plist.is_loaded());
        }
    }
}
