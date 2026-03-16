use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::error::ApiError;
use crate::extractors::AuthenticatedAccount;
use crate::response::ApiResponse;
use crate::services::{account_service, auth_service, partner_service};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct AccountResponse {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub email_verified: bool,
    pub mfa_enabled: bool,
    pub timezone: String,
    pub locale: String,
    pub organization_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAccountRequest {
    pub display_name: Option<String>,
    pub timezone: Option<String>,
    pub locale: Option<String>,
    pub email: Option<String>,
    pub current_password: Option<String>,
    pub new_password: Option<String>,
}

// ---------------------------------------------------------------------------
// GET /accounts/me
// ---------------------------------------------------------------------------

pub async fn get_me(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
) -> Result<(StatusCode, Json<ApiResponse<AccountResponse>>), ApiError> {
    let account = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    Ok(ApiResponse::ok(account_to_response(&account)))
}

// ---------------------------------------------------------------------------
// PATCH /accounts/me
// ---------------------------------------------------------------------------

pub async fn update_me(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Json(req): Json<UpdateAccountRequest>,
) -> Result<(StatusCode, Json<ApiResponse<AccountResponse>>), ApiError> {
    let account = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    // If email or password change is requested, require current_password
    if (req.email.is_some() || req.new_password.is_some()) && req.current_password.is_none() {
        return Err(ApiError::Validation {
            message: "current_password is required when changing email or password".into(),
            details: None,
        });
    }

    // Verify current password if provided
    if let Some(ref current_pw) = req.current_password {
        let valid = auth_service::verify_password(current_pw, &account.password_hash)?;
        if !valid {
            return Err(ApiError::Unauthorized {
                code: "INCORRECT_PASSWORD".into(),
                message: "Current password is incorrect".into(),
            });
        }
    }

    // Validate new password if provided
    let password_hash = if let Some(ref new_pw) = req.new_password {
        let errors = auth_service::validate_password(new_pw);
        if !errors.is_empty() {
            return Err(ApiError::Validation {
                message: "New password does not meet requirements".into(),
                details: Some(json!({ "rules": errors })),
            });
        }
        Some(auth_service::hash_password(new_pw)?)
    } else {
        None
    };

    // Validate display_name if provided
    if let Some(ref dn) = req.display_name {
        if dn.len() < 2 || dn.len() > 100 {
            return Err(ApiError::Validation {
                message: "Display name must be 2-100 characters".into(),
                details: None,
            });
        }
    }

    let email_verified = if req.email.is_some() {
        Some(false) // Changing email resets verification
    } else {
        None
    };

    let updated = account_service::update_account(
        &state.db,
        account.id,
        req.display_name.as_deref(),
        req.timezone.as_deref(),
        req.locale.as_deref(),
        req.email.as_deref(),
        password_hash.as_deref(),
        email_verified,
    )
    .await?;

    // If password changed, revoke all refresh tokens
    if req.new_password.is_some() {
        sqlx::query(
            "UPDATE refresh_tokens SET revoked_at = NOW() WHERE account_id = $1 AND revoked_at IS NULL",
        )
        .bind(account.id)
        .execute(&state.db)
        .await?;
    }

    Ok(ApiResponse::ok(account_to_response(&updated)))
}

// ---------------------------------------------------------------------------
// GET /accounts/:id
// ---------------------------------------------------------------------------

pub async fn get_account(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    let target = account_service::get_account_by_public_id(&state.db, id)
        .await
        .map_err(|_| ApiError::NotFound {
            code: "ACCOUNT_NOT_FOUND".into(),
            message: "Account does not exist".into(),
        })?;

    // Check authorization: admin sees all, partner/authority must have relationship
    if caller.role == "admin" {
        return Ok(ApiResponse::ok(json!({
            "id": target.public_id.to_string(),
            "email": target.email,
            "display_name": target.display_name,
            "role": target.role,
            "email_verified": target.email_verified,
            "mfa_enabled": target.mfa_enabled,
            "timezone": target.timezone,
            "locale": target.locale,
            "created_at": target.created_at.to_rfc3339(),
            "updated_at": target.updated_at.to_rfc3339(),
        })));
    }

    // Check for active partner relationship
    let partnership =
        partner_service::has_active_partnership(&state.db, caller.id, target.id).await?;

    if partnership.is_none() {
        return Err(ApiError::Forbidden {
            message: "No active partner/authority relationship with this account".into(),
        });
    }

    // Partners see limited fields, authority sees more
    if caller.role == "authority" {
        Ok(ApiResponse::ok(json!({
            "id": target.public_id.to_string(),
            "email": target.email,
            "display_name": target.display_name,
            "email_verified": target.email_verified,
            "created_at": target.created_at.to_rfc3339(),
        })))
    } else {
        Ok(ApiResponse::ok(json!({
            "id": target.public_id.to_string(),
            "display_name": target.display_name,
            "email_verified": target.email_verified,
            "created_at": target.created_at.to_rfc3339(),
        })))
    }
}

fn account_to_response(account: &account_service::AccountRow) -> AccountResponse {
    AccountResponse {
        id: account.public_id.to_string(),
        email: account.email.clone(),
        display_name: account.display_name.clone(),
        role: account.role.clone(),
        email_verified: account.email_verified,
        mfa_enabled: account.mfa_enabled,
        timezone: account.timezone.clone(),
        locale: account.locale.clone(),
        organization_id: None, // Phase 2
        created_at: account.created_at.to_rfc3339(),
        updated_at: account.updated_at.to_rfc3339(),
    }
}
