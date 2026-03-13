use axum::{
    extract::{FromRef, FromRequestParts, Query},
    http::request::Parts,
};
use jsonwebtoken::{decode, Algorithm, Validation};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::ApiError;
use crate::services::auth_service::Claims;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// AuthenticatedAccount extractor
// ---------------------------------------------------------------------------

/// Extractor that validates the JWT from the `Authorization: Bearer <token>`
/// header and provides the authenticated account identity.
#[derive(Debug, Clone)]
pub struct AuthenticatedAccount {
    pub account_id: Uuid,
    pub email: String,
    pub role: String,
}

impl<S> FromRequestParts<S> for AuthenticatedAccount
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(ApiError::Unauthorized {
                code: "UNAUTHORIZED".into(),
                message: "Missing or malformed Authorization header".into(),
            })?;

        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.set_issuer(&["betblocker-api"]);

        let token_data =
            decode::<Claims>(auth_header, &app_state.jwt_decoding_key, &validation).map_err(
                |e| ApiError::Unauthorized {
                    code: "INVALID_TOKEN".into(),
                    message: format!("Invalid or expired token: {e}"),
                },
            )?;

        Ok(AuthenticatedAccount {
            account_id: token_data.claims.sub,
            email: token_data.claims.email,
            role: token_data.claims.role,
        })
    }
}

// ---------------------------------------------------------------------------
// RequireAdmin extractor
// ---------------------------------------------------------------------------

/// Extractor that requires the authenticated user to have the `admin` role.
#[derive(Debug, Clone)]
pub struct RequireAdmin(pub AuthenticatedAccount);

impl<S> FromRequestParts<S> for RequireAdmin
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let account = AuthenticatedAccount::from_request_parts(parts, state).await?;
        if account.role != "admin" {
            return Err(ApiError::Forbidden {
                message: "Admin access required".into(),
            });
        }
        Ok(Self(account))
    }
}

// ---------------------------------------------------------------------------
// RequirePartnerOrAbove extractor
// ---------------------------------------------------------------------------

/// Extractor that requires the authenticated user to have at least `partner` role.
/// Allowed roles: partner, authority, admin.
#[derive(Debug, Clone)]
pub struct RequirePartnerOrAbove(pub AuthenticatedAccount);

impl<S> FromRequestParts<S> for RequirePartnerOrAbove
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let account = AuthenticatedAccount::from_request_parts(parts, state).await?;
        match account.role.as_str() {
            "partner" | "authority" | "admin" => Ok(Self(account)),
            _ => Err(ApiError::Forbidden {
                message: "Partner or higher role required".into(),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Pagination extractor
// ---------------------------------------------------------------------------

/// Query parameters for paginated list endpoints.
#[derive(Debug, Clone, Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    50
}

/// Validated pagination parameters.
#[derive(Debug, Clone)]
pub struct Pagination {
    pub page: i64,
    pub per_page: i64,
    pub offset: i64,
}

impl<S> FromRequestParts<S> for Pagination
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(params) = Query::<PaginationParams>::from_request_parts(parts, state)
            .await
            .map_err(|e| ApiError::Validation {
                message: format!("Invalid pagination parameters: {e}"),
                details: None,
            })?;

        let page = params.page.max(1);
        let per_page = params.per_page.clamp(1, 200);
        let offset = (page - 1) * per_page;

        Ok(Pagination {
            page,
            per_page,
            offset,
        })
    }
}
