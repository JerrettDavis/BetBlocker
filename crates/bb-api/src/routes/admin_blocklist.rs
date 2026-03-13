use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::error::ApiError;
use crate::extractors::{Pagination, RequireAdmin};
use crate::response::{ApiResponse, PaginatedResponse};
use crate::services::blocklist_service;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateEntryRequest {
    pub domain: Option<String>,
    pub pattern: Option<String>,
    pub category: String,
    pub evidence_url: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct EntryFilters {
    pub search: Option<String>,
    pub category: Option<String>,
    pub source: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEntryRequest {
    pub category: Option<String>,
    pub status: Option<String>,
    pub evidence_url: Option<String>,
    pub tags: Option<Vec<String>>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReviewQueueFilters {
    pub min_reports: Option<i64>,
    pub min_confidence: Option<f64>,
    pub sort: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveRequest {
    pub action: String, // "promote" or "reject"
    pub category: Option<String>,
}

// ---------------------------------------------------------------------------
// POST /admin/blocklist/entries
// ---------------------------------------------------------------------------

pub async fn create_entry(
    State(state): State<AppState>,
    admin: RequireAdmin,
    Json(req): Json<CreateEntryRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    // Validate: exactly one of domain or pattern must be provided
    if req.domain.is_none() && req.pattern.is_none() {
        return Err(ApiError::Validation {
            message: "Either domain or pattern must be provided".into(),
            details: None,
        });
    }
    if req.domain.is_some() && req.pattern.is_some() {
        return Err(ApiError::Validation {
            message: "Only one of domain or pattern can be provided".into(),
            details: None,
        });
    }

    let domain = req.domain.as_deref().or(req.pattern.as_deref()).unwrap_or_default();

    let caller = crate::services::account_service::get_account_by_public_id(
        &state.db,
        admin.0.account_id,
    )
    .await?;

    let entry = blocklist_service::create_blocklist_entry(
        &state.db,
        domain,
        req.pattern.as_deref(),
        &req.category,
        "curated",
        1.0,
        caller.id,
        req.evidence_url.as_deref(),
        &req.tags,
    )
    .await?;

    Ok(ApiResponse::created(json!({
        "id": entry.public_id.to_string(),
        "domain": entry.domain,
        "pattern": entry.pattern,
        "category": entry.category,
        "source": entry.source,
        "confidence": entry.confidence,
        "status": entry.status,
        "tags": entry.tags,
        "created_at": entry.created_at.to_rfc3339(),
    })))
}

// ---------------------------------------------------------------------------
// GET /admin/blocklist/entries
// ---------------------------------------------------------------------------

pub async fn list_entries(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    pagination: Pagination,
    Query(filters): Query<EntryFilters>,
) -> Result<PaginatedResponse<serde_json::Value>, ApiError> {
    let (entries, total) = blocklist_service::list_blocklist_entries(
        &state.db,
        filters.search.as_deref(),
        filters.category.as_deref(),
        filters.source.as_deref(),
        filters.status.as_deref(),
        pagination.per_page,
        pagination.offset,
    )
    .await?;

    let data: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            json!({
                "id": e.public_id.to_string(),
                "domain": e.domain,
                "pattern": e.pattern,
                "category": e.category,
                "source": e.source,
                "confidence": e.confidence,
                "status": e.status,
                "evidence_url": e.evidence_url,
                "tags": e.tags,
                "blocklist_version_added": e.blocklist_version_added,
                "blocklist_version_removed": e.blocklist_version_removed,
                "created_at": e.created_at.to_rfc3339(),
                "updated_at": e.updated_at.to_rfc3339(),
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
// PATCH /admin/blocklist/entries/:id
// ---------------------------------------------------------------------------

pub async fn update_entry(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateEntryRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let entry = blocklist_service::get_blocklist_entry_by_public_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound {
            code: "ENTRY_NOT_FOUND".into(),
            message: "Blocklist entry not found".into(),
        })?;

    // If status changes to inactive, compute blocklist_version_removed
    let version_removed = if req.status.as_deref() == Some("inactive") {
        let latest = blocklist_service::get_latest_version(&state.db).await?;
        latest.map(|v| v.id) // Use the version ID (FK reference)
    } else {
        None
    };

    let updated = blocklist_service::update_blocklist_entry(
        &state.db,
        entry.id,
        req.category.as_deref(),
        req.status.as_deref(),
        req.evidence_url.as_deref(),
        req.tags.as_deref(),
        version_removed,
    )
    .await?;

    Ok(ApiResponse::ok(json!({
        "id": updated.public_id.to_string(),
        "domain": updated.domain,
        "category": updated.category,
        "status": updated.status,
        "tags": updated.tags,
        "updated_at": updated.updated_at.to_rfc3339(),
    })))
}

// ---------------------------------------------------------------------------
// DELETE /admin/blocklist/entries/:id
// ---------------------------------------------------------------------------

pub async fn delete_entry(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let entry = blocklist_service::get_blocklist_entry_by_public_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound {
            code: "ENTRY_NOT_FOUND".into(),
            message: "Blocklist entry not found".into(),
        })?;

    // Soft delete: set inactive
    let latest = blocklist_service::get_latest_version(&state.db).await?;
    let version_removed = latest.map(|v| v.id);

    blocklist_service::update_blocklist_entry(
        &state.db,
        entry.id,
        None,
        Some("inactive"),
        None,
        None,
        version_removed,
    )
    .await?;

    Ok(ApiResponse::ok(json!({
        "id": entry.public_id.to_string(),
        "status": "inactive",
    })))
}

// ---------------------------------------------------------------------------
// GET /admin/blocklist/review-queue
// ---------------------------------------------------------------------------

pub async fn review_queue(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    pagination: Pagination,
    Query(filters): Query<ReviewQueueFilters>,
) -> Result<PaginatedResponse<serde_json::Value>, ApiError> {
    let (entries, total) = blocklist_service::get_review_queue(
        &state.db,
        filters.min_reports,
        filters.min_confidence,
        filters.sort.as_deref(),
        pagination.per_page,
        pagination.offset,
    )
    .await?;

    let data: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            json!({
                "domain": e.domain,
                "report_count": e.report_count,
                "first_reported": e.first_reported.to_rfc3339(),
                "last_reported": e.last_reported.to_rfc3339(),
                "avg_confidence": e.avg_confidence,
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
// POST /admin/blocklist/review-queue/:domain/resolve
// ---------------------------------------------------------------------------

pub async fn resolve_review(
    State(state): State<AppState>,
    admin: RequireAdmin,
    Path(domain): Path<String>,
    Json(req): Json<ResolveRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let caller = crate::services::account_service::get_account_by_public_id(
        &state.db,
        admin.0.account_id,
    )
    .await?;

    let resolved_entry_id = if req.action == "promote" {
        // Create a new blocklist entry from the aggregated reports
        let category = req.category.as_deref().unwrap_or("other");
        let entry = blocklist_service::create_blocklist_entry(
            &state.db,
            &domain,
            None,
            category,
            "federated",
            0.8, // default confidence for federated entries
            caller.id,
            None,
            &[],
        )
        .await?;
        Some(entry.id)
    } else {
        None
    };

    let affected = blocklist_service::resolve_review_queue_domain(
        &state.db,
        &domain,
        &req.action,
        caller.id,
        resolved_entry_id,
    )
    .await?;

    Ok(ApiResponse::ok(json!({
        "domain": domain,
        "action": req.action,
        "reports_resolved": affected,
    })))
}
