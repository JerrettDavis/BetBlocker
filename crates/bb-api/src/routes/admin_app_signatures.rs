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
use crate::services::app_signature_service;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateAppSigRequest {
    pub name: String,
    #[serde(default)]
    pub package_names: Vec<String>,
    #[serde(default)]
    pub executable_names: Vec<String>,
    #[serde(default)]
    pub cert_hashes: Vec<String>,
    #[serde(default)]
    pub display_name_patterns: Vec<String>,
    #[serde(default)]
    pub platforms: Vec<String>,
    pub category: String,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    #[serde(default = "default_source")]
    pub source: String,
    pub evidence_url: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_status() -> String {
    "pending_review".into()
}

fn default_confidence() -> f64 {
    0.0
}

fn default_source() -> String {
    "curated".into()
}

#[derive(Debug, Deserialize)]
pub struct UpdateAppSigRequest {
    pub name: Option<String>,
    pub package_names: Option<Vec<String>>,
    pub executable_names: Option<Vec<String>>,
    pub cert_hashes: Option<Vec<String>>,
    pub display_name_patterns: Option<Vec<String>>,
    pub platforms: Option<Vec<String>>,
    pub category: Option<String>,
    pub status: Option<String>,
    pub confidence: Option<f64>,
    pub source: Option<String>,
    pub evidence_url: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct AppSigFilters {
    pub search: Option<String>,
    pub category: Option<String>,
    pub platform: Option<String>,
    pub status: Option<String>,
}

// ---------------------------------------------------------------------------
// POST /v1/admin/app-signatures
// ---------------------------------------------------------------------------

pub async fn create_signature(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Json(req): Json<CreateAppSigRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    if req.name.trim().is_empty() {
        return Err(ApiError::Validation {
            message: "Name cannot be empty".into(),
            details: None,
        });
    }

    let sig = app_signature_service::create_signature(
        &state.db,
        &req.name,
        &req.package_names,
        &req.executable_names,
        &req.cert_hashes,
        &req.display_name_patterns,
        &req.platforms,
        &req.category,
        &req.status,
        req.confidence,
        &req.source,
        req.evidence_url.as_deref(),
        &req.tags,
    )
    .await?;

    Ok(ApiResponse::created(json!({
        "id": sig.public_id.to_string(),
        "name": sig.name,
        "package_names": sig.package_names,
        "executable_names": sig.executable_names,
        "cert_hashes": sig.cert_hashes,
        "display_name_patterns": sig.display_name_patterns,
        "platforms": sig.platforms,
        "category": sig.category,
        "status": sig.status,
        "confidence": sig.confidence,
        "source": sig.source,
        "evidence_url": sig.evidence_url,
        "tags": sig.tags,
        "created_at": sig.created_at.to_rfc3339(),
        "updated_at": sig.updated_at.to_rfc3339(),
    })))
}

// ---------------------------------------------------------------------------
// GET /v1/admin/app-signatures
// ---------------------------------------------------------------------------

pub async fn list_signatures(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    pagination: Pagination,
    Query(filters): Query<AppSigFilters>,
) -> Result<PaginatedResponse<serde_json::Value>, ApiError> {
    let (sigs, total) = app_signature_service::list_signatures(
        &state.db,
        filters.search.as_deref(),
        filters.category.as_deref(),
        filters.platform.as_deref(),
        filters.status.as_deref(),
        pagination.per_page,
        pagination.offset,
    )
    .await?;

    let data: Vec<serde_json::Value> = sigs
        .iter()
        .map(|s| {
            json!({
                "id": s.public_id.to_string(),
                "name": s.name,
                "package_names": s.package_names,
                "executable_names": s.executable_names,
                "platforms": s.platforms,
                "category": s.category,
                "status": s.status,
                "confidence": s.confidence,
                "source": s.source,
                "tags": s.tags,
                "created_at": s.created_at.to_rfc3339(),
                "updated_at": s.updated_at.to_rfc3339(),
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
// GET /v1/admin/app-signatures/{id}
// ---------------------------------------------------------------------------

pub async fn get_signature(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let sig = app_signature_service::get_signature(&state.db, id).await?;

    Ok(ApiResponse::ok(json!({
        "id": sig.public_id.to_string(),
        "name": sig.name,
        "package_names": sig.package_names,
        "executable_names": sig.executable_names,
        "cert_hashes": sig.cert_hashes,
        "display_name_patterns": sig.display_name_patterns,
        "platforms": sig.platforms,
        "category": sig.category,
        "status": sig.status,
        "confidence": sig.confidence,
        "source": sig.source,
        "evidence_url": sig.evidence_url,
        "tags": sig.tags,
        "created_at": sig.created_at.to_rfc3339(),
        "updated_at": sig.updated_at.to_rfc3339(),
    })))
}

// ---------------------------------------------------------------------------
// PUT /v1/admin/app-signatures/{id}
// ---------------------------------------------------------------------------

pub async fn update_signature(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateAppSigRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let existing = app_signature_service::get_signature(&state.db, id).await?;

    let sig = app_signature_service::update_signature(
        &state.db,
        existing.id,
        req.name.as_deref(),
        req.package_names.as_deref(),
        req.executable_names.as_deref(),
        req.cert_hashes.as_deref(),
        req.display_name_patterns.as_deref(),
        req.platforms.as_deref(),
        req.category.as_deref(),
        req.status.as_deref(),
        req.confidence,
        req.source.as_deref(),
        req.evidence_url.as_deref(),
        req.tags.as_deref(),
    )
    .await?;

    Ok(ApiResponse::ok(json!({
        "id": sig.public_id.to_string(),
        "name": sig.name,
        "package_names": sig.package_names,
        "executable_names": sig.executable_names,
        "cert_hashes": sig.cert_hashes,
        "display_name_patterns": sig.display_name_patterns,
        "platforms": sig.platforms,
        "category": sig.category,
        "status": sig.status,
        "confidence": sig.confidence,
        "source": sig.source,
        "evidence_url": sig.evidence_url,
        "tags": sig.tags,
        "created_at": sig.created_at.to_rfc3339(),
        "updated_at": sig.updated_at.to_rfc3339(),
    })))
}

// ---------------------------------------------------------------------------
// DELETE /v1/admin/app-signatures/{id}
// ---------------------------------------------------------------------------

pub async fn delete_signature(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    let existing = app_signature_service::get_signature(&state.db, id).await?;

    app_signature_service::delete_signature(&state.db, existing.id).await?;

    Ok(ApiResponse::ok(json!({
        "deleted": true,
        "id": id.to_string(),
    })))
}
