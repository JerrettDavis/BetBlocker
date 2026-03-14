use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::error::ApiError;
use crate::extractors::{AuthenticatedAccount, Pagination};
use crate::response::{ApiResponse, PaginatedResponse};
use crate::services::{account_service, enrollment_token_service, organization_service};
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

#[derive(Debug, Deserialize)]
pub struct AssignDeviceRequest {
    pub device_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateTokenRequest {
    pub label: Option<String>,
    pub protection_config: serde_json::Value,
    pub reporting_config: serde_json::Value,
    pub unenrollment_policy: serde_json::Value,
    pub max_uses: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
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

// ---------------------------------------------------------------------------
// POST /v1/organizations/{id}/devices
// ---------------------------------------------------------------------------

pub async fn assign_device(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path(org_id): Path<Uuid>,
    Json(req): Json<AssignDeviceRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;
    let org = organization_service::get_organization(&state.db, org_id).await?;

    // Require admin+
    organization_service::check_org_permission(&state.db, org.id, caller.id, "admin").await?;

    let device = organization_service::assign_device(
        &state.db,
        org.id,
        req.device_id,
        caller.id,
    )
    .await?;

    tracing::info!(org_id = org.id, device_id = req.device_id, "Device assigned to organization");

    Ok(ApiResponse::created(json!({
        "id": device.id,
        "organization_id": org.public_id.to_string(),
        "device_id": device.device_id,
        "assigned_by": device.assigned_by,
        "assigned_at": device.assigned_at.to_rfc3339(),
    })))
}

// ---------------------------------------------------------------------------
// DELETE /v1/organizations/{id}/devices/{device_id}
// ---------------------------------------------------------------------------

pub async fn unassign_device(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path((org_id, device_id)): Path<(Uuid, i64)>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;
    let org = organization_service::get_organization(&state.db, org_id).await?;

    // Require admin+
    organization_service::check_org_permission(&state.db, org.id, caller.id, "admin").await?;

    organization_service::unassign_device(&state.db, org.id, device_id).await?;

    tracing::info!(org_id = org.id, device_id = device_id, "Device unassigned from organization");

    Ok(ApiResponse::ok(json!({
        "deleted": true,
        "organization_id": org.public_id.to_string(),
        "device_id": device_id,
    })))
}

// ---------------------------------------------------------------------------
// GET /v1/organizations/{id}/devices
// ---------------------------------------------------------------------------

pub async fn list_org_devices(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path(org_id): Path<Uuid>,
    pagination: Pagination,
) -> Result<PaginatedResponse<serde_json::Value>, ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;
    let org = organization_service::get_organization(&state.db, org_id).await?;

    // Require member+
    organization_service::check_org_permission(&state.db, org.id, caller.id, "member").await?;

    let (devices, total) = organization_service::list_org_devices(
        &state.db,
        org.id,
        pagination.per_page,
        pagination.offset,
    )
    .await?;

    let data: Vec<serde_json::Value> = devices
        .iter()
        .map(|d| {
            json!({
                "id": d.id,
                "organization_id": org.public_id.to_string(),
                "device_id": d.device_id,
                "assigned_by": d.assigned_by,
                "assigned_at": d.assigned_at.to_rfc3339(),
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
// POST /v1/organizations/{id}/tokens
// ---------------------------------------------------------------------------

pub async fn create_token(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path(org_id): Path<Uuid>,
    Json(req): Json<CreateTokenRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;
    let org = organization_service::get_organization(&state.db, org_id).await?;

    // Require admin+
    organization_service::check_org_permission(&state.db, org.id, caller.id, "admin").await?;

    let token = enrollment_token_service::create_enrollment_token(
        &state.db,
        org.id,
        caller.id,
        req.label.as_deref(),
        req.protection_config,
        req.reporting_config,
        req.unenrollment_policy,
        req.max_uses,
        req.expires_at,
    )
    .await?;

    tracing::info!(org_id = org.id, token_id = token.id, "Enrollment token created");

    Ok(ApiResponse::created(json!({
        "id": token.id,
        "public_id": token.public_id.to_string(),
        "organization_id": org.public_id.to_string(),
        "label": token.label,
        "max_uses": token.max_uses,
        "uses_count": token.uses_count,
        "expires_at": token.expires_at.map(|t| t.to_rfc3339()),
        "created_at": token.created_at.to_rfc3339(),
    })))
}

// ---------------------------------------------------------------------------
// GET /v1/organizations/{id}/tokens
// ---------------------------------------------------------------------------

pub async fn list_tokens(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path(org_id): Path<Uuid>,
    pagination: Pagination,
) -> Result<PaginatedResponse<serde_json::Value>, ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;
    let org = organization_service::get_organization(&state.db, org_id).await?;

    // Require admin+
    organization_service::check_org_permission(&state.db, org.id, caller.id, "admin").await?;

    let (tokens, total) = enrollment_token_service::list_enrollment_tokens(
        &state.db,
        org.id,
        pagination.per_page,
        pagination.offset,
    )
    .await?;

    let data: Vec<serde_json::Value> = tokens
        .iter()
        .map(|t| {
            json!({
                "id": t.id,
                "public_id": t.public_id.to_string(),
                "organization_id": org.public_id.to_string(),
                "label": t.label,
                "max_uses": t.max_uses,
                "uses_count": t.uses_count,
                "expires_at": t.expires_at.map(|ts| ts.to_rfc3339()),
                "created_at": t.created_at.to_rfc3339(),
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
// DELETE /v1/organizations/{id}/tokens/{token_id}
// ---------------------------------------------------------------------------

pub async fn revoke_token(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path((org_id, token_id)): Path<(Uuid, i64)>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;
    let org = organization_service::get_organization(&state.db, org_id).await?;

    // Require admin+
    organization_service::check_org_permission(&state.db, org.id, caller.id, "admin").await?;

    // Verify token belongs to org
    let _token = enrollment_token_service::get_enrollment_token_by_id(
        &state.db,
        token_id,
        org.id,
    )
    .await
    .map_err(|_| ApiError::NotFound {
        code: "TOKEN_NOT_FOUND".into(),
        message: "Enrollment token not found in this organization".into(),
    })?;

    enrollment_token_service::revoke_enrollment_token(&state.db, token_id).await?;

    tracing::info!(org_id = org.id, token_id = token_id, "Enrollment token revoked");

    Ok(ApiResponse::ok(json!({
        "revoked": true,
        "token_id": token_id,
        "organization_id": org.public_id.to_string(),
    })))
}

// ---------------------------------------------------------------------------
// POST /v1/enroll/{token_public_id}
// ---------------------------------------------------------------------------

pub async fn redeem_token(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path(token_public_id): Path<Uuid>,
    Json(req): Json<AssignDeviceRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let _caller = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    let token = enrollment_token_service::redeem_enrollment_token(
        &state.db,
        token_public_id,
        req.device_id,
    )
    .await?;

    tracing::info!(
        token_id = token.id,
        device_id = req.device_id,
        "Enrollment token redeemed"
    );

    Ok(ApiResponse::created(json!({
        "redeemed": true,
        "token_public_id": token_public_id.to_string(),
        "organization_id": token.organization_id,
        "device_id": req.device_id,
    })))
}

// ---------------------------------------------------------------------------
// GET /v1/organizations/{id}/tokens/{token_id}/qr
// ---------------------------------------------------------------------------

pub async fn get_token_qr(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path((org_id, token_id)): Path<(Uuid, i64)>,
) -> Result<axum::response::Response, ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;
    let org = organization_service::get_organization(&state.db, org_id).await?;

    // Require admin+
    organization_service::check_org_permission(&state.db, org.id, caller.id, "admin").await?;

    // Verify token belongs to org
    let token = enrollment_token_service::get_enrollment_token_by_id(
        &state.db,
        token_id,
        org.id,
    )
    .await
    .map_err(|_| ApiError::NotFound {
        code: "TOKEN_NOT_FOUND".into(),
        message: "Enrollment token not found in this organization".into(),
    })?;

    // Build the enrollment URL
    let enroll_url = format!(
        "{}/v1/enroll/{}",
        state.config.public_base_url.as_deref().unwrap_or("https://api.betblocker.org"),
        token.public_id
    );

    // Generate QR code
    let qr = qrcode::QrCode::new(enroll_url.as_bytes()).map_err(|e| ApiError::Internal {
        message: format!("Failed to generate QR code: {e}"),
    })?;

    let img = qr.render::<image::Luma<u8>>().quiet_zone(true).build();

    let mut png_bytes: Vec<u8> = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
    image::ImageEncoder::write_image(
        encoder,
        &img,
        img.width(),
        img.height(),
        image::ExtendedColorType::L8,
    )
    .map_err(|e| ApiError::Internal {
        message: format!("Failed to encode PNG: {e}"),
    })?;

    Ok(axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "image/png")
        .header(
            "Content-Disposition",
            format!("inline; filename=\"token-{}.png\"", token.public_id),
        )
        .body(axum::body::Body::from(png_bytes))
        .unwrap())
}
