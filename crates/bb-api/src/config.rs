use serde::Deserialize;

/// API server configuration, loaded from environment variables.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiConfig {
    /// PostgreSQL connection string
    pub database_url: String,

    /// Redis connection URL
    #[serde(default = "default_redis_url")]
    pub redis_url: String,

    /// Bind host
    #[serde(default = "default_host")]
    pub host: String,

    /// Bind port
    #[serde(default = "default_port")]
    pub port: u16,

    /// Path to Ed25519 private key PEM file for JWT signing
    pub jwt_private_key_path: String,

    /// Path to Ed25519 public key PEM file for JWT verification
    pub jwt_public_key_path: String,

    /// Access token TTL in seconds (default: 3600)
    #[serde(default = "default_access_token_ttl")]
    pub jwt_access_token_ttl_secs: i64,

    /// Refresh token TTL in days (default: 30)
    #[serde(default = "default_refresh_token_ttl")]
    pub jwt_refresh_token_ttl_days: i64,

    /// Allowed CORS origins
    #[serde(default = "default_cors_origins")]
    pub cors_allowed_origins: Vec<String>,

    /// Public base URL for generating external links (e.g., QR codes)
    pub public_base_url: Option<String>,

    /// Whether billing endpoints are enabled
    #[serde(default)]
    pub billing_enabled: bool,

    /// Stripe secret key (required if billing_enabled)
    pub stripe_secret_key: Option<String>,

    /// Stripe webhook secret (required if billing_enabled)
    pub stripe_webhook_secret: Option<String>,
}

impl ApiConfig {
    /// Load configuration from environment variables.
    ///
    /// Environment variables are prefixed with `BB_` and use uppercase
    /// with underscores, e.g., `BB_DATABASE_URL`, `BB_PORT`.
    pub fn from_env() -> anyhow::Result<Self> {
        let config = config::Config::builder()
            .add_source(
                config::Environment::with_prefix("BB")
                    .separator("__")
                    .try_parsing(true)
                    .list_separator(","),
            )
            .build()?;

        let api_config: Self = config.try_deserialize()?;
        Ok(api_config)
    }
}

fn default_redis_url() -> String {
    "redis://localhost:6379".to_string()
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_access_token_ttl() -> i64 {
    3600
}

fn default_refresh_token_ttl() -> i64 {
    30
}

fn default_cors_origins() -> Vec<String> {
    vec!["*".to_string()]
}
