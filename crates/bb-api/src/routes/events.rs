use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;

use crate::error::ApiError;
use crate::extractors::{AuthenticatedAccount, Pagination};
use crate::response::{ApiResponse, PaginatedResponse};
use crate::services::{account_service, event_service};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct BatchEventRequest {
    pub events: Vec<event_service::EventInput>,
}

#[derive(Debug, Deserialize)]
pub struct EventFilters {
    pub device_id: Option<i64>,
    pub enrollment_id: Option<i64>,
    pub event_type: Option<String>,
    pub category: Option<String>,
    pub severity: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct SummaryParams {
    #[serde(default = "default_period")]
    pub period: String,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

fn default_period() -> String {
    "day".to_string()
}

// ---------------------------------------------------------------------------
// POST /events
// ---------------------------------------------------------------------------

pub async fn batch_ingest(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Json(req): Json<BatchEventRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    if req.events.is_empty() || req.events.len() > 100 {
        return Err(ApiError::Validation {
            message: "Must submit between 1 and 100 events".into(),
            details: None,
        });
    }

    let account = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    // Find the caller's device and enrollment
    // For Phase 1, get the first device with an active enrollment
    let device_row = sqlx::query_as::<_, (i64, Option<i64>)>(
        r#"SELECT d.id, e.id as enrollment_id
           FROM devices d
           LEFT JOIN enrollments e ON e.device_id = d.id AND e.status = 'active'
           WHERE d.account_id = $1
           LIMIT 1"#,
    )
    .bind(account.id)
    .fetch_optional(&state.db)
    .await?;

    let (device_id, enrollment_id) = device_row.unwrap_or((0, None));

    // Apply reporting_config filtering if enrollment exists
    let filtered_events = if let Some(_eid) = enrollment_id {
        let enrollment = crate::services::enrollment_service::get_enrollment_by_public_id(
            &state.db,
            uuid::Uuid::nil(),
        )
        .await
        .ok()
        .flatten();
        // For simplicity in Phase 1, pass through all events.
        // Full filtering based on reporting_config would strip domain_details etc.
        let _ = enrollment;
        req.events
    } else {
        req.events
    };

    let (accepted, rejected, errors) =
        event_service::batch_insert_events(&state.db, device_id, enrollment_id, &filtered_events)
            .await?;

    Ok(ApiResponse::ok(json!({
        "accepted": accepted,
        "rejected": rejected,
        "errors": errors,
    })))
}

// ---------------------------------------------------------------------------
// GET /events
// ---------------------------------------------------------------------------

pub async fn query_events(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    pagination: Pagination,
    Query(filters): Query<EventFilters>,
) -> Result<PaginatedResponse<serde_json::Value>, ApiError> {
    let account = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    // Get all enrollment IDs visible to this account
    let visible_ids = event_service::get_visible_enrollment_ids(&state.db, account.id).await?;

    if visible_ids.is_empty() {
        return Ok(PaginatedResponse::new(
            vec![],
            0,
            pagination.page,
            pagination.per_page,
        ));
    }

    let (events, total) = event_service::query_events(
        &state.db,
        &visible_ids,
        filters.device_id,
        filters.enrollment_id,
        filters.event_type.as_deref(),
        filters.category.as_deref(),
        filters.severity.as_deref(),
        filters.from,
        filters.to,
        pagination.per_page,
        pagination.offset,
    )
    .await?;

    let data: Vec<serde_json::Value> = events
        .iter()
        .map(|e| {
            json!({
                "id": e.public_id.to_string(),
                "device_id": e.device_id,
                "enrollment_id": e.enrollment_id,
                "type": e.event_type,
                "category": e.category,
                "severity": e.severity,
                "payload": e.metadata,
                "occurred_at": e.occurred_at.to_rfc3339(),
                "received_at": e.received_at.to_rfc3339(),
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
// GET /events/summary
// ---------------------------------------------------------------------------

pub async fn event_summary(
    State(state): State<AppState>,
    auth: AuthenticatedAccount,
    Query(params): Query<SummaryParams>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let account = account_service::get_account_by_public_id(&state.db, auth.account_id).await?;

    let visible_ids = event_service::get_visible_enrollment_ids(&state.db, account.id).await?;

    if visible_ids.is_empty() {
        return Ok(ApiResponse::ok(json!({
            "totals": {
                "total_blocks": 0,
                "total_bypass_attempts": 0,
                "total_tamper_events": 0,
            },
            "timeseries": [],
        })));
    }

    let buckets = event_service::get_event_summary(
        &state.db,
        &visible_ids,
        &params.period,
        params.from,
        params.to,
    )
    .await?;

    let total_blocks: i64 = buckets.iter().map(|b| b.total_blocks).sum();
    let total_bypass: i64 = buckets.iter().map(|b| b.total_bypass_attempts).sum();
    let total_tamper: i64 = buckets.iter().map(|b| b.total_tamper_events).sum();

    let timeseries: Vec<serde_json::Value> = buckets
        .iter()
        .map(|b| {
            json!({
                "period": b.period.to_rfc3339(),
                "total_blocks": b.total_blocks,
                "total_bypass_attempts": b.total_bypass_attempts,
                "total_tamper_events": b.total_tamper_events,
                "total_events": b.total_events,
            })
        })
        .collect();

    Ok(ApiResponse::ok(json!({
        "totals": {
            "total_blocks": total_blocks,
            "total_bypass_attempts": total_bypass,
            "total_tamper_events": total_tamper,
        },
        "timeseries": timeseries,
    })))
}
