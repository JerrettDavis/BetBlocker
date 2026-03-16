use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;

/// Run trend computations for all active devices.
///
/// Iterates every device with status `'active'` and computes all metric types,
/// storing results in `analytics_trends`.
///
/// # Errors
/// Returns an error if a database query fails.
#[allow(unused)]
pub async fn compute_trends(db: &PgPool) -> Result<()> {
    let now = Utc::now();
    let period_start = now - Duration::days(30);
    let period_end = now;

    // Fetch active device IDs.
    let device_ids: Vec<(i64,)> = sqlx::query_as("SELECT id FROM devices WHERE status = 'active'")
        .fetch_all(db)
        .await?;

    for (device_id,) in device_ids {
        tracing::debug!(device_id, "computing trends");

        // Compute all metrics, logging errors but continuing on failure.
        let metrics: Vec<(&str, Result<serde_json::Value>)> = vec![
            (
                "peak_hour",
                compute_peak_hour(db, device_id, period_start, period_end).await,
            ),
            (
                "day_of_week_pattern",
                compute_day_of_week_pattern(db, device_id, period_start, period_end).await,
            ),
            (
                "category_distribution",
                compute_category_distribution(db, device_id, period_start, period_end).await,
            ),
            ("streak", compute_streak(db, device_id).await),
            ("weekly_trend", compute_weekly_trend(db, device_id).await),
        ];

        for (metric_name, result) in metrics {
            match result {
                Ok(value) => {
                    if let Err(e) = upsert_trend(
                        db,
                        device_id,
                        metric_name,
                        &value,
                        now,
                        period_start,
                        period_end,
                    )
                    .await
                    {
                        tracing::warn!(
                            device_id,
                            metric_name,
                            error = %e,
                            "failed to upsert trend"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        device_id,
                        metric_name,
                        error = %e,
                        "failed to compute metric"
                    );
                }
            }
        }
    }

    Ok(())
}

/// Find the hour of day with the most blocks in the given period.
#[allow(unused)]
pub async fn compute_peak_hour(
    db: &PgPool,
    device_id: i64,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<serde_json::Value> {
    let rows: Vec<(f64, i64)> = sqlx::query_as(
        r"
        SELECT EXTRACT(HOUR FROM bucket)::float8 AS hour,
               SUM(event_count)::int8 AS total
        FROM hourly_block_stats
        WHERE device_id = $1
          AND bucket >= $2
          AND bucket < $3
        GROUP BY hour
        ORDER BY total DESC
        LIMIT 1
        ",
    )
    .bind(device_id)
    .bind(period_start)
    .bind(period_end)
    .fetch_all(db)
    .await?;

    let result = if let Some((hour, total)) = rows.first() {
        serde_json::json!({
            "peak_hour": *hour as u8,
            "total_blocks": total,
        })
    } else {
        serde_json::json!({ "peak_hour": null, "total_blocks": 0 })
    };

    Ok(result)
}

/// Aggregate block counts by day of week for the given period.
#[allow(unused)]
pub async fn compute_day_of_week_pattern(
    db: &PgPool,
    device_id: i64,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<serde_json::Value> {
    let rows: Vec<(f64, i64)> = sqlx::query_as(
        r"
        SELECT EXTRACT(DOW FROM bucket)::float8 AS dow,
               SUM(event_count)::int8 AS total
        FROM hourly_block_stats
        WHERE device_id = $1
          AND bucket >= $2
          AND bucket < $3
        GROUP BY dow
        ORDER BY dow
        ",
    )
    .bind(device_id)
    .bind(period_start)
    .bind(period_end)
    .fetch_all(db)
    .await?;

    let day_names = [
        "sunday",
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
    ];

    let mut pattern = serde_json::Map::new();
    for (dow, total) in &rows {
        let idx = *dow as usize;
        let name = day_names.get(idx).unwrap_or(&"unknown");
        pattern.insert((*name).to_string(), serde_json::json!(total));
    }

    Ok(serde_json::Value::Object(pattern))
}

/// Count blocks per category from events metadata for the given period.
#[allow(unused)]
pub async fn compute_category_distribution(
    db: &PgPool,
    device_id: i64,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<serde_json::Value> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        r"
        SELECT COALESCE(metadata->>'category', 'unknown') AS category,
               COUNT(*)::int8 AS cnt
        FROM events
        WHERE device_id = $1
          AND event_type = 'block'
          AND created_at >= $2
          AND created_at < $3
        GROUP BY category
        ORDER BY cnt DESC
        ",
    )
    .bind(device_id)
    .bind(period_start)
    .bind(period_end)
    .fetch_all(db)
    .await?;

    let mut distribution = serde_json::Map::new();
    for (category, count) in &rows {
        distribution.insert(category.clone(), serde_json::json!(count));
    }

    Ok(serde_json::Value::Object(distribution))
}

/// Compute the longest streak of consecutive zero-block days.
#[allow(unused)]
pub async fn compute_streak(db: &PgPool, device_id: i64) -> Result<serde_json::Value> {
    // Fetch daily block counts ordered by day.
    let rows: Vec<(DateTime<Utc>, i64)> = sqlx::query_as(
        r"
        SELECT day, SUM(event_count)::int8 AS total
        FROM daily_block_stats
        WHERE device_id = $1
        GROUP BY day
        ORDER BY day
        ",
    )
    .bind(device_id)
    .fetch_all(db)
    .await?;

    let daily_counts: Vec<(DateTime<Utc>, i64)> = rows;
    let result = compute_streak_from_daily(&daily_counts);

    Ok(result)
}

/// Pure computation: find the longest consecutive run of zero-block days.
///
/// `daily_counts` must be sorted by date ascending. Days not present in the
/// input are assumed to have zero blocks (they extend the streak).
pub fn compute_streak_from_daily(daily_counts: &[(DateTime<Utc>, i64)]) -> serde_json::Value {
    if daily_counts.is_empty() {
        return serde_json::json!({
            "longest_streak": 0,
            "current_streak": 0,
        });
    }

    let mut longest = 0i64;
    let mut current = 0i64;

    // We need to account for gaps between days in the data.
    // A missing day means zero blocks, so it extends the streak.
    let mut prev_date: Option<DateTime<Utc>> = None;

    for (day, count) in daily_counts {
        // If there's a gap from the previous day, those missing days are zero-block days.
        if let Some(prev) = prev_date {
            let gap_days = (*day - prev).num_days() - 1;
            if gap_days > 0 {
                // Previous day was a recorded day. If current streak was active
                // (or the previous recorded day had 0), extend through the gap.
                // If the previous recorded day broke the streak (count > 0),
                // the gap starts a new streak.
                if current > 0 {
                    // We were in a streak, the gap extends it.
                    current += gap_days;
                } else {
                    // Previous day broke the streak; gap starts a new one.
                    current = gap_days;
                }
                longest = longest.max(current);
            }
        }

        if *count == 0 {
            current += 1;
        } else {
            current = 0;
        }
        longest = longest.max(current);
        prev_date = Some(*day);
    }

    serde_json::json!({
        "longest_streak": longest,
        "current_streak": current,
    })
}

/// Compare this week's blocks vs last week's blocks.
#[allow(unused)]
pub async fn compute_weekly_trend(db: &PgPool, device_id: i64) -> Result<serde_json::Value> {
    let now = Utc::now();
    let this_week_start = now - Duration::days(7);
    let last_week_start = now - Duration::days(14);

    let this_week: (Option<i64>,) = sqlx::query_as(
        r"
        SELECT SUM(event_count)::int8
        FROM daily_block_stats
        WHERE device_id = $1
          AND day >= $2
          AND day < $3
        ",
    )
    .bind(device_id)
    .bind(this_week_start)
    .bind(now)
    .fetch_one(db)
    .await?;

    let last_week: (Option<i64>,) = sqlx::query_as(
        r"
        SELECT SUM(event_count)::int8
        FROM daily_block_stats
        WHERE device_id = $1
          AND day >= $2
          AND day < $3
        ",
    )
    .bind(device_id)
    .bind(last_week_start)
    .bind(this_week_start)
    .fetch_one(db)
    .await?;

    let this_count = this_week.0.unwrap_or(0);
    let last_count = last_week.0.unwrap_or(0);

    let result = compute_weekly_trend_percentage(this_count, last_count);
    Ok(result)
}

/// Pure computation: calculate the percentage change between two weeks.
///
/// Returns a JSON value with `this_week`, `last_week`, `change_percent`, and
/// `direction` fields.
pub fn compute_weekly_trend_percentage(this_week: i64, last_week: i64) -> serde_json::Value {
    let (change_percent, direction) = if last_week == 0 {
        if this_week == 0 {
            (0.0, "flat")
        } else {
            (100.0, "up")
        }
    } else {
        let pct = ((this_week - last_week) as f64 / last_week as f64) * 100.0;
        let dir = if pct > 0.0 {
            "up"
        } else if pct < 0.0 {
            "down"
        } else {
            "flat"
        };
        (pct, dir)
    };

    serde_json::json!({
        "this_week": this_week,
        "last_week": last_week,
        "change_percent": change_percent,
        "direction": direction,
    })
}

/// Upsert a computed trend metric into the `analytics_trends` table.
#[allow(unused)]
async fn upsert_trend(
    db: &PgPool,
    device_id: i64,
    metric_name: &str,
    metric_value: &serde_json::Value,
    computed_at: DateTime<Utc>,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<()> {
    sqlx::query(
        r"
        INSERT INTO analytics_trends (device_id, metric_name, metric_value, computed_at, period_start, period_end)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (device_id, metric_name) DO UPDATE
        SET metric_value = EXCLUDED.metric_value,
            computed_at  = EXCLUDED.computed_at,
            period_start = EXCLUDED.period_start,
            period_end   = EXCLUDED.period_end
        ",
    )
    .bind(device_id)
    .bind(metric_name)
    .bind(metric_value)
    .bind(computed_at)
    .bind(period_start)
    .bind(period_end)
    .execute(db)
    .await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn day(year: i32, month: u32, d: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, d, 0, 0, 0).unwrap()
    }

    // ── Streak computation ────────────────────────────────────────────

    #[test]
    fn streak_empty_input() {
        let result = compute_streak_from_daily(&[]);
        assert_eq!(result["longest_streak"], 0);
        assert_eq!(result["current_streak"], 0);
    }

    #[test]
    fn streak_all_zero() {
        let data = vec![
            (day(2025, 1, 1), 0),
            (day(2025, 1, 2), 0),
            (day(2025, 1, 3), 0),
        ];
        let result = compute_streak_from_daily(&data);
        assert_eq!(result["longest_streak"], 3);
        assert_eq!(result["current_streak"], 3);
    }

    #[test]
    fn streak_no_zero_days() {
        let data = vec![
            (day(2025, 1, 1), 5),
            (day(2025, 1, 2), 3),
            (day(2025, 1, 3), 1),
        ];
        let result = compute_streak_from_daily(&data);
        assert_eq!(result["longest_streak"], 0);
        assert_eq!(result["current_streak"], 0);
    }

    #[test]
    fn streak_mixed() {
        let data = vec![
            (day(2025, 1, 1), 5),
            (day(2025, 1, 2), 0),
            (day(2025, 1, 3), 0),
            (day(2025, 1, 4), 0),
            (day(2025, 1, 5), 2),
            (day(2025, 1, 6), 0),
        ];
        let result = compute_streak_from_daily(&data);
        assert_eq!(result["longest_streak"], 3);
        assert_eq!(result["current_streak"], 1);
    }

    #[test]
    fn streak_with_gaps_extends_streak() {
        // Days 1, 2 have data (0 blocks), then gap to day 5 (zero blocks assumed),
        // then day 5 has 0 blocks.
        let data = vec![
            (day(2025, 1, 1), 0),
            (day(2025, 1, 2), 0),
            // gap: day 3 and 4 missing → treated as zero-block days
            (day(2025, 1, 5), 0),
        ];
        let result = compute_streak_from_daily(&data);
        // Days 1, 2, (3), (4), 5 = 5 consecutive zero-block days
        assert_eq!(result["longest_streak"], 5);
        assert_eq!(result["current_streak"], 5);
    }

    #[test]
    fn streak_gap_after_nonzero_starts_new() {
        let data = vec![
            (day(2025, 1, 1), 3), // breaks streak
            // gap: day 2 and 3 missing → 2 zero-block days
            (day(2025, 1, 4), 0), // extends to 3
        ];
        let result = compute_streak_from_daily(&data);
        assert_eq!(result["longest_streak"], 3);
        assert_eq!(result["current_streak"], 3);
    }

    // ── Weekly trend percentage ───────────────────────────────────────

    #[test]
    fn weekly_trend_both_zero() {
        let result = compute_weekly_trend_percentage(0, 0);
        assert_eq!(result["change_percent"], 0.0);
        assert_eq!(result["direction"], "flat");
    }

    #[test]
    fn weekly_trend_last_week_zero_this_week_nonzero() {
        let result = compute_weekly_trend_percentage(10, 0);
        assert_eq!(result["change_percent"], 100.0);
        assert_eq!(result["direction"], "up");
    }

    #[test]
    fn weekly_trend_increase() {
        let result = compute_weekly_trend_percentage(150, 100);
        assert_eq!(result["change_percent"], 50.0);
        assert_eq!(result["direction"], "up");
    }

    #[test]
    fn weekly_trend_decrease() {
        let result = compute_weekly_trend_percentage(50, 100);
        assert_eq!(result["change_percent"], -50.0);
        assert_eq!(result["direction"], "down");
    }

    #[test]
    fn weekly_trend_no_change() {
        let result = compute_weekly_trend_percentage(100, 100);
        assert_eq!(result["change_percent"], 0.0);
        assert_eq!(result["direction"], "flat");
    }

    #[test]
    fn weekly_trend_fields_present() {
        let result = compute_weekly_trend_percentage(75, 50);
        assert!(result.get("this_week").is_some());
        assert!(result.get("last_week").is_some());
        assert!(result.get("change_percent").is_some());
        assert!(result.get("direction").is_some());
        assert_eq!(result["this_week"], 75);
        assert_eq!(result["last_week"], 50);
    }
}
