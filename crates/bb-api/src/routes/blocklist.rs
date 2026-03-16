use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::json;

use crate::error::ApiError;
use crate::extractors::AuthenticatedAccount;
use crate::response::ApiResponse;
use crate::services::blocklist_service;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// GET /blocklist/version
// ---------------------------------------------------------------------------

pub async fn get_version(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let version = blocklist_service::get_latest_version(&state.db)
        .await?
        .ok_or(ApiError::NotFound {
            code: "NO_BLOCKLIST_VERSION".into(),
            message: "No blocklist version has been published yet".into(),
        })?;

    Ok(ApiResponse::ok(json!({
        "version": version.version_number,
        "entry_count": version.entry_count,
        "last_updated_at": version.published_at.to_rfc3339(),
        "signature": hex::encode(&version.signature),
    })))
}

// ---------------------------------------------------------------------------
// GET /blocklist/delta
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct DeltaParams {
    pub from_version: i64,
}

pub async fn get_delta(
    State(state): State<AppState>,
    Query(params): Query<DeltaParams>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let current = blocklist_service::get_latest_version(&state.db)
        .await?
        .ok_or(ApiError::NotFound {
            code: "NO_BLOCKLIST_VERSION".into(),
            message: "No blocklist version has been published yet".into(),
        })?;

    // If more than 100 versions behind, require full sync
    if current.version_number - params.from_version > 100 {
        return Err(ApiError::Conflict {
            code: "FULL_SYNC_REQUIRED".into(),
            message: "Client is too far behind; full sync required".into(),
        });
    }

    let (additions, removals) =
        blocklist_service::get_delta(&state.db, params.from_version, current.version_number)
            .await?;

    Ok(ApiResponse::ok(json!({
        "from_version": params.from_version,
        "to_version": current.version_number,
        "additions": additions,
        "removals": removals,
        "signature": hex::encode(&current.signature),
    })))
}

// ---------------------------------------------------------------------------
// POST /blocklist/report
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ReportRequest {
    pub reports: Vec<SingleReport>,
}

#[derive(Debug, Deserialize)]
pub struct SingleReport {
    pub domain: String,
    pub heuristic_match_type: Option<String>,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
}

fn default_confidence() -> f64 {
    0.5
}

pub async fn submit_report(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Json(req): Json<ReportRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    if req.reports.is_empty() || req.reports.len() > 50 {
        return Err(ApiError::Validation {
            message: "Must submit between 1 and 50 reports".into(),
            details: None,
        });
    }

    // For Phase 1, use the account's first device as the reporting device
    // In production, this would use device cert/token auth
    let account =
        crate::services::account_service::get_account_by_public_id(&state.db, auth.account_id)
            .await?;

    // Find a device for this account (best effort)
    let device_id =
        sqlx::query_scalar::<_, i64>("SELECT id FROM devices WHERE account_id = $1 LIMIT 1")
            .bind(account.id)
            .fetch_optional(&state.db)
            .await?
            .unwrap_or(0);

    let reports: Vec<(String, Option<String>, f64)> = req
        .reports
        .into_iter()
        .map(|r| (r.domain, r.heuristic_match_type, r.confidence))
        .collect();

    let (accepted, duplicates) =
        blocklist_service::insert_federated_reports(&state.db, device_id, &reports).await?;

    Ok(ApiResponse::ok(json!({
        "accepted": accepted,
        "duplicates": duplicates,
        "total_submitted": accepted + duplicates,
    })))
}
