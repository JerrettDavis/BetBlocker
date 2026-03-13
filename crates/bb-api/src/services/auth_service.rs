use std::sync::Arc;

use chrono::{Duration, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::ApiError;

// ---------------------------------------------------------------------------
// JWT Claims
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (account public_id)
    pub sub: Uuid,
    /// Email address
    pub email: String,
    /// Account role
    pub role: String,
    /// Issuer
    pub iss: String,
    /// Issued at (UNIX timestamp)
    pub iat: i64,
    /// Expiration (UNIX timestamp)
    pub exp: i64,
    /// JWT ID (unique per token)
    pub jti: Uuid,
}

// ---------------------------------------------------------------------------
// Password hashing
// ---------------------------------------------------------------------------

/// Hash a plaintext password with bcrypt (cost factor 12).
pub fn hash_password(plain: &str) -> Result<String, ApiError> {
    bcrypt::hash(plain, 12).map_err(|e| ApiError::Internal {
        message: format!("Failed to hash password: {e}"),
    })
}

/// Verify a plaintext password against a bcrypt hash.
pub fn verify_password(plain: &str, hash: &str) -> Result<bool, ApiError> {
    bcrypt::verify(plain, hash).map_err(|e| ApiError::Internal {
        message: format!("Failed to verify password: {e}"),
    })
}

// ---------------------------------------------------------------------------
// Password validation
// ---------------------------------------------------------------------------

/// Validate password meets strength requirements.
/// Returns a list of failed rules, or an empty vec if valid.
pub fn validate_password(password: &str) -> Vec<String> {
    let mut failures = Vec::new();

    if password.len() < 12 {
        failures.push("Password must be at least 12 characters".to_string());
    }
    if !password.chars().any(|c| c.is_uppercase()) {
        failures.push("Password must contain at least one uppercase letter".to_string());
    }
    if !password.chars().any(|c| c.is_lowercase()) {
        failures.push("Password must contain at least one lowercase letter".to_string());
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        failures.push("Password must contain at least one digit".to_string());
    }
    if !password.chars().any(|c| !c.is_alphanumeric()) {
        failures.push("Password must contain at least one special character".to_string());
    }

    failures
}

// ---------------------------------------------------------------------------
// JWT issuance
// ---------------------------------------------------------------------------

/// Issue an access token (JWT) for the given account.
/// Returns `(token_string, expires_in_seconds)`.
pub fn issue_access_token(
    account_public_id: Uuid,
    email: &str,
    role: &str,
    encoding_key: &Arc<EncodingKey>,
    ttl_secs: i64,
) -> Result<(String, i64), ApiError> {
    let now = Utc::now();
    let exp = now + Duration::seconds(ttl_secs);

    let claims = Claims {
        sub: account_public_id,
        email: email.to_string(),
        role: role.to_string(),
        iss: "betblocker-api".to_string(),
        iat: now.timestamp(),
        exp: exp.timestamp(),
        jti: Uuid::now_v7(),
    };

    let header = Header::new(Algorithm::EdDSA);
    let token = encode(&header, &claims, encoding_key).map_err(|e| ApiError::Internal {
        message: format!("Failed to encode JWT: {e}"),
    })?;

    Ok((token, ttl_secs))
}

// ---------------------------------------------------------------------------
// Refresh tokens
// ---------------------------------------------------------------------------

/// Generate a new refresh token: 256-bit random, hex-encoded, prefixed `rtk_`.
pub fn generate_refresh_token() -> String {
    use rand::Rng;
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    format!("rtk_{}", hex::encode(bytes))
}

/// Compute SHA-256 hash of a refresh token for storage.
pub fn hash_token(token: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hasher.finalize().to_vec()
}

/// Generate a password reset token: 256-bit random, hex-encoded, prefixed `rst_`.
pub fn generate_reset_token() -> String {
    use rand::Rng;
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    format!("rst_{}", hex::encode(bytes))
}

/// Generate a device token: 256-bit random, hex-encoded, prefixed `dtk_`.
pub fn generate_device_token() -> String {
    use rand::Rng;
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    format!("dtk_{}", hex::encode(bytes))
}

// ---------------------------------------------------------------------------
// Account lockout (Redis-backed)
// ---------------------------------------------------------------------------

/// Check if an account is locked out due to failed login attempts.
/// Returns Some(remaining_seconds) if locked, None if not locked.
pub async fn check_lockout(
    redis: &redis::Client,
    email: &str,
) -> Result<Option<u64>, ApiError> {
    let mut conn = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| ApiError::Internal {
            message: format!("Redis connection failed: {e}"),
        })?;

    let key = format!("lockout:{email}");
    let ttl: i64 = redis::cmd("TTL")
        .arg(&key)
        .query_async(&mut conn)
        .await
        .unwrap_or(-2);

    if ttl > 0 {
        Ok(Some(ttl as u64))
    } else {
        Ok(None)
    }
}

/// Increment failed login counter. After 5 consecutive failures, set 15-minute lockout.
pub async fn record_failed_login(
    redis: &redis::Client,
    email: &str,
) -> Result<(), ApiError> {
    let mut conn = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| ApiError::Internal {
            message: format!("Redis connection failed: {e}"),
        })?;

    let key = format!("login_failures:{email}");
    let count: i64 = redis::cmd("INCR")
        .arg(&key)
        .query_async(&mut conn)
        .await
        .unwrap_or(1);

    // Set TTL on first increment so it auto-expires
    if count == 1 {
        let _: () = redis::cmd("EXPIRE")
            .arg(&key)
            .arg(900i64) // 15 minutes
            .query_async(&mut conn)
            .await
            .unwrap_or(());
    }

    if count >= 5 {
        let lockout_key = format!("lockout:{email}");
        let _: () = redis::cmd("SET")
            .arg(&lockout_key)
            .arg("locked")
            .arg("EX")
            .arg(900i64)
            .query_async(&mut conn)
            .await
            .unwrap_or(());
    }

    Ok(())
}

/// Clear failed login counter on successful login.
pub async fn clear_login_failures(
    redis: &redis::Client,
    email: &str,
) -> Result<(), ApiError> {
    let mut conn = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| ApiError::Internal {
            message: format!("Redis connection failed: {e}"),
        })?;

    let _: () = redis::cmd("DEL")
        .arg(format!("login_failures:{email}"))
        .arg(format!("lockout:{email}"))
        .query_async(&mut conn)
        .await
        .unwrap_or(());

    Ok(())
}
