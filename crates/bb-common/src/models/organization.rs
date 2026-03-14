use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::OrganizationType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: i64,
    pub public_id: Uuid,
    pub name: String,
    pub org_type: OrganizationType,
    pub owner_id: i64,
    pub default_protection_config: Option<serde_json::Value>,
    pub default_reporting_config: Option<serde_json::Value>,
    pub default_unenrollment_policy: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
