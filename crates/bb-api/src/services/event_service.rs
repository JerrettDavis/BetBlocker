use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct EventRow {
    pub id: i64,
    pub public_id: Uuid,
    pub device_id: i64,
    pub enrollment_id: Option<i64>,
    pub event_type: String,
    pub category: String,
    pub severity: String,
    pub metadata: serde_json::Value,
    pub occurred_at: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
}

/// Batch-insert events, returning counts of accepted/rejected.
pub async fn batch_insert_events(
    db: &PgPool,
    device_id: i64,
    enrollment_id: Option<i64>,
    events: &[EventInput],
) -> Result<(i64, i64, Vec<String>), ApiError> {
    let mut accepted = 0i64;
    let mut rejected = 0i64;
    let mut errors = Vec::new();

    let now = Utc::now();
    let seven_days_ago = now - chrono::Duration::days(7);

    for (i, event) in events.iter().enumerate() {
        // Validate occurred_at
        if event.occurred_at > now {
            rejected += 1;
            errors.push(format!("events[{i}]: occurred_at is in the future"));
            continue;
        }
        if event.occurred_at < seven_days_ago {
            rejected += 1;
            errors.push(format!("events[{i}]: occurred_at is older than 7 days"));
            continue;
        }

        let result = sqlx::query(
            r#"INSERT INTO events
                   (device_id, enrollment_id, event_type, category, severity, metadata, occurred_at)
               VALUES ($1, $2, $3::event_type, $4::event_category, $5::event_severity, $6, $7)"#,
        )
        .bind(device_id)
        .bind(enrollment_id)
        .bind(&event.event_type)
        .bind(&event.category)
        .bind(&event.severity)
        .bind(&event.metadata)
        .bind(event.occurred_at)
        .execute(db)
        .await;

        match result {
            Ok(_) => accepted += 1,
            Err(e) => {
                rejected += 1;
                errors.push(format!("events[{i}]: {e}"));
            }
        }
    }

    Ok((accepted, rejected, errors))
}

/// Input for a single event in a batch.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct EventInput {
    pub event_type: String,
    pub category: String,
    pub severity: String,
    pub metadata: serde_json::Value,
    pub occurred_at: DateTime<Utc>,
}

/// Query events with enrollment-scoped visibility.
pub async fn query_events(
    db: &PgPool,
    visible_enrollment_ids: &[i64],
    device_id_filter: Option<i64>,
    enrollment_id_filter: Option<i64>,
    event_type_filter: Option<&str>,
    category_filter: Option<&str>,
    severity_filter: Option<&str>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    limit: i64,
    offset: i64,
) -> Result<(Vec<EventRow>, i64), ApiError> {
    let rows = sqlx::query_as::<_, EventRow>(
        r#"SELECT id, public_id, device_id, enrollment_id,
                  event_type::text, category::text, severity::text,
                  metadata, occurred_at, received_at
           FROM events
           WHERE enrollment_id = ANY($1)
             AND ($2::bigint IS NULL OR device_id = $2)
             AND ($3::bigint IS NULL OR enrollment_id = $3)
             AND ($4::text IS NULL OR event_type::text = $4)
             AND ($5::text IS NULL OR category::text = $5)
             AND ($6::text IS NULL OR severity::text = $6)
             AND ($7::timestamptz IS NULL OR occurred_at >= $7)
             AND ($8::timestamptz IS NULL OR occurred_at <= $8)
           ORDER BY occurred_at DESC
           LIMIT $9 OFFSET $10"#,
    )
    .bind(visible_enrollment_ids)
    .bind(device_id_filter)
    .bind(enrollment_id_filter)
    .bind(event_type_filter)
    .bind(category_filter)
    .bind(severity_filter)
    .bind(from)
    .bind(to)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    let total = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*)
           FROM events
           WHERE enrollment_id = ANY($1)
             AND ($2::bigint IS NULL OR device_id = $2)
             AND ($3::bigint IS NULL OR enrollment_id = $3)
             AND ($4::text IS NULL OR event_type::text = $4)
             AND ($5::text IS NULL OR category::text = $5)
             AND ($6::text IS NULL OR severity::text = $6)
             AND ($7::timestamptz IS NULL OR occurred_at >= $7)
             AND ($8::timestamptz IS NULL OR occurred_at <= $8)"#,
    )
    .bind(visible_enrollment_ids)
    .bind(device_id_filter)
    .bind(enrollment_id_filter)
    .bind(event_type_filter)
    .bind(category_filter)
    .bind(severity_filter)
    .bind(from)
    .bind(to)
    .fetch_one(db)
    .await?;

    Ok((rows, total))
}

/// Summary of events grouped by period.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct EventSummaryBucket {
    pub period: DateTime<Utc>,
    pub total_blocks: i64,
    pub total_bypass_attempts: i64,
    pub total_tamper_events: i64,
    pub total_events: i64,
}

/// Get event summary for the given enrollments and time range.
pub async fn get_event_summary(
    db: &PgPool,
    visible_enrollment_ids: &[i64],
    period: &str, // "hour", "day", "week", "month"
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
) -> Result<Vec<EventSummaryBucket>, ApiError> {
    let trunc = match period {
        "hour" => "hour",
        "day" => "day",
        "week" => "week",
        "month" => "month",
        _ => "day",
    };

    let query = format!(
        r#"SELECT date_trunc('{trunc}', occurred_at) as period,
                  COUNT(*) FILTER (WHERE event_type = 'block') as total_blocks,
                  COUNT(*) FILTER (WHERE event_type = 'bypass_attempt') as total_bypass_attempts,
                  COUNT(*) FILTER (WHERE event_type IN ('tamper_detected', 'tamper_self_healed')) as total_tamper_events,
                  COUNT(*) as total_events
           FROM events
           WHERE enrollment_id = ANY($1)
             AND ($2::timestamptz IS NULL OR occurred_at >= $2)
             AND ($3::timestamptz IS NULL OR occurred_at <= $3)
           GROUP BY period
           ORDER BY period ASC"#
    );

    let rows = sqlx::query_as::<_, EventSummaryBucket>(&query)
        .bind(visible_enrollment_ids)
        .bind(from)
        .bind(to)
        .fetch_all(db)
        .await?;

    Ok(rows)
}

/// Get all enrollment IDs visible to an account (own + partner/authority enrollments).
pub async fn get_visible_enrollment_ids(
    db: &PgPool,
    account_id: i64,
) -> Result<Vec<i64>, ApiError> {
    let ids = sqlx::query_scalar::<_, i64>(
        r#"SELECT id FROM enrollments
           WHERE account_id = $1 OR enrolled_by = $1"#,
    )
    .bind(account_id)
    .fetch_all(db)
    .await?;
    Ok(ids)
}
