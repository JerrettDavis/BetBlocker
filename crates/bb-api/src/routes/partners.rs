use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::json;

use crate::error::ApiError;
use crate::extractors::{AuthenticatedAccount, Pagination};
use crate::response::{ApiResponse, PaginatedResponse};
use crate::services::{account_service, partner_service};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct InvitePartnerRequest {
    pub email: String,
    pub permissions: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// POST /partners/invite
// ---------------------------------------------------------------------------

pub async fn invite_partner(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Json(req): Json<InvitePartnerRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    // Require verified email
    if !caller.email_verified {
        return Err(ApiError::Forbidden {
            message: "Email must be verified before inviting partners".into(),
        });
    }

    // Look up partner by email
    let partner = account_service::get_account_by_email(&state.db, &req.email)
        .await?
        .ok_or(ApiError::NotFound {
            code: "ACCOUNT_NOT_FOUND".into(),
            message: "No account found with that email".into(),
        })?;

    if partner.id == caller.id {
        return Err(ApiError::Validation {
            message: "Cannot invite yourself as a partner".into(),
            details: None,
        });
    }

    // Check for existing relationship
    let existing =
        partner_service::has_active_partnership(&state.db, caller.id, partner.id).await?;
    if existing.is_some() {
        return Err(ApiError::Conflict {
            code: "PARTNERSHIP_ALREADY_EXISTS".into(),
            message: "A partner relationship already exists between these accounts".into(),
        });
    }

    let relationship = partner_service::create_partner_invite(
        &state.db,
        caller.id,
        partner.id,
        "accountability_partner",
        caller.id,
    )
    .await
    .map_err(|e| {
        if let ApiError::Conflict { .. } = &e {
            ApiError::Conflict {
                code: "PARTNERSHIP_ALREADY_EXISTS".into(),
                message: "A partner relationship already exists between these accounts".into(),
            }
        } else {
            e
        }
    })?;

    tracing::info!(
        relationship_id = relationship.id,
        "Partner invitation sent (Phase 1 -- notification would be emailed)"
    );

    Ok(ApiResponse::created(json!({
        "id": relationship.id,
        "account_id": caller.public_id.to_string(),
        "partner_account_id": partner.public_id.to_string(),
        "status": "pending",
        "invited_at": relationship.invited_at.to_rfc3339(),
    })))
}

// ---------------------------------------------------------------------------
// POST /partners/:id/accept
// ---------------------------------------------------------------------------

pub async fn accept_partner(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    let relationship = partner_service::get_partner_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound {
            code: "PARTNER_NOT_FOUND".into(),
            message: "Partner relationship not found".into(),
        })?;

    // Verify caller is the partner_account_id on the relationship
    if relationship.partner_account_id != caller.id {
        return Err(ApiError::Forbidden {
            message: "Only the invited partner can accept this invitation".into(),
        });
    }

    if relationship.status != "pending" {
        return Err(ApiError::Conflict {
            code: "NOT_PENDING".into(),
            message: "This invitation is not in a pending state".into(),
        });
    }

    partner_service::accept_partner_invite(&state.db, id).await?;

    // Update partner's role to "partner" if currently "user"
    if caller.role == "user" {
        account_service::update_account_role(&state.db, caller.id, "partner").await?;
    }

    Ok(ApiResponse::ok(json!({
        "id": relationship.id,
        "status": "active",
        "accepted_at": chrono::Utc::now().to_rfc3339(),
    })))
}

// ---------------------------------------------------------------------------
// GET /partners
// ---------------------------------------------------------------------------

pub async fn list_partners(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    pagination: Pagination,
) -> Result<PaginatedResponse<serde_json::Value>, ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    let (partners, total) = partner_service::list_partners(
        &state.db,
        caller.id,
        pagination.per_page,
        pagination.offset,
    )
    .await?;

    let data: Vec<serde_json::Value> = partners
        .iter()
        .map(|p| {
            json!({
                "id": p.id,
                "account_id": p.account_public_id,
                "partner_account_id": p.partner_public_id,
                "account_display_name": p.account_display_name,
                "partner_display_name": p.partner_display_name,
                "status": p.status,
                "role": p.role,
                "invited_at": p.invited_at.to_rfc3339(),
                "accepted_at": p.accepted_at.map(|t| t.to_rfc3339()),
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
// DELETE /partners/:id
// ---------------------------------------------------------------------------

pub async fn remove_partner(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    let relationship = partner_service::get_partner_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound {
            code: "PARTNER_NOT_FOUND".into(),
            message: "Partner relationship not found".into(),
        })?;

    // Verify caller is account_id or partner_account_id
    if relationship.account_id != caller.id && relationship.partner_account_id != caller.id {
        return Err(ApiError::Forbidden {
            message: "Not authorized to modify this partner relationship".into(),
        });
    }

    partner_service::revoke_partner(&state.db, id).await?;

    Ok(ApiResponse::ok(json!({
        "id": relationship.id,
        "status": "revoked",
    })))
}
