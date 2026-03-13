use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::{DeviceStatus, Platform};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: i64,
    pub public_id: Uuid,
    pub account_id: i64,
    pub name: String,
    pub platform: Platform,
    pub os_version: String,
    pub agent_version: String,
    pub hostname: String,
    pub hardware_id: String,
    pub status: DeviceStatus,
    pub blocklist_version: Option<i64>,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    pub enrollment_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
