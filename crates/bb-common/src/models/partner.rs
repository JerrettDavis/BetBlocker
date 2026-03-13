use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::{PartnerRelationshipStatus, PartnerRole};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartnerRelationship {
    pub id: i64,
    pub public_id: Uuid,
    pub account_id: i64,
    pub partner_account_id: i64,
    pub status: PartnerRelationshipStatus,
    pub role: PartnerRole,
    pub invited_by: i64,
    pub invite_token_hash: Option<String>,
    pub invited_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}
