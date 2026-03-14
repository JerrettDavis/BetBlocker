use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::enums::{CrawlerSource, DiscoveryCandidateStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryCandidate {
    pub id: i64,
    pub domain: String,
    pub source: CrawlerSource,
    pub source_metadata: serde_json::Value,
    pub confidence_score: f64,
    pub classification: serde_json::Value,
    pub status: DiscoveryCandidateStatus,
    pub reviewed_by: Option<i64>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_candidate_roundtrips_json() {
        let candidate = DiscoveryCandidate {
            id: 1,
            domain: "example-casino.com".to_string(),
            source: CrawlerSource::LicenseRegistry,
            source_metadata: serde_json::json!({"registry": "mga"}),
            confidence_score: 0.95,
            classification: serde_json::json!({"category": "online_casino"}),
            status: DiscoveryCandidateStatus::Pending,
            reviewed_by: None,
            reviewed_at: None,
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&candidate).unwrap();
        let back: DiscoveryCandidate = serde_json::from_str(&json).unwrap();
        assert_eq!(candidate.domain, back.domain);
        assert_eq!(candidate.confidence_score, back.confidence_score);
    }
}
