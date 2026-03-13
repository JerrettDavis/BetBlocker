use axum::{
    extract::{Path, Query, State},
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
use crate::services::{
    account_service, device_service, enrollment_service, partner_service,
};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateEnrollmentRequest {
    pub device_id: Uuid,
    pub tier: String,
    pub protection_config: Option<serde_json::Value>,
    pub reporting_config: Option<serde_json::Value>,
    pub unenrollment_policy: Option<serde_json::Value>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct EnrollmentFilters {
    pub status: Option<String>,
    pub tier: Option<String>,
    pub device_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEnrollmentRequest {
    pub protection_config: Option<serde_json::Value>,
    pub reporting_config: Option<serde_json::Value>,
    pub unenrollment_policy: Option<serde_json::Value>,
    pub expires_at: Option<Option<DateTime<Utc>>>,
}

#[derive(Debug, Deserialize)]
pub struct UnenrollRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApproveUnenrollRequest {
    pub approved: bool,
}

// ---------------------------------------------------------------------------
// POST /enrollments
// ---------------------------------------------------------------------------

pub async fn create_enrollment(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Json(req): Json<CreateEnrollmentRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller =
        account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    // Look up device
    let device = device_service::get_device_by_public_id(&state.db, req.device_id)
        .await?
        .ok_or(ApiError::NotFound {
            code: "DEVICE_NOT_FOUND".into(),
            message: "Device does not exist".into(),
        })?;

    // Check device doesn't already have an active enrollment
    if enrollment_service::device_has_active_enrollment(&state.db, device.id).await? {
        return Err(ApiError::Conflict {
            code: "DEVICE_ALREADY_ENROLLED".into(),
            message: "Device already has an active enrollment".into(),
        });
    }

    // Get account that owns the device
    let device_owner = account_service::get_account_by_id(&state.db, device.account_id).await?;

    // Tier-specific authorization
    match req.tier.as_str() {
        "self" => {
            // Self tier: caller must own the device
            if device.account_id != caller.id {
                return Err(ApiError::Forbidden {
                    message: "Self-enrollment requires device ownership".into(),
                });
            }
        }
        "partner" => {
            // Partner tier: caller must have active partner relationship with device owner
            let partnership =
                partner_service::has_active_partnership(&state.db, device.account_id, caller.id)
                    .await?;
            if partnership.is_none() {
                return Err(ApiError::Forbidden {
                    message: "Partner enrollment requires active partner relationship".into(),
                });
            }
        }
        "authority" => {
            // Authority tier: caller must be authority role
            if caller.role != "authority" && caller.role != "admin" {
                return Err(ApiError::Forbidden {
                    message: "Authority enrollment requires authority role".into(),
                });
            }
        }
        _ => {
            return Err(ApiError::Validation {
                message: "Invalid tier. Must be 'self', 'partner', or 'authority'".into(),
                details: None,
            });
        }
    }

    // Apply tier defaults for omitted configs
    let protection_config = req.protection_config.unwrap_or_else(|| {
        match req.tier.as_str() {
            "self" => json!({
                "dns_blocking": true, "app_blocking": false, "browser_blocking": false,
                "vpn_detection": "log", "tamper_response": "log"
            }),
            "partner" => json!({
                "dns_blocking": true, "app_blocking": false, "browser_blocking": false,
                "vpn_detection": "alert", "tamper_response": "alert_partner"
            }),
            "authority" => json!({
                "dns_blocking": true, "app_blocking": false, "browser_blocking": false,
                "vpn_detection": "lockdown", "tamper_response": "alert_authority"
            }),
            _ => json!({}),
        }
    });

    let reporting_config = req.reporting_config.unwrap_or_else(|| {
        match req.tier.as_str() {
            "self" => json!({
                "level": "none", "blocked_attempt_counts": true,
                "domain_details": false, "tamper_alerts": true
            }),
            "partner" => json!({
                "level": "aggregated", "blocked_attempt_counts": true,
                "domain_details": false, "tamper_alerts": true
            }),
            "authority" => json!({
                "level": "full_audit", "blocked_attempt_counts": true,
                "domain_details": true, "tamper_alerts": true
            }),
            _ => json!({}),
        }
    });

    let unenrollment_policy = req.unenrollment_policy.unwrap_or_else(|| {
        match req.tier.as_str() {
            "self" => json!({
                "type": "time_delayed", "cooldown_hours": 48,
                "requires_approval_from": null
            }),
            "partner" => json!({
                "type": "partner_approval", "cooldown_hours": null,
                "requires_approval_from": caller.public_id.to_string()
            }),
            "authority" => json!({
                "type": "authority_approval", "cooldown_hours": null,
                "requires_approval_from": caller.public_id.to_string()
            }),
            _ => json!({}),
        }
    });

    // Validate self-tier cooldown_hours
    if req.tier == "self" {
        if let Some(hours) = unenrollment_policy["cooldown_hours"].as_i64() {
            if !(24..=72).contains(&hours) {
                return Err(ApiError::Validation {
                    message: "Self-tier cooldown_hours must be between 24 and 72".into(),
                    details: None,
                });
            }
        }
    }

    let enrollment = enrollment_service::create_enrollment(
        &state.db,
        device.id,
        device_owner.id,
        caller.id,
        &req.tier,
        &protection_config,
        &reporting_config,
        &unenrollment_policy,
        req.expires_at,
    )
    .await?;

    Ok(ApiResponse::created(enrollment_to_json(&enrollment)))
}

// ---------------------------------------------------------------------------
// GET /enrollments
// ---------------------------------------------------------------------------

pub async fn list_enrollments(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    pagination: Pagination,
    Query(filters): Query<EnrollmentFilters>,
) -> Result<PaginatedResponse<serde_json::Value>, ApiError> {
    let caller =
        account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    // Resolve device_id filter if provided (UUID -> internal ID)
    let device_id_internal = if let Some(device_uuid) = filters.device_id {
        let device = device_service::get_device_by_public_id(&state.db, device_uuid).await?;
        device.map(|d| d.id)
    } else {
        None
    };

    let (enrollments, total) = enrollment_service::list_enrollments(
        &state.db,
        caller.id,
        filters.status.as_deref(),
        filters.tier.as_deref(),
        device_id_internal,
        pagination.per_page,
        pagination.offset,
    )
    .await?;

    let data: Vec<serde_json::Value> = enrollments.iter().map(enrollment_to_json).collect();

    Ok(PaginatedResponse::new(
        data,
        total,
        pagination.page,
        pagination.per_page,
    ))
}

// ---------------------------------------------------------------------------
// GET /enrollments/:id
// ---------------------------------------------------------------------------

pub async fn get_enrollment(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller =
        account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    let enrollment = enrollment_service::get_enrollment_by_public_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound {
            code: "ENROLLMENT_NOT_FOUND".into(),
            message: "Enrollment does not exist".into(),
        })?;

    // Authorization check
    if enrollment.account_id != caller.id
        && enrollment.enrolled_by != caller.id
        && caller.role != "admin"
    {
        return Err(ApiError::Forbidden {
            message: "Not authorized to view this enrollment".into(),
        });
    }

    Ok(ApiResponse::ok(enrollment_to_json(&enrollment)))
}

// ---------------------------------------------------------------------------
// PATCH /enrollments/:id
// ---------------------------------------------------------------------------

pub async fn update_enrollment(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateEnrollmentRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller =
        account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    let enrollment = enrollment_service::get_enrollment_by_public_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound {
            code: "ENROLLMENT_NOT_FOUND".into(),
            message: "Enrollment does not exist".into(),
        })?;

    // Enforce modification permissions per tier
    match enrollment.tier.as_str() {
        "self" => {
            if enrollment.account_id != caller.id && caller.role != "admin" {
                return Err(ApiError::Forbidden {
                    message: "Only the enrollment owner can modify self-tier enrollments".into(),
                });
            }
        }
        "partner" => {
            if enrollment.enrolled_by != caller.id && caller.role != "admin" {
                return Err(ApiError::Forbidden {
                    message: "Only the enrolling partner can modify partner-tier enrollments".into(),
                });
            }
        }
        "authority" => {
            if enrollment.enrolled_by != caller.id && caller.role != "admin" {
                return Err(ApiError::Forbidden {
                    message: "Only the authority representative can modify authority-tier enrollments".into(),
                });
            }
        }
        _ => {}
    }

    let updated = enrollment_service::update_enrollment_config(
        &state.db,
        enrollment.id,
        req.protection_config.as_ref(),
        req.reporting_config.as_ref(),
        req.unenrollment_policy.as_ref(),
        req.expires_at,
    )
    .await?;

    Ok(ApiResponse::ok(enrollment_to_json(&updated)))
}

// ---------------------------------------------------------------------------
// POST /enrollments/:id/unenroll
// ---------------------------------------------------------------------------

pub async fn request_unenroll(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path(id): Path<Uuid>,
    body: Option<Json<UnenrollRequest>>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller =
        account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    let enrollment = enrollment_service::get_enrollment_by_public_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound {
            code: "ENROLLMENT_NOT_FOUND".into(),
            message: "Enrollment does not exist".into(),
        })?;

    // Only the enrollment owner can request unenrollment
    if enrollment.account_id != caller.id {
        return Err(ApiError::Forbidden {
            message: "Only the enrolled account can request unenrollment".into(),
        });
    }

    if enrollment.status != "active" {
        return Err(ApiError::Conflict {
            code: "ENROLLMENT_NOT_ACTIVE".into(),
            message: "Enrollment is not active".into(),
        });
    }

    // Check for existing pending request
    if enrollment_service::has_pending_unenroll_request(&state.db, enrollment.id).await? {
        return Err(ApiError::Conflict {
            code: "ALREADY_UNENROLLING".into(),
            message: "An unenrollment request is already pending".into(),
        });
    }

    let policy = &enrollment.unenrollment_policy;
    let policy_type = policy["type"].as_str().unwrap_or("time_delayed");
    let _reason = body.and_then(|b| b.reason.clone());

    match policy_type {
        "time_delayed" => {
            let cooldown_hours = policy["cooldown_hours"].as_i64().unwrap_or(48);
            let eligible_at = chrono::Utc::now() + chrono::Duration::hours(cooldown_hours);

            enrollment_service::insert_unenroll_request(
                &state.db,
                enrollment.id,
                caller.id,
                None,
                Some(eligible_at),
            )
            .await?;
            enrollment_service::update_enrollment_status(
                &state.db,
                enrollment.id,
                "unenroll_requested",
            )
            .await?;

            Ok(ApiResponse::ok(json!({
                "enrollment_id": enrollment.public_id.to_string(),
                "status": "unenroll_requested",
                "type": "time_delayed",
                "eligible_at": eligible_at.to_rfc3339(),
            })))
        }
        "partner_approval" | "authority_approval" => {
            let approver_uuid_str = policy["requires_approval_from"].as_str();
            let required_approver_id = if let Some(uuid_str) = approver_uuid_str {
                let uuid = Uuid::parse_str(uuid_str).map_err(|_| ApiError::Internal {
                    message: "Invalid approver UUID in policy".into(),
                })?;
                let approver =
                    account_service::get_account_by_public_id(&state.db, uuid).await?;
                Some(approver.id)
            } else {
                None
            };

            enrollment_service::insert_unenroll_request(
                &state.db,
                enrollment.id,
                caller.id,
                required_approver_id,
                None,
            )
            .await?;
            enrollment_service::update_enrollment_status(
                &state.db,
                enrollment.id,
                "unenroll_requested",
            )
            .await?;

            tracing::info!(
                enrollment_id = %enrollment.public_id,
                "Unenrollment request requires approval (Phase 1 -- notification would be sent)"
            );

            Ok(ApiResponse::ok(json!({
                "enrollment_id": enrollment.public_id.to_string(),
                "status": "unenroll_requested",
                "type": policy_type,
                "requires_approval_from": approver_uuid_str,
            })))
        }
        _ => Err(ApiError::Internal {
            message: "Unknown unenrollment policy type".into(),
        }),
    }
}

// ---------------------------------------------------------------------------
// POST /enrollments/:id/approve-unenroll
// ---------------------------------------------------------------------------

pub async fn approve_unenroll(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path(id): Path<Uuid>,
    Json(req): Json<ApproveUnenrollRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller =
        account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    let enrollment = enrollment_service::get_enrollment_by_public_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound {
            code: "ENROLLMENT_NOT_FOUND".into(),
            message: "Enrollment does not exist".into(),
        })?;

    if enrollment.status != "unenroll_requested" {
        return Err(ApiError::Conflict {
            code: "NO_PENDING_REQUEST".into(),
            message: "No pending unenrollment request".into(),
        });
    }

    // Verify caller is the designated approver
    let policy = &enrollment.unenrollment_policy;
    let approver_uuid_str = policy["requires_approval_from"]
        .as_str()
        .ok_or(ApiError::Internal {
            message: "Enrollment missing approval authority".into(),
        })?;
    let approver_uuid = Uuid::parse_str(approver_uuid_str).map_err(|_| ApiError::Internal {
        message: "Invalid approver UUID".into(),
    })?;

    if caller.public_id != approver_uuid && caller.role != "admin" {
        return Err(ApiError::Forbidden {
            message: "Only the designated approver can respond to this request".into(),
        });
    }

    if req.approved {
        enrollment_service::resolve_unenroll_request(&state.db, enrollment.id, caller.id, true)
            .await?;
        enrollment_service::update_enrollment_status(
            &state.db,
            enrollment.id,
            "unenroll_approved",
        )
        .await?;

        // Update device status to unenrolling
        device_service::update_device_status(&state.db, enrollment.device_id, "unenrolling")
            .await?;

        Ok(ApiResponse::ok(json!({
            "enrollment_id": enrollment.public_id.to_string(),
            "status": "unenroll_approved",
            "approved": true,
        })))
    } else {
        enrollment_service::resolve_unenroll_request(&state.db, enrollment.id, caller.id, false)
            .await?;
        enrollment_service::update_enrollment_status(&state.db, enrollment.id, "active").await?;

        Ok(ApiResponse::ok(json!({
            "enrollment_id": enrollment.public_id.to_string(),
            "status": "active",
            "approved": false,
        })))
    }
}

fn enrollment_to_json(e: &enrollment_service::EnrollmentRow) -> serde_json::Value {
    json!({
        "id": e.public_id.to_string(),
        "device_id": e.device_id,
        "account_id": e.account_id,
        "enrolled_by": e.enrolled_by,
        "tier": e.tier,
        "status": e.status,
        "protection_config": e.protection_config,
        "reporting_config": e.reporting_config,
        "unenrollment_policy": e.unenrollment_policy,
        "created_at": e.created_at.to_rfc3339(),
        "updated_at": e.updated_at.to_rfc3339(),
        "expires_at": e.expires_at.map(|t| t.to_rfc3339()),
    })
}
