//! AppArmor profile management.
//!
//! Generates, loads, and verifies AppArmor profiles that confine
//! the BetBlocker agent process and protect its data files.

use crate::mac::{MacError, MacProtection, MacStatus, MacSystem};

/// Abstraction over command execution for testability.
pub trait CommandRunner: Send + Sync {
    /// Run a command with arguments and return its stdout.
    fn run(&self, cmd: &str, args: &[&str]) -> Result<String, MacError>;
}

/// Real command runner that delegates to `std::process::Command`.
#[cfg(target_os = "linux")]
pub struct SystemCommandRunner;

#[cfg(target_os = "linux")]
impl CommandRunner for SystemCommandRunner {
    fn run(&self, cmd: &str, args: &[&str]) -> Result<String, MacError> {
        let output = std::process::Command::new(cmd)
            .args(args)
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    MacError::PermissionDenied
                } else {
                    MacError::Io(e)
                }
            })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("Permission denied") || stderr.contains("Operation not permitted") {
                Err(MacError::PermissionDenied)
            } else {
                Err(MacError::CommandFailed(format!(
                    "{cmd} exited with {}: {stderr}",
                    output.status
                )))
            }
        }
    }
}

/// Default profile name for the BetBlocker agent.
pub const PROFILE_NAME: &str = "betblocker-agent";

/// Default path where the agent binary is installed.
pub const DEFAULT_AGENT_PATH: &str = "/usr/lib/betblocker/bb-agent-linux";

/// System directory where AppArmor profiles are stored.
pub const SYSTEM_PROFILES_DIR: &str = "/etc/apparmor.d";

/// Path to the kernel's AppArmor profiles list.
pub const APPARMOR_PROFILES_PATH: &str = "/sys/kernel/security/apparmor/profiles";

/// Manages AppArmor profile installation and verification.
pub struct AppArmorProtection {
    /// Path to the source profile file to install from.
    pub profile_path: String,
    /// Name of the profile (used for lookups).
    pub profile_name: String,
    /// Directory where system profiles are stored.
    pub profiles_dir: String,
    /// Command runner for executing system commands.
    command_runner: Box<dyn CommandRunner>,
}

impl AppArmorProtection {
    /// Create a new `AppArmorProtection` with the given command runner.
    pub fn new(
        profile_path: String,
        profile_name: String,
        profiles_dir: String,
        command_runner: Box<dyn CommandRunner>,
    ) -> Self {
        Self {
            profile_path,
            profile_name,
            profiles_dir,
            command_runner,
        }
    }

    /// Create with default paths and a custom command runner.
    pub fn with_defaults(command_runner: Box<dyn CommandRunner>) -> Self {
        Self {
            profile_path: format!("{SYSTEM_PROFILES_DIR}/{PROFILE_NAME}"),
            profile_name: PROFILE_NAME.to_string(),
            profiles_dir: SYSTEM_PROFILES_DIR.to_string(),
            command_runner,
        }
    }

    /// Destination path for the profile in the system profiles directory.
    fn dest_profile_path(&self) -> String {
        format!("{}/{}", self.profiles_dir, self.profile_name)
    }

    /// Parse the kernel profiles file content to find our profile's mode.
    ///
    /// Each line in `/sys/kernel/security/apparmor/profiles` has the format:
    /// `<profile_name> (<mode>)`
    fn parse_profile_mode(content: &str, profile_name: &str) -> Option<String> {
        for line in content.lines() {
            // Match lines like: "/usr/lib/betblocker/bb-agent-linux (enforce)"
            // or "betblocker-agent (enforce)"
            let line = line.trim();
            if line.contains(profile_name) {
                if let Some(start) = line.rfind('(') {
                    if let Some(end) = line.rfind(')') {
                        if start < end {
                            return Some(line[start + 1..end].to_string());
                        }
                    }
                }
            }
        }
        None
    }
}

impl MacProtection for AppArmorProtection {
    fn install(&self) -> Result<(), MacError> {
        // Copy profile to system profiles directory
        self.command_runner.run(
            "cp",
            &[&self.profile_path, &self.dest_profile_path()],
        )?;

        // Load/replace the profile in enforce mode
        self.command_runner
            .run("apparmor_parser", &["-r", "-W", &self.dest_profile_path()])?;

        Ok(())
    }

    fn verify(&self) -> Result<MacStatus, MacError> {
        let profiles_content = self
            .command_runner
            .run("cat", &[APPARMOR_PROFILES_PATH]);

        match profiles_content {
            Ok(content) => {
                let mode = Self::parse_profile_mode(&content, &self.profile_name);
                match mode {
                    Some(m) => Ok(MacStatus {
                        system: MacSystem::AppArmor,
                        profile_loaded: true,
                        enforcing: m == "enforce",
                        profile_name: Some(self.profile_name.clone()),
                    }),
                    None => Ok(MacStatus {
                        system: MacSystem::AppArmor,
                        profile_loaded: false,
                        enforcing: false,
                        profile_name: Some(self.profile_name.clone()),
                    }),
                }
            }
            Err(_) => Ok(MacStatus {
                system: MacSystem::AppArmor,
                profile_loaded: false,
                enforcing: false,
                profile_name: Some(self.profile_name.clone()),
            }),
        }
    }

    fn is_enforcing(&self) -> bool {
        self.verify()
            .is_ok_and(|status| status.enforcing)
    }

    fn uninstall(&self) -> Result<(), MacError> {
        // Remove the profile from the kernel
        self.command_runner
            .run("apparmor_parser", &["-R", &self.dest_profile_path()])?;

        // Remove the profile file
        self.command_runner
            .run("rm", &["-f", &self.dest_profile_path()])?;

        Ok(())
    }

    fn verify_and_repair(&self) -> Result<MacStatus, MacError> {
        let status = self.verify()?;

        if !status.profile_loaded || !status.enforcing {
            tracing::warn!(
                profile = %self.profile_name,
                loaded = status.profile_loaded,
                enforcing = status.enforcing,
                "AppArmor profile needs repair, reloading in enforce mode"
            );
            self.install()?;
            return self.verify();
        }

        Ok(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Records all commands that were invoked, and returns preconfigured results.
    struct MockCommandRunner {
        calls: Arc<Mutex<Vec<(String, Vec<String>)>>>,
        responses: Arc<Mutex<Vec<Result<String, MacError>>>>,
    }

    impl MockCommandRunner {
        fn new(responses: Vec<Result<String, MacError>>) -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
                responses: Arc::new(Mutex::new(responses)),
            }
        }

        #[allow(dead_code)]
        fn calls(&self) -> Vec<(String, Vec<String>)> {
            self.calls.lock().expect("lock").clone()
        }
    }

    impl CommandRunner for MockCommandRunner {
        fn run(&self, cmd: &str, args: &[&str]) -> Result<String, MacError> {
            self.calls.lock().expect("lock").push((
                cmd.to_string(),
                args.iter().map(|s| (*s).to_string()).collect(),
            ));
            let mut responses = self.responses.lock().expect("lock");
            if responses.is_empty() {
                Ok(String::new())
            } else {
                responses.remove(0)
            }
        }
    }

    fn make_protection(runner: MockCommandRunner) -> (AppArmorProtection, Arc<Mutex<Vec<(String, Vec<String>)>>>) {
        let calls = Arc::clone(&runner.calls);
        let prot = AppArmorProtection::new(
            "/deploy/apparmor/betblocker-agent".to_string(),
            "betblocker-agent".to_string(),
            "/etc/apparmor.d".to_string(),
            Box::new(runner),
        );
        (prot, calls)
    }

    #[test]
    fn install_success() {
        let runner = MockCommandRunner::new(vec![
            Ok(String::new()), // cp
            Ok(String::new()), // apparmor_parser -r
        ]);
        let (prot, calls) = make_protection(runner);

        let result = prot.install();
        assert!(result.is_ok());

        let calls = calls.lock().expect("lock");
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].0, "cp");
        assert_eq!(calls[1].0, "apparmor_parser");
        assert!(calls[1].1.contains(&"-r".to_string()));
    }

    #[test]
    fn install_copy_failure() {
        let runner = MockCommandRunner::new(vec![
            Err(MacError::PermissionDenied),
        ]);
        let (prot, _calls) = make_protection(runner);

        let result = prot.install();
        assert!(result.is_err());
    }

    #[test]
    fn install_parser_failure() {
        let runner = MockCommandRunner::new(vec![
            Ok(String::new()), // cp succeeds
            Err(MacError::CommandFailed("apparmor_parser failed".to_string())),
        ]);
        let (prot, _calls) = make_protection(runner);

        let result = prot.install();
        assert!(result.is_err());
    }

    #[test]
    fn verify_profile_enforcing() {
        let profiles = "betblocker-agent (enforce)\nother-profile (complain)\n";
        let runner = MockCommandRunner::new(vec![Ok(profiles.to_string())]);
        let (prot, _calls) = make_protection(runner);

        let status = prot.verify().expect("verify");
        assert_eq!(status.system, MacSystem::AppArmor);
        assert!(status.profile_loaded);
        assert!(status.enforcing);
    }

    #[test]
    fn verify_profile_complain() {
        let profiles = "betblocker-agent (complain)\n";
        let runner = MockCommandRunner::new(vec![Ok(profiles.to_string())]);
        let (prot, _calls) = make_protection(runner);

        let status = prot.verify().expect("verify");
        assert!(status.profile_loaded);
        assert!(!status.enforcing);
    }

    #[test]
    fn verify_profile_not_found() {
        let profiles = "some-other-profile (enforce)\n";
        let runner = MockCommandRunner::new(vec![Ok(profiles.to_string())]);
        let (prot, _calls) = make_protection(runner);

        let status = prot.verify().expect("verify");
        assert!(!status.profile_loaded);
        assert!(!status.enforcing);
    }

    #[test]
    fn verify_read_failure_returns_not_loaded() {
        let runner = MockCommandRunner::new(vec![
            Err(MacError::CommandFailed("file not found".to_string())),
        ]);
        let (prot, _calls) = make_protection(runner);

        let status = prot.verify().expect("verify");
        assert!(!status.profile_loaded);
        assert!(!status.enforcing);
    }

    #[test]
    fn is_enforcing_true() {
        let profiles = "betblocker-agent (enforce)\n";
        let runner = MockCommandRunner::new(vec![Ok(profiles.to_string())]);
        let (prot, _calls) = make_protection(runner);

        assert!(prot.is_enforcing());
    }

    #[test]
    fn is_enforcing_false_when_complain() {
        let profiles = "betblocker-agent (complain)\n";
        let runner = MockCommandRunner::new(vec![Ok(profiles.to_string())]);
        let (prot, _calls) = make_protection(runner);

        assert!(!prot.is_enforcing());
    }

    #[test]
    fn uninstall_success() {
        let runner = MockCommandRunner::new(vec![
            Ok(String::new()), // apparmor_parser -R
            Ok(String::new()), // rm
        ]);
        let (prot, calls) = make_protection(runner);

        let result = prot.uninstall();
        assert!(result.is_ok());

        let calls = calls.lock().expect("lock");
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].0, "apparmor_parser");
        assert!(calls[0].1.contains(&"-R".to_string()));
        assert_eq!(calls[1].0, "rm");
    }

    #[test]
    fn uninstall_parser_failure() {
        let runner = MockCommandRunner::new(vec![
            Err(MacError::CommandFailed("not loaded".to_string())),
        ]);
        let (prot, _calls) = make_protection(runner);

        let result = prot.uninstall();
        assert!(result.is_err());
    }

    #[test]
    fn verify_and_repair_already_enforcing() {
        let profiles = "betblocker-agent (enforce)\n";
        let runner = MockCommandRunner::new(vec![
            Ok(profiles.to_string()), // verify reads profiles
        ]);
        let (prot, calls) = make_protection(runner);

        let status = prot.verify_and_repair().expect("verify_and_repair");
        assert!(status.enforcing);

        // Should only have read profiles once (no repair needed)
        let calls = calls.lock().expect("lock");
        assert_eq!(calls.len(), 1);
    }

    #[test]
    fn verify_and_repair_not_loaded_triggers_reinstall() {
        let profiles_after = "betblocker-agent (enforce)\n";
        let runner = MockCommandRunner::new(vec![
            Ok("other-profile (enforce)\n".to_string()), // initial verify: not found
            Ok(String::new()),                            // install: cp
            Ok(String::new()),                            // install: apparmor_parser -r
            Ok(profiles_after.to_string()),               // re-verify after repair
        ]);
        let (prot, calls) = make_protection(runner);

        let status = prot.verify_and_repair().expect("verify_and_repair");
        assert!(status.profile_loaded);
        assert!(status.enforcing);

        let calls = calls.lock().expect("lock");
        // verify(cat) + cp + apparmor_parser + verify(cat)
        assert_eq!(calls.len(), 4);
    }

    #[test]
    fn verify_and_repair_complain_mode_triggers_reinstall() {
        let runner = MockCommandRunner::new(vec![
            Ok("betblocker-agent (complain)\n".to_string()), // initial verify: complain
            Ok(String::new()),                                // install: cp
            Ok(String::new()),                                // install: apparmor_parser -r
            Ok("betblocker-agent (enforce)\n".to_string()),   // re-verify
        ]);
        let (prot, _calls) = make_protection(runner);

        let status = prot.verify_and_repair().expect("verify_and_repair");
        assert!(status.enforcing);
    }

    #[test]
    fn parse_profile_mode_with_path_based_name() {
        let content = "/usr/lib/betblocker/bb-agent-linux (enforce)\n";
        let mode = AppArmorProtection::parse_profile_mode(content, "betblocker");
        assert_eq!(mode, Some("enforce".to_string()));
    }

    #[test]
    fn parse_profile_mode_no_match() {
        let content = "unrelated-profile (enforce)\n";
        let mode = AppArmorProtection::parse_profile_mode(content, "betblocker-agent");
        assert_eq!(mode, None);
    }

    #[test]
    fn with_defaults_sets_expected_paths() {
        let runner = MockCommandRunner::new(vec![]);
        let prot = AppArmorProtection::with_defaults(Box::new(runner));
        assert_eq!(prot.profiles_dir, "/etc/apparmor.d");
        assert_eq!(prot.profile_name, "betblocker-agent");
    }
}
