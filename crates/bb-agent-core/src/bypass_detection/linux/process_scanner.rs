use async_trait::async_trait;

use crate::bypass_detection::traits::{BypassDetectionError, ProcessScanner};

/// Linux process scanner that reads `/proc/*/comm` to find running processes
/// matching known bypass tool names.
pub struct LinuxProcessScanner;

impl LinuxProcessScanner {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LinuxProcessScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "linux")]
#[async_trait]
impl ProcessScanner for LinuxProcessScanner {
    async fn scan_for_processes(
        &self,
        known_names: &[&str],
    ) -> Result<Vec<String>, BypassDetectionError> {
        let mut found = Vec::new();
        let entries = std::fs::read_dir("/proc").map_err(BypassDetectionError::Io)?;

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Only look at numeric directories (PIDs).
            if !name_str.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }

            let comm_path = entry.path().join("comm");
            if let Ok(comm) = std::fs::read_to_string(&comm_path) {
                let comm = comm.trim();
                if known_names.iter().any(|&kn| comm == kn) && !found.contains(&comm.to_string()) {
                    found.push(comm.to_string());
                }
            }
        }

        Ok(found)
    }
}

#[cfg(not(target_os = "linux"))]
#[async_trait]
impl ProcessScanner for LinuxProcessScanner {
    async fn scan_for_processes(
        &self,
        _known_names: &[&str],
    ) -> Result<Vec<String>, BypassDetectionError> {
        // Non-Linux stub: return empty results.
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn scan_returns_ok() {
        let scanner = LinuxProcessScanner::new();
        let result = scanner.scan_for_processes(&["openvpn", "tor"]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn scan_with_empty_known_names() {
        let scanner = LinuxProcessScanner::new();
        let result = scanner.scan_for_processes(&[]).await.unwrap();
        assert!(result.is_empty());
    }
}
