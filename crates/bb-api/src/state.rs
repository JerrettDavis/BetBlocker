use std::sync::Arc;

use jsonwebtoken::{DecodingKey, EncodingKey};
use sqlx::PgPool;

use crate::config::ApiConfig;

/// Shared application state, cloned into each Axum handler.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: redis::Client,
    pub jwt_encoding_key: Arc<EncodingKey>,
    pub jwt_decoding_key: Arc<DecodingKey>,
    pub config: Arc<ApiConfig>,
}

impl AppState {
    /// Build `AppState` from configuration.
    ///
    /// Connects to PostgreSQL and Redis, loads Ed25519 key pair from PEM files.
    pub async fn new(config: ApiConfig) -> anyhow::Result<Self> {
        let db = PgPool::connect(&config.database_url).await?;
        let redis = redis::Client::open(config.redis_url.as_str())?;

        // Load Ed25519 keys from PEM files
        let private_key_pem = std::fs::read(&config.jwt_private_key_path)?;
        let public_key_pem = std::fs::read(&config.jwt_public_key_path)?;
        let jwt_encoding_key = Arc::new(EncodingKey::from_ed_pem(&private_key_pem)?);
        let jwt_decoding_key = Arc::new(DecodingKey::from_ed_pem(&public_key_pem)?);

        Ok(Self {
            db,
            redis,
            jwt_encoding_key,
            jwt_decoding_key,
            config: Arc::new(config),
        })
    }
}
