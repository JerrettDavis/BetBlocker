use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;

/// Account row as returned from the database.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct AccountRow {
    pub id: i64,
    pub public_id: Uuid,
    pub email: String,
    #[serde(skip)]
    pub password_hash: String,
    pub display_name: String,
    pub role: String,
    pub email_verified: bool,
    pub mfa_enabled: bool,
    pub timezone: String,
    pub locale: String,
    pub organization_id: Option<i64>,
    pub locked_until: Option<DateTime<Utc>>,
    pub failed_login_attempts: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Fetch full account row by internal ID.
pub async fn get_account_by_id(db: &PgPool, id: i64) -> Result<AccountRow, ApiError> {
    let row = sqlx::query_as::<_, AccountRow>(
        r#"SELECT id, public_id, email, password_hash, display_name,
                  role::text, email_verified, mfa_enabled, timezone, locale,
                  organization_id, locked_until, failed_login_attempts,
                  created_at, updated_at
           FROM accounts WHERE id = $1"#,
    )
    .bind(id)
    .fetch_one(db)
    .await?;
    Ok(row)
}

/// Fetch full account row by public UUID.
pub async fn get_account_by_public_id(
    db: &PgPool,
    public_id: Uuid,
) -> Result<AccountRow, ApiError> {
    let row = sqlx::query_as::<_, AccountRow>(
        r#"SELECT id, public_id, email, password_hash, display_name,
                  role::text, email_verified, mfa_enabled, timezone, locale,
                  organization_id, locked_until, failed_login_attempts,
                  created_at, updated_at
           FROM accounts WHERE public_id = $1"#,
    )
    .bind(public_id)
    .fetch_one(db)
    .await?;
    Ok(row)
}

/// Fetch account row by email (for login).
pub async fn get_account_by_email(db: &PgPool, email: &str) -> Result<Option<AccountRow>, ApiError> {
    let row = sqlx::query_as::<_, AccountRow>(
        r#"SELECT id, public_id, email, password_hash, display_name,
                  role::text, email_verified, mfa_enabled, timezone, locale,
                  organization_id, locked_until, failed_login_attempts,
                  created_at, updated_at
           FROM accounts WHERE email = $1"#,
    )
    .bind(email)
    .fetch_optional(db)
    .await?;
    Ok(row)
}

/// Insert a new account. Returns the newly created row.
pub async fn create_account(
    db: &PgPool,
    email: &str,
    password_hash: &str,
    display_name: &str,
    timezone: &str,
    locale: &str,
) -> Result<AccountRow, ApiError> {
    let row = sqlx::query_as::<_, AccountRow>(
        r#"INSERT INTO accounts (email, password_hash, display_name, timezone, locale)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id, public_id, email, password_hash, display_name,
                     role::text, email_verified, mfa_enabled, timezone, locale,
                     organization_id, locked_until, failed_login_attempts,
                     created_at, updated_at"#,
    )
    .bind(email)
    .bind(password_hash)
    .bind(display_name)
    .bind(timezone)
    .bind(locale)
    .fetch_one(db)
    .await?;
    Ok(row)
}

/// Update account profile fields (partial update).
pub async fn update_account(
    db: &PgPool,
    account_id: i64,
    display_name: Option<&str>,
    timezone: Option<&str>,
    locale: Option<&str>,
    email: Option<&str>,
    password_hash: Option<&str>,
    email_verified: Option<bool>,
) -> Result<AccountRow, ApiError> {
    let row = sqlx::query_as::<_, AccountRow>(
        r#"UPDATE accounts SET
               display_name = COALESCE($2, display_name),
               timezone = COALESCE($3, timezone),
               locale = COALESCE($4, locale),
               email = COALESCE($5, email),
               password_hash = COALESCE($6, password_hash),
               email_verified = COALESCE($7, email_verified),
               updated_at = NOW()
           WHERE id = $1
           RETURNING id, public_id, email, password_hash, display_name,
                     role::text, email_verified, mfa_enabled, timezone, locale,
                     organization_id, locked_until, failed_login_attempts,
                     created_at, updated_at"#,
    )
    .bind(account_id)
    .bind(display_name)
    .bind(timezone)
    .bind(locale)
    .bind(email)
    .bind(password_hash)
    .bind(email_verified)
    .fetch_one(db)
    .await?;
    Ok(row)
}

/// Update account role.
pub async fn update_account_role(
    db: &PgPool,
    account_id: i64,
    role: &str,
) -> Result<(), ApiError> {
    sqlx::query("UPDATE accounts SET role = $2::account_role, updated_at = NOW() WHERE id = $1")
        .bind(account_id)
        .bind(role)
        .execute(db)
        .await?;
    Ok(())
}
