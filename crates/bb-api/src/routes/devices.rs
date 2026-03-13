use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::error::ApiError;
use crate::extractors::{AuthenticatedAccount, Pagination};
use crate::response::{ApiResponse, PaginatedResponse};
use crate::services::{
    account_service, auth_service, device_service, enrollment_service,
};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct RegisterDeviceRequest {
    pub name: String,
    pub platform: String,
    pub os_version: String,
    pub agent_version: String,
    pub hostname: String,
    pub hardware_id: String,
}

#[derive(Debug, Serialize)]
pub struct DeviceResponse {
    pub id: String,
    pub account_id: String,
    pub name: Option<String>,
    pub platform: String,
    pub os_version: Option<String>,
    pub agent_version: Option<String>,
    pub hostname: Option<String>,
    pub status: String,
    pub blocklist_version: Option<i64>,
    pub last_heartbeat_at: Option<String>,
    pub enrollment_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct DeviceFilters {
    pub status: Option<String>,
    pub platform: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HeartbeatRequest {
    pub agent_version: String,
    pub os_version: String,
    pub blocklist_version: i64,
    #[serde(default)]
    pub uptime_seconds: i64,
    #[serde(default)]
    pub blocking_active: bool,
    pub integrity_check: Option<serde_json::Value>,
    pub stats: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteDeviceRequest {
    pub reason: Option<String>,
}

// ---------------------------------------------------------------------------
// POST /devices
// ---------------------------------------------------------------------------

pub async fn register_device(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Json(req): Json<RegisterDeviceRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    // Validate input
    if req.name.is_empty() || req.name.len() > 100 {
        return Err(ApiError::Validation {
            message: "Device name must be 1-100 characters".into(),
            details: None,
        });
    }

    let valid_platforms = ["windows", "macos", "linux", "android", "ios"];
    if !valid_platforms.contains(&req.platform.as_str()) {
        return Err(ApiError::Validation {
            message: "Invalid platform".into(),
            details: None,
        });
    }

    let account =
        account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    let device = device_service::create_device(
        &state.db,
        account.id,
        &req.name,
        &req.platform,
        &req.os_version,
        &req.agent_version,
        &req.hostname,
        &req.hardware_id,
    )
    .await?;

    // Generate device token (Phase 1 -- no mTLS/CSR handling)
    let device_token = auth_service::generate_device_token();
    let token_hash = auth_service::hash_token(&device_token);

    // Store device token hash in Redis for Phase 1
    if let Ok(mut conn) = state.redis.get_multiplexed_async_connection().await {
        let _: () = redis::cmd("SET")
            .arg(format!("device_token:{}", hex::encode(&token_hash)))
            .arg(device.id.to_string())
            .query_async(&mut conn)
            .await
            .unwrap_or(());
    }

    Ok(ApiResponse::created(json!({
        "device": {
            "id": device.public_id.to_string(),
            "account_id": account.public_id.to_string(),
            "name": device.name,
            "platform": device.platform,
            "os_version": device.os_version,
            "agent_version": device.agent_version,
            "hostname": device.hostname,
            "status": device.status,
            "enrollment_id": null,
            "created_at": device.created_at.to_rfc3339(),
        },
        "certificate": null,
        "device_token": device_token,
        "api_endpoints": {
            "heartbeat": format!("/v1/devices/{}/heartbeat", device.public_id),
            "config": format!("/v1/devices/{}/config", device.public_id),
            "events": "/v1/events",
            "blocklist": "/v1/blocklist",
        }
    })))
}

// ---------------------------------------------------------------------------
// GET /devices
// ---------------------------------------------------------------------------

pub async fn list_devices(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    pagination: Pagination,
    Query(filters): Query<DeviceFilters>,
) -> Result<PaginatedResponse<DeviceResponse>, ApiError> {
    let account =
        account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    let (devices, total) = device_service::list_devices_for_account(
        &state.db,
        account.id,
        filters.status.as_deref(),
        filters.platform.as_deref(),
        pagination.per_page,
        pagination.offset,
    )
    .await?;

    let data: Vec<DeviceResponse> = devices.into_iter().map(|d| device_to_response(&d)).collect();

    Ok(PaginatedResponse::new(
        data,
        total,
        pagination.page,
        pagination.per_page,
    ))
}

// ---------------------------------------------------------------------------
// GET /devices/:id
// ---------------------------------------------------------------------------

pub async fn get_device(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<ApiResponse<DeviceResponse>>), ApiError> {
    let account =
        account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    let device = device_service::get_device_by_public_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound {
            code: "DEVICE_NOT_FOUND".into(),
            message: "Device does not exist".into(),
        })?;

    // Authorization: owner, partner with enrollment, authority, or admin
    if device.account_id != account.id && account.role != "admin" {
        // Check if caller is enrolled_by on an active enrollment for this device
        let enrollment =
            enrollment_service::get_active_enrollment_for_device(&state.db, device.id).await?;
        let authorized = enrollment
            .as_ref()
            .is_some_and(|e| e.enrolled_by == account.id);

        if !authorized {
            return Err(ApiError::Forbidden {
                message: "Not authorized to view this device".into(),
            });
        }
    }

    Ok(ApiResponse::ok(device_to_response(&device)))
}

// ---------------------------------------------------------------------------
// DELETE /devices/:id
// ---------------------------------------------------------------------------

pub async fn delete_device(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path(id): Path<Uuid>,
    body: Option<Json<DeleteDeviceRequest>>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let account =
        account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    let device = device_service::get_device_by_public_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound {
            code: "DEVICE_NOT_FOUND".into(),
            message: "Device does not exist".into(),
        })?;

    // Only owner or admin can delete
    if device.account_id != account.id && account.role != "admin" {
        return Err(ApiError::Forbidden {
            message: "Not authorized to delete this device".into(),
        });
    }

    let reason = body.and_then(|b| b.reason.clone());

    // Check for active enrollment
    let enrollment =
        enrollment_service::get_active_enrollment_for_device(&state.db, device.id).await?;

    if let Some(enrollment) = enrollment {
        // Delegate to unenrollment flow
        let policy: serde_json::Value = enrollment.unenrollment_policy.clone();
        let policy_type = policy["type"].as_str().unwrap_or("time_delayed");

        // Check for existing pending unenroll request
        if enrollment_service::has_pending_unenroll_request(&state.db, enrollment.id).await? {
            return Err(ApiError::Conflict {
                code: "ALREADY_UNENROLLING".into(),
                message: "An unenrollment request is already in progress".into(),
            });
        }

        match policy_type {
            "time_delayed" => {
                let cooldown_hours = policy["cooldown_hours"].as_i64().unwrap_or(48);
                let eligible_at =
                    chrono::Utc::now() + chrono::Duration::hours(cooldown_hours);

                enrollment_service::insert_unenroll_request(
                    &state.db,
                    enrollment.id,
                    account.id,
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
                    "device": { "id": device.public_id.to_string(), "status": "unenrolling" },
                    "unenrollment": {
                        "type": "time_delayed",
                        "eligible_at": eligible_at.to_rfc3339(),
                        "message": format!("Unenrollment will complete after {cooldown_hours}-hour cooling-off period.")
                    }
                })))
            }
            "partner_approval" | "authority_approval" => {
                let approver_uuid = policy["requires_approval_from"].as_str();

                let required_approver_id = if let Some(approver_uuid_str) = approver_uuid {
                    let approver_uuid = Uuid::parse_str(approver_uuid_str).map_err(|_| {
                        ApiError::Internal {
                            message: "Invalid approver UUID in policy".into(),
                        }
                    })?;
                    let approver =
                        account_service::get_account_by_public_id(&state.db, approver_uuid)
                            .await?;
                    Some(approver.id)
                } else {
                    None
                };

                enrollment_service::insert_unenroll_request(
                    &state.db,
                    enrollment.id,
                    account.id,
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

                let _ = reason; // Would be included in notification

                Ok(ApiResponse::ok(json!({
                    "device": { "id": device.public_id.to_string(), "status": "unenrolling" },
                    "unenrollment": {
                        "type": policy_type,
                        "requires_approval_from": approver_uuid,
                        "message": "Your accountability partner has been notified and must approve this request."
                    }
                })))
            }
            _ => Err(ApiError::Internal {
                message: "Unknown unenrollment policy type".into(),
            }),
        }
    } else {
        // No active enrollment, just set to unenrolled
        device_service::update_device_status(&state.db, device.id, "unenrolled").await?;

        Ok(ApiResponse::ok(json!({
            "device": { "id": device.public_id.to_string(), "status": "unenrolled" },
            "unenrollment": null
        })))
    }
}

// ---------------------------------------------------------------------------
// POST /devices/:id/heartbeat
// ---------------------------------------------------------------------------

pub async fn heartbeat(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path(id): Path<Uuid>,
    Json(req): Json<HeartbeatRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let device = device_service::get_device_by_public_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound {
            code: "DEVICE_NOT_FOUND".into(),
            message: "Device does not exist".into(),
        })?;

    // Verify caller owns this device
    let account =
        account_service::get_account_by_public_id(&state.db, auth.account_id).await?;
    if device.account_id != account.id {
        return Err(ApiError::Forbidden {
            message: "Device does not belong to authenticated account".into(),
        });
    }

    // Update heartbeat data
    device_service::update_heartbeat(
        &state.db,
        device.id,
        &req.agent_version,
        &req.os_version,
        req.blocklist_version,
    )
    .await?;

    // Compute pending commands
    let mut commands: Vec<serde_json::Value> = Vec::new();

    // Check if blocklist needs update
    if let Ok(Some(latest)) =
        crate::services::blocklist_service::get_latest_version(&state.db).await
    {
        if latest.version_number > req.blocklist_version {
            commands.push(json!({
                "type": "update_blocklist",
                "params": { "target_version": latest.version_number }
            }));
        }
    }

    Ok(ApiResponse::ok(json!({
        "ack": true,
        "server_time": chrono::Utc::now().to_rfc3339(),
        "next_heartbeat_seconds": 300,
        "commands": commands
    })))
}

// ---------------------------------------------------------------------------
// GET /devices/:id/config
// ---------------------------------------------------------------------------

pub async fn get_device_config(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let device = device_service::get_device_by_public_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound {
            code: "DEVICE_NOT_FOUND".into(),
            message: "Device does not exist".into(),
        })?;

    // Verify caller owns this device
    let account =
        account_service::get_account_by_public_id(&state.db, auth.account_id).await?;
    if device.account_id != account.id {
        return Err(ApiError::Forbidden {
            message: "Device does not belong to authenticated account".into(),
        });
    }

    // Get active enrollment
    let enrollment =
        enrollment_service::get_active_enrollment_for_device(&state.db, device.id).await?;

    let enrollment_data = enrollment.map(|e| {
        json!({
            "id": e.public_id.to_string(),
            "tier": e.tier,
            "status": e.status,
            "protection_config": e.protection_config,
            "reporting_config": e.reporting_config,
        })
    });

    // Get latest blocklist version
    let blocklist = crate::services::blocklist_service::get_latest_version(&state.db)
        .await
        .ok()
        .flatten();

    let blocklist_data = blocklist.map(|bv| {
        json!({
            "current_version": bv.version_number,
            "download_url": format!(
                "/v1/blocklist/delta?from_version={}",
                device.blocklist_version.unwrap_or(0)
            )
        })
    });

    Ok(ApiResponse::ok(json!({
        "device_id": device.public_id.to_string(),
        "enrollment": enrollment_data,
        "blocklist": blocklist_data,
        "heartbeat": {
            "interval_seconds": 300,
            "missed_threshold": 3
        },
        "agent_update": null
    })))
}

fn device_to_response(d: &device_service::DeviceRow) -> DeviceResponse {
    DeviceResponse {
        id: d.public_id.to_string(),
        account_id: String::new(), // Will be resolved at query time if needed
        name: d.name.clone(),
        platform: d.platform.clone(),
        os_version: d.os_version.clone(),
        agent_version: d.agent_version.clone(),
        hostname: d.hostname.clone(),
        status: d.status.clone(),
        blocklist_version: d.blocklist_version,
        last_heartbeat_at: d.last_heartbeat_at.map(|t| t.to_rfc3339()),
        enrollment_id: None, // Resolved at query time
        created_at: d.created_at.to_rfc3339(),
        updated_at: d.updated_at.to_rfc3339(),
    }
}
