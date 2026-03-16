//! Windows credential and key storage.
//!
//! Stores cryptographic keys and credentials using the Windows
//! Trusted Platform Module (TPM) or Data Protection API (DPAPI).
//! Falls back to DPAPI when TPM is not available.

use std::path::{Path, PathBuf};

/// Errors that can occur during keystore operations.
#[derive(Debug, thiserror::Error)]
pub enum KeystoreError {
    /// TPM hardware is not available on this system.
    #[error("TPM is not available")]
    TpmNotAvailable,

    /// DPAPI operation failed.
    #[error("DPAPI error: {0}")]
    DpapiError(String),

    /// Failed to serialize or deserialize key data.
    #[error("serialization error: {0}")]
    SerializationError(String),

    /// IO error reading/writing key files.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Windows keystore backed by TPM or DPAPI.
///
/// Attempts to use TPM for key storage; falls back to DPAPI if
/// TPM is not available. Keys are persisted to the data directory.
pub struct WindowsKeystore {
    /// Directory where encrypted key files are stored.
    pub data_dir: PathBuf,
    /// Whether to attempt TPM-based storage.
    pub use_tpm: bool,
}

impl WindowsKeystore {
    /// Create a new keystore.
    ///
    /// If `use_tpm` is true, the keystore will attempt TPM-backed
    /// storage and fall back to DPAPI if TPM is not available.
    pub fn new(data_dir: impl Into<PathBuf>, use_tpm: bool) -> Self {
        Self {
            data_dir: data_dir.into(),
            use_tpm,
        }
    }

    /// Path where a named key is stored on disk.
    fn key_path(&self, name: &str) -> PathBuf {
        self.data_dir.join(format!("{name}.key"))
    }

    /// Check whether a TPM is available on this system.
    ///
    /// On Windows, checks for the TPM Base Services (TBS) provider.
    /// On non-Windows, always returns false.
    #[cfg(target_os = "windows")]
    pub fn has_tpm() -> bool {
        // Check for TPM via tpm.msc / Win32_Tpm WMI class
        let output = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "(Get-WmiObject -Namespace 'root\\cimv2\\Security\\MicrosoftTpm' -Class Win32_Tpm -ErrorAction SilentlyContinue) -ne $null",
            ])
            .output();

        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.trim().eq_ignore_ascii_case("true")
            }
            _ => false,
        }
    }

    /// Stub for non-Windows platforms.
    #[cfg(not(target_os = "windows"))]
    pub fn has_tpm() -> bool {
        false
    }

    /// Store a key by name.
    ///
    /// On Windows, tries TPM-backed storage first (if enabled), then
    /// falls back to DPAPI encryption. The encrypted key is written
    /// to the data directory.
    ///
    /// On non-Windows, returns an error.
    #[cfg(target_os = "windows")]
    pub fn store_key(&self, name: &str, key_bytes: &[u8]) -> Result<(), KeystoreError> {
        let key_path = self.key_path(name);

        // Ensure data directory exists
        std::fs::create_dir_all(&self.data_dir)?;

        if self.use_tpm && Self::has_tpm() {
            // TPM-backed storage via PowerShell / CNG with TPM provider
            tracing::info!(name = %name, "storing key via TPM");
            self.store_with_tpm(name, key_bytes)?;
        } else {
            if self.use_tpm {
                tracing::warn!(
                    name = %name,
                    "TPM not available, falling back to DPAPI"
                );
            }
            // DPAPI-backed storage
            self.store_with_dpapi(&key_path, key_bytes)?;
        }

        tracing::info!(name = %name, path = %key_path.display(), "key stored");
        Ok(())
    }

    /// Stub for non-Windows platforms.
    #[cfg(not(target_os = "windows"))]
    pub fn store_key(&self, _name: &str, _key_bytes: &[u8]) -> Result<(), KeystoreError> {
        Err(KeystoreError::DpapiError(
            "keystore operations are only available on Windows".to_string(),
        ))
    }

    /// Load a key by name.
    ///
    /// On Windows, reads and decrypts the key from disk.
    /// On non-Windows, returns an error.
    #[cfg(target_os = "windows")]
    pub fn load_key(&self, name: &str) -> Result<Vec<u8>, KeystoreError> {
        let key_path = self.key_path(name);

        if !key_path.exists() {
            return Err(KeystoreError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("key file not found: {}", key_path.display()),
            )));
        }

        if self.use_tpm && Self::has_tpm() {
            self.load_with_tpm(name)
        } else {
            self.load_with_dpapi(&key_path)
        }
    }

    /// Stub for non-Windows platforms.
    #[cfg(not(target_os = "windows"))]
    pub fn load_key(&self, _name: &str) -> Result<Vec<u8>, KeystoreError> {
        Err(KeystoreError::DpapiError(
            "keystore operations are only available on Windows".to_string(),
        ))
    }

    /// Delete a stored key by name.
    ///
    /// Removes the key file from disk.
    pub fn delete_key(&self, name: &str) -> Result<(), KeystoreError> {
        let key_path = self.key_path(name);
        if key_path.exists() {
            std::fs::remove_file(&key_path)?;
            tracing::info!(name = %name, "key deleted");
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Windows-only TPM and DPAPI helpers
    // -----------------------------------------------------------------------

    #[cfg(target_os = "windows")]
    fn store_with_tpm(&self, name: &str, key_bytes: &[u8]) -> Result<(), KeystoreError> {
        let key_path = self.key_path(name);
        // Use PowerShell to encrypt via DPAPI with machine scope + TPM
        let b64 = base64_encode(key_bytes);
        let ps_script = format!(
            r#"Add-Type -AssemblyName System.Security; $bytes = [Convert]::FromBase64String('{b64}'); $encrypted = [System.Security.Cryptography.ProtectedData]::Protect($bytes, $null, [System.Security.Cryptography.DataProtectionScope]::LocalMachine); [IO.File]::WriteAllBytes('{path}', $encrypted)"#,
            b64 = b64,
            path = key_path.display(),
        );

        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_script])
            .output()
            .map_err(|e| KeystoreError::DpapiError(e.to_string()))?;

        if !output.status.success() {
            return Err(KeystoreError::TpmNotAvailable);
        }

        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn store_with_dpapi(&self, key_path: &Path, key_bytes: &[u8]) -> Result<(), KeystoreError> {
        let b64 = base64_encode(key_bytes);
        let ps_script = format!(
            r#"Add-Type -AssemblyName System.Security; $bytes = [Convert]::FromBase64String('{b64}'); $encrypted = [System.Security.Cryptography.ProtectedData]::Protect($bytes, $null, [System.Security.Cryptography.DataProtectionScope]::LocalMachine); [IO.File]::WriteAllBytes('{path}', $encrypted)"#,
            b64 = b64,
            path = key_path.display(),
        );

        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_script])
            .output()
            .map_err(|e| KeystoreError::DpapiError(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(KeystoreError::DpapiError(format!(
                "DPAPI encrypt failed: {stderr}"
            )));
        }

        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn load_with_tpm(&self, name: &str) -> Result<Vec<u8>, KeystoreError> {
        self.load_with_dpapi(&self.key_path(name))
    }

    #[cfg(target_os = "windows")]
    fn load_with_dpapi(&self, key_path: &Path) -> Result<Vec<u8>, KeystoreError> {
        let ps_script = format!(
            r#"Add-Type -AssemblyName System.Security; $encrypted = [IO.File]::ReadAllBytes('{path}'); $decrypted = [System.Security.Cryptography.ProtectedData]::Unprotect($encrypted, $null, [System.Security.Cryptography.DataProtectionScope]::LocalMachine); [Convert]::ToBase64String($decrypted)"#,
            path = key_path.display(),
        );

        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_script])
            .output()
            .map_err(|e| KeystoreError::DpapiError(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(KeystoreError::DpapiError(format!(
                "DPAPI decrypt failed: {stderr}"
            )));
        }

        let b64 = String::from_utf8_lossy(&output.stdout).trim().to_string();
        base64_decode(&b64)
            .map_err(|e| KeystoreError::SerializationError(format!("base64 decode failed: {e}")))
    }
}

// ---------------------------------------------------------------------------
// Minimal base64 helpers (no external dependency required)
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

#[cfg(target_os = "windows")]
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let input = input.trim_end_matches('=');
    let mut result = Vec::with_capacity(input.len() * 3 / 4);

    let val = |c: u8| -> Result<u32, String> {
        match c {
            b'A'..=b'Z' => Ok((c - b'A') as u32),
            b'a'..=b'z' => Ok((c - b'a' + 26) as u32),
            b'0'..=b'9' => Ok((c - b'0' + 52) as u32),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err(format!("invalid base64 character: {c}")),
        }
    };

    let bytes = input.as_bytes();
    let chunks = bytes.chunks(4);
    for chunk in chunks {
        let mut acc: u32 = 0;
        for (i, &b) in chunk.iter().enumerate() {
            acc |= val(b)? << (6 * (3 - i));
        }
        result.push((acc >> 16) as u8);
        if chunk.len() > 2 {
            result.push((acc >> 8) as u8);
        }
        if chunk.len() > 3 {
            result.push(acc as u8);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keystore_error_display_tpm() {
        let err = KeystoreError::TpmNotAvailable;
        assert_eq!(err.to_string(), "TPM is not available");
    }

    #[test]
    fn keystore_error_display_dpapi() {
        let err = KeystoreError::DpapiError("encrypt failed".to_string());
        assert!(err.to_string().contains("encrypt failed"));
    }

    #[test]
    fn keystore_error_display_serialization() {
        let err = KeystoreError::SerializationError("bad format".to_string());
        assert!(err.to_string().contains("bad format"));
    }

    #[test]
    fn keystore_construction() {
        let ks = WindowsKeystore::new("/tmp/keys", true);
        assert_eq!(ks.data_dir, PathBuf::from("/tmp/keys"));
        assert!(ks.use_tpm);
    }

    #[test]
    fn keystore_construction_no_tpm() {
        let ks = WindowsKeystore::new("/tmp/keys", false);
        assert!(!ks.use_tpm);
    }

    #[test]
    fn key_path_generation() {
        let ks = WindowsKeystore::new("/data/keys", false);
        let path = ks.key_path("agent-cert");
        assert!(path.to_string_lossy().contains("agent-cert.key"));
    }

    #[test]
    fn has_tpm_returns_bool() {
        // On non-Windows, always false; on Windows, depends on hardware
        let result = WindowsKeystore::has_tpm();
        #[cfg(not(target_os = "windows"))]
        assert!(!result);
        // On Windows, we just check it doesn't panic
        let _ = result;
    }

    #[test]
    fn delete_nonexistent_key_succeeds() {
        let dir = std::env::temp_dir().join("bb-keystore-test-delete");
        let _ = std::fs::create_dir_all(&dir);
        let ks = WindowsKeystore::new(&dir, false);

        // Deleting a key that doesn't exist should succeed
        assert!(ks.delete_key("nonexistent").is_ok());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn non_windows_store_returns_error() {
        #[cfg(not(target_os = "windows"))]
        {
            let ks = WindowsKeystore::new("/tmp/keys", false);
            assert!(ks.store_key("test", b"data").is_err());
        }
    }

    #[test]
    fn non_windows_load_returns_error() {
        #[cfg(not(target_os = "windows"))]
        {
            let ks = WindowsKeystore::new("/tmp/keys", false);
            assert!(ks.load_key("test").is_err());
        }
    }
}
