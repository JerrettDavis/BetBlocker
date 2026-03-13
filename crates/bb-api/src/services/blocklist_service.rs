use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct BlocklistVersionRow {
    pub id: i64,
    pub version_number: i64,
    pub entry_count: i64,
    pub signature: Vec<u8>,
    pub published_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct BlocklistEntryRow {
    pub id: i64,
    pub public_id: Uuid,
    pub domain: String,
    pub pattern: Option<String>,
    pub category: String,
    pub source: String,
    pub confidence: f64,
    pub status: String,
    pub added_by: Option<i64>,
    pub reviewed_by: Option<i64>,
    pub evidence_url: Option<String>,
    pub tags: Vec<String>,
    pub blocklist_version_added: Option<i64>,
    pub blocklist_version_removed: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct BlocklistDeltaEntry {
    pub domain: String,
    pub category: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct FederatedReportRow {
    pub id: i64,
    pub device_id: i64,
    pub domain: String,
    pub heuristic_match_type: Option<String>,
    pub confidence: f64,
    pub reported_at: DateTime<Utc>,
    pub review_status: String,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ReviewQueueEntry {
    pub domain: String,
    pub report_count: i64,
    pub first_reported: DateTime<Utc>,
    pub last_reported: DateTime<Utc>,
    pub avg_confidence: f64,
}

/// Get the latest blocklist version.
pub async fn get_latest_version(db: &PgPool) -> Result<Option<BlocklistVersionRow>, ApiError> {
    let row = sqlx::query_as::<_, BlocklistVersionRow>(
        r#"SELECT id, version_number, entry_count, signature, published_at
           FROM blocklist_versions
           ORDER BY version_number DESC
           LIMIT 1"#,
    )
    .fetch_optional(db)
    .await?;
    Ok(row)
}

/// Compute delta between two versions.
pub async fn get_delta(
    db: &PgPool,
    from_version: i64,
    to_version: i64,
) -> Result<(Vec<BlocklistDeltaEntry>, Vec<String>), ApiError> {
    // Additions: entries added after from_version and before/at to_version
    let additions = sqlx::query_as::<_, BlocklistDeltaEntry>(
        r#"SELECT be.domain, be.category::text as category, be.confidence
           FROM blocklist_entries be
           JOIN blocklist_versions bv_add ON bv_add.id = be.blocklist_version_added
           WHERE bv_add.version_number > $1
             AND bv_add.version_number <= $2
             AND be.status = 'active'"#,
    )
    .bind(from_version)
    .bind(to_version)
    .fetch_all(db)
    .await?;

    // Removals: entries removed after from_version and before/at to_version
    let removals = sqlx::query_scalar::<_, String>(
        r#"SELECT be.domain
           FROM blocklist_entries be
           JOIN blocklist_versions bv_rem ON bv_rem.id = be.blocklist_version_removed
           WHERE bv_rem.version_number > $1
             AND bv_rem.version_number <= $2"#,
    )
    .bind(from_version)
    .bind(to_version)
    .fetch_all(db)
    .await?;

    Ok((additions, removals))
}

/// Create a new blocklist entry (admin).
pub async fn create_blocklist_entry(
    db: &PgPool,
    domain: &str,
    pattern: Option<&str>,
    category: &str,
    source: &str,
    confidence: f64,
    added_by: i64,
    evidence_url: Option<&str>,
    tags: &[String],
) -> Result<BlocklistEntryRow, ApiError> {
    let row = sqlx::query_as::<_, BlocklistEntryRow>(
        r#"INSERT INTO blocklist_entries
               (domain, pattern, category, source, confidence, added_by, evidence_url, tags, status)
           VALUES ($1, $2, $3::gambling_category, $4::blocklist_source, $5, $6, $7, $8, 'active')
           RETURNING id, public_id, domain, pattern, category::text, source::text,
                     confidence, status::text, added_by, reviewed_by, evidence_url, tags,
                     blocklist_version_added, blocklist_version_removed, created_at, updated_at"#,
    )
    .bind(domain)
    .bind(pattern)
    .bind(category)
    .bind(source)
    .bind(confidence)
    .bind(added_by)
    .bind(evidence_url)
    .bind(tags)
    .fetch_one(db)
    .await?;
    Ok(row)
}

/// List blocklist entries with filters (admin).
pub async fn list_blocklist_entries(
    db: &PgPool,
    search: Option<&str>,
    category: Option<&str>,
    source: Option<&str>,
    status: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<(Vec<BlocklistEntryRow>, i64), ApiError> {
    let rows = sqlx::query_as::<_, BlocklistEntryRow>(
        r#"SELECT id, public_id, domain, pattern, category::text, source::text,
                  confidence, status::text, added_by, reviewed_by, evidence_url, tags,
                  blocklist_version_added, blocklist_version_removed, created_at, updated_at
           FROM blocklist_entries
           WHERE ($1::text IS NULL OR domain ILIKE '%' || $1 || '%')
             AND ($2::text IS NULL OR category::text = $2)
             AND ($3::text IS NULL OR source::text = $3)
             AND ($4::text IS NULL OR status::text = $4)
           ORDER BY created_at DESC
           LIMIT $5 OFFSET $6"#,
    )
    .bind(search)
    .bind(category)
    .bind(source)
    .bind(status)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    let total = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*)
           FROM blocklist_entries
           WHERE ($1::text IS NULL OR domain ILIKE '%' || $1 || '%')
             AND ($2::text IS NULL OR category::text = $2)
             AND ($3::text IS NULL OR source::text = $3)
             AND ($4::text IS NULL OR status::text = $4)"#,
    )
    .bind(search)
    .bind(category)
    .bind(source)
    .bind(status)
    .fetch_one(db)
    .await?;

    Ok((rows, total))
}

/// Update a blocklist entry (admin).
pub async fn update_blocklist_entry(
    db: &PgPool,
    entry_id: i64,
    category: Option<&str>,
    status: Option<&str>,
    evidence_url: Option<&str>,
    tags: Option<&[String]>,
    blocklist_version_removed: Option<i64>,
) -> Result<BlocklistEntryRow, ApiError> {
    let row = sqlx::query_as::<_, BlocklistEntryRow>(
        r#"UPDATE blocklist_entries SET
               category = COALESCE($2::gambling_category, category),
               status = COALESCE($3::blocklist_entry_status, status),
               evidence_url = COALESCE($4, evidence_url),
               tags = COALESCE($5, tags),
               blocklist_version_removed = COALESCE($6, blocklist_version_removed),
               updated_at = NOW()
           WHERE id = $1
           RETURNING id, public_id, domain, pattern, category::text, source::text,
                     confidence, status::text, added_by, reviewed_by, evidence_url, tags,
                     blocklist_version_added, blocklist_version_removed, created_at, updated_at"#,
    )
    .bind(entry_id)
    .bind(category)
    .bind(status)
    .bind(evidence_url)
    .bind(tags)
    .bind(blocklist_version_removed)
    .fetch_one(db)
    .await?;
    Ok(row)
}

/// Get a blocklist entry by ID.
pub async fn get_blocklist_entry_by_id(
    db: &PgPool,
    entry_id: i64,
) -> Result<Option<BlocklistEntryRow>, ApiError> {
    let row = sqlx::query_as::<_, BlocklistEntryRow>(
        r#"SELECT id, public_id, domain, pattern, category::text, source::text,
                  confidence, status::text, added_by, reviewed_by, evidence_url, tags,
                  blocklist_version_added, blocklist_version_removed, created_at, updated_at
           FROM blocklist_entries WHERE id = $1"#,
    )
    .bind(entry_id)
    .fetch_optional(db)
    .await?;
    Ok(row)
}

/// Get a blocklist entry by public_id.
pub async fn get_blocklist_entry_by_public_id(
    db: &PgPool,
    public_id: Uuid,
) -> Result<Option<BlocklistEntryRow>, ApiError> {
    let row = sqlx::query_as::<_, BlocklistEntryRow>(
        r#"SELECT id, public_id, domain, pattern, category::text, source::text,
                  confidence, status::text, added_by, reviewed_by, evidence_url, tags,
                  blocklist_version_added, blocklist_version_removed, created_at, updated_at
           FROM blocklist_entries WHERE public_id = $1"#,
    )
    .bind(public_id)
    .fetch_optional(db)
    .await?;
    Ok(row)
}

/// Insert federated reports (batch).
pub async fn insert_federated_reports(
    db: &PgPool,
    device_id: i64,
    reports: &[(String, Option<String>, f64)], // (domain, heuristic_match_type, confidence)
) -> Result<(i64, i64), ApiError> {
    let mut accepted = 0i64;
    let mut duplicates = 0i64;

    for (domain, heuristic, confidence) in reports {
        // Check for existing pending report from same device for same domain
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM federated_reports WHERE device_id = $1 AND domain = $2 AND review_status = 'pending'",
        )
        .bind(device_id)
        .bind(domain)
        .fetch_one(db)
        .await?;

        if exists > 0 {
            duplicates += 1;
            continue;
        }

        sqlx::query(
            r#"INSERT INTO federated_reports (device_id, domain, heuristic_match_type, confidence)
               VALUES ($1, $2, $3, $4)"#,
        )
        .bind(device_id)
        .bind(domain)
        .bind(heuristic.as_deref())
        .bind(confidence)
        .execute(db)
        .await?;

        accepted += 1;
    }

    Ok((accepted, duplicates))
}

/// Get the review queue: aggregate federated reports by domain.
pub async fn get_review_queue(
    db: &PgPool,
    min_reports: Option<i64>,
    min_confidence: Option<f64>,
    sort_by: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<(Vec<ReviewQueueEntry>, i64), ApiError> {
    let order = match sort_by {
        Some("confidence_desc") => "avg_conf DESC",
        Some("reports_desc") => "report_count DESC",
        Some("oldest_first") => "first_reported ASC",
        _ => "report_count DESC",
    };

    // We need dynamic ORDER BY, so build the query string
    let query = format!(
        r#"SELECT domain,
                  COUNT(*) as report_count,
                  MIN(reported_at) as first_reported,
                  MAX(reported_at) as last_reported,
                  AVG(confidence) as avg_confidence
           FROM federated_reports
           WHERE review_status = 'pending'
           GROUP BY domain
           HAVING COUNT(*) >= $1
              AND AVG(confidence) >= $2
           ORDER BY {order}
           LIMIT $3 OFFSET $4"#
    );

    let rows: Vec<ReviewQueueEntry> = sqlx::query_as(&query)
        .bind(min_reports.unwrap_or(1))
        .bind(min_confidence.unwrap_or(0.0))
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await
        .map_err(|e| ApiError::Internal {
            message: format!("Review queue query failed: {e}"),
        })?;

    let total_query = r#"SELECT COUNT(DISTINCT domain)
           FROM federated_reports
           WHERE review_status = 'pending'"#;
    let total = sqlx::query_scalar::<_, i64>(total_query)
        .fetch_one(db)
        .await?;

    Ok((rows, total))
}

/// Create a new blocklist version (admin, when publishing changes).
pub async fn create_blocklist_version(
    db: &PgPool,
    version_number: i64,
    entry_count: i64,
    signature: &[u8],
) -> Result<BlocklistVersionRow, ApiError> {
    let row = sqlx::query_as::<_, BlocklistVersionRow>(
        r#"INSERT INTO blocklist_versions (version_number, entry_count, signature)
           VALUES ($1, $2, $3)
           RETURNING id, version_number, entry_count, signature, published_at"#,
    )
    .bind(version_number)
    .bind(entry_count)
    .bind(signature)
    .fetch_one(db)
    .await?;
    Ok(row)
}

/// Resolve a review queue domain: promote or reject all pending reports.
pub async fn resolve_review_queue_domain(
    db: &PgPool,
    domain: &str,
    action: &str, // "promote" or "reject"
    reviewed_by: i64,
    resolved_to_entry_id: Option<i64>,
) -> Result<i64, ApiError> {
    let new_status = match action {
        "promote" => "promoted",
        "reject" => "rejected",
        _ => {
            return Err(ApiError::Validation {
                message: "action must be 'promote' or 'reject'".into(),
                details: None,
            })
        }
    };

    let result = sqlx::query(
        r#"UPDATE federated_reports
           SET review_status = $3::federated_report_status,
               reviewed_by_account_id = $2,
               resolved_to_entry_id = $4
           WHERE domain = $1 AND review_status = 'pending'"#,
    )
    .bind(domain)
    .bind(reviewed_by)
    .bind(new_status)
    .bind(resolved_to_entry_id)
    .execute(db)
    .await?;

    Ok(result.rows_affected() as i64)
}
