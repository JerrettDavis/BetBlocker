use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::enums::OrgMemberRole;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgMember {
    pub id: i64,
    pub organization_id: i64,
    pub account_id: i64,
    pub role: OrgMemberRole,
    pub invited_by: Option<i64>,
    pub joined_at: DateTime<Utc>,
}
