use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::AccountRole;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: i64,
    pub public_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: AccountRole,
    pub email_verified: bool,
    pub mfa_enabled: bool,
    pub timezone: String,
    pub locale: String,
    pub organization_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
