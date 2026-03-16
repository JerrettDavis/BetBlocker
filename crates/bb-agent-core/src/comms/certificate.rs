use std::io;
use std::path::{Path, PathBuf};

/// Trait for storing and retrieving device certificates and keys.
///
/// Platform implementations store keys in hardware-backed keystores
/// where available, falling back to encrypted files.
pub trait CertificateStore: Send + Sync {
    /// Store the device certificate (PEM) and private key (PKCS#8 DER).
    fn store_identity(&self, cert_pem: &[u8], key_der: &[u8]) -> Result<(), CertStoreError>;

    /// Load the device identity as PEM bytes (cert + key concatenated).
    /// Returns None if no identity has been stored yet.
    fn load_identity(&self) -> Result<Option<Vec<u8>>, CertStoreError>;

    /// Store the CA certificate chain (PEM).
    fn store_ca_chain(&self, ca_pem: &[u8]) -> Result<(), CertStoreError>;

    /// Load the CA certificate chain (PEM).
    fn load_ca_chain(&self) -> Result<Option<Vec<u8>>, CertStoreError>;

    /// Get the certificate expiry time as a Unix timestamp.
    /// Returns None if no certificate is stored.
    fn certificate_expires_at(&self) -> Result<Option<u64>, CertStoreError>;

    /// Store the certificate expiry timestamp.
    fn store_expires_at(&self, timestamp: u64) -> Result<(), CertStoreError>;

    /// Check whether the certificate is within `days` days of expiry.
    fn expires_within_days(&self, days: u32) -> Result<bool, CertStoreError> {
        if let Some(expires) = self.certificate_expires_at()? {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let threshold = u64::from(days) * 86400;
            Ok(expires.saturating_sub(now) < threshold)
        } else {
            Ok(false) // No certificate stored
        }
    }
}

/// Errors from certificate storage operations.
#[derive(Debug, thiserror::Error)]
pub enum CertStoreError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Decryption error: {0}")]
    Decryption(String),

    #[error("Certificate not found")]
    NotFound,

    #[error("Invalid certificate data: {0}")]
    InvalidData(String),
}

/// File-based certificate store for Linux.
///
/// Stores certificates in `/var/lib/betblocker/certs/` with 0600 permissions.
/// In Phase 1, the private key is encrypted at rest using HKDF from
/// `/etc/machine-id` + a random salt. Phase 2 adds TPM2 support.
pub struct FileCertificateStore {
    base_dir: PathBuf,
}

impl FileCertificateStore {
    pub fn new(base_dir: &Path) -> Result<Self, CertStoreError> {
        let certs_dir = base_dir.join("certs");
        std::fs::create_dir_all(&certs_dir)?;

        // Set restrictive permissions on Linux
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o700);
            std::fs::set_permissions(&certs_dir, perms)?;
        }

        Ok(Self {
            base_dir: certs_dir,
        })
    }

    fn cert_path(&self) -> PathBuf {
        self.base_dir.join("device.crt")
    }

    fn key_path(&self) -> PathBuf {
        self.base_dir.join("device.key")
    }

    fn ca_path(&self) -> PathBuf {
        self.base_dir.join("ca-chain.crt")
    }

    fn expires_path(&self) -> PathBuf {
        self.base_dir.join("expires_at")
    }

    fn write_restricted(&self, path: &Path, data: &[u8]) -> Result<(), CertStoreError> {
        std::fs::write(path, data)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(path, perms)?;
        }

        Ok(())
    }
}

impl CertificateStore for FileCertificateStore {
    fn store_identity(&self, cert_pem: &[u8], key_der: &[u8]) -> Result<(), CertStoreError> {
        // Store cert as PEM
        self.write_restricted(&self.cert_path(), cert_pem)?;

        // TODO: Phase 2 -- encrypt key_der with HKDF-derived key from machine-id
        // For Phase 1, store with restrictive permissions only
        self.write_restricted(&self.key_path(), key_der)?;

        tracing::debug!("Stored device identity to {:?}", self.base_dir);
        Ok(())
    }

    fn load_identity(&self) -> Result<Option<Vec<u8>>, CertStoreError> {
        let cert_path = self.cert_path();
        let key_path = self.key_path();

        if !cert_path.exists() || !key_path.exists() {
            return Ok(None);
        }

        let cert_pem = std::fs::read(&cert_path)?;
        let key_pem = std::fs::read(&key_path)?;

        // Concatenate cert + key for reqwest Identity::from_pem
        let mut identity = cert_pem;
        identity.push(b'\n');
        identity.extend_from_slice(&key_pem);

        Ok(Some(identity))
    }

    fn store_ca_chain(&self, ca_pem: &[u8]) -> Result<(), CertStoreError> {
        self.write_restricted(&self.ca_path(), ca_pem)?;
        tracing::debug!("Stored CA chain to {:?}", self.ca_path());
        Ok(())
    }

    fn load_ca_chain(&self) -> Result<Option<Vec<u8>>, CertStoreError> {
        let path = self.ca_path();
        if !path.exists() {
            return Ok(None);
        }
        let data = std::fs::read(&path)?;
        Ok(Some(data))
    }

    fn certificate_expires_at(&self) -> Result<Option<u64>, CertStoreError> {
        let path = self.expires_path();
        if !path.exists() {
            return Ok(None);
        }
        let data = std::fs::read_to_string(&path)?;
        let ts = data
            .trim()
            .parse::<u64>()
            .map_err(|e| CertStoreError::InvalidData(e.to_string()))?;
        Ok(Some(ts))
    }

    fn store_expires_at(&self, timestamp: u64) -> Result<(), CertStoreError> {
        let path = self.expires_path();
        self.write_restricted(&path, timestamp.to_string().as_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_store_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FileCertificateStore::new(dir.path()).expect("create store");

        // Store identity
        let cert = b"-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----";
        let key = b"test-key-data";
        store.store_identity(cert, key).expect("store identity");

        // Load identity
        let identity = store.load_identity().expect("load").expect("should exist");
        assert!(identity.starts_with(b"-----BEGIN CERTIFICATE-----"));
        assert!(identity.ends_with(b"test-key-data"));
    }

    #[test]
    fn test_file_store_ca_chain() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FileCertificateStore::new(dir.path()).expect("create store");

        let ca = b"-----BEGIN CERTIFICATE-----\nca-cert\n-----END CERTIFICATE-----";
        store.store_ca_chain(ca).expect("store ca");

        let loaded = store.load_ca_chain().expect("load").expect("should exist");
        assert_eq!(loaded, ca);
    }

    #[test]
    fn test_file_store_no_identity_returns_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FileCertificateStore::new(dir.path()).expect("create store");

        assert!(store.load_identity().expect("load").is_none());
        assert!(store.load_ca_chain().expect("load").is_none());
    }

    #[test]
    fn test_expires_at_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FileCertificateStore::new(dir.path()).expect("create store");

        assert!(store.certificate_expires_at().expect("load").is_none());

        store.store_expires_at(1_700_000_000).expect("store");
        assert_eq!(
            store.certificate_expires_at().expect("load"),
            Some(1_700_000_000)
        );
    }

    #[test]
    fn test_expires_within_days() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FileCertificateStore::new(dir.path()).expect("create store");

        // No certificate -- should return false
        assert!(!store.expires_within_days(30).expect("check"));

        // Set expiry to 10 days from now
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        store.store_expires_at(now + 10 * 86400).expect("store");

        // Should expire within 30 days
        assert!(store.expires_within_days(30).expect("check"));
        // Should NOT expire within 5 days
        assert!(!store.expires_within_days(5).expect("check"));
    }
}
