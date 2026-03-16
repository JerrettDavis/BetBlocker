use axum::{
    Json,
    body::Body,
    extract::{Query, State},
    http::{StatusCode, header},
    response::Response,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;

use crate::error::ApiError;
use crate::extractors::AuthenticatedAccount;
use crate::response::ApiResponse;
use crate::services::{account_service, analytics_service, export_service};
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
    let account = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    analytics_service::enforce_enrollment_visibility(&state.db, account.id, params.device_id)
        .await?;

    match params.period.as_str() {
        "hourly" | "hour" => {
            let rows = analytics_service::get_hourly_stats(
                &state.db,
                params.device_id,
                params.from,
                params.to,
            )
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
            let rows = analytics_service::get_daily_stats(
                &state.db,
                params.device_id,
                params.from,
                params.to,
            )
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
    let account = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    analytics_service::enforce_enrollment_visibility(&state.db, account.id, params.device_id)
        .await?;

    let metric_names: Vec<String> = params
        .metrics
        .map(|m| m.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let rows = analytics_service::get_trends(&state.db, params.device_id, &metric_names).await?;

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
    let account = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    analytics_service::enforce_enrollment_visibility(&state.db, account.id, params.device_id)
        .await?;

    let s =
        analytics_service::get_summary(&state.db, params.device_id, params.from, params.to).await?;

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
    let account = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    analytics_service::enforce_enrollment_visibility(&state.db, account.id, params.device_id)
        .await?;

    let cells =
        analytics_service::get_heatmap(&state.db, params.device_id, params.from, params.to).await?;

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

// ---------------------------------------------------------------------------
// Export query params (shared by CSV and PDF)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ExportParams {
    pub device_id: i64,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// GET /v1/analytics/export/csv
// ---------------------------------------------------------------------------

/// Export daily analytics as a CSV file download.
///
/// TODO (routes/mod.rs): Register as:
///   .route("/export/csv", get(analytics::export_csv))
pub async fn export_csv(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Query(params): Query<ExportParams>,
) -> Result<Response<Body>, ApiError> {
    let account = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    analytics_service::enforce_enrollment_visibility(&state.db, account.id, params.device_id)
        .await?;

    let csv_bytes =
        export_service::generate_csv_report(&state.db, params.device_id, params.from, params.to)
            .await?;

    let filename = format!(
        "analytics_device{}_{}_to_{}.csv",
        params.device_id,
        params.from.format("%Y%m%d"),
        params.to.format("%Y%m%d"),
    );

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .header(header::CONTENT_LENGTH, csv_bytes.len())
        .body(Body::from(csv_bytes))
        .map_err(|e| ApiError::Internal {
            message: format!("Response build error: {e}"),
        })?;

    Ok(response)
}

// ---------------------------------------------------------------------------
// GET /v1/analytics/export/pdf
// ---------------------------------------------------------------------------

/// Export analytics as a PDF file download.
///
/// TODO (routes/mod.rs): Register as:
///   .route("/export/pdf", get(analytics::export_pdf))
pub async fn export_pdf(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Query(params): Query<ExportParams>,
) -> Result<Response<Body>, ApiError> {
    let account = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    analytics_service::enforce_enrollment_visibility(&state.db, account.id, params.device_id)
        .await?;

    let pdf_bytes =
        export_service::generate_pdf_report(&state.db, params.device_id, params.from, params.to)
            .await?;

    let filename = format!(
        "analytics_device{}_{}_to_{}.pdf",
        params.device_id,
        params.from.format("%Y%m%d"),
        params.to.format("%Y%m%d"),
    );

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/pdf")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .header(header::CONTENT_LENGTH, pdf_bytes.len())
        .body(Body::from(pdf_bytes))
        .map_err(|e| ApiError::Internal {
            message: format!("Response build error: {e}"),
        })?;

    Ok(response)
}

// ---------------------------------------------------------------------------
// GET /v1/analytics/org/:org_id/summary  (stub)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct OrgAnalyticsSummaryParams {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

/// Org-scoped analytics summary stub.
///
/// TODO (routes/mod.rs): Register under organization routes or as:
///   .route("/analytics/org/{org_id}/summary", get(analytics::org_summary))
pub async fn org_summary(
    State(_state): State<AppState>,
    auth: AuthenticatedAccount,
    axum::extract::Path(org_id): axum::extract::Path<String>,
    Query(_params): Query<OrgAnalyticsSummaryParams>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    // Stub: returns a placeholder until org-level aggregation is implemented
    let _ = auth;
    Ok(ApiResponse::ok(json!({
        "org_id": org_id,
        "message": "Org-scoped analytics aggregation is not yet implemented",
        "total_events": null,
        "total_blocks": null,
    })))
}
