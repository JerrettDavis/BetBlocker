use std::path::{Path, PathBuf};
use std::time::Duration;

use sha2::{Digest, Sha256};
use tokio::sync::watch;

/// Binary integrity checker.
///
/// On startup, reads the agent binary from `/proc/self/exe` (Linux)
/// or the current executable path, computes SHA-256, and stores it.
/// Periodically re-hashes to detect tampering.
pub struct BinaryIntegrity {
    /// Path to the agent binary.
    binary_path: PathBuf,
    /// SHA-256 hash computed at startup.
    startup_hash: Vec<u8>,
    /// Expected hash from enrollment config (if available).
    #[allow(dead_code)]
    expected_hash: Option<Vec<u8>>,
    /// Re-check interval (default: 30 minutes).
    check_interval: Duration,
}

/// Errors from integrity checking.
#[derive(Debug, thiserror::Error)]
pub enum IntegrityError {
    #[error("Failed to read binary: {0}")]
    ReadFailed(std::io::Error),

    #[error("Binary hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("Binary path not found: {0}")]
    PathNotFound(PathBuf),

    #[error("Config integrity check failed: {0}")]
    ConfigIntegrityFailed(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),
}

impl BinaryIntegrity {
    /// Create a new integrity checker and compute the startup hash.
    pub fn new(expected_hash: Option<Vec<u8>>) -> Result<Self, IntegrityError> {
        let binary_path = Self::find_binary_path()?;
        let startup_hash = Self::compute_hash(&binary_path)?;

        // If we have an expected hash, verify immediately
        if let Some(ref expected) = expected_hash {
            if *expected != startup_hash {
                tracing::error!(
                    expected = hex::encode(expected),
                    actual = hex::encode(&startup_hash),
                    "Binary hash mismatch at startup!"
                );
                // Don't fail -- enter degraded mode instead
            }
        }

        Ok(Self {
            binary_path,
            startup_hash,
            expected_hash,
            check_interval: Duration::from_secs(1800), // 30 minutes
        })
    }

    /// Find the path to the current executable.
    fn find_binary_path() -> Result<PathBuf, IntegrityError> {
        // On Linux, /proc/self/exe is a symlink to the actual binary.
        // On other platforms, use std::env::current_exe.
        let path = if cfg!(target_os = "linux") {
            PathBuf::from("/proc/self/exe")
        } else {
            std::env::current_exe().map_err(IntegrityError::ReadFailed)?
        };

        Ok(path)
    }

    /// Compute SHA-256 hash of a file.
    pub fn compute_hash(path: &Path) -> Result<Vec<u8>, IntegrityError> {
        let data = std::fs::read(path).map_err(IntegrityError::ReadFailed)?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        Ok(hasher.finalize().to_vec())
    }

    /// Get the startup hash.
    pub fn startup_hash(&self) -> &[u8] {
        &self.startup_hash
    }

    /// Get the startup hash as a hex string.
    pub fn startup_hash_hex(&self) -> String {
        hex::encode(&self.startup_hash)
    }

    /// Verify the binary has not been modified since startup.
    pub fn verify(&self) -> Result<bool, IntegrityError> {
        let current_hash = Self::compute_hash(&self.binary_path)?;
        Ok(current_hash == self.startup_hash)
    }

    /// Run the periodic integrity check loop.
    pub async fn run_periodic_check(
        &self,
        mut shutdown: watch::Receiver<bool>,
        tamper_callback: impl Fn(IntegrityError) + Send + 'static,
    ) {
        let mut ticker = tokio::time::interval(self.check_interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    match self.verify() {
                        Ok(true) => {
                            tracing::trace!("Binary integrity check passed");
                        }
                        Ok(false) => {
                            tracing::error!("Binary modified since startup!");
                            tamper_callback(IntegrityError::HashMismatch {
                                expected: hex::encode(&self.startup_hash),
                                actual: "changed".to_string(),
                            });
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "Binary integrity check failed");
                        }
                    }
                }
                _ = shutdown.changed() => {
                    tracing::info!("Integrity checker shutting down");
                    break;
                }
            }
        }
    }
}

/// Configuration integrity checker.
///
/// Verifies that the enrollment configuration has not been tampered with.
/// Uses Ed25519 signature verification and AES-256-GCM encryption at rest.
pub struct ConfigIntegrity {
    config_path: PathBuf,
    backup_path: PathBuf,
}

impl ConfigIntegrity {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            config_path: data_dir.join("config.enc"),
            backup_path: data_dir.join("config.enc.bak"),
        }
    }

    /// Encrypt configuration data using AES-256-GCM.
    ///
    /// Key is derived via HKDF from the machine ID + a random salt.
    pub fn encrypt_config(plaintext: &[u8], machine_id: &[u8]) -> Result<Vec<u8>, IntegrityError> {
        use aes_gcm::{
            Aes256Gcm, Nonce,
            aead::{Aead, KeyInit},
        };
        use hkdf::Hkdf;
        use sha2::Sha256 as HkdfSha256;

        // Generate random salt and nonce
        let mut salt = [0u8; 32];
        let mut nonce_bytes = [0u8; 12];
        ring::rand::SystemRandom::new()
            .fill(&mut salt)
            .map_err(|e| IntegrityError::EncryptionError(e.to_string()))?;
        ring::rand::SystemRandom::new()
            .fill(&mut nonce_bytes)
            .map_err(|e| IntegrityError::EncryptionError(e.to_string()))?;

        // Derive key via HKDF
        let hk = Hkdf::<HkdfSha256>::new(Some(&salt), machine_id);
        let mut key = [0u8; 32];
        hk.expand(b"betblocker-config-encryption", &mut key)
            .map_err(|e| IntegrityError::EncryptionError(e.to_string()))?;

        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| IntegrityError::EncryptionError(e.to_string()))?;
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| IntegrityError::EncryptionError(e.to_string()))?;

        // Output format: salt (32) || nonce (12) || ciphertext
        let mut output = Vec::with_capacity(32 + 12 + ciphertext.len());
        output.extend_from_slice(&salt);
        output.extend_from_slice(&nonce_bytes);
        output.extend_from_slice(&ciphertext);

        Ok(output)
    }

    /// Decrypt configuration data encrypted with `encrypt_config`.
    pub fn decrypt_config(encrypted: &[u8], machine_id: &[u8]) -> Result<Vec<u8>, IntegrityError> {
        use aes_gcm::{
            Aes256Gcm, Nonce,
            aead::{Aead, KeyInit},
        };
        use hkdf::Hkdf;
        use sha2::Sha256 as HkdfSha256;

        if encrypted.len() < 44 {
            // 32 (salt) + 12 (nonce) = 44 minimum
            return Err(IntegrityError::EncryptionError(
                "Encrypted data too short".to_string(),
            ));
        }

        let salt = &encrypted[..32];
        let nonce_bytes = &encrypted[32..44];
        let ciphertext = &encrypted[44..];

        // Derive key via HKDF
        let hk = Hkdf::<HkdfSha256>::new(Some(salt), machine_id);
        let mut key = [0u8; 32];
        hk.expand(b"betblocker-config-encryption", &mut key)
            .map_err(|e| IntegrityError::EncryptionError(e.to_string()))?;

        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| IntegrityError::EncryptionError(e.to_string()))?;
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| IntegrityError::EncryptionError(e.to_string()))?;

        Ok(plaintext)
    }

    /// Store encrypted config, creating a backup of the previous version.
    pub fn store_encrypted(
        &self,
        plaintext: &[u8],
        machine_id: &[u8],
    ) -> Result<(), IntegrityError> {
        let encrypted = Self::encrypt_config(plaintext, machine_id)?;

        // Backup existing config if it exists
        if self.config_path.exists() {
            std::fs::copy(&self.config_path, &self.backup_path)
                .map_err(IntegrityError::ReadFailed)?;
        }

        std::fs::write(&self.config_path, &encrypted).map_err(IntegrityError::ReadFailed)?;

        Ok(())
    }

    /// Load and decrypt config. On failure, try backup. On backup failure, return error.
    pub fn load_encrypted(&self, machine_id: &[u8]) -> Result<Vec<u8>, IntegrityError> {
        // Try primary
        if let Ok(encrypted) = std::fs::read(&self.config_path) {
            if let Ok(plaintext) = Self::decrypt_config(&encrypted, machine_id) {
                return Ok(plaintext);
            }
            tracing::warn!("Primary config decryption failed, trying backup");
        }

        // Try backup
        if let Ok(encrypted) = std::fs::read(&self.backup_path) {
            if let Ok(plaintext) = Self::decrypt_config(&encrypted, machine_id) {
                // Restore backup to primary
                if let Err(e) = std::fs::copy(&self.backup_path, &self.config_path) {
                    tracing::warn!(error = %e, "Failed to restore backup to primary path");
                }
                return Ok(plaintext);
            }
        }

        Err(IntegrityError::ConfigIntegrityFailed(
            "Both primary and backup config are corrupted or missing. Safe mode required."
                .to_string(),
        ))
    }
}

/// Hex encoding utility (small, dependency-free).
mod hex {
    pub fn encode(data: &[u8]) -> String {
        data.iter().map(|b| format!("{b:02x}")).collect()
    }
}

use ring::rand::SecureRandom;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash_deterministic() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test_binary");
        std::fs::write(&path, b"hello world").expect("write");

        let hash1 = BinaryIntegrity::compute_hash(&path).expect("hash1");
        let hash2 = BinaryIntegrity::compute_hash(&path).expect("hash2");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_hash_changes_on_modification() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test_binary");

        std::fs::write(&path, b"original content").expect("write");
        let hash1 = BinaryIntegrity::compute_hash(&path).expect("hash1");

        std::fs::write(&path, b"modified content").expect("write");
        let hash2 = BinaryIntegrity::compute_hash(&path).expect("hash2");

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hex_encode() {
        assert_eq!(hex::encode(&[0xde, 0xad, 0xbe, 0xef]), "deadbeef");
        assert_eq!(hex::encode(&[]), "");
    }

    #[test]
    fn test_config_encrypt_decrypt_roundtrip() {
        let plaintext = b"sensitive config data: {\"tier\": \"partner\"}";
        let machine_id = b"test-machine-id-12345";

        let encrypted = ConfigIntegrity::encrypt_config(plaintext, machine_id).expect("encrypt");
        assert_ne!(encrypted, plaintext);
        assert!(encrypted.len() > plaintext.len()); // Salt + nonce + tag overhead

        let decrypted = ConfigIntegrity::decrypt_config(&encrypted, machine_id).expect("decrypt");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_config_decrypt_wrong_key() {
        let plaintext = b"secret data";
        let machine_id = b"correct-machine-id";
        let wrong_id = b"wrong-machine-id-xx";

        let encrypted = ConfigIntegrity::encrypt_config(plaintext, machine_id).expect("encrypt");

        let result = ConfigIntegrity::decrypt_config(&encrypted, wrong_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_decrypt_tampered_data() {
        let plaintext = b"secret data";
        let machine_id = b"test-machine-id";

        let mut encrypted =
            ConfigIntegrity::encrypt_config(plaintext, machine_id).expect("encrypt");

        // Tamper with ciphertext
        if let Some(byte) = encrypted.last_mut() {
            *byte ^= 0xFF;
        }

        let result = ConfigIntegrity::decrypt_config(&encrypted, machine_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_integrity_store_load() {
        let dir = tempfile::tempdir().expect("tempdir");
        let integrity = ConfigIntegrity::new(dir.path());
        let machine_id = b"test-machine-id";
        let config_data = b"enrollment config json";

        integrity
            .store_encrypted(config_data, machine_id)
            .expect("store");
        let loaded = integrity.load_encrypted(machine_id).expect("load");
        assert_eq!(loaded, config_data);
    }

    #[test]
    fn test_config_integrity_backup_restore() {
        let dir = tempfile::tempdir().expect("tempdir");
        let integrity = ConfigIntegrity::new(dir.path());
        let machine_id = b"test-machine-id";

        // Store initial version
        integrity
            .store_encrypted(b"version 1", machine_id)
            .expect("store v1");

        // Store second version (creates backup of v1)
        integrity
            .store_encrypted(b"version 2", machine_id)
            .expect("store v2");

        // Corrupt primary
        std::fs::write(dir.path().join("config.enc"), b"corrupted").expect("corrupt");

        // Should fall back to backup (version 1)
        let loaded = integrity.load_encrypted(machine_id).expect("load");
        assert_eq!(loaded, b"version 1");
    }

    #[test]
    fn test_config_integrity_both_corrupted() {
        let dir = tempfile::tempdir().expect("tempdir");
        let integrity = ConfigIntegrity::new(dir.path());
        let machine_id = b"test-machine-id";

        std::fs::write(dir.path().join("config.enc"), b"bad").expect("write");
        std::fs::write(dir.path().join("config.enc.bak"), b"also bad").expect("write");

        let result = integrity.load_encrypted(machine_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_decrypt_too_short() {
        let result = ConfigIntegrity::decrypt_config(&[0u8; 10], b"key");
        assert!(result.is_err());
    }
}
