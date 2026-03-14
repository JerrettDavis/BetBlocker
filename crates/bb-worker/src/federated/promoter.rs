use crate::scheduler::AppContext;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Configuration for the federated auto-promoter.
#[derive(Debug, Clone)]
pub struct PromoterConfig {
    /// Whether auto-promotion is active. Default: `false` (disabled by default
    /// to require explicit opt-in).
    pub enabled: bool,
    /// Minimum number of unique reporters before a domain can be auto-promoted.
    pub min_unique_reporters: i32,
    /// Minimum confidence score required for auto-promotion.
    pub min_confidence: f64,
    /// Maximum number of days since the aggregate was first reported.
    /// Domains older than this are not auto-promoted (stale data).
    pub max_domain_age_days: i64,
}

impl Default for PromoterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_unique_reporters: 10,
            min_confidence: 0.95,
            max_domain_age_days: 30,
        }
    }
}

// ---------------------------------------------------------------------------
// AutoPromoter
// ---------------------------------------------------------------------------

/// Automatically promotes high-confidence federated candidates to the
/// blocklist without manual review.
///
/// Runs on a 30-minute schedule but is **disabled by default** – must be
/// explicitly enabled via [`PromoterConfig::enabled`].
pub struct AutoPromoter {
    pub(crate) config: PromoterConfig,
}

impl AutoPromoter {
    pub fn new(config: PromoterConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(PromoterConfig::default())
    }

    /// Run one promotion cycle.
    ///
    /// Selects qualifying `discovery_candidates` with `federated` source and
    /// inserts them into `blocklist_entries` (ignoring duplicates).
    pub async fn run(&self, ctx: &AppContext) -> anyhow::Result<()> {
        if !self.config.enabled {
            tracing::debug!("auto-promoter: disabled, skipping");
            return Ok(());
        }

        tracing::info!("auto-promoter: starting promotion cycle");

        let promoted = sqlx::query_scalar::<_, i64>(
            r#"WITH candidates AS (
                   SELECT dc.id, dc.domain, dc.confidence_score, dc.classification
                   FROM discovery_candidates dc
                   JOIN federated_aggregates fa ON fa.domain = dc.domain
                   WHERE dc.source            = 'federated'::crawler_source
                     AND dc.status            = 'pending'::discovery_candidate_status
                     AND dc.confidence_score  >= $1
                     AND fa.unique_reporters  >= $2
                     AND fa.first_reported_at >= NOW() - ($3 || ' days')::interval
               ),
               inserted AS (
                   INSERT INTO blocklist_entries (domain, category, source, confidence, status)
                   SELECT
                       c.domain,
                       COALESCE(c.classification::text, 'other')::gambling_category,
                       'federated'::blocklist_source,
                       c.confidence_score,
                       'active'::blocklist_entry_status
                   FROM candidates c
                   ON CONFLICT (domain) DO NOTHING
                   RETURNING domain
               ),
               updated AS (
                   UPDATE discovery_candidates
                   SET status     = 'approved'::discovery_candidate_status,
                       reviewed_at = NOW()
                   WHERE domain IN (SELECT domain FROM inserted)
                   RETURNING 1
               )
               SELECT COUNT(*) FROM updated"#,
        )
        .bind(self.config.min_confidence)
        .bind(self.config.min_unique_reporters)
        .bind(self.config.max_domain_age_days)
        .fetch_one(&ctx.db)
        .await?;

        tracing::info!(promoted, "auto-promoter: cycle complete");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_disabled() {
        let config = PromoterConfig::default();
        assert!(!config.enabled, "auto-promoter must default to disabled");
    }

    #[test]
    fn default_config_thresholds() {
        let config = PromoterConfig::default();
        assert_eq!(config.min_unique_reporters, 10);
        assert!((config.min_confidence - 0.95).abs() < f64::EPSILON);
        assert_eq!(config.max_domain_age_days, 30);
    }

    #[test]
    fn can_enable_via_config() {
        let config = PromoterConfig {
            enabled: true,
            ..PromoterConfig::default()
        };
        let promoter = AutoPromoter::new(config);
        assert!(promoter.config.enabled);
    }

    #[test]
    fn with_defaults_produces_disabled_promoter() {
        let promoter = AutoPromoter::with_defaults();
        assert!(!promoter.config.enabled);
    }
}
