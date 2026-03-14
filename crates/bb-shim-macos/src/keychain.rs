//! macOS Keychain integration.
//!
//! Stores and retrieves cryptographic keys and credentials
//! using the macOS Security framework Keychain Services API.
//! On non-macOS platforms, provides type-compatible stubs.

/// Errors from Keychain operations.
#[derive(Debug, thiserror::Error)]
pub enum KeychainError {
    /// Failed to access the System Keychain.
    #[error("keychain access failed: {0}")]
    AccessFailed(String),

    /// The requested item was not found in the Keychain.
    #[error("item not found: {0}")]
    NotFound(String),

    /// Failed to store an item in the Keychain.
    #[error("store failed: {0}")]
    StoreFailed(String),

    /// Failed to delete an item from the Keychain.
    #[error("delete failed: {0}")]
    DeleteFailed(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Service name used for BetBlocker Keychain items.
const KEYCHAIN_SERVICE: &str = "com.betblocker.agent";

/// Manages certificate and key storage in the macOS System Keychain.
pub struct KeychainCertificateStore {
    /// The Keychain service label for BetBlocker items.
    pub service: String,
}

impl KeychainCertificateStore {
    /// Create a new `KeychainCertificateStore` with the default service name.
    pub fn new() -> Self {
        Self {
            service: KEYCHAIN_SERVICE.to_string(),
        }
    }

    /// Create with a custom service name (useful for testing).
    pub fn with_service(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    /// Store a PEM-encoded identity (certificate + private key) in the System Keychain.
    ///
    /// On macOS, uses `security import` to store the identity.
    /// On non-macOS, returns an error.
    #[cfg(target_os = "macos")]
    pub fn store_identity(&self, pem: &[u8]) -> Result<(), KeychainError> {
        use std::io::Write;

        // Write PEM to a temporary file
        let tmp_path = std::env::temp_dir().join("bb-identity.pem");
        let mut file = std::fs::File::create(&tmp_path)
            .map_err(|e| KeychainError::StoreFailed(e.to_string()))?;
        file.write_all(pem)
            .map_err(|e| KeychainError::StoreFailed(e.to_string()))?;
        drop(file);

        let output = std::process::Command::new("security")
            .args([
                "import",
                tmp_path.to_str().unwrap_or(""),
                "-k",
                "/Library/Keychains/System.keychain",
                "-T",
                "/usr/sbin/bb-agent-macos",
                "-l",
                &self.service,
            ])
            .output()
            .map_err(|e| KeychainError::StoreFailed(e.to_string()))?;

        // Clean up temp file
        let _ = std::fs::remove_file(&tmp_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(KeychainError::StoreFailed(format!(
                "security import failed: {stderr}"
            )));
        }

        tracing::info!(service = %self.service, "identity stored in System Keychain");
        Ok(())
    }

    /// Stub for non-macOS platforms.
    #[cfg(not(target_os = "macos"))]
    pub fn store_identity(&self, _pem: &[u8]) -> Result<(), KeychainError> {
        Err(KeychainError::AccessFailed(
            "Keychain is only available on macOS".to_string(),
        ))
    }

    /// Load an identity from the System Keychain.
    ///
    /// Returns the PEM-encoded identity bytes, or `None` if not found.
    #[cfg(target_os = "macos")]
    pub fn load_identity(&self) -> Result<Option<Vec<u8>>, KeychainError> {
        let output = std::process::Command::new("security")
            .args([
                "find-identity",
                "-v",
                "-p",
                "ssl-client",
                "/Library/Keychains/System.keychain",
            ])
            .output()
            .map_err(|e| KeychainError::AccessFailed(e.to_string()))?;

        if !output.status.success() {
            return Ok(None);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains(&self.service) {
            // Export the identity
            let export_output = std::process::Command::new("security")
                .args([
                    "export",
                    "-k",
                    "/Library/Keychains/System.keychain",
                    "-t",
                    "identities",
                    "-f",
                    "pemseq",
                ])
                .output()
                .map_err(|e| KeychainError::AccessFailed(e.to_string()))?;

            if export_output.status.success() {
                Ok(Some(export_output.stdout))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Stub for non-macOS platforms.
    #[cfg(not(target_os = "macos"))]
    pub fn load_identity(&self) -> Result<Option<Vec<u8>>, KeychainError> {
        Err(KeychainError::AccessFailed(
            "Keychain is only available on macOS".to_string(),
        ))
    }

    /// Store a PEM-encoded CA certificate chain in the System Keychain.
    #[cfg(target_os = "macos")]
    pub fn store_ca_chain(&self, pem: &[u8]) -> Result<(), KeychainError> {
        use std::io::Write;

        let tmp_path = std::env::temp_dir().join("bb-ca-chain.pem");
        let mut file = std::fs::File::create(&tmp_path)
            .map_err(|e| KeychainError::StoreFailed(e.to_string()))?;
        file.write_all(pem)
            .map_err(|e| KeychainError::StoreFailed(e.to_string()))?;
        drop(file);

        let output = std::process::Command::new("security")
            .args([
                "add-trusted-cert",
                "-d",
                "-k",
                "/Library/Keychains/System.keychain",
                tmp_path.to_str().unwrap_or(""),
            ])
            .output()
            .map_err(|e| KeychainError::StoreFailed(e.to_string()))?;

        let _ = std::fs::remove_file(&tmp_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(KeychainError::StoreFailed(format!(
                "failed to add CA chain: {stderr}"
            )));
        }

        tracing::info!(service = %self.service, "CA chain stored in System Keychain");
        Ok(())
    }

    /// Stub for non-macOS platforms.
    #[cfg(not(target_os = "macos"))]
    pub fn store_ca_chain(&self, _pem: &[u8]) -> Result<(), KeychainError> {
        Err(KeychainError::AccessFailed(
            "Keychain is only available on macOS".to_string(),
        ))
    }

    /// Load the CA certificate chain from the System Keychain.
    #[cfg(target_os = "macos")]
    pub fn load_ca_chain(&self) -> Result<Option<Vec<u8>>, KeychainError> {
        let output = std::process::Command::new("security")
            .args([
                "find-certificate",
                "-a",
                "-p",
                "/Library/Keychains/System.keychain",
            ])
            .output()
            .map_err(|e| KeychainError::AccessFailed(e.to_string()))?;

        if output.status.success() && !output.stdout.is_empty() {
            Ok(Some(output.stdout))
        } else {
            Ok(None)
        }
    }

    /// Stub for non-macOS platforms.
    #[cfg(not(target_os = "macos"))]
    pub fn load_ca_chain(&self) -> Result<Option<Vec<u8>>, KeychainError> {
        Err(KeychainError::AccessFailed(
            "Keychain is only available on macOS".to_string(),
        ))
    }

    /// Delete all BetBlocker items from the System Keychain.
    #[cfg(target_os = "macos")]
    pub fn delete_all(&self) -> Result<(), KeychainError> {
        let output = std::process::Command::new("security")
            .args([
                "delete-identity",
                "-t",
                "-Z",
                "-c",
                &self.service,
                "/Library/Keychains/System.keychain",
            ])
            .output()
            .map_err(|e| KeychainError::DeleteFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Not finding items to delete is acceptable
            if !stderr.contains("could not be found") {
                return Err(KeychainError::DeleteFailed(format!(
                    "failed to delete Keychain items: {stderr}"
                )));
            }
        }

        tracing::info!(service = %self.service, "Keychain items deleted");
        Ok(())
    }

    /// Stub for non-macOS platforms.
    #[cfg(not(target_os = "macos"))]
    pub fn delete_all(&self) -> Result<(), KeychainError> {
        Err(KeychainError::AccessFailed(
            "Keychain is only available on macOS".to_string(),
        ))
    }
}

impl Default for KeychainCertificateStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keychain_error_display() {
        let err = KeychainError::AccessFailed("no access".to_string());
        assert!(err.to_string().contains("no access"));

        let err = KeychainError::NotFound("cert".to_string());
        assert!(err.to_string().contains("cert"));

        let err = KeychainError::StoreFailed("disk full".to_string());
        assert!(err.to_string().contains("disk full"));

        let err = KeychainError::DeleteFailed("locked".to_string());
        assert!(err.to_string().contains("locked"));
    }

    #[test]
    fn construction_default_service() {
        let store = KeychainCertificateStore::new();
        assert_eq!(store.service, KEYCHAIN_SERVICE);
    }

    #[test]
    fn construction_custom_service() {
        let store = KeychainCertificateStore::with_service("com.test.app");
        assert_eq!(store.service, "com.test.app");
    }

    #[test]
    fn default_trait() {
        let store = KeychainCertificateStore::default();
        assert_eq!(store.service, KEYCHAIN_SERVICE);
    }

    #[test]
    fn non_macos_stubs_return_errors() {
        #[cfg(not(target_os = "macos"))]
        {
            let store = KeychainCertificateStore::new();
            assert!(store.store_identity(b"pem-data").is_err());
            assert!(store.load_identity().is_err());
            assert!(store.store_ca_chain(b"ca-pem").is_err());
            assert!(store.load_ca_chain().is_err());
            assert!(store.delete_all().is_err());
        }
    }
}
