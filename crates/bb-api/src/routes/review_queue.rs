use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::json;

use crate::error::ApiError;
use crate::extractors::{Pagination, RequireAdmin};
use crate::response::{ApiResponse, PaginatedResponse};
use crate::services::{account_service, review_queue_service};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ReviewQueueFilters {
    pub status: Option<String>,
    pub source: Option<String>,
    pub min_confidence: Option<f64>,
    pub search: Option<String>,
    pub sort_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApproveRequest {
    pub category: String,
}

#[derive(Debug, Deserialize)]
pub struct BulkApproveRequest {
    pub ids: Vec<i64>,
    pub category: String,
}

#[derive(Debug, Deserialize)]
pub struct BulkRejectRequest {
    pub ids: Vec<i64>,
}

// ---------------------------------------------------------------------------
// GET /v1/admin/review-queue
// ---------------------------------------------------------------------------

pub async fn list_items(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    pagination: Pagination,
    Query(filters): Query<ReviewQueueFilters>,
) -> Result<PaginatedResponse<serde_json::Value>, ApiError> {
    let svc_filters = review_queue_service::ReviewFilters {
        status: filters.status,
        source: filters.source,
        min_confidence: filters.min_confidence,
        search: filters.search,
        sort_by: filters.sort_by,
    };

    let (items, total) = review_queue_service::list_review_items(
        &state.db,
        &svc_filters,
        pagination.per_page,
        pagination.offset,
    )
    .await?;

    let data: Vec<serde_json::Value> = items
        .iter()
        .map(|item| {
            json!({
                "id": item.id,
                "domain": item.domain,
                "source": item.source,
                "source_metadata": item.source_metadata,
                "confidence_score": item.confidence_score,
                "classification": item.classification,
                "status": item.status,
                "reviewed_by": item.reviewed_by,
                "reviewed_at": item.reviewed_at.map(|t| t.to_rfc3339()),
                "created_at": item.created_at.to_rfc3339(),
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
// GET /v1/admin/review-queue/{id}
// ---------------------------------------------------------------------------

pub async fn get_item(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let item = review_queue_service::get_review_item(&state.db, id).await?;

    Ok(ApiResponse::ok(json!({
        "id": item.id,
        "domain": item.domain,
        "source": item.source,
        "source_metadata": item.source_metadata,
        "confidence_score": item.confidence_score,
        "classification": item.classification,
        "status": item.status,
        "reviewed_by": item.reviewed_by,
        "reviewed_at": item.reviewed_at.map(|t| t.to_rfc3339()),
        "created_at": item.created_at.to_rfc3339(),
    })))
}

// ---------------------------------------------------------------------------
// POST /v1/admin/review-queue/{id}/approve
// ---------------------------------------------------------------------------

pub async fn approve_item(
    State(state): State<AppState>,
    admin: RequireAdmin,
    Path(id): Path<i64>,
    Json(req): Json<ApproveRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, admin.0.account_id).await?;

    review_queue_service::approve_item(&state.db, id, caller.id, &req.category).await?;

    Ok(ApiResponse::ok(json!({
        "id": id,
        "status": "approved",
    })))
}

// ---------------------------------------------------------------------------
// POST /v1/admin/review-queue/{id}/reject
// ---------------------------------------------------------------------------

pub async fn reject_item(
    State(state): State<AppState>,
    admin: RequireAdmin,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, admin.0.account_id).await?;

    review_queue_service::reject_item(&state.db, id, caller.id).await?;

    Ok(ApiResponse::ok(json!({
        "id": id,
        "status": "rejected",
    })))
}

// ---------------------------------------------------------------------------
// POST /v1/admin/review-queue/{id}/defer
// ---------------------------------------------------------------------------

pub async fn defer_item(
    State(state): State<AppState>,
    admin: RequireAdmin,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller = account_service::get_account_by_public_id(&state.db, admin.0.account_id).await?;

    review_queue_service::defer_item(&state.db, id, caller.id).await?;

    Ok(ApiResponse::ok(json!({
        "id": id,
        "status": "deferred",
    })))
}

// ---------------------------------------------------------------------------
// POST /v1/admin/review-queue/bulk-approve
// ---------------------------------------------------------------------------

pub async fn bulk_approve(
    State(state): State<AppState>,
    admin: RequireAdmin,
    Json(req): Json<BulkApproveRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    if req.ids.is_empty() {
        return Err(ApiError::Validation {
            message: "ids must not be empty".into(),
            details: None,
        });
    }

    let caller = account_service::get_account_by_public_id(&state.db, admin.0.account_id).await?;

    let count =
        review_queue_service::bulk_approve(&state.db, &req.ids, caller.id, &req.category).await?;

    Ok(ApiResponse::ok(json!({
        "approved": count,
    })))
}

// ---------------------------------------------------------------------------
// POST /v1/admin/review-queue/bulk-reject
// ---------------------------------------------------------------------------

pub async fn bulk_reject(
    State(state): State<AppState>,
    admin: RequireAdmin,
    Json(req): Json<BulkRejectRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    if req.ids.is_empty() {
        return Err(ApiError::Validation {
            message: "ids must not be empty".into(),
            details: None,
        });
    }

    let caller = account_service::get_account_by_public_id(&state.db, admin.0.account_id).await?;

    let count = review_queue_service::bulk_reject(&state.db, &req.ids, caller.id).await?;

    Ok(ApiResponse::ok(json!({
        "rejected": count,
    })))
}
