use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// Standard API response envelope wrapping `data` and `meta`.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub data: T,
    pub meta: ResponseMeta,
}

/// Metadata included in every API response.
#[derive(Debug, Serialize)]
pub struct ResponseMeta {
    pub timestamp: String,
}

impl Default for ResponseMeta {
    fn default() -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> (StatusCode, Json<Self>) {
        (
            StatusCode::OK,
            Json(Self {
                data,
                meta: ResponseMeta::default(),
            }),
        )
    }

    pub fn created(data: T) -> (StatusCode, Json<Self>) {
        (
            StatusCode::CREATED,
            Json(Self {
                data,
                meta: ResponseMeta::default(),
            }),
        )
    }

    pub fn accepted(data: T) -> (StatusCode, Json<Self>) {
        (
            StatusCode::ACCEPTED,
            Json(Self {
                data,
                meta: ResponseMeta::default(),
            }),
        )
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// Paginated API response envelope.
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub meta: ResponseMeta,
    pub pagination: PaginationMeta,
}

/// Pagination metadata included in list responses.
#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

impl<T: Serialize> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: i64, page: i64, per_page: i64) -> Self {
        let total_pages = if per_page > 0 {
            (total + per_page - 1) / per_page
        } else {
            0
        };
        Self {
            data,
            meta: ResponseMeta::default(),
            pagination: PaginationMeta {
                total,
                page,
                per_page,
                total_pages,
            },
        }
    }
}

impl<T: Serialize> IntoResponse for PaginatedResponse<T> {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
