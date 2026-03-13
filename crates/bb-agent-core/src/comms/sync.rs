use std::sync::Arc;

use ring::signature;
use sha2::{Digest, Sha256};

use crate::comms::client::{ApiClient, ApiClientError};

/// Synchronizes the local blocklist with the API using version-based delta sync.
///
/// Delta sync sends the current version and receives only changes.
/// Full sync is used as a fallback when the delta cannot be applied.
/// All payloads are signature-verified before application.
pub struct BlocklistSyncer {
    api_client: Arc<ApiClient>,
    device_id: String,
    /// Current local blocklist version (0 = never synced).
    current_version: u64,
    /// Server's Ed25519 signing public key for blocklist verification.
    signing_public_key: Vec<u8>,
}

/// Result of a blocklist sync operation.
#[derive(Debug)]
pub struct SyncResult {
    pub previous_version: u64,
    pub new_version: u64,
    pub additions: Vec<BlocklistAddition>,
    pub removals: Vec<String>,
    pub was_full_sync: bool,
}

/// A single domain addition from a sync delta.
#[derive(Debug, Clone)]
pub struct BlocklistAddition {
    pub domain: String,
    pub category: String,
    pub confidence: f32,
}

/// Errors from blocklist sync operations.
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("API error: {0}")]
    ApiError(#[from] ApiClientError),

    #[error("Signature verification failed")]
    SignatureVerificationFailed,

    #[error("Decompression failed: {0}")]
    DecompressionFailed(String),

    #[error("Delta application failed: {0}")]
    DeltaApplicationFailed(String),

    #[error("No signing key configured")]
    NoSigningKey,
}

impl BlocklistSyncer {
    pub fn new(
        api_client: Arc<ApiClient>,
        device_id: String,
        signing_public_key: Vec<u8>,
    ) -> Self {
        Self {
            api_client,
            device_id,
            current_version: 0,
            signing_public_key,
        }
    }

    /// Set the current local blocklist version (loaded from disk).
    pub fn set_current_version(&mut self, version: u64) {
        self.current_version = version;
    }

    /// Get the current blocklist version.
    pub fn current_version(&self) -> u64 {
        self.current_version
    }

    /// Perform a delta sync from the current version.
    ///
    /// If the server indicates a full sync is required (version gap too large
    /// or corruption), falls back to full sync automatically.
    pub async fn sync(&mut self) -> Result<SyncResult, SyncError> {
        let request = bb_proto::blocklist::BlocklistDeltaRequest {
            current_version: self.current_version,
        };

        let path = format!("/api/v1/devices/{}/blocklist/sync", self.device_id);
        let response: bb_proto::blocklist::BlocklistDeltaResponse =
            self.api_client.post_proto(&path, &request).await?;

        if response.full_sync_required {
            tracing::info!(
                from = self.current_version,
                to = response.to_version,
                "Full sync required"
            );
            return self.full_sync().await;
        }

        // Verify signature over SHA-256(to_version || additions + removals)
        self.verify_signature(&response)?;

        let additions: Vec<BlocklistAddition> = response
            .additions
            .iter()
            .map(|a| BlocklistAddition {
                domain: a.domain.clone(),
                category: a.category.clone(),
                confidence: a.confidence,
            })
            .collect();

        let removals: Vec<String> = response.removals.clone();

        let previous_version = self.current_version;
        self.current_version = response.to_version;

        tracing::info!(
            from = previous_version,
            to = self.current_version,
            added = additions.len(),
            removed = removals.len(),
            "Delta sync complete"
        );

        Ok(SyncResult {
            previous_version,
            new_version: self.current_version,
            additions,
            removals,
            was_full_sync: false,
        })
    }

    /// Perform a full sync (version=0), replacing the entire local blocklist.
    async fn full_sync(&mut self) -> Result<SyncResult, SyncError> {
        let request = bb_proto::blocklist::BlocklistDeltaRequest {
            current_version: 0,
        };

        let path = format!("/api/v1/devices/{}/blocklist/sync", self.device_id);
        let response: bb_proto::blocklist::BlocklistDeltaResponse =
            self.api_client.post_proto(&path, &request).await?;

        self.verify_signature(&response)?;

        let additions: Vec<BlocklistAddition> = response
            .additions
            .iter()
            .map(|a| BlocklistAddition {
                domain: a.domain.clone(),
                category: a.category.clone(),
                confidence: a.confidence,
            })
            .collect();

        let previous_version = self.current_version;
        self.current_version = response.to_version;

        tracing::info!(
            to = self.current_version,
            entries = additions.len(),
            "Full sync complete"
        );

        Ok(SyncResult {
            previous_version,
            new_version: self.current_version,
            additions,
            removals: Vec::new(), // Full sync replaces everything
            was_full_sync: true,
        })
    }

    /// Verify the Ed25519 signature over the sync response.
    ///
    /// The signature covers: SHA-256(to_version as little-endian bytes || all domain additions || all removals).
    fn verify_signature(
        &self,
        response: &bb_proto::blocklist::BlocklistDeltaResponse,
    ) -> Result<(), SyncError> {
        if self.signing_public_key.is_empty() {
            return Err(SyncError::NoSigningKey);
        }

        if response.signature.is_empty() {
            return Err(SyncError::SignatureVerificationFailed);
        }

        // Build the signed message: to_version || domain data
        let mut hasher = Sha256::new();
        hasher.update(response.to_version.to_le_bytes());
        for addition in &response.additions {
            hasher.update(addition.domain.as_bytes());
            hasher.update(addition.category.as_bytes());
            hasher.update(addition.confidence.to_le_bytes());
        }
        for removal in &response.removals {
            hasher.update(removal.as_bytes());
        }
        let message_hash = hasher.finalize();

        let public_key = signature::UnparsedPublicKey::new(
            &signature::ED25519,
            &self.signing_public_key,
        );

        public_key
            .verify(&message_hash, &response.signature)
            .map_err(|_| SyncError::SignatureVerificationFailed)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::rand::SystemRandom;
    use ring::signature::{Ed25519KeyPair, KeyPair};

    fn generate_test_keypair() -> (Ed25519KeyPair, Vec<u8>) {
        let rng = SystemRandom::new();
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).expect("keygen");
        let kp = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).expect("parse");
        let public = kp.public_key().as_ref().to_vec();
        (kp, public)
    }

    #[test]
    fn test_signature_verification_valid() {
        let (keypair, public_key) = generate_test_keypair();

        let client = Arc::new(ApiClient::new_insecure("http://localhost:1".to_string()));
        let syncer = BlocklistSyncer::new(client, "test".to_string(), public_key);

        // Build a response and sign it
        let response = bb_proto::blocklist::BlocklistDeltaResponse {
            from_version: 0,
            to_version: 1,
            full_sync_required: false,
            additions: vec![bb_proto::blocklist::BlocklistAddition {
                domain: "bet365.com".to_string(),
                category: "sports_betting".to_string(),
                confidence: 0.99,
            }],
            removals: vec![],
            signature: Vec::new(), // Will be computed below
        };

        // Compute the hash the same way the syncer does
        let mut hasher = Sha256::new();
        hasher.update(response.to_version.to_le_bytes());
        for a in &response.additions {
            hasher.update(a.domain.as_bytes());
            hasher.update(a.category.as_bytes());
            hasher.update(a.confidence.to_le_bytes());
        }
        let message_hash = hasher.finalize();

        let sig = keypair.sign(&message_hash);

        let signed_response = bb_proto::blocklist::BlocklistDeltaResponse {
            signature: sig.as_ref().to_vec(),
            ..response
        };

        assert!(syncer.verify_signature(&signed_response).is_ok());
    }

    #[test]
    fn test_signature_verification_invalid() {
        let (_, public_key) = generate_test_keypair();

        let client = Arc::new(ApiClient::new_insecure("http://localhost:1".to_string()));
        let syncer = BlocklistSyncer::new(client, "test".to_string(), public_key);

        let response = bb_proto::blocklist::BlocklistDeltaResponse {
            from_version: 0,
            to_version: 1,
            full_sync_required: false,
            additions: vec![],
            removals: vec![],
            signature: vec![0u8; 64], // Invalid signature
        };

        assert!(syncer.verify_signature(&response).is_err());
    }

    #[test]
    fn test_signature_verification_empty_key() {
        let client = Arc::new(ApiClient::new_insecure("http://localhost:1".to_string()));
        let syncer = BlocklistSyncer::new(client, "test".to_string(), Vec::new());

        let response = bb_proto::blocklist::BlocklistDeltaResponse::default();
        assert!(matches!(
            syncer.verify_signature(&response),
            Err(SyncError::NoSigningKey)
        ));
    }

    #[test]
    fn test_version_tracking() {
        let client = Arc::new(ApiClient::new_insecure("http://localhost:1".to_string()));
        let mut syncer = BlocklistSyncer::new(client, "test".to_string(), vec![1, 2, 3]);

        assert_eq!(syncer.current_version(), 0);
        syncer.set_current_version(42);
        assert_eq!(syncer.current_version(), 42);
    }
}
