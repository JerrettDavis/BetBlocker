use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct PartnerRow {
    pub id: i64,
    pub account_id: i64,
    pub partner_account_id: i64,
    pub status: String,
    pub role: String,
    pub invited_by: i64,
    pub invited_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    // Joined fields
    #[sqlx(default)]
    pub account_display_name: Option<String>,
    #[sqlx(default)]
    pub partner_display_name: Option<String>,
    #[sqlx(default)]
    pub account_public_id: Option<Uuid>,
    #[sqlx(default)]
    pub partner_public_id: Option<Uuid>,
}

/// Create a partner invitation.
pub async fn create_partner_invite(
    db: &PgPool,
    account_id: i64,
    partner_account_id: i64,
    role: &str,
    invited_by: i64,
) -> Result<PartnerRow, ApiError> {
    let row = sqlx::query_as::<_, PartnerRow>(
        r#"INSERT INTO partner_relationships (account_id, partner_account_id, role, invited_by)
           VALUES ($1, $2, $3::partner_role, $4)
           RETURNING id, account_id, partner_account_id, status::text, role::text,
                     invited_by, invited_at, accepted_at, revoked_at,
                     NULL::text as account_display_name,
                     NULL::text as partner_display_name,
                     NULL::uuid as account_public_id,
                     NULL::uuid as partner_public_id"#,
    )
    .bind(account_id)
    .bind(partner_account_id)
    .bind(role)
    .bind(invited_by)
    .fetch_one(db)
    .await?;
    Ok(row)
}

/// Accept a partner invitation. Returns the updated row.
pub async fn accept_partner_invite(db: &PgPool, relationship_id: i64) -> Result<(), ApiError> {
    sqlx::query(
        r#"UPDATE partner_relationships
           SET status = 'active'::partner_relationship_status,
               accepted_at = NOW()
           WHERE id = $1 AND status = 'pending'"#,
    )
    .bind(relationship_id)
    .execute(db)
    .await?;
    Ok(())
}

/// Revoke a partner relationship.
pub async fn revoke_partner(db: &PgPool, relationship_id: i64) -> Result<(), ApiError> {
    sqlx::query(
        r#"UPDATE partner_relationships
           SET status = 'revoked'::partner_relationship_status,
               revoked_at = NOW()
           WHERE id = $1"#,
    )
    .bind(relationship_id)
    .execute(db)
    .await?;
    Ok(())
}

/// List partner relationships for an account (both directions).
pub async fn list_partners(
    db: &PgPool,
    account_id: i64,
    limit: i64,
    offset: i64,
) -> Result<(Vec<PartnerRow>, i64), ApiError> {
    let rows = sqlx::query_as::<_, PartnerRow>(
        r#"SELECT pr.id, pr.account_id, pr.partner_account_id,
                  pr.status::text, pr.role::text, pr.invited_by,
                  pr.invited_at, pr.accepted_at, pr.revoked_at,
                  a1.display_name as account_display_name,
                  a2.display_name as partner_display_name,
                  a1.public_id as account_public_id,
                  a2.public_id as partner_public_id
           FROM partner_relationships pr
           JOIN accounts a1 ON a1.id = pr.account_id
           JOIN accounts a2 ON a2.id = pr.partner_account_id
           WHERE pr.account_id = $1 OR pr.partner_account_id = $1
           ORDER BY pr.invited_at DESC
           LIMIT $2 OFFSET $3"#,
    )
    .bind(account_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    let total = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*)
           FROM partner_relationships
           WHERE account_id = $1 OR partner_account_id = $1"#,
    )
    .bind(account_id)
    .fetch_one(db)
    .await?;

    Ok((rows, total))
}

/// Get a partner relationship by ID.
pub async fn get_partner_by_id(
    db: &PgPool,
    relationship_id: i64,
) -> Result<Option<PartnerRow>, ApiError> {
    let row = sqlx::query_as::<_, PartnerRow>(
        r#"SELECT pr.id, pr.account_id, pr.partner_account_id,
                  pr.status::text, pr.role::text, pr.invited_by,
                  pr.invited_at, pr.accepted_at, pr.revoked_at,
                  a1.display_name as account_display_name,
                  a2.display_name as partner_display_name,
                  a1.public_id as account_public_id,
                  a2.public_id as partner_public_id
           FROM partner_relationships pr
           JOIN accounts a1 ON a1.id = pr.account_id
           JOIN accounts a2 ON a2.id = pr.partner_account_id
           WHERE pr.id = $1"#,
    )
    .bind(relationship_id)
    .fetch_optional(db)
    .await?;
    Ok(row)
}

/// Check if an active partner relationship exists between two accounts.
pub async fn has_active_partnership(
    db: &PgPool,
    account_id: i64,
    partner_account_id: i64,
) -> Result<Option<PartnerRow>, ApiError> {
    let row = sqlx::query_as::<_, PartnerRow>(
        r#"SELECT pr.id, pr.account_id, pr.partner_account_id,
                  pr.status::text, pr.role::text, pr.invited_by,
                  pr.invited_at, pr.accepted_at, pr.revoked_at,
                  NULL::text as account_display_name,
                  NULL::text as partner_display_name,
                  NULL::uuid as account_public_id,
                  NULL::uuid as partner_public_id
           FROM partner_relationships pr
           WHERE pr.status = 'active'
             AND ((pr.account_id = $1 AND pr.partner_account_id = $2)
               OR (pr.account_id = $2 AND pr.partner_account_id = $1))"#,
    )
    .bind(account_id)
    .bind(partner_account_id)
    .fetch_optional(db)
    .await?;
    Ok(row)
}
