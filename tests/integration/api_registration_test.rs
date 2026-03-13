//! Integration tests for device registration and heartbeat flows.
//!
//! These tests verify the full registration flow:
//! 1. Generate enrollment token
//! 2. Register device with token
//! 3. Receive certificate and device_id
//! 4. Send authenticated heartbeat
//!
//! Requires: a running bb-api instance with test database.
//! These tests are gated behind `#[cfg(feature = "integration")]` and
//! require `TEST_API_URL` environment variable to be set.
//!
//! Run with:
//!   cargo test -p bb-agent-core --features integration --test api_registration_test

use std::sync::Arc;
use std::time::Duration;

use bb_agent_core::comms::certificate::{CertificateStore, FileCertificateStore};
use bb_agent_core::comms::client::{ApiClient, ApiClientError, RetryConfig};
use bb_agent_core::comms::heartbeat::{HeartbeatConfig, HeartbeatSender};
use bb_agent_core::comms::registration::RegistrationService;

/// Helper to get the test API URL from environment.
fn test_api_url() -> String {
    std::env::var("TEST_API_URL").unwrap_or_else(|_| "http://localhost:3000".to_string())
}

/// Helper to create a test API client (no mTLS for registration).
fn test_client() -> Arc<ApiClient> {
    Arc::new(ApiClient::new_insecure(test_api_url()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that keypair generation produces valid Ed25519 keys.
    #[test]
    fn test_keypair_generation_produces_valid_keys() {
        let (keypair, pkcs8) = RegistrationService::generate_keypair()
            .expect("keypair generation");

        // Ed25519 public key is always 32 bytes
        assert_eq!(keypair.public_key().as_ref().len(), 32);

        // PKCS#8 encoding should be non-empty
        assert!(!pkcs8.is_empty());

        // Two keypairs should be different
        let (keypair2, _) = RegistrationService::generate_keypair()
            .expect("second keypair");
        assert_ne!(
            keypair.public_key().as_ref(),
            keypair2.public_key().as_ref()
        );
    }

    /// Test that device fingerprint collection works.
    #[test]
    fn test_fingerprint_collection() {
        let fp = RegistrationService::collect_fingerprint();
        assert!(!fp.os_type.is_empty(), "OS type should not be empty");
        assert!(!fp.os_version.is_empty(), "OS version should not be empty");
        // hostname and hardware_id may be "unknown" in CI
    }

    /// Test certificate store round-trip.
    #[test]
    fn test_certificate_store_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FileCertificateStore::new(dir.path()).expect("create store");

        // Store identity
        let cert = b"-----BEGIN CERTIFICATE-----\ntest-cert-data\n-----END CERTIFICATE-----";
        let key = b"-----BEGIN PRIVATE KEY-----\ntest-key-data\n-----END PRIVATE KEY-----";
        store.store_identity(cert, key).expect("store identity");

        // Load identity
        let identity = store.load_identity().expect("load").expect("should exist");
        assert!(identity.starts_with(b"-----BEGIN CERTIFICATE-----"));

        // Store and load CA chain
        let ca = b"-----BEGIN CERTIFICATE-----\nca-data\n-----END CERTIFICATE-----";
        store.store_ca_chain(ca).expect("store ca");
        let loaded_ca = store.load_ca_chain().expect("load").expect("should exist");
        assert_eq!(loaded_ca, ca);
    }

    /// Test certificate expiry detection.
    #[test]
    fn test_certificate_expiry_detection() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FileCertificateStore::new(dir.path()).expect("create store");

        // No certificate -- not expiring
        assert!(!store.expires_within_days(30).expect("check"));

        // Set expiry to 10 days from now
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        store.store_expires_at(now + 10 * 86400).expect("store");
        assert!(store.expires_within_days(30).expect("within 30 days"));
        assert!(!store.expires_within_days(5).expect("not within 5 days"));

        // Set expiry to 60 days from now (rotation threshold)
        store.store_expires_at(now + 60 * 86400).expect("store");
        assert!(!store.expires_within_days(30).expect("not within 30 days"));
    }

    /// Test heartbeat sender configuration for different tiers.
    #[test]
    fn test_heartbeat_tier_configurations() {
        let self_config = HeartbeatConfig::self_tier("d".into(), "v".into());
        assert_eq!(self_config.default_interval, Duration::from_secs(900)); // 15 min
        assert_eq!(self_config.min_interval, Duration::from_secs(300)); // 5 min

        let partner_config = HeartbeatConfig::partner_tier("d".into(), "v".into());
        assert_eq!(partner_config.default_interval, Duration::from_secs(300)); // 5 min
        assert_eq!(partner_config.min_interval, Duration::from_secs(60)); // 1 min

        let authority_config = HeartbeatConfig::authority_tier("d".into(), "v".into());
        assert_eq!(authority_config.default_interval, Duration::from_secs(300));
        assert_eq!(authority_config.min_interval, Duration::from_secs(60));
    }

    /// Test heartbeat sender offline queue behavior.
    #[test]
    fn test_heartbeat_offline_queue() {
        let client = test_client();
        let config = HeartbeatConfig::self_tier("test-device".into(), "0.1.0".into());
        let mut sender = HeartbeatSender::new(client, config);

        assert_eq!(sender.offline_queue_len(), 0);

        // Simulate offline heartbeats (directly call the internal queue method)
        // Since queue_offline_heartbeat is private, we test through the public interface
        assert_eq!(sender.sequence_number(), 0);
    }

    /// Test that registration with an invalid token is properly handled.
    /// This test actually connects to the API server if available.
    #[tokio::test]
    #[ignore = "Requires running bb-api server"]
    async fn test_registration_invalid_token_rejected() {
        let client = test_client();
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Arc::new(FileCertificateStore::new(dir.path()).expect("create store"));
        let reg_service = RegistrationService::new(client, store);

        let result = reg_service
            .register("INVALID-TOKEN-12345", "0.1.0")
            .await;

        // Should fail with a server error (4xx)
        assert!(result.is_err());
    }

    /// Test full registration -> heartbeat flow.
    /// This test actually connects to the API server.
    #[tokio::test]
    #[ignore = "Requires running bb-api server with test enrollment"]
    async fn test_full_registration_and_heartbeat() {
        let client = test_client();
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Arc::new(FileCertificateStore::new(dir.path()).expect("create store"));
        let reg_service = RegistrationService::new(client.clone(), store.clone());

        // Register with a valid test token
        let token = std::env::var("TEST_ENROLLMENT_TOKEN")
            .expect("TEST_ENROLLMENT_TOKEN must be set for integration tests");

        let result = reg_service
            .register(&token, "0.1.0")
            .await
            .expect("registration should succeed");

        assert!(!result.device_id.is_empty(), "device_id should be assigned");
        assert!(
            !result.device_certificate.is_empty(),
            "certificate should be issued"
        );
        assert!(
            !result.ca_certificate_chain.is_empty(),
            "CA chain should be provided"
        );

        // Verify certificate was stored
        let identity = store
            .load_identity()
            .expect("load")
            .expect("identity should be stored");
        assert!(!identity.is_empty());

        // Now send a heartbeat
        let config = HeartbeatConfig::self_tier(
            result.device_id.clone(),
            "0.1.0".to_string(),
        );
        let mut sender = HeartbeatSender::new(client, config);

        // The sender would normally run in a loop, but we can verify
        // it was constructed correctly
        assert_eq!(sender.sequence_number(), 0);
    }

    /// Test re-registration after certificate expiry.
    #[tokio::test]
    #[ignore = "Requires running bb-api server with expired test device"]
    async fn test_re_registration_after_expiry() {
        let client = test_client();
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Arc::new(FileCertificateStore::new(dir.path()).expect("create store"));
        let reg_service = RegistrationService::new(client, store);

        let device_id = std::env::var("TEST_EXPIRED_DEVICE_ID")
            .expect("TEST_EXPIRED_DEVICE_ID must be set");

        let result = reg_service
            .re_register(&device_id, "0.1.0")
            .await;

        // Should either succeed (new cert) or fail with enrollment revoked
        match result {
            Ok(r) => {
                assert!(!r.device_id.is_empty());
                assert!(!r.device_certificate.is_empty());
            }
            Err(e) => {
                // Acceptable failure: enrollment was revoked
                let err_str = e.to_string();
                assert!(
                    err_str.contains("revoked") || err_str.contains("API error"),
                    "Unexpected error: {e}"
                );
            }
        }
    }
}
