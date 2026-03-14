//! macOS installer helpers.
//!
//! Handles post-install setup (directory creation, permissions, launchd
//! daemon registration) and pre-uninstall teardown (daemon unload, plist
//! removal, pf rule cleanup).  All shell commands are issued through the
//! [`CommandRunner`] trait so that tests can inject a mock runner without
//! spawning real processes.

use thiserror::Error;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during macOS install / uninstall operations.
#[derive(Debug, Error)]
pub enum InstallerError {
    /// `pkgbuild` or `productbuild` exited with a non-zero status.
    #[error("pkg build failed: {0}")]
    PkgBuildFailed(String),

    /// `xcrun notarytool` reported a failure.
    #[error("notarization failed: {0}")]
    NotarizationFailed(String),

    /// `launchctl bootstrap` / `bootout` failed.
    #[error("launchd registration failed: {0}")]
    LaunchdRegistrationFailed(String),

    /// The caller does not have sufficient privileges (must run as root).
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// An underlying I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// CommandRunner trait
// ---------------------------------------------------------------------------

/// Abstracts over shell command execution so that installer logic can be
/// tested without spawning real processes.
pub trait CommandRunner: Send + Sync {
    /// Execute `program` with `args`.
    ///
    /// Returns `Ok((stdout, stderr))` when the process exits successfully,
    /// or an error string (stderr / reason) on failure.
    fn run(
        &self,
        program: &str,
        args: &[&str],
    ) -> Result<(String, String), String>;
}

// ---------------------------------------------------------------------------
// Real (process-spawning) implementation
// ---------------------------------------------------------------------------

/// [`CommandRunner`] that spawns real child processes via
/// [`std::process::Command`].
pub struct ProcessRunner;

impl CommandRunner for ProcessRunner {
    fn run(
        &self,
        program: &str,
        args: &[&str],
    ) -> Result<(String, String), String> {
        let output = std::process::Command::new(program)
            .args(args)
            .output()
            .map_err(|e| format!("failed to spawn '{program}': {e}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        if output.status.success() {
            Ok((stdout, stderr))
        } else {
            Err(stderr)
        }
    }
}

// ---------------------------------------------------------------------------
// Installer
// ---------------------------------------------------------------------------

/// Paths and labels used by the BetBlocker installer.
pub struct InstallerConfig {
    /// LaunchDaemon plist destination.
    pub plist_path: &'static str,
    /// launchd service label.
    pub label: &'static str,
    /// Agent binary destination.
    pub binary_path: &'static str,
    /// Application support directory.
    pub app_support_dir: &'static str,
    /// Log directory.
    pub log_dir: &'static str,
    /// Version file (written by the installer).
    pub version_file: &'static str,
}

impl InstallerConfig {
    /// Default BetBlocker production configuration.
    pub const fn default_config() -> Self {
        Self {
            plist_path: "/Library/LaunchDaemons/com.betblocker.agent.plist",
            label: "com.betblocker.agent",
            binary_path: "/usr/local/bin/bb-agent-macos",
            app_support_dir: "/Library/Application Support/BetBlocker",
            log_dir: "/var/log/betblocker",
            version_file: "/Library/Application Support/BetBlocker/version",
        }
    }
}

/// Provides install / uninstall logic for the BetBlocker macOS package.
pub struct Installer<R: CommandRunner> {
    config: InstallerConfig,
    runner: R,
}

impl<R: CommandRunner> Installer<R> {
    /// Create a new installer with the given configuration and command runner.
    pub fn new(config: InstallerConfig, runner: R) -> Self {
        Self { config, runner }
    }
}

impl Installer<ProcessRunner> {
    /// Create a production installer using real process execution.
    pub fn new_production() -> Self {
        Self::new(InstallerConfig::default_config(), ProcessRunner)
    }
}

impl<R: CommandRunner> Installer<R> {
    // -----------------------------------------------------------------------
    // post_install
    // -----------------------------------------------------------------------

    /// Run post-install setup steps.
    ///
    /// 1. Create required directories with restrictive permissions.
    /// 2. Set ownership and mode on the agent binary.
    /// 3. Write the LaunchDaemon plist.
    /// 4. Bootstrap the daemon via `launchctl bootstrap system <plist>`.
    pub fn post_install(&self) -> Result<(), InstallerError> {
        tracing::info!("BetBlocker post-install starting");

        // 1. Create directories
        self.create_directories()?;

        // 2. Set binary permissions (root:wheel 0755)
        self.set_binary_permissions()?;

        // 3. Install launchd plist
        self.install_plist()?;

        // 4. Bootstrap daemon
        self.bootstrap_daemon()?;

        tracing::info!("BetBlocker post-install complete");
        Ok(())
    }

    /// Create required runtime directories.
    fn create_directories(&self) -> Result<(), InstallerError> {
        let dirs = [self.config.app_support_dir, self.config.log_dir];

        for dir in &dirs {
            std::fs::create_dir_all(dir)?;
            tracing::debug!(dir = dir, "directory ensured");

            // Restrict to owner-only on unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = std::fs::Permissions::from_mode(0o755);
                std::fs::set_permissions(dir, perms)?;
            }
        }

        Ok(())
    }

    /// Set ownership and mode on the installed binary.
    fn set_binary_permissions(&self) -> Result<(), InstallerError> {
        if !std::path::Path::new(self.config.binary_path).exists() {
            // Not yet installed (component package may install it after us).
            tracing::debug!(
                path = self.config.binary_path,
                "binary not yet present; skipping permission step"
            );
            return Ok(());
        }

        self.runner
            .run("chown", &["root:wheel", self.config.binary_path])
            .map_err(|e| InstallerError::PermissionDenied(format!("chown failed: {e}")))?;

        self.runner
            .run("chmod", &["0755", self.config.binary_path])
            .map_err(|e| InstallerError::PermissionDenied(format!("chmod failed: {e}")))?;

        tracing::debug!(path = self.config.binary_path, "binary permissions set");
        Ok(())
    }

    /// Write the LaunchDaemon plist to disk and set its permissions.
    fn install_plist(&self) -> Result<(), InstallerError> {
        let plist_content = crate::launchd::LaunchdPlist::new_agent().generate();
        std::fs::write(self.config.plist_path, plist_content)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o644);
            std::fs::set_permissions(self.config.plist_path, perms)?;
        }

        tracing::debug!(path = self.config.plist_path, "plist written");
        Ok(())
    }

    /// Bootstrap the daemon into the system launchd domain.
    fn bootstrap_daemon(&self) -> Result<(), InstallerError> {
        let result = self
            .runner
            .run("launchctl", &["bootstrap", "system", self.config.plist_path]);

        match result {
            Ok(_) => {
                tracing::info!(label = self.config.label, "daemon bootstrapped");
                Ok(())
            }
            Err(stderr) => {
                // Tolerate "already bootstrapped" errors gracefully.
                if stderr.contains("already bootstrapped")
                    || stderr.contains("service already loaded")
                {
                    tracing::info!(label = self.config.label, "daemon was already loaded");
                    Ok(())
                } else {
                    Err(InstallerError::LaunchdRegistrationFailed(stderr))
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // pre_uninstall
    // -----------------------------------------------------------------------

    /// Run pre-uninstall teardown steps.
    ///
    /// 1. Unload the daemon via `launchctl bootout system/<label>`.
    /// 2. Remove the LaunchDaemon plist.
    /// 3. Flush the `com.betblocker` pf anchor rules.
    pub fn pre_uninstall(&self) -> Result<(), InstallerError> {
        tracing::info!("BetBlocker pre-uninstall starting");

        // 1. Unload daemon
        self.unload_daemon()?;

        // 2. Remove plist
        self.remove_plist()?;

        // 3. Remove pf rules
        self.remove_pf_rules()?;

        tracing::info!("BetBlocker pre-uninstall complete");
        Ok(())
    }

    /// Bootout the daemon from the system launchd domain.
    fn unload_daemon(&self) -> Result<(), InstallerError> {
        let service = format!("system/{}", self.config.label);
        let result = self.runner.run("launchctl", &["bootout", &service]);

        match result {
            Ok(_) => {
                tracing::info!(label = self.config.label, "daemon unloaded");
                Ok(())
            }
            Err(stderr) => {
                // Ignore "not found" / "no such process" errors.
                if stderr.contains("Could not find specified service")
                    || stderr.contains("No such process")
                    || stderr.contains("service is not loaded")
                {
                    tracing::info!(label = self.config.label, "daemon was not loaded");
                    Ok(())
                } else {
                    Err(InstallerError::LaunchdRegistrationFailed(stderr))
                }
            }
        }
    }

    /// Delete the LaunchDaemon plist file if it exists.
    fn remove_plist(&self) -> Result<(), InstallerError> {
        let path = std::path::Path::new(self.config.plist_path);
        if path.exists() {
            std::fs::remove_file(path)?;
            tracing::debug!(path = self.config.plist_path, "plist removed");
        }
        Ok(())
    }

    /// Flush the `com.betblocker` pf anchor.
    fn remove_pf_rules(&self) -> Result<(), InstallerError> {
        let result = self
            .runner
            .run("pfctl", &["-a", "com.betblocker", "-F", "all"]);

        match result {
            Ok(_) => {
                tracing::info!("pf anchor rules flushed");
                Ok(())
            }
            Err(stderr) => {
                // Ignore errors when pf is not running or anchor not found.
                if stderr.contains("No such")
                    || stderr.contains("does not exist")
                    || stderr.contains("pfctl: pf not enabled")
                {
                    tracing::debug!("pf not running or anchor not present; skipping");
                    Ok(())
                } else {
                    // Treat pf errors as non-fatal warnings so uninstall proceeds.
                    tracing::warn!(error = %stderr, "pf rule removal failed (non-fatal)");
                    Ok(())
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Status queries
    // -----------------------------------------------------------------------

    /// Return `true` if the BetBlocker daemon is currently loaded.
    pub fn is_installed(&self) -> bool {
        let service = format!("system/{}", self.config.label);
        self.runner
            .run("launchctl", &["print", &service])
            .is_ok()
    }

    /// Return the installed version string, or `None` if not installed.
    ///
    /// Reads from [`InstallerConfig::version_file`]
    /// (`/Library/Application Support/BetBlocker/version`).
    pub fn get_installed_version(&self) -> Option<String> {
        let path = std::path::Path::new(self.config.version_file);
        std::fs::read_to_string(path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    // -----------------------------------------------------------------------
    // Mock runner
    // -----------------------------------------------------------------------

    /// Command that was issued to the mock runner.
    #[derive(Debug, Clone)]
    pub struct RecordedCommand {
        pub program: String,
        pub args: Vec<String>,
    }

    /// Mock [`CommandRunner`] for testing installer logic.
    pub struct MockRunner {
        /// Pre-configured responses, consumed in order.
        responses: Arc<Mutex<VecDeque<Result<(String, String), String>>>>,
        /// Commands that were issued, in order.
        pub recorded: Arc<Mutex<Vec<RecordedCommand>>>,
    }

    impl MockRunner {
        pub fn new() -> Self {
            Self {
                responses: Arc::new(Mutex::new(VecDeque::new())),
                recorded: Arc::new(Mutex::new(Vec::new())),
            }
        }

        pub fn push_ok(&self, stdout: &str) {
            self.responses
                .lock()
                .unwrap()
                .push_back(Ok((stdout.to_string(), String::new())));
        }

        pub fn push_err(&self, stderr: &str) {
            self.responses
                .lock()
                .unwrap()
                .push_back(Err(stderr.to_string()));
        }
    }

    impl CommandRunner for MockRunner {
        fn run(
            &self,
            program: &str,
            args: &[&str],
        ) -> Result<(String, String), String> {
            self.recorded.lock().unwrap().push(RecordedCommand {
                program: program.to_string(),
                args: args.iter().map(|s| s.to_string()).collect(),
            });
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(Ok((String::new(), String::new())))
        }
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn test_config() -> InstallerConfig {
        InstallerConfig {
            plist_path: "/tmp/bb-test.plist",
            label: "com.betblocker.agent",
            binary_path: "/tmp/bb-agent-macos",
            app_support_dir: "/tmp/bb-app-support",
            log_dir: "/tmp/bb-logs",
            version_file: "/tmp/bb-version",
        }
    }

    // -----------------------------------------------------------------------
    // post_install tests
    // -----------------------------------------------------------------------

    #[test]
    fn post_install_bootstraps_daemon() {
        let runner = MockRunner::new();
        // chown, chmod not needed (binary doesn't exist at /tmp/bb-agent-macos)
        // launchctl bootstrap
        runner.push_ok(""); // launchctl bootstrap

        let installer = Installer::new(test_config(), runner);
        let result = installer.post_install();
        assert!(result.is_ok(), "post_install should succeed: {result:?}");

        let cmds = installer.runner.recorded.lock().unwrap();
        // Only launchctl bootstrap expected (binary absent → permission steps skipped)
        assert!(
            cmds.iter().any(|c| c.program == "launchctl" && c.args.contains(&"bootstrap".to_string())),
            "launchctl bootstrap must be called"
        );
    }

    #[test]
    fn post_install_tolerates_already_bootstrapped() {
        let runner = MockRunner::new();
        runner.push_err("already bootstrapped"); // launchctl bootstrap

        let installer = Installer::new(test_config(), runner);
        let result = installer.post_install();
        assert!(result.is_ok(), "already bootstrapped is not an error");
    }

    #[test]
    fn post_install_fails_on_launchd_error() {
        let runner = MockRunner::new();
        runner.push_err("some unrecognised error"); // launchctl bootstrap

        let installer = Installer::new(test_config(), runner);
        let result = installer.post_install();
        assert!(
            matches!(result, Err(InstallerError::LaunchdRegistrationFailed(_))),
            "unexpected result: {result:?}"
        );
    }

    // -----------------------------------------------------------------------
    // pre_uninstall tests
    // -----------------------------------------------------------------------

    #[test]
    fn pre_uninstall_calls_bootout_and_pf_flush() {
        let runner = MockRunner::new();
        runner.push_ok(""); // launchctl bootout
        // plist removal: no runner call needed (file won't exist at /tmp/bb-test.plist)
        runner.push_ok(""); // pfctl -a com.betblocker -F all

        let installer = Installer::new(test_config(), runner);
        let result = installer.pre_uninstall();
        assert!(result.is_ok(), "pre_uninstall should succeed: {result:?}");

        let cmds = installer.runner.recorded.lock().unwrap();
        assert!(
            cmds.iter().any(|c| c.program == "launchctl" && c.args.contains(&"bootout".to_string())),
            "launchctl bootout must be called"
        );
        assert!(
            cmds.iter().any(|c| c.program == "pfctl"),
            "pfctl must be called"
        );
    }

    #[test]
    fn pre_uninstall_tolerates_daemon_not_loaded() {
        let runner = MockRunner::new();
        runner.push_err("Could not find specified service"); // launchctl bootout
        runner.push_ok(""); // pfctl

        let installer = Installer::new(test_config(), runner);
        let result = installer.pre_uninstall();
        assert!(result.is_ok(), "daemon not loaded is not an error");
    }

    #[test]
    fn pre_uninstall_non_fatal_pf_error() {
        let runner = MockRunner::new();
        runner.push_ok(""); // launchctl bootout
        runner.push_err("pfctl: pf not enabled"); // pfctl

        let installer = Installer::new(test_config(), runner);
        let result = installer.pre_uninstall();
        assert!(result.is_ok(), "pf not enabled should be non-fatal");
    }

    // -----------------------------------------------------------------------
    // is_installed
    // -----------------------------------------------------------------------

    #[test]
    fn is_installed_true_when_launchctl_print_succeeds() {
        let runner = MockRunner::new();
        runner.push_ok("some output"); // launchctl print

        let installer = Installer::new(test_config(), runner);
        assert!(installer.is_installed());
    }

    #[test]
    fn is_installed_false_when_launchctl_print_fails() {
        let runner = MockRunner::new();
        runner.push_err("service not found"); // launchctl print

        let installer = Installer::new(test_config(), runner);
        assert!(!installer.is_installed());
    }

    // -----------------------------------------------------------------------
    // get_installed_version
    // -----------------------------------------------------------------------

    #[test]
    fn get_installed_version_reads_file() {
        let version_path = "/tmp/bb-version-test";
        std::fs::write(version_path, "1.2.3\n").expect("write version");

        let config = InstallerConfig {
            version_file: version_path,
            ..InstallerConfig::default_config()
        };

        let installer = Installer::new(config, MockRunner::new());
        let version = installer.get_installed_version();
        assert_eq!(version, Some("1.2.3".to_string()));

        std::fs::remove_file(version_path).ok();
    }

    #[test]
    fn get_installed_version_none_when_file_missing() {
        let config = InstallerConfig {
            version_file: "/tmp/bb-version-that-does-not-exist",
            ..InstallerConfig::default_config()
        };

        let installer = Installer::new(config, MockRunner::new());
        assert!(installer.get_installed_version().is_none());
    }

    // -----------------------------------------------------------------------
    // Error display
    // -----------------------------------------------------------------------

    #[test]
    fn installer_error_display() {
        assert!(InstallerError::PkgBuildFailed("x".to_string()).to_string().contains("x"));
        assert!(InstallerError::NotarizationFailed("y".to_string()).to_string().contains("y"));
        assert!(InstallerError::LaunchdRegistrationFailed("z".to_string()).to_string().contains("z"));
        assert!(InstallerError::PermissionDenied("denied".to_string()).to_string().contains("denied"));
    }
}
