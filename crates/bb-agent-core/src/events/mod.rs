pub mod emitter;
pub mod privacy;
pub mod store;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use bb_common::enums::{EventCategory, EventSeverity, EventType};

/// Local representation of an agent event before it is sent to the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    /// Row ID from SQLite (set after insert).
    pub id: Option<i64>,
    /// The type of event.
    pub event_type: EventType,
    /// Event category (DNS, App, Tamper, etc.).
    pub category: EventCategory,
    /// Severity level.
    pub severity: EventSeverity,
    /// The domain that was blocked, if applicable.
    pub domain: Option<String>,
    /// Which plugin generated this event.
    pub plugin_id: String,
    /// Arbitrary metadata for diagnostics.
    pub metadata: serde_json::Value,
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
    /// Whether this event has been reported to the API.
    pub reported: bool,
}

impl AgentEvent {
    /// Create a new DNS block event.
    pub fn dns_block(domain: &str, plugin_id: &str) -> Self {
        Self {
            id: None,
            event_type: EventType::Block,
            category: EventCategory::Dns,
            severity: EventSeverity::Info,
            domain: Some(domain.to_string()),
            plugin_id: plugin_id.to_string(),
            metadata: serde_json::json!({}),
            timestamp: Utc::now(),
            reported: false,
        }
    }

    /// Create a tamper detection event.
    pub fn tamper_detected(plugin_id: &str, details: &str) -> Self {
        Self {
            id: None,
            event_type: EventType::TamperDetected,
            category: EventCategory::Tamper,
            severity: EventSeverity::Critical,
            domain: None,
            plugin_id: plugin_id.to_string(),
            metadata: serde_json::json!({ "details": details }),
            timestamp: Utc::now(),
            reported: false,
        }
    }

    /// Create a heartbeat event.
    pub fn heartbeat() -> Self {
        Self {
            id: None,
            event_type: EventType::Heartbeat,
            category: EventCategory::Heartbeat,
            severity: EventSeverity::Info,
            domain: None,
            plugin_id: "agent".to_string(),
            metadata: serde_json::json!({}),
            timestamp: Utc::now(),
            reported: false,
        }
    }
}
