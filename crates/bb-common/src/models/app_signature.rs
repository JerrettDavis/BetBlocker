use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::{AppSignaturePlatform, AppSignatureStatus, BlocklistSource, GamblingCategory};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSignature {
    pub id: i64,
    pub public_id: Uuid,
    pub name: String,
    pub package_names: Vec<String>,
    pub executable_names: Vec<String>,
    pub cert_hashes: Vec<String>,
    pub display_name_patterns: Vec<String>,
    pub platforms: Vec<AppSignaturePlatform>,
    pub category: GamblingCategory,
    pub status: AppSignatureStatus,
    pub confidence: f64,
    pub source: BlocklistSource,
    pub evidence_url: Option<String>,
    pub tags: Vec<String>,
    pub blocklist_version_added: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_signature_roundtrips_json() {
        let sig = AppSignature {
            id: 1,
            public_id: Uuid::nil(),
            name: "Test Casino App".to_string(),
            package_names: vec!["com.casino.test".to_string()],
            executable_names: vec!["casino.exe".to_string()],
            cert_hashes: vec!["abc123".to_string()],
            display_name_patterns: vec!["Casino*".to_string()],
            platforms: vec![AppSignaturePlatform::Windows, AppSignaturePlatform::Android],
            category: GamblingCategory::OnlineCasino,
            status: AppSignatureStatus::Active,
            confidence: 0.95,
            source: BlocklistSource::Curated,
            evidence_url: Some("https://example.com/evidence".to_string()),
            tags: vec!["high-priority".to_string()],
            blocklist_version_added: Some(1),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&sig).unwrap();
        let back: AppSignature = serde_json::from_str(&json).unwrap();
        assert_eq!(sig.name, back.name);
        assert_eq!(sig.platforms.len(), back.platforms.len());
        assert_eq!(sig.package_names, back.package_names);
    }
}
