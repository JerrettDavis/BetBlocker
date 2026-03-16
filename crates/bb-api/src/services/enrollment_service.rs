use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct EnrollmentRow {
    pub id: i64,
    pub public_id: Uuid,
    pub device_id: i64,
    pub account_id: i64,
    pub enrolled_by: i64,
    pub tier: String,
    pub status: String,
    pub protection_config: serde_json::Value,
    pub reporting_config: serde_json::Value,
    pub unenrollment_policy: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Create a new enrollment.
pub async fn create_enrollment(
    db: &PgPool,
    device_id: i64,
    account_id: i64,
    enrolled_by: i64,
    tier: &str,
    protection_config: &serde_json::Value,
    reporting_config: &serde_json::Value,
    unenrollment_policy: &serde_json::Value,
    expires_at: Option<DateTime<Utc>>,
) -> Result<EnrollmentRow, ApiError> {
    let row = sqlx::query_as::<_, EnrollmentRow>(
        r#"INSERT INTO enrollments
               (device_id, account_id, enrolled_by, tier, protection_config,
                reporting_config, unenrollment_policy, expires_at)
           VALUES ($1, $2, $3, $4::enrollment_tier, $5, $6, $7, $8)
           RETURNING id, public_id, device_id, account_id, enrolled_by,
                     tier::text, status::text, protection_config, reporting_config,
                     unenrollment_policy, created_at, updated_at, expires_at"#,
    )
    .bind(device_id)
    .bind(account_id)
    .bind(enrolled_by)
    .bind(tier)
    .bind(protection_config)
    .bind(reporting_config)
    .bind(unenrollment_policy)
    .bind(expires_at)
    .fetch_one(db)
    .await?;

    // Update device with enrollment_id and set status to active
    sqlx::query(
        "UPDATE devices SET enrollment_id = $1, status = 'active'::device_status, updated_at = NOW() WHERE id = $2",
    )
    .bind(row.id)
    .bind(device_id)
    .execute(db)
    .await?;

    Ok(row)
}

/// Fetch enrollment by public ID.
pub async fn get_enrollment_by_public_id(
    db: &PgPool,
    public_id: Uuid,
) -> Result<Option<EnrollmentRow>, ApiError> {
    let row = sqlx::query_as::<_, EnrollmentRow>(
        r#"SELECT id, public_id, device_id, account_id, enrolled_by,
                  tier::text, status::text, protection_config, reporting_config,
                  unenrollment_policy, created_at, updated_at, expires_at
           FROM enrollments WHERE public_id = $1"#,
    )
    .bind(public_id)
    .fetch_optional(db)
    .await?;
    Ok(row)
}

/// List enrollments visible to a user.
pub async fn list_enrollments(
    db: &PgPool,
    account_id: i64,
    status_filter: Option<&str>,
    tier_filter: Option<&str>,
    device_id_filter: Option<i64>,
    limit: i64,
    offset: i64,
) -> Result<(Vec<EnrollmentRow>, i64), ApiError> {
    let rows = sqlx::query_as::<_, EnrollmentRow>(
        r#"SELECT id, public_id, device_id, account_id, enrolled_by,
                  tier::text, status::text, protection_config, reporting_config,
                  unenrollment_policy, created_at, updated_at, expires_at
           FROM enrollments
           WHERE (account_id = $1 OR enrolled_by = $1)
             AND ($2::text IS NULL OR status::text = $2)
             AND ($3::text IS NULL OR tier::text = $3)
             AND ($4::bigint IS NULL OR device_id = $4)
           ORDER BY created_at DESC
           LIMIT $5 OFFSET $6"#,
    )
    .bind(account_id)
    .bind(status_filter)
    .bind(tier_filter)
    .bind(device_id_filter)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    let total = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*)
           FROM enrollments
           WHERE (account_id = $1 OR enrolled_by = $1)
             AND ($2::text IS NULL OR status::text = $2)
             AND ($3::text IS NULL OR tier::text = $3)
             AND ($4::bigint IS NULL OR device_id = $4)"#,
    )
    .bind(account_id)
    .bind(status_filter)
    .bind(tier_filter)
    .bind(device_id_filter)
    .fetch_one(db)
    .await?;

    Ok((rows, total))
}

/// Update enrollment status.
pub async fn update_enrollment_status(
    db: &PgPool,
    enrollment_id: i64,
    status: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        "UPDATE enrollments SET status = $2::enrollment_status, updated_at = NOW() WHERE id = $1",
    )
    .bind(enrollment_id)
    .bind(status)
    .execute(db)
    .await?;
    Ok(())
}

/// Partially update enrollment configuration.
pub async fn update_enrollment_config(
    db: &PgPool,
    enrollment_id: i64,
    protection_config: Option<&serde_json::Value>,
    reporting_config: Option<&serde_json::Value>,
    unenrollment_policy: Option<&serde_json::Value>,
    expires_at: Option<Option<DateTime<Utc>>>,
) -> Result<EnrollmentRow, ApiError> {
    // Build SET clauses dynamically. For simplicity, use COALESCE pattern.
    let row = sqlx::query_as::<_, EnrollmentRow>(
        r#"UPDATE enrollments SET
               protection_config = COALESCE($2, protection_config),
               reporting_config = COALESCE($3, reporting_config),
               unenrollment_policy = COALESCE($4, unenrollment_policy),
               expires_at = CASE WHEN $5::bool THEN $6 ELSE expires_at END,
               updated_at = NOW()
           WHERE id = $1
           RETURNING id, public_id, device_id, account_id, enrolled_by,
                     tier::text, status::text, protection_config, reporting_config,
                     unenrollment_policy, created_at, updated_at, expires_at"#,
    )
    .bind(enrollment_id)
    .bind(protection_config)
    .bind(reporting_config)
    .bind(unenrollment_policy)
    .bind(expires_at.is_some())
    .bind(expires_at.flatten())
    .fetch_one(db)
    .await?;
    Ok(row)
}

/// Insert an unenrollment request.
pub async fn insert_unenroll_request(
    db: &PgPool,
    enrollment_id: i64,
    requested_by: i64,
    required_approver: Option<i64>,
    delay_until: Option<DateTime<Utc>>,
) -> Result<i64, ApiError> {
    let id = sqlx::query_scalar::<_, i64>(
        r#"INSERT INTO enrollment_unenroll_requests
               (enrollment_id, requested_by_account_id, required_approver_account_id, delay_until)
           VALUES ($1, $2, $3, $4)
           RETURNING id"#,
    )
    .bind(enrollment_id)
    .bind(requested_by)
    .bind(required_approver)
    .bind(delay_until)
    .fetch_one(db)
    .await?;
    Ok(id)
}

/// Approve or deny an unenrollment request.
pub async fn resolve_unenroll_request(
    db: &PgPool,
    enrollment_id: i64,
    approved_by: i64,
    approved: bool,
) -> Result<(), ApiError> {
    let new_status = if approved { "approved" } else { "denied" };
    sqlx::query(
        r#"UPDATE enrollment_unenroll_requests
           SET status = $3::unenroll_request_status,
               approved_by_account_id = $2,
               approved_at = NOW()
           WHERE enrollment_id = $1 AND status = 'pending'"#,
    )
    .bind(enrollment_id)
    .bind(approved_by)
    .bind(new_status)
    .execute(db)
    .await?;
    Ok(())
}

/// Check if a device already has an active enrollment.
pub async fn device_has_active_enrollment(db: &PgPool, device_id: i64) -> Result<bool, ApiError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM enrollments WHERE device_id = $1 AND status = 'active'",
    )
    .bind(device_id)
    .fetch_one(db)
    .await?;
    Ok(count > 0)
}

/// Check if there's a pending unenroll request for an enrollment.
pub async fn has_pending_unenroll_request(
    db: &PgPool,
    enrollment_id: i64,
) -> Result<bool, ApiError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM enrollment_unenroll_requests WHERE enrollment_id = $1 AND status = 'pending'",
    )
    .bind(enrollment_id)
    .fetch_one(db)
    .await?;
    Ok(count > 0)
}

/// Find the active enrollment for a device.
pub async fn get_active_enrollment_for_device(
    db: &PgPool,
    device_id: i64,
) -> Result<Option<EnrollmentRow>, ApiError> {
    let row = sqlx::query_as::<_, EnrollmentRow>(
        r#"SELECT id, public_id, device_id, account_id, enrolled_by,
                  tier::text, status::text, protection_config, reporting_config,
                  unenrollment_policy, created_at, updated_at, expires_at
           FROM enrollments WHERE device_id = $1 AND status = 'active'"#,
    )
    .bind(device_id)
    .fetch_optional(db)
    .await?;
    Ok(row)
}
