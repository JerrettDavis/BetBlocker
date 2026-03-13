use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::ApiError;
use crate::extractors::AuthenticatedAccount;
use crate::response::ApiResponse;
use crate::services::{
    account_service,
    auth_service::{
        clear_login_failures, check_lockout, generate_refresh_token,
        generate_reset_token, hash_password, hash_token, issue_access_token,
        record_failed_login, validate_password, verify_password,
    },
};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub display_name: String,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default = "default_locale")]
    pub locale: String,
}

fn default_timezone() -> String {
    "UTC".to_string()
}
fn default_locale() -> String {
    "en-US".to_string()
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub account: AccountSummary,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

#[derive(Debug, Serialize)]
pub struct AccountSummary {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub email_verified: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}

// ---------------------------------------------------------------------------
// POST /auth/register
// ---------------------------------------------------------------------------

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<ApiResponse<AuthResponse>>), ApiError> {
    // Validate email format (basic check)
    if !req.email.contains('@') || req.email.len() > 255 {
        return Err(ApiError::Validation {
            message: "Invalid email address".into(),
            details: None,
        });
    }

    // Validate display_name
    if req.display_name.len() < 2 || req.display_name.len() > 100 {
        return Err(ApiError::Validation {
            message: "Display name must be 2-100 characters".into(),
            details: None,
        });
    }

    // Validate password strength
    let pw_errors = validate_password(&req.password);
    if !pw_errors.is_empty() {
        return Err(ApiError::Validation {
            message: "Password does not meet requirements".into(),
            details: Some(json!({ "rules": pw_errors })),
        });
    }

    // Hash password
    let hashed = hash_password(&req.password)?;

    // Insert account
    let account = account_service::create_account(
        &state.db,
        &req.email,
        &hashed,
        &req.display_name,
        &req.timezone,
        &req.locale,
    )
    .await
    .map_err(|e| {
        // Map unique constraint to EMAIL_ALREADY_EXISTS
        if let ApiError::Conflict { .. } = &e {
            ApiError::Conflict {
                code: "EMAIL_ALREADY_EXISTS".into(),
                message: "An account with this email already exists".into(),
            }
        } else {
            e
        }
    })?;

    // Issue tokens
    let role_str = &account.role;
    let (access_token, expires_in) = issue_access_token(
        account.public_id,
        &account.email,
        role_str,
        &state.jwt_encoding_key,
        state.config.jwt_access_token_ttl_secs,
    )?;

    let refresh_token = generate_refresh_token();
    let token_hash = hash_token(&refresh_token);

    // Store refresh token
    let expires_at =
        chrono::Utc::now() + chrono::Duration::days(state.config.jwt_refresh_token_ttl_days);
    sqlx::query(
        "INSERT INTO refresh_tokens (account_id, token_hash, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(account.id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(&state.db)
    .await?;

    // Log email verification token (Phase 1)
    tracing::info!(
        account_id = %account.public_id,
        "Email verification token would be sent to {}",
        account.email
    );

    let response = AuthResponse {
        account: AccountSummary {
            id: account.public_id.to_string(),
            email: account.email,
            display_name: account.display_name,
            role: account.role.clone(),
            email_verified: account.email_verified,
            created_at: account.created_at.to_rfc3339(),
        },
        access_token,
        refresh_token,
        expires_in,
    };

    Ok(ApiResponse::created(response))
}

// ---------------------------------------------------------------------------
// POST /auth/login
// ---------------------------------------------------------------------------

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<(StatusCode, Json<ApiResponse<AuthResponse>>), ApiError> {
    // Check lockout
    if let Some(_remaining) = check_lockout(&state.redis, &req.email).await? {
        return Err(ApiError::Forbidden {
            message: "Account temporarily locked due to too many failed login attempts".into(),
        });
    }

    // Look up account
    let account = account_service::get_account_by_email(&state.db, &req.email).await?;

    let account = match account {
        Some(a) => a,
        None => {
            // Don't reveal whether email exists
            record_failed_login(&state.redis, &req.email).await?;
            return Err(ApiError::Unauthorized {
                code: "INVALID_CREDENTIALS".into(),
                message: "Invalid email or password".into(),
            });
        }
    };

    // Verify password
    let valid = verify_password(&req.password, &account.password_hash)?;
    if !valid {
        record_failed_login(&state.redis, &req.email).await?;
        return Err(ApiError::Unauthorized {
            code: "INVALID_CREDENTIALS".into(),
            message: "Invalid email or password".into(),
        });
    }

    // Clear failure counter
    clear_login_failures(&state.redis, &req.email).await?;

    // Issue tokens
    let (access_token, expires_in) = issue_access_token(
        account.public_id,
        &account.email,
        &account.role,
        &state.jwt_encoding_key,
        state.config.jwt_access_token_ttl_secs,
    )?;

    let refresh_token = generate_refresh_token();
    let token_hash = hash_token(&refresh_token);

    let expires_at =
        chrono::Utc::now() + chrono::Duration::days(state.config.jwt_refresh_token_ttl_days);
    sqlx::query(
        "INSERT INTO refresh_tokens (account_id, token_hash, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(account.id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(&state.db)
    .await?;

    let response = AuthResponse {
        account: AccountSummary {
            id: account.public_id.to_string(),
            email: account.email,
            display_name: account.display_name,
            role: account.role.clone(),
            email_verified: account.email_verified,
            created_at: account.created_at.to_rfc3339(),
        },
        access_token,
        refresh_token,
        expires_in,
    };

    Ok(ApiResponse::ok(response))
}

// ---------------------------------------------------------------------------
// POST /auth/refresh
// ---------------------------------------------------------------------------

pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<(StatusCode, Json<ApiResponse<TokenResponse>>), ApiError> {
    let token_hash = hash_token(&req.refresh_token);

    // Look up refresh token
    let row = sqlx::query_as::<_, RefreshTokenRow>(
        r#"SELECT id, account_id, expires_at, revoked_at
           FROM refresh_tokens
           WHERE token_hash = $1"#,
    )
    .bind(&token_hash)
    .fetch_optional(&state.db)
    .await?;

    let row = match row {
        Some(r) => r,
        None => {
            return Err(ApiError::Unauthorized {
                code: "INVALID_REFRESH_TOKEN".into(),
                message: "Invalid refresh token".into(),
            });
        }
    };

    // Check if already revoked (reuse detection)
    if row.revoked_at.is_some() {
        // Token reuse detected -- revoke all tokens for this account (family revocation)
        sqlx::query(
            "UPDATE refresh_tokens SET revoked_at = NOW() WHERE account_id = $1 AND revoked_at IS NULL",
        )
        .bind(row.account_id)
        .execute(&state.db)
        .await?;

        return Err(ApiError::Unauthorized {
            code: "TOKEN_FAMILY_REVOKED".into(),
            message: "Refresh token reuse detected; all sessions have been revoked".into(),
        });
    }

    // Check expiration
    if row.expires_at < chrono::Utc::now() {
        return Err(ApiError::Unauthorized {
            code: "INVALID_REFRESH_TOKEN".into(),
            message: "Refresh token has expired".into(),
        });
    }

    // Revoke the old token (rotation)
    sqlx::query("UPDATE refresh_tokens SET revoked_at = NOW() WHERE id = $1")
        .bind(row.id)
        .execute(&state.db)
        .await?;

    // Get account for new token claims
    let account = account_service::get_account_by_id(&state.db, row.account_id).await?;

    // Issue new access token
    let (access_token, expires_in) = issue_access_token(
        account.public_id,
        &account.email,
        &account.role,
        &state.jwt_encoding_key,
        state.config.jwt_access_token_ttl_secs,
    )?;

    // Issue new refresh token
    let new_refresh_token = generate_refresh_token();
    let new_token_hash = hash_token(&new_refresh_token);
    let expires_at =
        chrono::Utc::now() + chrono::Duration::days(state.config.jwt_refresh_token_ttl_days);

    sqlx::query(
        "INSERT INTO refresh_tokens (account_id, token_hash, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(account.id)
    .bind(&new_token_hash)
    .bind(expires_at)
    .execute(&state.db)
    .await?;

    Ok(ApiResponse::ok(TokenResponse {
        access_token,
        refresh_token: new_refresh_token,
        expires_in,
    }))
}

#[derive(sqlx::FromRow)]
struct RefreshTokenRow {
    id: i64,
    account_id: i64,
    expires_at: chrono::DateTime<chrono::Utc>,
    revoked_at: Option<chrono::DateTime<chrono::Utc>>,
}

// ---------------------------------------------------------------------------
// POST /auth/logout
// ---------------------------------------------------------------------------

pub async fn logout(
    State(state): State<AppState>,
    _auth: AuthenticatedAccount,
    Json(req): Json<LogoutRequest>,
) -> Result<StatusCode, ApiError> {
    let token_hash = hash_token(&req.refresh_token);

    sqlx::query("UPDATE refresh_tokens SET revoked_at = NOW() WHERE token_hash = $1")
        .bind(&token_hash)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// POST /auth/forgot-password
// ---------------------------------------------------------------------------

pub async fn forgot_password(
    State(state): State<AppState>,
    Json(req): Json<ForgotPasswordRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    // Always return 202 to prevent enumeration
    let account = account_service::get_account_by_email(&state.db, &req.email).await?;

    if let Some(account) = account {
        let reset_token = generate_reset_token();
        let token_hash = hash_token(&reset_token);
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);

        // Store reset token hash (reuse email_verification_token column for Phase 1)
        sqlx::query(
            "UPDATE accounts SET email_verification_token = $2, updated_at = NOW() WHERE id = $1",
        )
        .bind(account.id)
        .bind(hex::encode(&token_hash))
        .execute(&state.db)
        .await?;

        // Store expiry in Redis
        let mut conn = state
            .redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| ApiError::Internal {
                message: format!("Redis error: {e}"),
            })?;
        let _: () = redis::cmd("SET")
            .arg(format!("reset_token:{}", hex::encode(&token_hash)))
            .arg(account.id.to_string())
            .arg("EX")
            .arg(3600i64)
            .query_async(&mut conn)
            .await
            .unwrap_or(());

        // Phase 1: log the token instead of sending email
        tracing::info!(
            account_id = %account.public_id,
            reset_token = %reset_token,
            expires_at = %expires_at,
            "Password reset token generated (Phase 1 -- would email)"
        );
    }

    Ok(ApiResponse::accepted(json!({
        "message": "If an account with that email exists, a reset link has been sent."
    })))
}

// ---------------------------------------------------------------------------
// POST /auth/reset-password
// ---------------------------------------------------------------------------

pub async fn reset_password(
    State(state): State<AppState>,
    Json(req): Json<ResetPasswordRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    // Validate new password
    let pw_errors = validate_password(&req.new_password);
    if !pw_errors.is_empty() {
        return Err(ApiError::Validation {
            message: "Password does not meet requirements".into(),
            details: Some(json!({ "rules": pw_errors })),
        });
    }

    let token_hash = hash_token(&req.token);
    let token_hash_hex = hex::encode(&token_hash);

    // Look up account by reset token hash in Redis
    let mut conn = state
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| ApiError::Internal {
            message: format!("Redis error: {e}"),
        })?;

    let account_id_str: Option<String> = redis::cmd("GET")
        .arg(format!("reset_token:{token_hash_hex}"))
        .query_async(&mut conn)
        .await
        .unwrap_or(None);

    let account_id: i64 = account_id_str
        .and_then(|s| s.parse().ok())
        .ok_or(ApiError::Unauthorized {
            code: "INVALID_RESET_TOKEN".into(),
            message: "Invalid or expired reset token".into(),
        })?;

    // Hash new password
    let hashed = hash_password(&req.new_password)?;

    // Update password
    account_service::update_account(
        &state.db,
        account_id,
        None,
        None,
        None,
        None,
        Some(&hashed),
        None,
    )
    .await?;

    // Revoke all refresh tokens for this account
    sqlx::query("UPDATE refresh_tokens SET revoked_at = NOW() WHERE account_id = $1 AND revoked_at IS NULL")
        .bind(account_id)
        .execute(&state.db)
        .await?;

    // Delete the reset token from Redis
    let _: () = redis::cmd("DEL")
        .arg(format!("reset_token:{token_hash_hex}"))
        .query_async(&mut conn)
        .await
        .unwrap_or(());

    Ok(ApiResponse::ok(json!({
        "message": "Password has been reset. Please log in with your new password."
    })))
}
