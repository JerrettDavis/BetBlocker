use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::FederatedAggregateStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedReport {
    pub id: i64,
    pub domain: String,
    pub reporter_token: String,
    pub heuristic_score: f64,
    pub category_guess: Option<String>,
    pub reported_at: DateTime<Utc>,
    pub batch_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedAggregate {
    pub id: i64,
    pub domain: String,
    pub unique_reporters: i32,
    pub avg_heuristic_score: f64,
    pub first_reported_at: DateTime<Utc>,
    pub last_reported_at: DateTime<Utc>,
    pub status: FederatedAggregateStatus,
    pub discovery_candidate_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn federated_report_roundtrips_json() {
        let report = FederatedReport {
            id: 1,
            domain: "gambling-site.com".to_string(),
            reporter_token: "tok_abc123".to_string(),
            heuristic_score: 0.87,
            category_guess: Some("sports_betting".to_string()),
            reported_at: Utc::now(),
            batch_id: Uuid::nil(),
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&report).unwrap();
        let back: FederatedReport = serde_json::from_str(&json).unwrap();
        assert_eq!(report.domain, back.domain);
        assert_eq!(report.heuristic_score, back.heuristic_score);
    }

    #[test]
    fn federated_aggregate_roundtrips_json() {
        let agg = FederatedAggregate {
            id: 1,
            domain: "gambling-site.com".to_string(),
            unique_reporters: 5,
            avg_heuristic_score: 0.82,
            first_reported_at: Utc::now(),
            last_reported_at: Utc::now(),
            status: FederatedAggregateStatus::ThresholdMet,
            discovery_candidate_id: Some(42),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&agg).unwrap();
        let back: FederatedAggregate = serde_json::from_str(&json).unwrap();
        assert_eq!(agg.domain, back.domain);
        assert_eq!(agg.unique_reporters, back.unique_reporters);
    }
}
