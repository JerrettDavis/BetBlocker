//! Windows ACL (Access Control List) enforcement.
//!
//! Manages DACL entries to protect BetBlocker files and registry keys
//! from tampering by non-admin users.

use std::path::Path;

/// Errors that can occur during ACL operations.
#[derive(Debug, thiserror::Error)]
pub enum AclError {
    /// Underlying Win32 API error.
    #[error("Win32 error: {0}")]
    Win32(String),

    /// Permission denied when modifying ACLs.
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// The specified path was not found.
    #[error("path not found: {0}")]
    PathNotFound(String),

    /// Failed to set registry key ACL.
    #[error("registry ACL error: {0}")]
    RegistryError(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Set a restrictive DACL on a file.
///
/// On Windows, configures the file ACL so that only SYSTEM and
/// Administrators have write access. All other users get read-only.
///
/// On non-Windows platforms, returns an error.
#[cfg(target_os = "windows")]
pub fn set_restrictive_file_acl(path: &Path) -> Result<(), AclError> {
    let path_str = path
        .to_str()
        .ok_or_else(|| AclError::PathNotFound(format!("{}", path.display())))?;

    // Use icacls to set restrictive permissions:
    // - Remove inherited permissions
    // - Grant SYSTEM and Administrators full control
    // - Grant Users read-only
    let output = std::process::Command::new("icacls")
        .args([
            path_str,
            "/inheritance:r",
            "/grant:r",
            "SYSTEM:(F)",
            "/grant:r",
            "Administrators:(F)",
            "/grant:r",
            "Users:(R)",
        ])
        .output()
        .map_err(|e| AclError::Io(e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AclError::Win32(format!("icacls failed: {stderr}")));
    }

    tracing::info!(path = %path.display(), "restrictive file ACL applied");
    Ok(())
}

/// Stub for non-Windows platforms.
#[cfg(not(target_os = "windows"))]
pub fn set_restrictive_file_acl(_path: &Path) -> Result<(), AclError> {
    Err(AclError::Win32(
        "ACL operations are only available on Windows".to_string(),
    ))
}

/// Set restrictive DACLs on a directory and its contents recursively.
///
/// On Windows, applies restrictive ACLs using `icacls /T` for recursive
/// application. On non-Windows platforms, returns an error.
#[cfg(target_os = "windows")]
pub fn set_restrictive_directory_acl(dir: &Path) -> Result<(), AclError> {
    let dir_str = dir
        .to_str()
        .ok_or_else(|| AclError::PathNotFound(format!("{}", dir.display())))?;

    // Apply restrictive ACL recursively
    let output = std::process::Command::new("icacls")
        .args([
            dir_str,
            "/inheritance:r",
            "/grant:r",
            "SYSTEM:(OI)(CI)(F)",
            "/grant:r",
            "Administrators:(OI)(CI)(F)",
            "/grant:r",
            "Users:(OI)(CI)(RX)",
            "/T",
        ])
        .output()
        .map_err(|e| AclError::Io(e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AclError::Win32(format!("icacls failed: {stderr}")));
    }

    tracing::info!(dir = %dir.display(), "restrictive directory ACL applied recursively");
    Ok(())
}

/// Stub for non-Windows platforms.
#[cfg(not(target_os = "windows"))]
pub fn set_restrictive_directory_acl(_dir: &Path) -> Result<(), AclError> {
    Err(AclError::Win32(
        "ACL operations are only available on Windows".to_string(),
    ))
}

/// Protect a Windows Registry key with restrictive ACLs.
///
/// On Windows, uses `reg.exe` to restrict write access to the specified
/// registry key so that only SYSTEM and Administrators can modify it.
///
/// On non-Windows platforms, returns an error.
#[cfg(target_os = "windows")]
pub fn protect_registry_key(hive: &str, subkey: &str) -> Result<(), AclError> {
    let full_key = format!("{hive}\\{subkey}");

    // Use regini or SubInACL to modify registry ACLs.
    // Here we use a PowerShell command for ACL modification.
    let ps_script = format!(
        r#"$acl = Get-Acl 'Registry::{full_key}'; $acl.SetAccessRuleProtection($true, $false); $rule = New-Object System.Security.AccessControl.RegistryAccessRule('BUILTIN\Users', 'ReadKey', 'Allow'); $acl.AddAccessRule($rule); $rule2 = New-Object System.Security.AccessControl.RegistryAccessRule('BUILTIN\Administrators', 'FullControl', 'Allow'); $acl.AddAccessRule($rule2); $rule3 = New-Object System.Security.AccessControl.RegistryAccessRule('NT AUTHORITY\SYSTEM', 'FullControl', 'Allow'); $acl.AddAccessRule($rule3); Set-Acl 'Registry::{full_key}' $acl"#
    );

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps_script])
        .output()
        .map_err(|e| AclError::Io(e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AclError::RegistryError(format!(
            "failed to protect registry key {full_key}: {stderr}"
        )));
    }

    tracing::info!(key = %full_key, "registry key ACL protected");
    Ok(())
}

/// Stub for non-Windows platforms.
#[cfg(not(target_os = "windows"))]
pub fn protect_registry_key(_hive: &str, _subkey: &str) -> Result<(), AclError> {
    Err(AclError::RegistryError(
        "registry operations are only available on Windows".to_string(),
    ))
}

/// Verify that a file or directory has the expected restrictive ACL.
///
/// On Windows, checks via `icacls` that only SYSTEM and Administrators
/// have write access. Returns `Ok(true)` if the ACL is correct.
///
/// On non-Windows platforms, returns an error.
#[cfg(target_os = "windows")]
pub fn verify_acl(path: &Path) -> Result<bool, AclError> {
    let path_str = path
        .to_str()
        .ok_or_else(|| AclError::PathNotFound(format!("{}", path.display())))?;

    let output = std::process::Command::new("icacls")
        .args([path_str])
        .output()
        .map_err(|e| AclError::Io(e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AclError::Win32(format!("icacls query failed: {stderr}")));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify that Users do NOT have write/modify/full access
    // Look for patterns like "Users:(W)" or "Users:(M)" or "Users:(F)"
    let has_user_write = stdout.lines().any(|line| {
        let line_upper = line.to_uppercase();
        line_upper.contains("USERS")
            && (line_upper.contains("(F)")
                || line_upper.contains("(M)")
                || line_upper.contains("(W)"))
    });

    Ok(!has_user_write)
}

/// Stub for non-Windows platforms.
#[cfg(not(target_os = "windows"))]
pub fn verify_acl(_path: &Path) -> Result<bool, AclError> {
    Err(AclError::Win32(
        "ACL verification is only available on Windows".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acl_error_display_win32() {
        let err = AclError::Win32("access denied".to_string());
        assert!(err.to_string().contains("access denied"));
    }

    #[test]
    fn acl_error_display_permission_denied() {
        let err = AclError::PermissionDenied("cannot modify".to_string());
        assert!(err.to_string().contains("cannot modify"));
    }

    #[test]
    fn acl_error_display_path_not_found() {
        let err = AclError::PathNotFound("/missing/file".to_string());
        assert!(err.to_string().contains("/missing/file"));
    }

    #[test]
    fn acl_error_display_registry() {
        let err = AclError::RegistryError("key not found".to_string());
        assert!(err.to_string().contains("key not found"));
    }

    #[test]
    fn non_windows_stubs_return_errors() {
        #[cfg(not(target_os = "windows"))]
        {
            use std::path::PathBuf;

            let path = PathBuf::from("/tmp/test");
            assert!(set_restrictive_file_acl(&path).is_err());
            assert!(set_restrictive_directory_acl(&path).is_err());
            assert!(protect_registry_key("HKLM", "SOFTWARE\\Test").is_err());
            assert!(verify_acl(&path).is_err());
        }
    }
}
