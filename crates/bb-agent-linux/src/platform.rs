/// Linux-specific platform bridge.
///
/// Provides platform-specific implementations for:
/// - Machine ID reading (from /etc/machine-id)
/// - Directory creation with correct ownership
/// - systemd notify integration

/// Read the machine ID from /etc/machine-id.
///
/// This is a stable, unique identifier for the Linux installation.
/// Falls back to a placeholder if the file cannot be read.
#[allow(dead_code)]
pub fn read_machine_id() -> String {
    std::fs::read_to_string("/etc/machine-id")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown-machine-id".to_string())
}

/// Ensure required directories exist with correct permissions.
#[allow(dead_code)]
pub fn ensure_directories() -> Result<(), std::io::Error> {
    let dirs = [
        "/var/lib/betblocker",
        "/var/lib/betblocker/certs",
        "/var/log/betblocker",
    ];

    for dir in &dirs {
        std::fs::create_dir_all(dir)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o700);
            std::fs::set_permissions(dir, perms)?;
        }
    }

    Ok(())
}

/// Get the current process UID (for nftables loop prevention).
#[cfg(unix)]
#[allow(unsafe_code)]
pub fn current_uid() -> u32 {
    // Safety: getuid() is always safe to call — it has no preconditions,
    // takes no arguments, and simply returns the real user ID.
    unsafe { libc::getuid() }
}

#[cfg(not(unix))]
pub fn current_uid() -> u32 {
    0
}

/// Notify systemd that the service is ready.
///
/// Sends READY=1 via the sd_notify protocol. If the socket is not
/// available (not running under systemd), this is a no-op.
pub fn sd_notify_ready() {
    // Use the SD_NOTIFY socket directly to avoid an extra dependency.
    // The sd_notify protocol sends "READY=1\n" to the Unix socket
    // specified in $NOTIFY_SOCKET.
    #[cfg(unix)]
    {
        if let Ok(socket_path) = std::env::var("NOTIFY_SOCKET") {
            use std::os::unix::net::UnixDatagram;
            if let Ok(sock) = UnixDatagram::unbound() {
                let _ = sock.send_to(b"READY=1", &socket_path);
                tracing::info!("Sent sd_notify READY=1");
            }
        }
    }
}

/// Notify systemd of stopping.
pub fn sd_notify_stopping() {
    #[cfg(unix)]
    {
        if let Ok(socket_path) = std::env::var("NOTIFY_SOCKET") {
            use std::os::unix::net::UnixDatagram;
            if let Ok(sock) = UnixDatagram::unbound() {
                let _ = sock.send_to(b"STOPPING=1", &socket_path);
            }
        }
    }
}

/// Notify systemd of current status.
pub fn sd_notify_status(status: &str) {
    #[cfg(unix)]
    {
        if let Ok(socket_path) = std::env::var("NOTIFY_SOCKET") {
            use std::os::unix::net::UnixDatagram;
            if let Ok(sock) = UnixDatagram::unbound() {
                let msg = format!("STATUS={status}");
                let _ = sock.send_to(msg.as_bytes(), &socket_path);
            }
        }
    }
    let _ = status; // Suppress unused warning on non-unix
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_uid() {
        // Should not panic
        let _uid = current_uid();
    }

    #[test]
    fn test_sd_notify_ready_no_socket() {
        // Should be a no-op when NOTIFY_SOCKET is not set
        sd_notify_ready();
    }

    #[test]
    fn test_sd_notify_status_no_socket() {
        sd_notify_status("test status");
    }
}
