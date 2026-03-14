use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgDevice {
    pub id: i64,
    pub organization_id: i64,
    pub device_id: i64,
    pub assigned_by: Option<i64>,
    pub assigned_at: DateTime<Utc>,
}
