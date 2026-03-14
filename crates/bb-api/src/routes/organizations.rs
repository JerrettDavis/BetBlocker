use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::error::ApiError;
use crate::extractors::{AuthenticatedAccount, Pagination};
use crate::response::{ApiResponse, PaginatedResponse};
use crate::services::{account_service, organization_service};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateOrgRequest {
    pub name: String,
    pub org_type: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrgRequest {
    pub name: Option<String>,
    pub org_type: Option<String>,
    pub default_protection_config: Option<serde_json::Value>,
    pub default_reporting_config: Option<serde_json::Value>,
    pub default_unenrollment_policy: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// POST /v1/organizations
// ---------------------------------------------------------------------------

pub async fn create_org(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Json(req): Json<CreateOrgRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    // Validate name
    if req.name.trim().is_empty() {
        return Err(ApiError::Validation {
            message: "Organization name cannot be empty".into(),
            details: None,
        });
    }

    let org = organization_service::create_organization(
        &state.db,
        &req.name,
        &req.org_type,
        caller.id,
    )
    .await?;

    tracing::info!(org_id = org.id, "Organization created");

    Ok(ApiResponse::created(json!({
        "id": org.public_id.to_string(),
        "name": org.name,
        "org_type": org.org_type,
        "owner_id": auth.account_id.to_string(),
        "created_at": org.created_at.to_rfc3339(),
    })))
}

// ---------------------------------------------------------------------------
// GET /v1/organizations
// ---------------------------------------------------------------------------

pub async fn list_orgs(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    pagination: Pagination,
) -> Result<PaginatedResponse<serde_json::Value>, ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    let (orgs, total) = organization_service::list_organizations_for_account(
        &state.db,
        caller.id,
        pagination.per_page,
        pagination.offset,
    )
    .await?;

    let data: Vec<serde_json::Value> = orgs
        .iter()
        .map(|o| {
            json!({
                "id": o.public_id.to_string(),
                "name": o.name,
                "org_type": o.org_type,
                "owner_id": o.owner_id,
                "created_at": o.created_at.to_rfc3339(),
                "updated_at": o.updated_at.to_rfc3339(),
            })
        })
        .collect();

    Ok(PaginatedResponse::new(
        data,
        total,
        pagination.page,
        pagination.per_page,
    ))
}

// ---------------------------------------------------------------------------
// GET /v1/organizations/{id}
// ---------------------------------------------------------------------------

pub async fn get_org(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    let org = organization_service::get_organization(&state.db, id).await?;

    // Check membership
    organization_service::check_org_permission(&state.db, org.id, caller.id, "member").await?;

    Ok(ApiResponse::ok(json!({
        "id": org.public_id.to_string(),
        "name": org.name,
        "org_type": org.org_type,
        "owner_id": org.owner_id,
        "default_protection_config": org.default_protection_config,
        "default_reporting_config": org.default_reporting_config,
        "default_unenrollment_policy": org.default_unenrollment_policy,
        "created_at": org.created_at.to_rfc3339(),
        "updated_at": org.updated_at.to_rfc3339(),
    })))
}

// ---------------------------------------------------------------------------
// PATCH /v1/organizations/{id}
// ---------------------------------------------------------------------------

pub async fn update_org(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateOrgRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    let org = organization_service::get_organization(&state.db, id).await?;

    // Require admin or owner
    organization_service::check_org_permission(&state.db, org.id, caller.id, "admin").await?;

    let updated = organization_service::update_organization(
        &state.db,
        org.id,
        req.name.as_deref(),
        req.org_type.as_deref(),
        req.default_protection_config,
        req.default_reporting_config,
        req.default_unenrollment_policy,
    )
    .await?;

    Ok(ApiResponse::ok(json!({
        "id": updated.public_id.to_string(),
        "name": updated.name,
        "org_type": updated.org_type,
        "owner_id": updated.owner_id,
        "default_protection_config": updated.default_protection_config,
        "default_reporting_config": updated.default_reporting_config,
        "default_unenrollment_policy": updated.default_unenrollment_policy,
        "created_at": updated.created_at.to_rfc3339(),
        "updated_at": updated.updated_at.to_rfc3339(),
    })))
}

// ---------------------------------------------------------------------------
// DELETE /v1/organizations/{id}
// ---------------------------------------------------------------------------

pub async fn delete_org(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    let org = organization_service::get_organization(&state.db, id).await?;

    // Require owner
    organization_service::check_org_permission(&state.db, org.id, caller.id, "owner").await?;

    organization_service::delete_organization(&state.db, org.id).await?;

    tracing::info!(org_id = org.id, "Organization deleted");

    Ok(ApiResponse::ok(json!({
        "deleted": true,
        "id": org.public_id.to_string(),
    })))
}
