use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::{AppSignaturePlatform, BlocklistEntryStatus, BlocklistSource, GamblingCategory};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlocklistEntry {
    pub id: i64,
    pub public_id: Uuid,
    pub domain: Option<String>,
    pub pattern: Option<String>,
    pub category: GamblingCategory,
    pub source: BlocklistSource,
    pub confidence: f64,
    pub status: BlocklistEntryStatus,
    pub added_by: Option<i64>,
    pub reviewed_by: Option<i64>,
    pub evidence_url: Option<String>,
    pub tags: Vec<String>,
    pub blocklist_version_added: Option<i64>,
    pub blocklist_version_removed: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlocklistVersion {
    pub id: i64,
    pub version_number: i64,
    pub entry_count: i64,
    pub signature: Vec<u8>,
    pub published_at: DateTime<Utc>,
}

/// Delta between two blocklist versions for sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlocklistDelta {
    pub from_version: i64,
    pub to_version: i64,
    pub added: Vec<BlocklistDeltaEntry>,
    pub removed: Vec<String>,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlocklistDeltaEntry {
    pub domain: String,
    pub category: GamblingCategory,
    pub confidence: f64,
}

/// Delta entry for app signature changes (subset of AppSignature for sync).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSignatureDeltaEntry {
    pub public_id: Uuid,
    pub name: String,
    pub package_names: Vec<String>,
    pub executable_names: Vec<String>,
    pub cert_hashes: Vec<String>,
    pub display_name_patterns: Vec<String>,
    pub platforms: Vec<AppSignaturePlatform>,
    pub category: GamblingCategory,
    pub confidence: f64,
}
