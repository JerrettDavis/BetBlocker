use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;

// ---------------------------------------------------------------------------
// Row types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct TokenRow {
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

// ---------------------------------------------------------------------------
// Service functions
// ---------------------------------------------------------------------------

/// Create a new enrollment token for an organization.
pub async fn create_enrollment_token(
    db: &PgPool,
    org_id: i64,
    created_by: i64,
    label: Option<&str>,
    protection_config: serde_json::Value,
    reporting_config: serde_json::Value,
    unenrollment_policy: serde_json::Value,
    max_uses: Option<i32>,
    expires_at: Option<DateTime<Utc>>,
) -> Result<TokenRow, ApiError> {
    let row = sqlx::query_as::<_, TokenRow>(
        r#"INSERT INTO enrollment_tokens
               (organization_id, created_by, label, protection_config,
                reporting_config, unenrollment_policy, max_uses, expires_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           RETURNING id, public_id, organization_id, created_by, label,
                     protection_config, reporting_config, unenrollment_policy,
                     max_uses, uses_count, expires_at, created_at"#,
    )
    .bind(org_id)
    .bind(created_by)
    .bind(label)
    .bind(&protection_config)
    .bind(&reporting_config)
    .bind(&unenrollment_policy)
    .bind(max_uses)
    .bind(expires_at)
    .fetch_one(db)
    .await?;

    Ok(row)
}

/// List enrollment tokens for an organization.
pub async fn list_enrollment_tokens(
    db: &PgPool,
    org_id: i64,
    limit: i64,
    offset: i64,
) -> Result<(Vec<TokenRow>, i64), ApiError> {
    let rows = sqlx::query_as::<_, TokenRow>(
        r#"SELECT id, public_id, organization_id, created_by, label,
                  protection_config, reporting_config, unenrollment_policy,
                  max_uses, uses_count, expires_at, created_at
           FROM enrollment_tokens
           WHERE organization_id = $1
           ORDER BY created_at DESC
           LIMIT $2 OFFSET $3"#,
    )
    .bind(org_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM enrollment_tokens WHERE organization_id = $1",
    )
    .bind(org_id)
    .fetch_one(db)
    .await?;

    Ok((rows, total))
}

/// Get an enrollment token by its public UUID.
pub async fn get_enrollment_token(
    db: &PgPool,
    public_id: Uuid,
) -> Result<TokenRow, ApiError> {
    let row = sqlx::query_as::<_, TokenRow>(
        r#"SELECT id, public_id, organization_id, created_by, label,
                  protection_config, reporting_config, unenrollment_policy,
                  max_uses, uses_count, expires_at, created_at
           FROM enrollment_tokens
           WHERE public_id = $1"#,
    )
    .bind(public_id)
    .fetch_one(db)
    .await?;

    Ok(row)
}

/// Get an enrollment token by its internal ID, scoped to an organization.
pub async fn get_enrollment_token_by_id(
    db: &PgPool,
    token_id: i64,
    org_id: i64,
) -> Result<TokenRow, ApiError> {
    let row = sqlx::query_as::<_, TokenRow>(
        r#"SELECT id, public_id, organization_id, created_by, label,
                  protection_config, reporting_config, unenrollment_policy,
                  max_uses, uses_count, expires_at, created_at
           FROM enrollment_tokens
           WHERE id = $1 AND organization_id = $2"#,
    )
    .bind(token_id)
    .bind(org_id)
    .fetch_one(db)
    .await?;

    Ok(row)
}

/// Revoke an enrollment token by setting its expires_at to now.
pub async fn revoke_enrollment_token(
    db: &PgPool,
    token_id: i64,
) -> Result<(), ApiError> {
    let result = sqlx::query(
        "UPDATE enrollment_tokens SET expires_at = NOW() WHERE id = $1",
    )
    .bind(token_id)
    .execute(db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound {
            code: "TOKEN_NOT_FOUND".into(),
            message: "Enrollment token not found".into(),
        });
    }

    Ok(())
}

/// Redeem an enrollment token: increment uses_count, check limits, assign device to org.
pub async fn redeem_enrollment_token(
    db: &PgPool,
    public_id: Uuid,
    device_id: i64,
) -> Result<TokenRow, ApiError> {
    let token = get_enrollment_token(db, public_id).await?;

    // Check if expired
    if let Some(expires_at) = token.expires_at {
        if expires_at < Utc::now() {
            return Err(ApiError::Validation {
                message: "This enrollment token has expired".into(),
                details: None,
            });
        }
    }

    // Check if max uses exceeded
    if let Some(max_uses) = token.max_uses {
        if token.uses_count >= max_uses {
            return Err(ApiError::Validation {
                message: "This enrollment token has reached its maximum number of uses".into(),
                details: None,
            });
        }
    }

    let mut tx = db.begin().await?;

    // Increment uses_count
    sqlx::query("UPDATE enrollment_tokens SET uses_count = uses_count + 1 WHERE id = $1")
        .bind(token.id)
        .execute(&mut *tx)
        .await?;

    // Assign device to org (ignore conflict if already assigned)
    sqlx::query(
        r#"INSERT INTO organization_devices (organization_id, device_id, assigned_by)
           VALUES ($1, $2, $3)
           ON CONFLICT (organization_id, device_id) DO NOTHING"#,
    )
    .bind(token.organization_id)
    .bind(device_id)
    .bind(token.created_by)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(token)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_row_serialization() {
        let row = TokenRow {
            id: 1,
            public_id: Uuid::new_v4(),
            organization_id: 10,
            created_by: 42,
            label: Some("Office token".into()),
            protection_config: serde_json::json!({"dns_blocking": true}),
            reporting_config: serde_json::json!({"level": "basic"}),
            unenrollment_policy: serde_json::json!({"type": "time_delayed"}),
            max_uses: Some(50),
            uses_count: 3,
            expires_at: None,
            created_at: Utc::now(),
        };

        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(json["label"], "Office token");
        assert_eq!(json["max_uses"], 50);
        assert_eq!(json["uses_count"], 3);
        assert!(json["expires_at"].is_null());
    }

    #[test]
    fn test_token_row_with_no_label() {
        let row = TokenRow {
            id: 1,
            public_id: Uuid::new_v4(),
            organization_id: 10,
            created_by: 42,
            label: None,
            protection_config: serde_json::json!({}),
            reporting_config: serde_json::json!({}),
            unenrollment_policy: serde_json::json!({}),
            max_uses: None,
            uses_count: 0,
            expires_at: None,
            created_at: Utc::now(),
        };

        let json = serde_json::to_value(&row).unwrap();
        assert!(json["label"].is_null());
        assert!(json["max_uses"].is_null());
    }
}
