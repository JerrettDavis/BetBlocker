//! SELinux policy management.
//!
//! Manages SELinux policy modules and contexts for BetBlocker,
//! providing mandatory access control on SELinux-enabled distributions.

use crate::apparmor::CommandRunner;
use crate::mac::{MacError, MacProtection, MacStatus, MacSystem};

/// Default module name for the BetBlocker SELinux policy.
pub const MODULE_NAME: &str = "betblocker";

/// Default path to the policy source directory.
pub const POLICY_SOURCE_DIR: &str = "/usr/share/betblocker/selinux";

/// Paths to restore SELinux contexts on.
const RESTORECON_PATHS: &[&str] = &[
    "/usr/lib/betblocker",
    "/var/lib/betblocker",
    "/var/log/betblocker",
];

/// Manages SELinux policy installation and verification for BetBlocker.
pub struct SELinuxProtection {
    /// Name of the SELinux policy module.
    pub module_name: String,
    /// Directory containing the .te, .fc, and .if policy source files.
    pub policy_source_dir: String,
    /// Command runner for executing system commands.
    command_runner: Box<dyn CommandRunner>,
}

impl SELinuxProtection {
    /// Create a new `SELinuxProtection` with the given command runner.
    pub fn new(
        module_name: String,
        policy_source_dir: String,
        command_runner: Box<dyn CommandRunner>,
    ) -> Self {
        Self {
            module_name,
            policy_source_dir,
            command_runner,
        }
    }

    /// Create with default paths and a custom command runner.
    pub fn with_defaults(command_runner: Box<dyn CommandRunner>) -> Self {
        Self {
            module_name: MODULE_NAME.to_string(),
            policy_source_dir: POLICY_SOURCE_DIR.to_string(),
            command_runner,
        }
    }

    /// Check whether SELinux is in enforcing mode by parsing `getenforce` output.
    fn parse_getenforce(output: &str) -> bool {
        output.trim().eq_ignore_ascii_case("enforcing")
    }

    /// Check whether the module is listed in `semodule -l` output.
    fn is_module_listed(output: &str, module_name: &str) -> bool {
        output.lines().any(|line| {
            let trimmed = line.trim();
            // semodule -l output: "module_name  version" or just "module_name"
            trimmed == module_name || trimmed.starts_with(&format!("{module_name}\t")) || trimmed.starts_with(&format!("{module_name} "))
        })
    }
}

impl MacProtection for SELinuxProtection {
    /// Install the SELinux policy module.
    ///
    /// Compiles the Type Enforcement file with `checkmodule`, packages
    /// it with `semodule_package`, and installs with `semodule -i`.
    /// Finally runs `restorecon` on BetBlocker paths.
    fn install(&self) -> Result<(), MacError> {
        let te_file = format!("{}/{}.te", self.policy_source_dir, self.module_name);
        let mod_file = format!("/tmp/{}.mod", self.module_name);
        let pp_file = format!("/tmp/{}.pp", self.module_name);
        let fc_file = format!("{}/{}.fc", self.policy_source_dir, self.module_name);

        // Step 1: Compile .te -> .mod
        self.command_runner.run(
            "checkmodule",
            &["-M", "-m", "-o", &mod_file, &te_file],
        )?;

        // Step 2: Package .mod + .fc -> .pp
        self.command_runner.run(
            "semodule_package",
            &["-o", &pp_file, "-m", &mod_file, "-f", &fc_file],
        )?;

        // Step 3: Install the policy package
        self.command_runner.run("semodule", &["-i", &pp_file])?;

        // Step 4: Restore file contexts on BetBlocker paths
        for path in RESTORECON_PATHS {
            // Best-effort restorecon; path may not exist yet
            let _ = self.command_runner.run("restorecon", &["-R", "-v", path]);
        }

        // Clean up temporary files
        let _ = self.command_runner.run("rm", &["-f", &mod_file, &pp_file]);

        tracing::info!(
            module = %self.module_name,
            "SELinux policy module installed"
        );

        Ok(())
    }

    /// Verify the current SELinux status for BetBlocker.
    ///
    /// Checks that the policy module is loaded (via `semodule -l`)
    /// and that SELinux is in enforcing mode (via `getenforce`).
    fn verify(&self) -> Result<MacStatus, MacError> {
        // Check if module is loaded
        let module_list = self
            .command_runner
            .run("semodule", &["-l"]);

        let profile_loaded = match &module_list {
            Ok(output) => Self::is_module_listed(output, &self.module_name),
            Err(_) => false,
        };

        // Check enforcing mode
        let enforcing = match self.command_runner.run("getenforce", &[]) {
            Ok(output) => Self::parse_getenforce(&output),
            Err(_) => false,
        };

        Ok(MacStatus {
            system: MacSystem::SELinux,
            profile_loaded,
            enforcing,
            profile_name: Some(self.module_name.clone()),
        })
    }

    /// Check whether SELinux is in enforcing mode.
    fn is_enforcing(&self) -> bool {
        self.command_runner
            .run("getenforce", &[])
            .map(|output| Self::parse_getenforce(&output))
            .unwrap_or(false)
    }

    /// Remove the BetBlocker SELinux policy module.
    fn uninstall(&self) -> Result<(), MacError> {
        self.command_runner
            .run("semodule", &["-r", &self.module_name])?;

        tracing::info!(
            module = %self.module_name,
            "SELinux policy module removed"
        );

        Ok(())
    }

    /// Verify the policy and repair it if it is not loaded or not enforcing.
    fn verify_and_repair(&self) -> Result<MacStatus, MacError> {
        let status = self.verify()?;

        if !status.profile_loaded {
            tracing::warn!(
                module = %self.module_name,
                "SELinux policy module not loaded, reinstalling"
            );
            self.install()?;
            return self.verify();
        }

        if !status.enforcing {
            tracing::warn!(
                module = %self.module_name,
                "SELinux not in enforcing mode"
            );
            // We cannot change enforcing mode from the agent (that
            // requires root / setenforce), but we can ensure the
            // module is at least loaded.
        }

        Ok(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Records all commands that were invoked and returns preconfigured results.
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

    fn make_protection(
        runner: MockCommandRunner,
    ) -> (SELinuxProtection, Arc<Mutex<Vec<(String, Vec<String>)>>>) {
        let calls = Arc::clone(&runner.calls);
        let prot = SELinuxProtection::new(
            "betblocker".to_string(),
            "/usr/share/betblocker/selinux".to_string(),
            Box::new(runner),
        );
        (prot, calls)
    }

    #[test]
    fn install_success() {
        let runner = MockCommandRunner::new(vec![
            Ok(String::new()), // checkmodule
            Ok(String::new()), // semodule_package
            Ok(String::new()), // semodule -i
            Ok(String::new()), // restorecon /usr/lib/betblocker
            Ok(String::new()), // restorecon /var/lib/betblocker
            Ok(String::new()), // restorecon /var/log/betblocker
            Ok(String::new()), // rm temp files
        ]);
        let (prot, calls) = make_protection(runner);

        let result = prot.install();
        assert!(result.is_ok());

        let calls = calls.lock().expect("lock");
        assert_eq!(calls[0].0, "checkmodule");
        assert_eq!(calls[1].0, "semodule_package");
        assert_eq!(calls[2].0, "semodule");
        assert!(calls[2].1.contains(&"-i".to_string()));
    }

    #[test]
    fn install_checkmodule_failure() {
        let runner = MockCommandRunner::new(vec![
            Err(MacError::CommandFailed("checkmodule failed".to_string())),
        ]);
        let (prot, _calls) = make_protection(runner);

        let result = prot.install();
        assert!(result.is_err());
    }

    #[test]
    fn verify_module_loaded_and_enforcing() {
        let runner = MockCommandRunner::new(vec![
            Ok("betblocker\t1.0.0\nother_module\t2.0\n".to_string()), // semodule -l
            Ok("Enforcing\n".to_string()),                             // getenforce
        ]);
        let (prot, _calls) = make_protection(runner);

        let status = prot.verify().expect("verify");
        assert_eq!(status.system, MacSystem::SELinux);
        assert!(status.profile_loaded);
        assert!(status.enforcing);
    }

    #[test]
    fn verify_module_not_loaded() {
        let runner = MockCommandRunner::new(vec![
            Ok("other_module\t2.0\n".to_string()), // semodule -l
            Ok("Enforcing\n".to_string()),           // getenforce
        ]);
        let (prot, _calls) = make_protection(runner);

        let status = prot.verify().expect("verify");
        assert!(!status.profile_loaded);
        assert!(status.enforcing);
    }

    #[test]
    fn verify_permissive_mode() {
        let runner = MockCommandRunner::new(vec![
            Ok("betblocker\t1.0.0\n".to_string()), // semodule -l
            Ok("Permissive\n".to_string()),          // getenforce
        ]);
        let (prot, _calls) = make_protection(runner);

        let status = prot.verify().expect("verify");
        assert!(status.profile_loaded);
        assert!(!status.enforcing);
    }

    #[test]
    fn is_enforcing_true() {
        let runner = MockCommandRunner::new(vec![
            Ok("Enforcing\n".to_string()),
        ]);
        let (prot, _calls) = make_protection(runner);

        assert!(prot.is_enforcing());
    }

    #[test]
    fn is_enforcing_false_permissive() {
        let runner = MockCommandRunner::new(vec![
            Ok("Permissive\n".to_string()),
        ]);
        let (prot, _calls) = make_protection(runner);

        assert!(!prot.is_enforcing());
    }

    #[test]
    fn is_enforcing_false_disabled() {
        let runner = MockCommandRunner::new(vec![
            Ok("Disabled\n".to_string()),
        ]);
        let (prot, _calls) = make_protection(runner);

        assert!(!prot.is_enforcing());
    }

    #[test]
    fn is_enforcing_false_on_error() {
        let runner = MockCommandRunner::new(vec![
            Err(MacError::CommandFailed("not found".to_string())),
        ]);
        let (prot, _calls) = make_protection(runner);

        assert!(!prot.is_enforcing());
    }

    #[test]
    fn uninstall_success() {
        let runner = MockCommandRunner::new(vec![
            Ok(String::new()), // semodule -r
        ]);
        let (prot, calls) = make_protection(runner);

        let result = prot.uninstall();
        assert!(result.is_ok());

        let calls = calls.lock().expect("lock");
        assert_eq!(calls[0].0, "semodule");
        assert!(calls[0].1.contains(&"-r".to_string()));
        assert!(calls[0].1.contains(&"betblocker".to_string()));
    }

    #[test]
    fn uninstall_failure() {
        let runner = MockCommandRunner::new(vec![
            Err(MacError::CommandFailed("module not found".to_string())),
        ]);
        let (prot, _calls) = make_protection(runner);

        let result = prot.uninstall();
        assert!(result.is_err());
    }

    #[test]
    fn verify_and_repair_already_loaded() {
        let runner = MockCommandRunner::new(vec![
            Ok("betblocker\t1.0.0\n".to_string()), // semodule -l (verify)
            Ok("Enforcing\n".to_string()),           // getenforce (verify)
        ]);
        let (prot, calls) = make_protection(runner);

        let status = prot.verify_and_repair().expect("verify_and_repair");
        assert!(status.profile_loaded);
        assert!(status.enforcing);

        // Should only call semodule -l and getenforce (no reinstall)
        let calls = calls.lock().expect("lock");
        assert_eq!(calls.len(), 2);
    }

    #[test]
    fn verify_and_repair_not_loaded_triggers_reinstall() {
        let runner = MockCommandRunner::new(vec![
            // Initial verify
            Ok("other_module\t1.0\n".to_string()), // semodule -l: not found
            Ok("Enforcing\n".to_string()),           // getenforce
            // install
            Ok(String::new()), // checkmodule
            Ok(String::new()), // semodule_package
            Ok(String::new()), // semodule -i
            Ok(String::new()), // restorecon 1
            Ok(String::new()), // restorecon 2
            Ok(String::new()), // restorecon 3
            Ok(String::new()), // rm
            // Re-verify
            Ok("betblocker\t1.0.0\n".to_string()), // semodule -l
            Ok("Enforcing\n".to_string()),           // getenforce
        ]);
        let (prot, calls) = make_protection(runner);

        let status = prot.verify_and_repair().expect("verify_and_repair");
        assert!(status.profile_loaded);
        assert!(status.enforcing);

        let calls = calls.lock().expect("lock");
        // verify(2) + install(7) + verify(2) = 11
        assert!(calls.len() >= 4, "expected install to be triggered");
    }

    #[test]
    fn parse_getenforce_values() {
        assert!(SELinuxProtection::parse_getenforce("Enforcing\n"));
        assert!(SELinuxProtection::parse_getenforce("enforcing"));
        assert!(!SELinuxProtection::parse_getenforce("Permissive\n"));
        assert!(!SELinuxProtection::parse_getenforce("Disabled\n"));
        assert!(!SELinuxProtection::parse_getenforce(""));
    }

    #[test]
    fn is_module_listed_variants() {
        assert!(SELinuxProtection::is_module_listed(
            "betblocker\t1.0.0\nother\t2.0\n",
            "betblocker"
        ));
        assert!(SELinuxProtection::is_module_listed(
            "betblocker 1.0.0\n",
            "betblocker"
        ));
        assert!(SELinuxProtection::is_module_listed(
            "betblocker\n",
            "betblocker"
        ));
        assert!(!SELinuxProtection::is_module_listed(
            "other_module\t1.0\n",
            "betblocker"
        ));
        assert!(!SELinuxProtection::is_module_listed("", "betblocker"));
    }

    #[test]
    fn with_defaults_sets_expected_values() {
        let runner = MockCommandRunner::new(vec![]);
        let prot = SELinuxProtection::with_defaults(Box::new(runner));
        assert_eq!(prot.module_name, "betblocker");
        assert_eq!(prot.policy_source_dir, "/usr/share/betblocker/selinux");
    }
}
