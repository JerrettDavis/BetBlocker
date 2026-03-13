use std::sync::Arc;

use ring::rand::SystemRandom;
use ring::signature::{Ed25519KeyPair, KeyPair};

use crate::comms::certificate::CertificateStore;
use crate::comms::client::{ApiClient, ApiClientError};

/// Handles device registration and re-registration flows.
pub struct RegistrationService {
    api_client: Arc<ApiClient>,
    cert_store: Arc<dyn CertificateStore>,
}

/// Device fingerprint collected during registration.
#[derive(Debug, Clone)]
pub struct DeviceFingerprint {
    pub os_type: String,
    pub os_version: String,
    pub hardware_id: String,
    pub hostname: String,
}

/// Registration result containing all data needed to establish identity.
#[derive(Debug, Clone)]
pub struct RegistrationResult {
    pub device_id: String,
    pub device_certificate: Vec<u8>,
    pub ca_certificate_chain: Vec<u8>,
    pub initial_blocklist_url: String,
    pub initial_blocklist_version: u64,
    pub initial_blocklist_signature: Vec<u8>,
    pub certificate_expires_at: u64,
}

/// Errors specific to registration.
#[derive(Debug, thiserror::Error)]
pub enum RegistrationError {
    #[error("Key generation failed: {0}")]
    KeyGenerationFailed(String),

    #[error("API error: {0}")]
    ApiError(#[from] ApiClientError),

    #[error("Certificate storage failed: {0}")]
    CertStorageError(String),

    #[error("Enrollment revoked: {0}")]
    EnrollmentRevoked(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Fingerprint collection failed: {0}")]
    FingerprintError(String),
}

impl RegistrationService {
    pub fn new(api_client: Arc<ApiClient>, cert_store: Arc<dyn CertificateStore>) -> Self {
        Self {
            api_client,
            cert_store,
        }
    }

    /// Generate an Ed25519 keypair for device identity.
    pub fn generate_keypair() -> Result<(Ed25519KeyPair, Vec<u8>), RegistrationError> {
        let rng = SystemRandom::new();
        let pkcs8_bytes = Ed25519KeyPair::generate_pkcs8(&rng)
            .map_err(|e| RegistrationError::KeyGenerationFailed(e.to_string()))?;

        let keypair = Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref())
            .map_err(|e| RegistrationError::KeyGenerationFailed(e.to_string()))?;

        Ok((keypair, pkcs8_bytes.as_ref().to_vec()))
    }

    /// Collect device fingerprint from the current system.
    pub fn collect_fingerprint() -> DeviceFingerprint {
        let os_type = if cfg!(target_os = "linux") {
            "linux"
        } else if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else {
            "unknown"
        };

        let os_version = std::env::consts::OS.to_string();

        // On Linux, read /etc/machine-id for a stable hardware ID.
        // On other platforms, fall back to a placeholder.
        let hardware_id = Self::read_machine_id().unwrap_or_else(|| "unknown".to_string());

        let hostname = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown".to_string());

        DeviceFingerprint {
            os_type: os_type.to_string(),
            os_version,
            hardware_id,
            hostname,
        }
    }

    fn read_machine_id() -> Option<String> {
        std::fs::read_to_string("/etc/machine-id")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    /// Register a new device using an enrollment token.
    ///
    /// Generates a keypair, collects fingerprint, sends registration request,
    /// stores the returned certificate and CA chain.
    pub async fn register(
        &self,
        enrollment_token: &str,
        agent_version: &str,
    ) -> Result<RegistrationResult, RegistrationError> {
        let (keypair, _pkcs8_bytes) = Self::generate_keypair()?;
        let public_key = keypair.public_key().as_ref().to_vec();
        let fingerprint = Self::collect_fingerprint();

        let request = bb_proto::device::DeviceRegistrationRequest {
            enrollment_token: enrollment_token.to_string(),
            public_key,
            fingerprint: Some(bb_proto::device::DeviceFingerprint {
                os_type: fingerprint.os_type,
                os_version: fingerprint.os_version,
                hardware_id: fingerprint.hardware_id,
                hostname: fingerprint.hostname,
            }),
            agent_version: agent_version.to_string(),
        };

        let response: bb_proto::device::DeviceRegistrationResponse = self
            .api_client
            .post_proto("/api/v1/devices/register", &request)
            .await?;

        if response.device_id.is_empty() {
            return Err(RegistrationError::InvalidResponse(
                "Empty device_id in response".to_string(),
            ));
        }

        // Store certificates
        self.cert_store
            .store_identity(&response.device_certificate, &_pkcs8_bytes)
            .map_err(|e| RegistrationError::CertStorageError(e.to_string()))?;

        self.cert_store
            .store_ca_chain(&response.ca_certificate_chain)
            .map_err(|e| RegistrationError::CertStorageError(e.to_string()))?;

        // Set device ID on the client
        self.api_client
            .set_device_id(response.device_id.clone())
            .await;

        tracing::info!(device_id = %response.device_id, "Device registered successfully");

        Ok(RegistrationResult {
            device_id: response.device_id,
            device_certificate: response.device_certificate,
            ca_certificate_chain: response.ca_certificate_chain,
            initial_blocklist_url: response.initial_blocklist_url,
            initial_blocklist_version: response.initial_blocklist_version,
            initial_blocklist_signature: response.initial_blocklist_signature,
            certificate_expires_at: response.certificate_expires_at,
        })
    }

    /// Re-register a device whose certificate has expired (offline > 90 days).
    ///
    /// Uses the device_id and hardware_id to prove identity without a valid certificate.
    /// On 410 Gone, the enrollment has been revoked.
    pub async fn re_register(
        &self,
        device_id: &str,
        agent_version: &str,
    ) -> Result<RegistrationResult, RegistrationError> {
        let (keypair, pkcs8_bytes) = Self::generate_keypair()?;
        let public_key = keypair.public_key().as_ref().to_vec();
        let fingerprint = Self::collect_fingerprint();

        let request = bb_proto::device::DeviceRegistrationRequest {
            enrollment_token: String::new(), // Not needed for re-registration
            public_key,
            fingerprint: Some(bb_proto::device::DeviceFingerprint {
                os_type: fingerprint.os_type,
                os_version: fingerprint.os_version,
                hardware_id: fingerprint.hardware_id,
                hostname: fingerprint.hostname,
            }),
            agent_version: agent_version.to_string(),
        };

        let path = format!("/api/v1/devices/{device_id}/re-register");
        let response: bb_proto::device::DeviceRegistrationResponse = self
            .api_client
            .post_proto(&path, &request)
            .await
            .map_err(|e| {
                // Check for 410 Gone (enrollment revoked)
                if let ApiClientError::ServerError { status, body } = &e {
                    if status.as_u16() == 410 {
                        return RegistrationError::EnrollmentRevoked(body.clone());
                    }
                }
                RegistrationError::ApiError(e)
            })?;

        // Store new certificates
        self.cert_store
            .store_identity(&response.device_certificate, &pkcs8_bytes)
            .map_err(|e| RegistrationError::CertStorageError(e.to_string()))?;

        self.cert_store
            .store_ca_chain(&response.ca_certificate_chain)
            .map_err(|e| RegistrationError::CertStorageError(e.to_string()))?;

        self.api_client
            .set_device_id(response.device_id.clone())
            .await;

        tracing::info!(device_id = %response.device_id, "Device re-registered successfully");

        Ok(RegistrationResult {
            device_id: response.device_id,
            device_certificate: response.device_certificate,
            ca_certificate_chain: response.ca_certificate_chain,
            initial_blocklist_url: response.initial_blocklist_url,
            initial_blocklist_version: response.initial_blocklist_version,
            initial_blocklist_signature: response.initial_blocklist_signature,
            certificate_expires_at: response.certificate_expires_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_keypair() {
        let (keypair, pkcs8) = RegistrationService::generate_keypair()
            .expect("keypair generation should succeed");
        assert_eq!(keypair.public_key().as_ref().len(), 32);
        assert!(!pkcs8.is_empty());
    }

    #[test]
    fn test_generate_keypair_unique() {
        let (kp1, _) =
            RegistrationService::generate_keypair().expect("keypair");
        let (kp2, _) =
            RegistrationService::generate_keypair().expect("keypair");
        assert_ne!(kp1.public_key().as_ref(), kp2.public_key().as_ref());
    }

    #[test]
    fn test_collect_fingerprint() {
        let fp = RegistrationService::collect_fingerprint();
        assert!(!fp.os_type.is_empty());
        // hostname may be "unknown" in CI but should not panic
    }

    #[test]
    fn test_device_fingerprint_clone() {
        let fp = DeviceFingerprint {
            os_type: "linux".to_string(),
            os_version: "6.1".to_string(),
            hardware_id: "abc123".to_string(),
            hostname: "testhost".to_string(),
        };
        let fp2 = fp.clone();
        assert_eq!(fp.os_type, fp2.os_type);
    }
}
