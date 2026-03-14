use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::error::ApiError;

// ---------------------------------------------------------------------------
// Row types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct HourlyStatRow {
    pub bucket: DateTime<Utc>,
    pub device_id: i64,
    pub event_type: String,
    pub event_count: i64,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct DailyStatRow {
    pub day: DateTime<Utc>,
    pub device_id: i64,
    pub event_type: String,
    pub event_count: i64,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct TrendRow {
    pub id: i64,
    pub device_id: i64,
    pub metric_name: String,
    pub metric_value: serde_json::Value,
    pub computed_at: DateTime<Utc>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AnalyticsSummary {
    pub total_events: i64,
    pub total_blocks: i64,
    pub total_bypass_attempts: i64,
    pub total_tamper_events: i64,
    pub unique_event_types: i64,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct HeatmapCell {
    pub hour_of_day: i32,
    pub day_of_week: i32,
    pub event_count: i64,
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Get hourly aggregated stats for a device within a time range.
pub async fn get_hourly_stats(
    db: &PgPool,
    device_id: i64,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<HourlyStatRow>, ApiError> {
    let rows = sqlx::query_as::<_, HourlyStatRow>(
        r#"SELECT bucket, device_id, event_type, event_count
           FROM hourly_block_stats
           WHERE device_id = $1
             AND bucket >= $2
             AND bucket <= $3
           ORDER BY bucket ASC"#,
    )
    .bind(device_id)
    .bind(from)
    .bind(to)
    .fetch_all(db)
    .await?;
    Ok(rows)
}

/// Get daily aggregated stats for a device within a time range.
pub async fn get_daily_stats(
    db: &PgPool,
    device_id: i64,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<DailyStatRow>, ApiError> {
    let rows = sqlx::query_as::<_, DailyStatRow>(
        r#"SELECT day, device_id, event_type, event_count
           FROM daily_block_stats
           WHERE device_id = $1
             AND day >= $2
             AND day <= $3
           ORDER BY day ASC"#,
    )
    .bind(device_id)
    .bind(from)
    .bind(to)
    .fetch_all(db)
    .await?;
    Ok(rows)
}

/// Get pre-computed trends for a device, optionally filtered by metric names.
pub async fn get_trends(
    db: &PgPool,
    device_id: i64,
    metric_names: &[String],
) -> Result<Vec<TrendRow>, ApiError> {
    let rows = if metric_names.is_empty() {
        sqlx::query_as::<_, TrendRow>(
            r#"SELECT id, device_id, metric_name, metric_value,
                      computed_at, period_start, period_end
               FROM analytics_trends
               WHERE device_id = $1
               ORDER BY computed_at DESC"#,
        )
        .bind(device_id)
        .fetch_all(db)
        .await?
    } else {
        sqlx::query_as::<_, TrendRow>(
            r#"SELECT id, device_id, metric_name, metric_value,
                      computed_at, period_start, period_end
               FROM analytics_trends
               WHERE device_id = $1
                 AND metric_name = ANY($2)
               ORDER BY computed_at DESC"#,
        )
        .bind(device_id)
        .bind(metric_names)
        .fetch_all(db)
        .await?
    };
    Ok(rows)
}

/// Get an aggregate summary of events for a device within a time range.
pub async fn get_summary(
    db: &PgPool,
    device_id: i64,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<AnalyticsSummary, ApiError> {
    let row = sqlx::query_as::<_, (i64, i64, i64, i64, i64)>(
        r#"SELECT
               COUNT(*) AS total_events,
               COUNT(*) FILTER (WHERE event_type = 'block') AS total_blocks,
               COUNT(*) FILTER (WHERE event_type = 'bypass_attempt') AS total_bypass_attempts,
               COUNT(*) FILTER (WHERE event_type IN ('tamper_detected', 'tamper_self_healed')) AS total_tamper_events,
               COUNT(DISTINCT event_type) AS unique_event_types
           FROM events
           WHERE device_id = $1
             AND created_at >= $2
             AND created_at <= $3"#,
    )
    .bind(device_id)
    .bind(from)
    .bind(to)
    .fetch_one(db)
    .await?;

    Ok(AnalyticsSummary {
        total_events: row.0,
        total_blocks: row.1,
        total_bypass_attempts: row.2,
        total_tamper_events: row.3,
        unique_event_types: row.4,
    })
}

/// Get a heatmap of event counts by hour-of-day and day-of-week for a device.
pub async fn get_heatmap(
    db: &PgPool,
    device_id: i64,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<HeatmapCell>, ApiError> {
    let rows = sqlx::query_as::<_, HeatmapCell>(
        r#"SELECT
               EXTRACT(HOUR FROM created_at)::int AS hour_of_day,
               EXTRACT(ISODOW FROM created_at)::int AS day_of_week,
               COUNT(*)::bigint AS event_count
           FROM events
           WHERE device_id = $1
             AND created_at >= $2
             AND created_at <= $3
           GROUP BY hour_of_day, day_of_week
           ORDER BY day_of_week, hour_of_day"#,
    )
    .bind(device_id)
    .bind(from)
    .bind(to)
    .fetch_all(db)
    .await?;
    Ok(rows)
}

/// Enforce that the caller has visibility into the given device.
///
/// Checks whether the caller owns the device, is a partner of the device owner,
/// or is in the same organization. Returns 403 if access is denied.
pub async fn enforce_enrollment_visibility(
    db: &PgPool,
    caller_account_id: i64,
    device_id: i64,
) -> Result<(), ApiError> {
    // Check 1: Caller owns the device directly
    let owns = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM devices WHERE id = $1 AND account_id = $2",
    )
    .bind(device_id)
    .bind(caller_account_id)
    .fetch_one(db)
    .await?;

    if owns > 0 {
        return Ok(());
    }

    // Check 2: Caller has a partner relationship with the device owner
    let partner = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*)
           FROM devices d
           JOIN partner_relationships pr
               ON (pr.account_id = $2 AND pr.partner_id = d.account_id)
               OR (pr.partner_id = $2 AND pr.account_id = d.account_id)
           WHERE d.id = $1
             AND pr.status = 'accepted'"#,
    )
    .bind(device_id)
    .bind(caller_account_id)
    .fetch_one(db)
    .await?;

    if partner > 0 {
        return Ok(());
    }

    // Check 3: Caller is in the same organization as the device
    let org_member = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*)
           FROM organization_devices od
           JOIN organization_members om ON om.organization_id = od.organization_id
           WHERE od.device_id = $1
             AND om.account_id = $2"#,
    )
    .bind(device_id)
    .bind(caller_account_id)
    .fetch_one(db)
    .await?;

    if org_member > 0 {
        return Ok(());
    }

    Err(ApiError::Forbidden {
        message: "You do not have access to analytics for this device".into(),
    })
}
