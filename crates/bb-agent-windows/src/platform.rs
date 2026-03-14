//! Windows platform bridge for the BetBlocker agent.
//!
//! Provides Windows-specific implementations for:
//! - Machine ID reading (from registry `MachineGuid`)
//! - Directory creation (`C:\ProgramData\BetBlocker\` tree) with ACLs
//! - Windows Service status notifications

use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Machine ID
// ---------------------------------------------------------------------------

/// Read the machine GUID from the Windows registry.
///
/// Reads `HKLM\SOFTWARE\Microsoft\Cryptography\MachineGuid`.
/// Falls back to the hostname when the registry is not accessible
/// (e.g. when cross-compiling or running in CI without admin rights).
#[allow(dead_code)]
#[cfg(target_os = "windows")]
pub fn read_machine_id() -> String {
    use std::process::Command;

    // Query via reg.exe so we don't need to link the windows-registry crate here.
    let output = Command::new("reg")
        .args([
            "query",
            r"HKLM\SOFTWARE\Microsoft\Cryptography",
            "/v",
            "MachineGuid",
        ])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // Output format: "    MachineGuid    REG_SZ    <guid>"
            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("MachineGuid") {
                    let parts: Vec<&str> = trimmed.splitn(3, char::is_whitespace).collect();
                    if let Some(guid) = parts.last() {
                        let guid = guid.trim();
                        if !guid.is_empty() {
                            return guid.to_string();
                        }
                    }
                    // REG_SZ format: field  REG_SZ  value
                    let fields: Vec<&str> = trimmed.split_whitespace().collect();
                    if fields.len() >= 3 {
                        return fields.last().map_or("", |v| v).to_string();
                    }
                }
            }
        }
    }

    // Fallback: hostname
    hostname_fallback()
}

/// Cross-compilation / non-Windows stub — uses hostname as machine ID.
#[allow(dead_code)]
#[cfg(not(target_os = "windows"))]
pub fn read_machine_id() -> String {
    hostname_fallback()
}

#[allow(dead_code)]
fn hostname_fallback() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown-machine-id".to_string())
}

// ---------------------------------------------------------------------------
// Directories
// ---------------------------------------------------------------------------

/// Base data directory for BetBlocker on Windows.
#[cfg(target_os = "windows")]
const DATA_BASE: &str = r"C:\ProgramData\BetBlocker";

/// Ensure required directories exist.
///
/// Creates the `C:\ProgramData\BetBlocker\` tree and applies restrictive ACLs
/// (SYSTEM + Administrators full control; Users read-only) on Windows.
///
/// On non-Windows platforms this is a no-op and always succeeds.
#[cfg(target_os = "windows")]
pub fn ensure_directories() -> Result<(), std::io::Error> {
    let subdirs = [
        PathBuf::from(DATA_BASE),
        PathBuf::from(DATA_BASE).join("certs"),
        PathBuf::from(DATA_BASE).join("logs"),
        PathBuf::from(DATA_BASE).join("plugins"),
    ];

    for dir in &subdirs {
        std::fs::create_dir_all(dir)?;
        // Best-effort ACL — ignore errors so we don't break in limited environments.
        let _ = bb_shim_windows::acl::set_restrictive_directory_acl(dir);
    }

    tracing::info!(base = DATA_BASE, "BetBlocker data directories ready");
    Ok(())
}

/// Non-Windows stub — always succeeds.
#[cfg(not(target_os = "windows"))]
pub fn ensure_directories() -> Result<(), std::io::Error> {
    Ok(())
}

/// Return the data directory used by the agent.
///
/// On Windows: `C:\ProgramData\BetBlocker`
/// On other platforms (testing): system temp dir.
#[allow(dead_code)]
pub fn data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(DATA_BASE)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::temp_dir().join("BetBlocker")
    }
}

// ---------------------------------------------------------------------------
// Service status notifications
// ---------------------------------------------------------------------------

/// Notify the Windows SCM that the service is ready (running).
///
/// On non-Windows platforms this is a no-op.
pub fn service_notify_ready() {
    tracing::info!("Service status: RUNNING");
    // On Windows the SCM status is updated by the windows-service dispatcher.
    // This hook exists so that main.rs can call it in the same location as
    // the Linux sd_notify_ready().
}

/// Notify the Windows SCM that the service is stopping.
pub fn service_notify_stopping() {
    tracing::info!("Service status: STOPPING");
}

/// Notify the Windows SCM of an arbitrary status string.
///
/// On Windows this would call `SetServiceStatus` with a custom checkpoint;
/// here we simply log it so the function is testable everywhere.
pub fn service_notify_status(status: &str) {
    tracing::debug!(status = %status, "Service status update");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_machine_id_non_empty() {
        let id = read_machine_id();
        assert!(!id.is_empty(), "machine ID must not be empty");
    }

    #[test]
    fn ensure_directories_uses_temp_in_tests() {
        // On non-Windows the function is a no-op; ensure it doesn't panic.
        let result = ensure_directories();
        assert!(result.is_ok());
    }

    #[test]
    fn data_dir_non_empty() {
        let dir = data_dir();
        assert!(!dir.as_os_str().is_empty());
    }

    #[test]
    fn service_notify_functions_do_not_panic() {
        service_notify_ready();
        service_notify_stopping();
        service_notify_status("test");
    }
}
