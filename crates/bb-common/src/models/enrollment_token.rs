use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrollmentToken {
    pub id: i64,
    pub public_id: Uuid,
    pub organization_id: i64,
    pub created_by: i64,
    pub label: Option<String>,
    pub protection_config: serde_json::Value,
    pub reporting_config: serde_json::Value,
    pub unenrollment_policy: serde_json::Value,
    pub max_uses: Option<i32>,
    pub uses_count: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
