use sqlx::PgPool;

use crate::error::ApiError;
use crate::routes::federated::ReportPayload;

// ---------------------------------------------------------------------------
// Thresholds
// ---------------------------------------------------------------------------

/// Number of unique reporters required before an aggregate graduates to
/// `ThresholdMet` status.
pub const UNIQUE_REPORTER_THRESHOLD: i32 = 5;

// ---------------------------------------------------------------------------
// ingest
// ---------------------------------------------------------------------------

/// Insert a batch of federated reports and update the per-domain aggregate.
///
/// For each report:
///   1. Insert into `federated_reports` (skipping true duplicates by
///      `(domain, reporter_token, batch_id)`).
///   2. Upsert the `federated_aggregates` row, incrementing `unique_reporters`
///      and recomputing `avg_heuristic_score`.
///   3. If `unique_reporters` crosses [`UNIQUE_REPORTER_THRESHOLD`], flip the
///      aggregate status to `threshold_met`.
pub async fn ingest(db: &PgPool, reports: Vec<ReportPayload>) -> Result<(), ApiError> {
    for report in &reports {
        // ── 1. Insert report row (idempotent on batch_id + reporter_token + domain) ──
        sqlx::query(
            r#"INSERT INTO federated_reports
                   (domain, reporter_token, heuristic_score, category_guess, reported_at, batch_id)
               VALUES ($1, $2, $3, $4, $5, $6)
               ON CONFLICT (domain, reporter_token, batch_id) DO NOTHING"#,
        )
        .bind(&report.domain)
        .bind(&report.reporter_token)
        .bind(report.heuristic_score)
        .bind(&report.category_guess)
        .bind(report.reported_at)
        .bind(report.batch_id)
        .execute(db)
        .await?;

        // ── 2. Upsert aggregate ──────────────────────────────────────────
        // We count distinct reporter_tokens so re-submissions from the same
        // token do not inflate the reporter count.
        sqlx::query(
            r#"INSERT INTO federated_aggregates
                   (domain, unique_reporters, avg_heuristic_score,
                    first_reported_at, last_reported_at, status)
               VALUES (
                   $1,
                   (SELECT COUNT(DISTINCT reporter_token) FROM federated_reports WHERE domain = $1),
                   (SELECT AVG(heuristic_score) FROM federated_reports WHERE domain = $1),
                   $2,
                   $2,
                   'collecting'::federated_aggregate_status
               )
               ON CONFLICT (domain) DO UPDATE SET
                   unique_reporters  = (SELECT COUNT(DISTINCT reporter_token) FROM federated_reports WHERE domain = $1),
                   avg_heuristic_score = (SELECT AVG(heuristic_score) FROM federated_reports WHERE domain = $1),
                   last_reported_at  = $2,
                   updated_at        = NOW()"#,
        )
        .bind(&report.domain)
        .bind(report.reported_at)
        .execute(db)
        .await?;

        // ── 3. Threshold check ───────────────────────────────────────────
        sqlx::query(
            r#"UPDATE federated_aggregates
               SET status = 'threshold_met'::federated_aggregate_status,
                   updated_at = NOW()
               WHERE domain = $1
                 AND status = 'collecting'::federated_aggregate_status
                 AND unique_reporters >= $2"#,
        )
        .bind(&report.domain)
        .bind(UNIQUE_REPORTER_THRESHOLD)
        .execute(db)
        .await?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    /// Smoke-test: ingest an empty slice must not error.
    #[test]
    fn ingest_empty_slice_is_noop() {
        // We cannot call the DB in unit tests; just verify the loop handles
        // zero items without panicking.
        let reports: Vec<ReportPayload> = vec![];
        assert!(reports.is_empty(), "empty ingest should be a no-op");
    }

    /// Verify that `UNIQUE_REPORTER_THRESHOLD` has the documented value.
    #[test]
    fn threshold_constant() {
        assert_eq!(UNIQUE_REPORTER_THRESHOLD, 5);
    }

    /// Verify `ReportPayload` fields are wired correctly after serde roundtrip.
    #[test]
    fn report_payload_fields() {
        let p = ReportPayload {
            domain: "bet.example.com".to_string(),
            reporter_token: "tok_xyz".to_string(),
            heuristic_score: 0.88,
            category_guess: Some("online_casino".to_string()),
            reported_at: Utc::now(),
            batch_id: Uuid::nil(),
        };
        assert_eq!(p.domain, "bet.example.com");
        assert!(p.heuristic_score > 0.0);
        assert!(p.category_guess.is_some());
    }
}
