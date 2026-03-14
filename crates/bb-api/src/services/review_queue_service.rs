use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::error::ApiError;

// ---------------------------------------------------------------------------
// Row types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct ReviewItemRow {
    pub id: i64,
    pub domain: String,
    pub source: String,
    pub source_metadata: serde_json::Value,
    pub confidence_score: f64,
    pub classification: serde_json::Value,
    pub status: String,
    pub reviewed_by: Option<i64>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Filter / sort helpers
// ---------------------------------------------------------------------------

/// Filters for listing review queue items.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct ReviewFilters {
    pub status: Option<String>,
    pub source: Option<String>,
    pub min_confidence: Option<f64>,
    pub search: Option<String>,
    pub sort_by: Option<String>,
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// List discovery candidates with filters and pagination.
///
/// Federated-sourced candidates are included alongside other sources and can
/// be filtered with `source = "federated"` in [`ReviewFilters`].
pub async fn list_review_items(
    db: &PgPool,
    filters: &ReviewFilters,
    limit: i64,
    offset: i64,
) -> Result<(Vec<ReviewItemRow>, i64), ApiError> {
    let order = match filters.sort_by.as_deref() {
        Some("confidence_desc") => "confidence_score DESC",
        Some("confidence_asc") => "confidence_score ASC",
        Some("oldest_first") => "created_at ASC",
        Some("newest_first") => "created_at DESC",
        _ => "created_at DESC",
    };

    let query = format!(
        r#"SELECT id, domain, source::text, source_metadata, confidence_score,
                  classification, status::text, reviewed_by, reviewed_at, created_at
           FROM discovery_candidates
           WHERE ($1::text IS NULL OR status::text = $1)
             AND ($2::text IS NULL OR source::text = $2)
             AND ($3::double precision IS NULL OR confidence_score >= $3)
             AND ($4::text IS NULL OR domain ILIKE '%' || $4 || '%')
           ORDER BY {order}
           LIMIT $5 OFFSET $6"#
    );

    let rows = sqlx::query_as::<_, ReviewItemRow>(&query)
        .bind(filters.status.as_deref())
        .bind(filters.source.as_deref())
        .bind(filters.min_confidence)
        .bind(filters.search.as_deref())
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?;

    let total = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*)
           FROM discovery_candidates
           WHERE ($1::text IS NULL OR status::text = $1)
             AND ($2::text IS NULL OR source::text = $2)
             AND ($3::double precision IS NULL OR confidence_score >= $3)
             AND ($4::text IS NULL OR domain ILIKE '%' || $4 || '%')"#,
    )
    .bind(filters.status.as_deref())
    .bind(filters.source.as_deref())
    .bind(filters.min_confidence)
    .bind(filters.search.as_deref())
    .fetch_one(db)
    .await?;

    Ok((rows, total))
}

/// Get a single review item by ID.
pub async fn get_review_item(db: &PgPool, id: i64) -> Result<ReviewItemRow, ApiError> {
    let row = sqlx::query_as::<_, ReviewItemRow>(
        r#"SELECT id, domain, source::text, source_metadata, confidence_score,
                  classification, status::text, reviewed_by, reviewed_at, created_at
           FROM discovery_candidates
           WHERE id = $1"#,
    )
    .bind(id)
    .fetch_one(db)
    .await?;
    Ok(row)
}

/// Approve a discovery candidate: set status to approved and insert into blocklist_entries.
///
/// For federated-sourced candidates, also updates `federated_aggregates` to
/// `promoted` status.
pub async fn approve_item(
    db: &PgPool,
    id: i64,
    reviewer_id: i64,
    category: &str,
) -> Result<(), ApiError> {
    // Update discovery candidate status
    sqlx::query(
        r#"UPDATE discovery_candidates
           SET status = 'approved'::discovery_candidate_status,
               reviewed_by = $2,
               reviewed_at = NOW()
           WHERE id = $1"#,
    )
    .bind(id)
    .bind(reviewer_id)
    .execute(db)
    .await?;

    // Fetch the candidate domain and source to create a blocklist entry
    let (domain, source) = sqlx::query_as::<_, (String, String)>(
        "SELECT domain, source::text FROM discovery_candidates WHERE id = $1",
    )
    .bind(id)
    .fetch_one(db)
    .await?;

    // Determine the blocklist source: preserve 'federated' origin when applicable.
    let blocklist_source = if source == "federated" {
        "federated"
    } else {
        "curated"
    };

    // Insert into blocklist_entries
    sqlx::query(
        r#"INSERT INTO blocklist_entries
               (domain, category, source, confidence, added_by, status)
           VALUES ($1, $2::gambling_category, $3::blocklist_source, 1.0, $4, 'active')
           ON CONFLICT (domain) DO NOTHING"#,
    )
    .bind(&domain)
    .bind(category)
    .bind(blocklist_source)
    .bind(reviewer_id)
    .execute(db)
    .await?;

    // If this was a federated candidate, advance the aggregate status.
    if source == "federated" {
        sqlx::query(
            r#"UPDATE federated_aggregates
               SET status     = 'promoted'::federated_aggregate_status,
                   updated_at = NOW()
               WHERE domain = $1"#,
        )
        .bind(&domain)
        .execute(db)
        .await?;
    }

    Ok(())
}

/// Reject a discovery candidate.
///
/// For federated-sourced candidates, also updates `federated_aggregates` to
/// `rejected` status.
pub async fn reject_item(db: &PgPool, id: i64, reviewer_id: i64) -> Result<(), ApiError> {
    let (domain, source) = sqlx::query_as::<_, (String, String)>(
        "SELECT domain, source::text FROM discovery_candidates WHERE id = $1",
    )
    .bind(id)
    .fetch_one(db)
    .await?;

    sqlx::query(
        r#"UPDATE discovery_candidates
           SET status = 'rejected'::discovery_candidate_status,
               reviewed_by = $2,
               reviewed_at = NOW()
           WHERE id = $1"#,
    )
    .bind(id)
    .bind(reviewer_id)
    .execute(db)
    .await?;

    // If this was a federated candidate, reject the aggregate too.
    if source == "federated" {
        sqlx::query(
            r#"UPDATE federated_aggregates
               SET status     = 'rejected'::federated_aggregate_status,
                   updated_at = NOW()
               WHERE domain = $1"#,
        )
        .bind(&domain)
        .execute(db)
        .await?;
    }

    Ok(())
}

/// Defer a discovery candidate for later review.
pub async fn defer_item(db: &PgPool, id: i64, reviewer_id: i64) -> Result<(), ApiError> {
    sqlx::query(
        r#"UPDATE discovery_candidates
           SET status = 'deferred'::discovery_candidate_status,
               reviewed_by = $2,
               reviewed_at = NOW()
           WHERE id = $1"#,
    )
    .bind(id)
    .bind(reviewer_id)
    .execute(db)
    .await?;
    Ok(())
}

/// Bulk approve multiple discovery candidates.
pub async fn bulk_approve(
    db: &PgPool,
    ids: &[i64],
    reviewer_id: i64,
    category: &str,
) -> Result<usize, ApiError> {
    let mut count = 0usize;
    for &id in ids {
        approve_item(db, id, reviewer_id, category).await?;
        count += 1;
    }
    Ok(count)
}

/// Bulk reject multiple discovery candidates.
pub async fn bulk_reject(
    db: &PgPool,
    ids: &[i64],
    reviewer_id: i64,
) -> Result<usize, ApiError> {
    let result = sqlx::query(
        r#"UPDATE discovery_candidates
           SET status = 'rejected'::discovery_candidate_status,
               reviewed_by = $2,
               reviewed_at = NOW()
           WHERE id = ANY($1)
             AND status = 'pending'::discovery_candidate_status"#,
    )
    .bind(ids)
    .bind(reviewer_id)
    .execute(db)
    .await?;
    Ok(result.rows_affected() as usize)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that `ReviewFilters` can represent a federated-only filter.
    #[test]
    fn review_filters_federated_source() {
        let filters = ReviewFilters {
            status: Some("pending".to_string()),
            source: Some("federated".to_string()),
            min_confidence: Some(0.7),
            search: None,
            sort_by: None,
        };
        assert_eq!(filters.source.as_deref(), Some("federated"));
        assert_eq!(filters.min_confidence, Some(0.7));
    }

    /// Verify the default ReviewFilters is all-None.
    #[test]
    fn review_filters_defaults_are_none() {
        let filters = ReviewFilters::default();
        assert!(filters.status.is_none());
        assert!(filters.source.is_none());
        assert!(filters.min_confidence.is_none());
        assert!(filters.search.is_none());
        assert!(filters.sort_by.is_none());
    }
}
