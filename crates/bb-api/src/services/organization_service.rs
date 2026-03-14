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
// Extended member row (with joined account fields)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct OrgMemberDetailRow {
    pub id: i64,
    pub organization_id: i64,
    pub account_id: i64,
    pub role: String,
    pub invited_by: Option<i64>,
    pub joined_at: DateTime<Utc>,
    // Joined from accounts
    #[sqlx(default)]
    pub display_name: Option<String>,
    #[sqlx(default)]
    pub email: Option<String>,
    #[sqlx(default)]
    pub account_public_id: Option<Uuid>,
}

// ---------------------------------------------------------------------------
// Member management service functions
// ---------------------------------------------------------------------------

/// Helper: numeric role level for comparison.
fn role_level(r: &str) -> i32 {
    match r {
        "owner" => 3,
        "admin" => 2,
        "member" => 1,
        _ => 0,
    }
}

/// Invite a member to an organization by email.
/// Looks up the account by email, then inserts into organization_members.
/// Returns error if already a member.
pub async fn invite_member(
    db: &PgPool,
    org_id: i64,
    email: &str,
    role: &str,
    invited_by: i64,
) -> Result<OrgMemberRow, ApiError> {
    // Look up the account by email
    let account = sqlx::query_scalar::<_, i64>(
        r#"SELECT id FROM accounts WHERE email = $1"#,
    )
    .bind(email)
    .fetch_optional(db)
    .await?
    .ok_or(ApiError::NotFound {
        code: "ACCOUNT_NOT_FOUND".into(),
        message: format!("No account found with email '{}'", email),
    })?;

    // Check not already a member
    let existing = sqlx::query_scalar::<_, i64>(
        r#"SELECT id FROM organization_members
           WHERE organization_id = $1 AND account_id = $2"#,
    )
    .bind(org_id)
    .bind(account)
    .fetch_optional(db)
    .await?;

    if existing.is_some() {
        return Err(ApiError::Conflict {
            code: "ALREADY_MEMBER".into(),
            message: "This account is already a member of the organization".into(),
        });
    }

    // Validate role
    if role_level(role) == 0 {
        return Err(ApiError::Validation {
            message: format!("Invalid role: '{}'. Must be 'member', 'admin', or 'owner'", role),
            details: None,
        });
    }

    let row = sqlx::query_as::<_, OrgMemberRow>(
        r#"INSERT INTO organization_members (organization_id, account_id, role, invited_by)
           VALUES ($1, $2, $3::org_member_role, $4)
           RETURNING id, organization_id, account_id, role::text, invited_by, joined_at"#,
    )
    .bind(org_id)
    .bind(account)
    .bind(role)
    .bind(invited_by)
    .fetch_one(db)
    .await?;

    Ok(row)
}

/// List members of an organization with joined account details.
pub async fn list_members(
    db: &PgPool,
    org_id: i64,
    limit: i64,
    offset: i64,
) -> Result<(Vec<OrgMemberDetailRow>, i64), ApiError> {
    let rows = sqlx::query_as::<_, OrgMemberDetailRow>(
        r#"SELECT om.id, om.organization_id, om.account_id, om.role::text,
                  om.invited_by, om.joined_at,
                  a.display_name, a.email, a.public_id as account_public_id
           FROM organization_members om
           JOIN accounts a ON a.id = om.account_id
           WHERE om.organization_id = $1
           ORDER BY om.joined_at ASC
           LIMIT $2 OFFSET $3"#,
    )
    .bind(org_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    let total = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*)
           FROM organization_members
           WHERE organization_id = $1"#,
    )
    .bind(org_id)
    .fetch_one(db)
    .await?;

    Ok((rows, total))
}

/// Update a member's role. Validates that the caller has a higher role than the target.
/// Prevents demoting the sole owner.
pub async fn update_member_role(
    db: &PgPool,
    org_id: i64,
    member_account_id: i64,
    new_role: &str,
    caller_account_id: i64,
) -> Result<OrgMemberRow, ApiError> {
    // Validate new_role
    if role_level(new_role) == 0 {
        return Err(ApiError::Validation {
            message: format!("Invalid role: '{}'. Must be 'member', 'admin', or 'owner'", new_role),
            details: None,
        });
    }

    // Get the caller's membership
    let caller_member = sqlx::query_as::<_, OrgMemberRow>(
        r#"SELECT id, organization_id, account_id, role::text, invited_by, joined_at
           FROM organization_members
           WHERE organization_id = $1 AND account_id = $2"#,
    )
    .bind(org_id)
    .bind(caller_account_id)
    .fetch_optional(db)
    .await?
    .ok_or(ApiError::Forbidden {
        message: "You are not a member of this organization".into(),
    })?;

    // Get the target member
    let target_member = sqlx::query_as::<_, OrgMemberRow>(
        r#"SELECT id, organization_id, account_id, role::text, invited_by, joined_at
           FROM organization_members
           WHERE organization_id = $1 AND account_id = $2"#,
    )
    .bind(org_id)
    .bind(member_account_id)
    .fetch_optional(db)
    .await?
    .ok_or(ApiError::NotFound {
        code: "MEMBER_NOT_FOUND".into(),
        message: "Member not found in this organization".into(),
    })?;

    // Caller must have higher role than target's current role
    if role_level(&caller_member.role) <= role_level(&target_member.role) {
        return Err(ApiError::Forbidden {
            message: "You cannot modify a member with equal or higher role".into(),
        });
    }

    // Caller must have higher role than the new role being assigned
    if role_level(&caller_member.role) <= role_level(new_role) {
        return Err(ApiError::Forbidden {
            message: "You cannot assign a role equal to or higher than your own".into(),
        });
    }

    // Prevent demoting the sole owner
    if target_member.role == "owner" {
        let owner_count = sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*)
               FROM organization_members
               WHERE organization_id = $1 AND role = 'owner'"#,
        )
        .bind(org_id)
        .fetch_one(db)
        .await?;

        if owner_count <= 1 && new_role != "owner" {
            return Err(ApiError::Validation {
                message: "Cannot demote the sole owner of the organization".into(),
                details: None,
            });
        }
    }

    let updated = sqlx::query_as::<_, OrgMemberRow>(
        r#"UPDATE organization_members
           SET role = $3::org_member_role
           WHERE organization_id = $1 AND account_id = $2
           RETURNING id, organization_id, account_id, role::text, invited_by, joined_at"#,
    )
    .bind(org_id)
    .bind(member_account_id)
    .bind(new_role)
    .fetch_one(db)
    .await?;

    Ok(updated)
}

/// Remove a member from an organization.
/// Prevents removing the sole owner. Also unassigns the member's devices from the org.
pub async fn remove_member(
    db: &PgPool,
    org_id: i64,
    member_account_id: i64,
    caller_account_id: i64,
) -> Result<(), ApiError> {
    // Get the caller's membership
    let caller_member = sqlx::query_as::<_, OrgMemberRow>(
        r#"SELECT id, organization_id, account_id, role::text, invited_by, joined_at
           FROM organization_members
           WHERE organization_id = $1 AND account_id = $2"#,
    )
    .bind(org_id)
    .bind(caller_account_id)
    .fetch_optional(db)
    .await?
    .ok_or(ApiError::Forbidden {
        message: "You are not a member of this organization".into(),
    })?;

    // Get the target member
    let target_member = sqlx::query_as::<_, OrgMemberRow>(
        r#"SELECT id, organization_id, account_id, role::text, invited_by, joined_at
           FROM organization_members
           WHERE organization_id = $1 AND account_id = $2"#,
    )
    .bind(org_id)
    .bind(member_account_id)
    .fetch_optional(db)
    .await?
    .ok_or(ApiError::NotFound {
        code: "MEMBER_NOT_FOUND".into(),
        message: "Member not found in this organization".into(),
    })?;

    // Caller must have higher role than target (unless removing self)
    if caller_account_id != member_account_id
        && role_level(&caller_member.role) <= role_level(&target_member.role)
    {
        return Err(ApiError::Forbidden {
            message: "You cannot remove a member with equal or higher role".into(),
        });
    }

    // Prevent removing the sole owner
    if target_member.role == "owner" {
        let owner_count = sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*)
               FROM organization_members
               WHERE organization_id = $1 AND role = 'owner'"#,
        )
        .bind(org_id)
        .fetch_one(db)
        .await?;

        if owner_count <= 1 {
            return Err(ApiError::Validation {
                message: "Cannot remove the sole owner of the organization".into(),
                details: None,
            });
        }
    }

    let mut tx = db.begin().await?;

    // Unassign member's devices from the org
    sqlx::query(
        r#"UPDATE devices SET organization_id = NULL
           WHERE account_id = $1 AND organization_id = (
               SELECT id FROM organizations WHERE id = $2
           )"#,
    )
    .bind(member_account_id)
    .bind(org_id)
    .execute(&mut *tx)
    .await?;

    // Remove the membership
    sqlx::query(
        r#"DELETE FROM organization_members
           WHERE organization_id = $1 AND account_id = $2"#,
    )
    .bind(org_id)
    .bind(member_account_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_hierarchy() {
        assert!(role_level("owner") > role_level("admin"));
        assert!(role_level("admin") > role_level("member"));
        assert!(role_level("owner") > role_level("member"));
        assert_eq!(role_level("unknown"), 0);
    }

    #[test]
    fn test_role_level_values() {
        assert_eq!(role_level("owner"), 3);
        assert_eq!(role_level("admin"), 2);
        assert_eq!(role_level("member"), 1);
        assert_eq!(role_level("invalid"), 0);
        assert_eq!(role_level(""), 0);
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

    #[test]
    fn test_org_member_detail_row_serialization() {
        let row = OrgMemberDetailRow {
            id: 1,
            organization_id: 10,
            account_id: 42,
            role: "admin".into(),
            invited_by: Some(1),
            joined_at: Utc::now(),
            display_name: Some("Test User".into()),
            email: Some("test@example.com".into()),
            account_public_id: Some(Uuid::new_v4()),
        };

        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(json["role"], "admin");
        assert_eq!(json["display_name"], "Test User");
        assert_eq!(json["email"], "test@example.com");
        assert!(json["account_public_id"].is_string());
    }

    #[test]
    fn test_org_member_detail_row_with_none_fields() {
        let row = OrgMemberDetailRow {
            id: 1,
            organization_id: 10,
            account_id: 42,
            role: "member".into(),
            invited_by: None,
            joined_at: Utc::now(),
            display_name: None,
            email: None,
            account_public_id: None,
        };

        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(json["role"], "member");
        assert!(json["display_name"].is_null());
        assert!(json["email"].is_null());
        assert!(json["account_public_id"].is_null());
    }

    #[test]
    fn test_role_comparison_for_permissions() {
        // Owner can manage admin and member
        assert!(role_level("owner") > role_level("admin"));
        assert!(role_level("owner") > role_level("member"));

        // Admin can manage member but not owner
        assert!(role_level("admin") > role_level("member"));
        assert!(role_level("admin") < role_level("owner"));

        // Member cannot manage anyone
        assert!(role_level("member") < role_level("admin"));
        assert!(role_level("member") < role_level("owner"));

        // Equal roles cannot manage each other
        assert_eq!(role_level("admin"), role_level("admin"));
        assert_eq!(role_level("owner"), role_level("owner"));
    }
}
