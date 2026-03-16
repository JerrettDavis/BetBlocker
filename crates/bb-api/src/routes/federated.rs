use axum::{Json, extract::State, http::StatusCode};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ApiError;
use crate::response::ApiResponse;
use crate::services::federated_service;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

/// A single domain report submitted by a federated reporter.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReportPayload {
    /// The domain being reported (e.g. "gambling-site.com").
    pub domain: String,
    /// Anonymous reporter token – used to count unique reporters, not for identity.
    pub reporter_token: String,
    /// Heuristic score computed client-side (0.0–1.0).
    pub heuristic_score: f64,
    /// Client-side category guess, if any.
    pub category_guess: Option<String>,
    /// When the reporter observed the domain.
    pub reported_at: DateTime<Utc>,
    /// Client-side batch identifier for deduplication.
    pub batch_id: Uuid,
}

/// Request body for `POST /v1/federated/reports`.
#[derive(Debug, Deserialize)]
pub struct IngestReportRequest {
    pub reports: Vec<ReportPayload>,
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

const MAX_BATCH_SIZE: usize = 500;

fn validate_payload(req: &IngestReportRequest) -> Result<(), ApiError> {
    if req.reports.is_empty() {
        return Err(ApiError::Validation {
            message: "reports must not be empty".into(),
            details: None,
        });
    }

    if req.reports.len() > MAX_BATCH_SIZE {
        return Err(ApiError::Validation {
            message: format!(
                "batch size {len} exceeds limit of {MAX_BATCH_SIZE}",
                len = req.reports.len()
            ),
            details: None,
        });
    }

    for (i, r) in req.reports.iter().enumerate() {
        if r.domain.is_empty() {
            return Err(ApiError::Validation {
                message: format!("report[{i}].domain is empty"),
                details: None,
            });
        }
        if r.reporter_token.is_empty() {
            return Err(ApiError::Validation {
                message: format!("report[{i}].reporter_token is empty"),
                details: None,
            });
        }
        if !(0.0..=1.0).contains(&r.heuristic_score) {
            return Err(ApiError::Validation {
                message: format!(
                    "report[{i}].heuristic_score must be in 0.0–1.0, got {}",
                    r.heuristic_score
                ),
                details: None,
            });
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// POST /v1/federated/reports
// ---------------------------------------------------------------------------

/// Ingest a batch of federated domain reports.
///
/// This endpoint is intentionally unauthenticated so reporters can remain
/// anonymous.  `StripSourceIp` middleware is applied at the router level to
/// prevent IP-based fingerprinting.
pub async fn ingest_reports(
    State(state): State<AppState>,
    Json(req): Json<IngestReportRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    validate_payload(&req)?;

    let count = req.reports.len();
    federated_service::ingest(&state.db, req.reports).await?;

    Ok(ApiResponse::accepted(serde_json::json!({
        "ingested": count,
    })))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_payload(count: usize) -> IngestReportRequest {
        IngestReportRequest {
            reports: (0..count)
                .map(|i| ReportPayload {
                    domain: format!("site-{i}.example.com"),
                    reporter_token: format!("tok_{i}"),
                    heuristic_score: 0.5,
                    category_guess: None,
                    reported_at: Utc::now(),
                    batch_id: Uuid::nil(),
                })
                .collect(),
        }
    }

    #[test]
    fn validate_empty_reports_rejected() {
        let req = IngestReportRequest { reports: vec![] };
        assert!(validate_payload(&req).is_err());
    }

    #[test]
    fn validate_oversized_batch_rejected() {
        let req = make_payload(MAX_BATCH_SIZE + 1);
        let err = validate_payload(&req).unwrap_err();
        match err {
            ApiError::Validation { message, .. } => assert!(message.contains("exceeds limit")),
            _ => panic!("expected Validation error"),
        }
    }

    #[test]
    fn validate_empty_domain_rejected() {
        let req = IngestReportRequest {
            reports: vec![ReportPayload {
                domain: "".to_string(),
                reporter_token: "tok_1".to_string(),
                heuristic_score: 0.5,
                category_guess: None,
                reported_at: Utc::now(),
                batch_id: Uuid::nil(),
            }],
        };
        let err = validate_payload(&req).unwrap_err();
        match err {
            ApiError::Validation { message, .. } => assert!(message.contains("domain is empty")),
            _ => panic!("expected Validation error"),
        }
    }

    #[test]
    fn validate_out_of_range_score_rejected() {
        let req = IngestReportRequest {
            reports: vec![ReportPayload {
                domain: "site.com".to_string(),
                reporter_token: "tok_1".to_string(),
                heuristic_score: 1.5,
                category_guess: None,
                reported_at: Utc::now(),
                batch_id: Uuid::nil(),
            }],
        };
        let err = validate_payload(&req).unwrap_err();
        match err {
            ApiError::Validation { message, .. } => assert!(message.contains("heuristic_score")),
            _ => panic!("expected Validation error"),
        }
    }

    #[test]
    fn validate_valid_batch_passes() {
        let req = make_payload(3);
        assert!(validate_payload(&req).is_ok());
    }

    #[test]
    fn report_payload_roundtrips_json() {
        let payload = ReportPayload {
            domain: "gambling.example.com".to_string(),
            reporter_token: "tok_abc".to_string(),
            heuristic_score: 0.75,
            category_guess: Some("sports_betting".to_string()),
            reported_at: Utc::now(),
            batch_id: Uuid::nil(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        let back: ReportPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(payload.domain, back.domain);
        assert_eq!(payload.heuristic_score, back.heuristic_score);
        assert_eq!(payload.category_guess, back.category_guess);
    }
}
