use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;

use crate::error::ApiError;
use crate::extractors::AuthenticatedAccount;
use crate::response::ApiResponse;
use crate::services::{account_service, analytics_service};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct TimeseriesParams {
    pub device_id: i64,
    #[serde(default = "default_period")]
    pub period: String,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

fn default_period() -> String {
    "hourly".to_string()
}

#[derive(Debug, Deserialize)]
pub struct TrendsParams {
    pub device_id: i64,
    /// Comma-separated metric names (optional).
    pub metrics: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SummaryParams {
    pub device_id: i64,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct HeatmapParams {
    pub device_id: i64,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// GET /v1/analytics/timeseries
// ---------------------------------------------------------------------------

pub async fn timeseries(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Query(params): Query<TimeseriesParams>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let account =
        account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    analytics_service::enforce_enrollment_visibility(&state.db, account.id, params.device_id)
        .await?;

    match params.period.as_str() {
        "hourly" | "hour" => {
            let rows =
                analytics_service::get_hourly_stats(&state.db, params.device_id, params.from, params.to)
                    .await?;
            let data: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    json!({
                        "timestamp": r.bucket.to_rfc3339(),
                        "device_id": r.device_id,
                        "event_type": r.event_type,
                        "event_count": r.event_count,
                    })
                })
                .collect();
            Ok(ApiResponse::ok(json!({
                "period": "hourly",
                "data": data,
            })))
        }
        "daily" | "day" => {
            let rows =
                analytics_service::get_daily_stats(&state.db, params.device_id, params.from, params.to)
                    .await?;
            let data: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    json!({
                        "timestamp": r.day.to_rfc3339(),
                        "device_id": r.device_id,
                        "event_type": r.event_type,
                        "event_count": r.event_count,
                    })
                })
                .collect();
            Ok(ApiResponse::ok(json!({
                "period": "daily",
                "data": data,
            })))
        }
        _ => Err(ApiError::Validation {
            message: "period must be 'hourly' or 'daily'".into(),
            details: None,
        }),
    }
}

// ---------------------------------------------------------------------------
// GET /v1/analytics/trends
// ---------------------------------------------------------------------------

pub async fn trends(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Query(params): Query<TrendsParams>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let account =
        account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    analytics_service::enforce_enrollment_visibility(&state.db, account.id, params.device_id)
        .await?;

    let metric_names: Vec<String> = params
        .metrics
        .map(|m| m.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let rows =
        analytics_service::get_trends(&state.db, params.device_id, &metric_names).await?;

    let data: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.id,
                "device_id": r.device_id,
                "metric_name": r.metric_name,
                "metric_value": r.metric_value,
                "computed_at": r.computed_at.to_rfc3339(),
                "period_start": r.period_start.to_rfc3339(),
                "period_end": r.period_end.to_rfc3339(),
            })
        })
        .collect();

    Ok(ApiResponse::ok(json!({ "trends": data })))
}

// ---------------------------------------------------------------------------
// GET /v1/analytics/summary
// ---------------------------------------------------------------------------

pub async fn summary(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Query(params): Query<SummaryParams>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let account =
        account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    analytics_service::enforce_enrollment_visibility(&state.db, account.id, params.device_id)
        .await?;

    let s =
        analytics_service::get_summary(&state.db, params.device_id, params.from, params.to)
            .await?;

    Ok(ApiResponse::ok(json!({
        "total_events": s.total_events,
        "total_blocks": s.total_blocks,
        "total_bypass_attempts": s.total_bypass_attempts,
        "total_tamper_events": s.total_tamper_events,
        "unique_event_types": s.unique_event_types,
    })))
}

// ---------------------------------------------------------------------------
// GET /v1/analytics/heatmap
// ---------------------------------------------------------------------------

pub async fn heatmap(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Query(params): Query<HeatmapParams>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let account =
        account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    analytics_service::enforce_enrollment_visibility(&state.db, account.id, params.device_id)
        .await?;

    let cells =
        analytics_service::get_heatmap(&state.db, params.device_id, params.from, params.to)
            .await?;

    let data: Vec<serde_json::Value> = cells
        .iter()
        .map(|c| {
            json!({
                "hour_of_day": c.hour_of_day,
                "day_of_week": c.day_of_week,
                "event_count": c.event_count,
            })
        })
        .collect();

    Ok(ApiResponse::ok(json!({ "heatmap": data })))
}
