use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;

// ---------------------------------------------------------------------------
// Row types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct AppSigRow {
    pub id: i64,
    pub public_id: Uuid,
    pub name: String,
    pub package_names: Vec<String>,
    pub executable_names: Vec<String>,
    pub cert_hashes: Vec<String>,
    pub display_name_patterns: Vec<String>,
    pub platforms: Vec<String>,
    pub category: String,
    pub status: String,
    pub confidence: f64,
    pub source: String,
    pub evidence_url: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Service functions
// ---------------------------------------------------------------------------

/// Create a new app signature.
pub async fn create_signature(
    db: &PgPool,
    name: &str,
    package_names: &[String],
    executable_names: &[String],
    cert_hashes: &[String],
    display_name_patterns: &[String],
    platforms: &[String],
    category: &str,
    status: &str,
    confidence: f64,
    source: &str,
    evidence_url: Option<&str>,
    tags: &[String],
) -> Result<AppSigRow, ApiError> {
    let row = sqlx::query_as::<_, AppSigRow>(
        r#"INSERT INTO app_signatures
               (name, package_names, executable_names, cert_hashes,
                display_name_patterns, platforms, category,
                status, confidence, source, evidence_url, tags)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8::app_signature_status, $9, $10::blocklist_source, $11, $12)
           RETURNING id, public_id, name, package_names, executable_names,
                     cert_hashes, display_name_patterns, platforms,
                     category, status::text, confidence, source::text,
                     evidence_url, tags, created_at, updated_at"#,
    )
    .bind(name)
    .bind(package_names)
    .bind(executable_names)
    .bind(cert_hashes)
    .bind(display_name_patterns)
    .bind(platforms)
    .bind(category)
    .bind(status)
    .bind(confidence)
    .bind(source)
    .bind(evidence_url)
    .bind(tags)
    .fetch_one(db)
    .await?;

    Ok(row)
}

/// Get an app signature by its public UUID.
pub async fn get_signature(db: &PgPool, public_id: Uuid) -> Result<AppSigRow, ApiError> {
    let row = sqlx::query_as::<_, AppSigRow>(
        r#"SELECT id, public_id, name, package_names, executable_names,
                  cert_hashes, display_name_patterns, platforms,
                  category, status::text, confidence, source::text,
                  evidence_url, tags, created_at, updated_at
           FROM app_signatures
           WHERE public_id = $1"#,
    )
    .bind(public_id)
    .fetch_one(db)
    .await?;

    Ok(row)
}

/// List app signatures with optional filters and pagination.
pub async fn list_signatures(
    db: &PgPool,
    search: Option<&str>,
    category: Option<&str>,
    platform: Option<&str>,
    status: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<(Vec<AppSigRow>, i64), ApiError> {
    let rows = sqlx::query_as::<_, AppSigRow>(
        r#"SELECT id, public_id, name, package_names, executable_names,
                  cert_hashes, display_name_patterns, platforms,
                  category, status::text, confidence, source::text,
                  evidence_url, tags, created_at, updated_at
           FROM app_signatures
           WHERE ($1::text IS NULL OR name ILIKE '%' || $1 || '%')
             AND ($2::text IS NULL OR category = $2)
             AND ($3::text IS NULL OR $3 = ANY(platforms))
             AND ($4::text IS NULL OR status::text = $4)
           ORDER BY created_at DESC
           LIMIT $5 OFFSET $6"#,
    )
    .bind(search)
    .bind(category)
    .bind(platform)
    .bind(status)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    let total = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*)
           FROM app_signatures
           WHERE ($1::text IS NULL OR name ILIKE '%' || $1 || '%')
             AND ($2::text IS NULL OR category = $2)
             AND ($3::text IS NULL OR $3 = ANY(platforms))
             AND ($4::text IS NULL OR status::text = $4)"#,
    )
    .bind(search)
    .bind(category)
    .bind(platform)
    .bind(status)
    .fetch_one(db)
    .await?;

    Ok((rows, total))
}

/// Update an app signature.
pub async fn update_signature(
    db: &PgPool,
    id: i64,
    name: Option<&str>,
    package_names: Option<&[String]>,
    executable_names: Option<&[String]>,
    cert_hashes: Option<&[String]>,
    display_name_patterns: Option<&[String]>,
    platforms: Option<&[String]>,
    category: Option<&str>,
    status: Option<&str>,
    confidence: Option<f64>,
    source: Option<&str>,
    evidence_url: Option<&str>,
    tags: Option<&[String]>,
) -> Result<AppSigRow, ApiError> {
    let row = sqlx::query_as::<_, AppSigRow>(
        r#"UPDATE app_signatures SET
               name = COALESCE($2, name),
               package_names = COALESCE($3, package_names),
               executable_names = COALESCE($4, executable_names),
               cert_hashes = COALESCE($5, cert_hashes),
               display_name_patterns = COALESCE($6, display_name_patterns),
               platforms = COALESCE($7, platforms),
               category = COALESCE($8, category),
               status = COALESCE($9::app_signature_status, status),
               confidence = COALESCE($10, confidence),
               source = COALESCE($11::blocklist_source, source),
               evidence_url = COALESCE($12, evidence_url),
               tags = COALESCE($13, tags),
               updated_at = NOW()
           WHERE id = $1
           RETURNING id, public_id, name, package_names, executable_names,
                     cert_hashes, display_name_patterns, platforms,
                     category, status::text, confidence, source::text,
                     evidence_url, tags, created_at, updated_at"#,
    )
    .bind(id)
    .bind(name)
    .bind(package_names)
    .bind(executable_names)
    .bind(cert_hashes)
    .bind(display_name_patterns)
    .bind(platforms)
    .bind(category)
    .bind(status)
    .bind(confidence)
    .bind(source)
    .bind(evidence_url)
    .bind(tags)
    .fetch_one(db)
    .await?;

    Ok(row)
}

/// Delete an app signature by internal ID.
pub async fn delete_signature(db: &PgPool, id: i64) -> Result<(), ApiError> {
    let result = sqlx::query("DELETE FROM app_signatures WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound {
            code: "APP_SIGNATURE_NOT_FOUND".into(),
            message: "App signature not found".into(),
        });
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_sig_row_serialization() {
        let row = AppSigRow {
            id: 1,
            public_id: Uuid::new_v4(),
            name: "Bet365".into(),
            package_names: vec!["com.bet365.app".into()],
            executable_names: vec!["bet365.exe".into()],
            cert_hashes: vec![],
            display_name_patterns: vec!["Bet365*".into()],
            platforms: vec!["windows".into(), "android".into()],
            category: "sports_betting".into(),
            status: "active".into(),
            confidence: 1.0,
            source: "curated".into(),
            evidence_url: Some("https://example.com".into()),
            tags: vec!["top-tier".into()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(json["name"], "Bet365");
        assert_eq!(json["confidence"], 1.0);
        assert_eq!(json["platforms"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_app_sig_row_empty_arrays() {
        let row = AppSigRow {
            id: 1,
            public_id: Uuid::new_v4(),
            name: "Test".into(),
            package_names: vec![],
            executable_names: vec![],
            cert_hashes: vec![],
            display_name_patterns: vec![],
            platforms: vec![],
            category: "other".into(),
            status: "pending_review".into(),
            confidence: 0.0,
            source: "curated".into(),
            evidence_url: None,
            tags: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_value(&row).unwrap();
        assert!(json["evidence_url"].is_null());
        assert_eq!(json["package_names"].as_array().unwrap().len(), 0);
    }
}
