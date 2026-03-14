//! macOS file protection.
//!
//! Protects BetBlocker files from tampering using macOS file flags,
//! ownership controls, and permission enforcement.
//! On non-macOS platforms, provides type-compatible stubs.

use std::path::Path;

/// Errors from file protection operations.
#[derive(Debug, thiserror::Error)]
pub enum FileProtectError {
    /// Failed to set file permissions.
    #[error("permission change failed: {0}")]
    PermissionFailed(String),

    /// Failed to set file flags.
    #[error("flag change failed: {0}")]
    FlagFailed(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Set agent file permissions: owner root:wheel, mode 0755.
///
/// On macOS, uses `chown` and `chmod` to enforce ownership and
/// permissions. On non-macOS, returns an error.
#[cfg(target_os = "macos")]
pub fn set_agent_file_permissions(path: &Path) -> Result<(), FileProtectError> {
    let path_str = path
        .to_str()
        .ok_or_else(|| FileProtectError::PermissionFailed(format!("invalid path: {}", path.display())))?;

    // Set owner to root:wheel
    let output = std::process::Command::new("chown")
        .args(["root:wheel", path_str])
        .output()
        .map_err(|e| FileProtectError::Io(e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(FileProtectError::PermissionFailed(format!(
            "chown failed: {stderr}"
        )));
    }

    // Set mode to 0755 (rwxr-xr-x)
    let output = std::process::Command::new("chmod")
        .args(["0755", path_str])
        .output()
        .map_err(|e| FileProtectError::Io(e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(FileProtectError::PermissionFailed(format!(
            "chmod failed: {stderr}"
        )));
    }

    tracing::info!(path = %path.display(), "agent file permissions set (root:wheel 0755)");
    Ok(())
}

/// Stub for non-macOS platforms.
#[cfg(not(target_os = "macos"))]
pub fn set_agent_file_permissions(_path: &Path) -> Result<(), FileProtectError> {
    Err(FileProtectError::PermissionFailed(
        "file protection is only available on macOS".to_string(),
    ))
}

/// Set the SF_IMMUTABLE flag on a file via `chflags schg`.
///
/// This prevents modification or deletion even by root (unless the
/// flag is first cleared). On non-macOS, returns an error.
#[cfg(target_os = "macos")]
pub fn set_immutable_flag(path: &Path) -> Result<(), FileProtectError> {
    let path_str = path
        .to_str()
        .ok_or_else(|| FileProtectError::FlagFailed(format!("invalid path: {}", path.display())))?;

    let output = std::process::Command::new("chflags")
        .args(["schg", path_str])
        .output()
        .map_err(|e| FileProtectError::Io(e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(FileProtectError::FlagFailed(format!(
            "chflags schg failed: {stderr}"
        )));
    }

    tracing::info!(path = %path.display(), "SF_IMMUTABLE flag set");
    Ok(())
}

/// Stub for non-macOS platforms.
#[cfg(not(target_os = "macos"))]
pub fn set_immutable_flag(_path: &Path) -> Result<(), FileProtectError> {
    Err(FileProtectError::FlagFailed(
        "chflags is only available on macOS".to_string(),
    ))
}

/// Verify that a file has the expected permissions (root:wheel, not world-writable).
///
/// On macOS, uses `stat` to check ownership and permissions.
/// On non-macOS, returns false.
#[cfg(target_os = "macos")]
pub fn verify_permissions(path: &Path) -> bool {
    let path_str = match path.to_str() {
        Some(s) => s,
        None => return false,
    };

    let output = std::process::Command::new("stat")
        .args(["-f", "%u:%g:%p", path_str])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let trimmed = stdout.trim();
            // Expected: uid 0 (root), gid 0 (wheel), no world-write
            // Format: "uid:gid:octal_mode"
            let parts: Vec<&str> = trimmed.split(':').collect();
            if parts.len() >= 3 {
                let uid = parts[0];
                let gid = parts[1];
                // Check root:wheel ownership
                if uid != "0" || gid != "0" {
                    return false;
                }
                // Check no world-writable (last octal digit should not have 2 set)
                if let Some(mode_str) = parts.get(2) {
                    if let Ok(mode) = u32::from_str_radix(mode_str, 8) {
                        return mode & 0o002 == 0;
                    }
                }
            }
            false
        }
        _ => false,
    }
}

/// Stub for non-macOS platforms.
#[cfg(not(target_os = "macos"))]
pub fn verify_permissions(_path: &Path) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn file_protect_error_display() {
        let err = FileProtectError::PermissionFailed("denied".to_string());
        assert!(err.to_string().contains("denied"));

        let err = FileProtectError::FlagFailed("not supported".to_string());
        assert!(err.to_string().contains("not supported"));
    }

    #[test]
    fn non_macos_stubs() {
        #[cfg(not(target_os = "macos"))]
        {
            let path = PathBuf::from("/tmp/test-file");
            assert!(set_agent_file_permissions(&path).is_err());
            assert!(set_immutable_flag(&path).is_err());
            assert!(!verify_permissions(&path));
        }
    }
}
