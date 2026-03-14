use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;

// ---------------------------------------------------------------------------
// Row types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct OrgRow {
    pub id: i64,
    pub public_id: Uuid,
    pub name: String,
    #[sqlx(rename = "type")]
    pub org_type: String,
    pub owner_id: i64,
    pub default_protection_config: Option<serde_json::Value>,
    pub default_reporting_config: Option<serde_json::Value>,
    pub default_unenrollment_policy: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct OrgMemberRow {
    pub id: i64,
    pub organization_id: i64,
    pub account_id: i64,
    pub role: String,
    pub invited_by: Option<i64>,
    pub joined_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Service functions
// ---------------------------------------------------------------------------

/// Create a new organization and add the caller as owner in a single transaction.
pub async fn create_organization(
    db: &PgPool,
    name: &str,
    org_type: &str,
    owner_id: i64,
) -> Result<OrgRow, ApiError> {
    let mut tx = db.begin().await?;

    let org = sqlx::query_as::<_, OrgRow>(
        r#"INSERT INTO organizations (name, type, owner_id)
           VALUES ($1, $2::org_type, $3)
           RETURNING id, public_id, name, type::text, owner_id,
                     default_protection_config, default_reporting_config,
                     default_unenrollment_policy, created_at, updated_at"#,
    )
    .bind(name)
    .bind(org_type)
    .bind(owner_id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"INSERT INTO organization_members (organization_id, account_id, role, invited_by)
           VALUES ($1, $2, 'owner'::org_member_role, $2)"#,
    )
    .bind(org.id)
    .bind(owner_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(org)
}

/// Get an organization by its public UUID.
pub async fn get_organization(db: &PgPool, public_id: Uuid) -> Result<OrgRow, ApiError> {
    let row = sqlx::query_as::<_, OrgRow>(
        r#"SELECT id, public_id, name, type::text, owner_id,
                  default_protection_config, default_reporting_config,
                  default_unenrollment_policy, created_at, updated_at
           FROM organizations
           WHERE public_id = $1"#,
    )
    .bind(public_id)
    .fetch_one(db)
    .await?;
    Ok(row)
}

/// List organizations for an account (via membership).
pub async fn list_organizations_for_account(
    db: &PgPool,
    account_id: i64,
    limit: i64,
    offset: i64,
) -> Result<(Vec<OrgRow>, i64), ApiError> {
    let rows = sqlx::query_as::<_, OrgRow>(
        r#"SELECT o.id, o.public_id, o.name, o.type::text, o.owner_id,
                  o.default_protection_config, o.default_reporting_config,
                  o.default_unenrollment_policy, o.created_at, o.updated_at
           FROM organizations o
           JOIN organization_members om ON om.organization_id = o.id
           WHERE om.account_id = $1
           ORDER BY o.created_at DESC
           LIMIT $2 OFFSET $3"#,
    )
    .bind(account_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    let total = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*)
           FROM organization_members
           WHERE account_id = $1"#,
    )
    .bind(account_id)
    .fetch_one(db)
    .await?;

    Ok((rows, total))
}

/// Partial update of an organization.
pub async fn update_organization(
    db: &PgPool,
    org_id: i64,
    name: Option<&str>,
    org_type: Option<&str>,
    default_protection_config: Option<serde_json::Value>,
    default_reporting_config: Option<serde_json::Value>,
    default_unenrollment_policy: Option<serde_json::Value>,
) -> Result<OrgRow, ApiError> {
    let row = sqlx::query_as::<_, OrgRow>(
        r#"UPDATE organizations SET
               name = COALESCE($2, name),
               type = COALESCE($3::org_type, type),
               default_protection_config = COALESCE($4, default_protection_config),
               default_reporting_config = COALESCE($5, default_reporting_config),
               default_unenrollment_policy = COALESCE($6, default_unenrollment_policy),
               updated_at = NOW()
           WHERE id = $1
           RETURNING id, public_id, name, type::text, owner_id,
                     default_protection_config, default_reporting_config,
                     default_unenrollment_policy, created_at, updated_at"#,
    )
    .bind(org_id)
    .bind(name)
    .bind(org_type)
    .bind(default_protection_config)
    .bind(default_reporting_config)
    .bind(default_unenrollment_policy)
    .fetch_one(db)
    .await?;
    Ok(row)
}

/// Delete an organization (CASCADE will remove members, devices, tokens).
pub async fn delete_organization(db: &PgPool, org_id: i64) -> Result<(), ApiError> {
    let result = sqlx::query("DELETE FROM organizations WHERE id = $1")
        .bind(org_id)
        .execute(db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound {
            code: "ORG_NOT_FOUND".into(),
            message: "Organization not found".into(),
        });
    }

    Ok(())
}

/// Check that an account has the required role (or higher) in an organization.
/// Role hierarchy: owner > admin > member.
pub async fn check_org_permission(
    db: &PgPool,
    org_id: i64,
    account_id: i64,
    required_role: &str,
) -> Result<OrgMemberRow, ApiError> {
    let member = sqlx::query_as::<_, OrgMemberRow>(
        r#"SELECT id, organization_id, account_id, role::text, invited_by, joined_at
           FROM organization_members
           WHERE organization_id = $1 AND account_id = $2"#,
    )
    .bind(org_id)
    .bind(account_id)
    .fetch_optional(db)
    .await?
    .ok_or(ApiError::Forbidden {
        message: "You are not a member of this organization".into(),
    })?;

    let role_level = |r: &str| -> i32 {
        match r {
            "owner" => 3,
            "admin" => 2,
            "member" => 1,
            _ => 0,
        }
    };

    if role_level(&member.role) < role_level(required_role) {
        return Err(ApiError::Forbidden {
            message: format!(
                "Requires '{}' role or higher in this organization",
                required_role
            ),
        });
    }

    Ok(member)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_hierarchy() {
        let role_level = |r: &str| -> i32 {
            match r {
                "owner" => 3,
                "admin" => 2,
                "member" => 1,
                _ => 0,
            }
        };

        assert!(role_level("owner") > role_level("admin"));
        assert!(role_level("admin") > role_level("member"));
        assert!(role_level("owner") > role_level("member"));
        assert_eq!(role_level("unknown"), 0);
    }

    #[test]
    fn test_org_row_serialization() {
        let row = OrgRow {
            id: 1,
            public_id: Uuid::new_v4(),
            name: "Test Org".into(),
            org_type: "family".into(),
            owner_id: 42,
            default_protection_config: None,
            default_reporting_config: Some(serde_json::json!({"level": "basic"})),
            default_unenrollment_policy: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(json["name"], "Test Org");
        assert_eq!(json["org_type"], "family");
        assert_eq!(json["owner_id"], 42);
        assert!(json["default_protection_config"].is_null());
        assert_eq!(json["default_reporting_config"]["level"], "basic");
    }

    #[test]
    fn test_org_member_row_serialization() {
        let row = OrgMemberRow {
            id: 1,
            organization_id: 10,
            account_id: 42,
            role: "owner".into(),
            invited_by: None,
            joined_at: Utc::now(),
        };

        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(json["role"], "owner");
        assert_eq!(json["account_id"], 42);
        assert!(json["invited_by"].is_null());
    }
}
