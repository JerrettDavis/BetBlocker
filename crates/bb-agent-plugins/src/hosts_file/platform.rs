use std::path::PathBuf;

/// Returns the platform-appropriate HOSTS file path.
pub fn hosts_file_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(r"C:\Windows\System32\drivers\etc\hosts")
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        PathBuf::from("/etc/hosts")
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        PathBuf::from("/etc/hosts")
    }
}
