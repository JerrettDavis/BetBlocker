//! macOS platform utilities.
//!
//! Provides macOS-specific helpers for system information,
//! directory management, and privilege handling.

use std::io;

/// Read the hardware UUID on macOS via IOKit / sysctl fallback.
///
/// On non-macOS platforms, returns a placeholder string.
#[cfg(target_os = "macos")]
pub fn read_machine_id() -> String {
    // Try IOPlatformExpertDevice UUID via ioreg
    if let Ok(output) = std::process::Command::new("ioreg")
        .args(["-rd1", "-c", "IOPlatformExpertDevice"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("IOPlatformUUID") {
                // Line format: "IOPlatformUUID" = "XXXXXXXX-XXXX-..."
                if let Some(uuid) = line.split('"').nth(3) {
                    if !uuid.is_empty() {
                        return uuid.to_string();
                    }
                }
            }
        }
    }

    // Fallback: sysctl kern.uuid
    if let Ok(output) = std::process::Command::new("sysctl")
        .args(["-n", "kern.uuid"])
        .output()
    {
        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !s.is_empty() {
            return s;
        }
    }

    "unknown-machine-id".to_string()
}

/// Stub for non-macOS platforms.
#[cfg(not(target_os = "macos"))]
pub fn read_machine_id() -> String {
    "unknown-machine-id-stub".to_string()
}

/// Ensure required directories exist with correct permissions.
///
/// Creates:
/// - `/Library/Application Support/BetBlocker/`
/// - `/Library/Application Support/BetBlocker/certs/`
/// - `/var/log/betblocker/`
#[cfg(target_os = "macos")]
pub fn ensure_directories() -> Result<(), io::Error> {
    let dirs = [
        "/Library/Application Support/BetBlocker",
        "/Library/Application Support/BetBlocker/certs",
        "/var/log/betblocker",
    ];

    for dir in &dirs {
        std::fs::create_dir_all(dir)?;

        // Restrict permissions to owner only (root)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o700);
            std::fs::set_permissions(dir, perms)?;
        }
    }

    Ok(())
}

/// Stub for non-macOS platforms.
#[cfg(not(target_os = "macos"))]
pub fn ensure_directories() -> Result<(), io::Error> {
    // No-op on non-macOS; directories are platform-specific.
    Ok(())
}

/// Get the current process UID.
///
/// Used for pf rule loop prevention (skip agent's own DNS queries).
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
pub fn current_uid() -> u32 {
    // Safety: getuid() is always safe to call, no preconditions.
    unsafe { libc::getuid() }
}

/// Stub for non-macOS platforms.
#[cfg(not(target_os = "macos"))]
pub fn current_uid() -> u32 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_machine_id_not_empty() {
        let id = read_machine_id();
        assert!(!id.is_empty(), "machine ID should not be empty");
    }

    #[test]
    fn test_current_uid_does_not_panic() {
        let _uid = current_uid();
    }

    #[test]
    fn test_ensure_directories_stub() {
        // On non-macOS, this is a no-op and should succeed.
        #[cfg(not(target_os = "macos"))]
        {
            let result = ensure_directories();
            assert!(result.is_ok());
        }
    }
}
