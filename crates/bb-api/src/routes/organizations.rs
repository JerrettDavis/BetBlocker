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

#[derive(Debug, Deserialize)]
pub struct InviteMemberRequest {
    pub email: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemberRoleRequest {
    pub role: String,
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

// ---------------------------------------------------------------------------
// POST /v1/organizations/{id}/members
// ---------------------------------------------------------------------------

pub async fn invite_member(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path(org_id): Path<Uuid>,
    Json(req): Json<InviteMemberRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    let org = organization_service::get_organization(&state.db, org_id).await?;

    // Require admin+
    organization_service::check_org_permission(&state.db, org.id, caller.id, "admin").await?;

    if req.email.trim().is_empty() {
        return Err(ApiError::Validation {
            message: "Email cannot be empty".into(),
            details: None,
        });
    }

    let member = organization_service::invite_member(
        &state.db,
        org.id,
        &req.email,
        &req.role,
        caller.id,
    )
    .await?;

    tracing::info!(org_id = org.id, member_id = member.account_id, "Member invited to organization");

    Ok(ApiResponse::created(json!({
        "id": member.id,
        "organization_id": org.public_id.to_string(),
        "account_id": member.account_id,
        "role": member.role,
        "invited_by": member.invited_by,
        "joined_at": member.joined_at.to_rfc3339(),
    })))
}

// ---------------------------------------------------------------------------
// GET /v1/organizations/{id}/members
// ---------------------------------------------------------------------------

pub async fn list_members(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path(org_id): Path<Uuid>,
    pagination: Pagination,
) -> Result<PaginatedResponse<serde_json::Value>, ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    let org = organization_service::get_organization(&state.db, org_id).await?;

    // Require member+
    organization_service::check_org_permission(&state.db, org.id, caller.id, "member").await?;

    let (members, total) = organization_service::list_members(
        &state.db,
        org.id,
        pagination.per_page,
        pagination.offset,
    )
    .await?;

    let data: Vec<serde_json::Value> = members
        .iter()
        .map(|m| {
            json!({
                "id": m.id,
                "organization_id": org.public_id.to_string(),
                "account_id": m.account_public_id.map(|id| id.to_string()),
                "role": m.role,
                "display_name": m.display_name,
                "email": m.email,
                "invited_by": m.invited_by,
                "joined_at": m.joined_at.to_rfc3339(),
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
// PATCH /v1/organizations/{id}/members/{member_id}
// ---------------------------------------------------------------------------

pub async fn update_member_role(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path((org_id, member_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateMemberRoleRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    let org = organization_service::get_organization(&state.db, org_id).await?;

    // Require admin+
    organization_service::check_org_permission(&state.db, org.id, caller.id, "admin").await?;

    // Look up the target member by their public account ID
    let target_account = account_service::get_account_by_public_id(&state.db, member_id).await?;

    let updated = organization_service::update_member_role(
        &state.db,
        org.id,
        target_account.id,
        &req.role,
        caller.id,
    )
    .await?;

    tracing::info!(
        org_id = org.id,
        member_id = target_account.id,
        new_role = req.role,
        "Member role updated"
    );

    Ok(ApiResponse::ok(json!({
        "id": updated.id,
        "organization_id": org.public_id.to_string(),
        "account_id": member_id.to_string(),
        "role": updated.role,
        "invited_by": updated.invited_by,
        "joined_at": updated.joined_at.to_rfc3339(),
    })))
}

// ---------------------------------------------------------------------------
// DELETE /v1/organizations/{id}/members/{member_id}
// ---------------------------------------------------------------------------

pub async fn remove_member(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path((org_id, member_id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    let org = organization_service::get_organization(&state.db, org_id).await?;

    // Require admin+
    organization_service::check_org_permission(&state.db, org.id, caller.id, "admin").await?;

    // Look up the target member by their public account ID
    let target_account = account_service::get_account_by_public_id(&state.db, member_id).await?;

    organization_service::remove_member(
        &state.db,
        org.id,
        target_account.id,
        caller.id,
    )
    .await?;

    tracing::info!(
        org_id = org.id,
        member_id = target_account.id,
        "Member removed from organization"
    );

    Ok(ApiResponse::ok(json!({
        "deleted": true,
        "organization_id": org.public_id.to_string(),
        "account_id": member_id.to_string(),
    })))
}
