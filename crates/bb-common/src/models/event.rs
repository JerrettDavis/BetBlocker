use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::{EventCategory, EventSeverity, EventType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: i64,
    pub public_id: Uuid,
    pub device_id: i64,
    pub enrollment_id: i64,
    pub event_type: EventType,
    pub category: EventCategory,
    pub severity: EventSeverity,
    pub metadata: serde_json::Value,
    pub occurred_at: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
}
