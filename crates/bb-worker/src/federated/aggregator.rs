use crate::discovery::classifier::{ClassifyContext, ContentClassifier, RuleBasedClassifier};
use crate::discovery::scorer::ConfidenceScorer;
use crate::scheduler::AppContext;

/// Processes `federated_aggregates` rows that have reached `threshold_met`
/// status, classifies their domains, and promotes them to `discovery_candidates`.
pub struct FederatedAggregator;

impl FederatedAggregator {
    /// Run one aggregation cycle.
    ///
    /// Steps for each `threshold_met` aggregate:
    /// 1. Classify with [`RuleBasedClassifier`].
    /// 2. Score with [`ConfidenceScorer`].
    /// 3. Upsert into `discovery_candidates` with source `federated`.
    /// 4. Update `confidence_score` on the candidate.
    /// 5. Advance the aggregate status to `reviewing`.
    pub async fn run(ctx: &AppContext) -> anyhow::Result<()> {
        // Query all aggregates that have hit the threshold.
        let domains = sqlx::query_scalar::<_, String>(
            r#"SELECT domain
               FROM federated_aggregates
               WHERE status = 'threshold_met'::federated_aggregate_status
               ORDER BY last_reported_at"#,
        )
        .fetch_all(&ctx.db)
        .await?;

        if domains.is_empty() {
            tracing::debug!("federated aggregator: no threshold-met domains");
            return Ok(());
        }

        tracing::info!(
            count = domains.len(),
            "federated aggregator: processing domains"
        );

        let classifier = RuleBasedClassifier::new();
        let scorer = ConfidenceScorer::default();
        let classify_ctx = ClassifyContext {
            http: ctx.http.clone(),
        };

        for domain in &domains {
            match process_domain(domain, &classifier, &scorer, &classify_ctx, ctx).await {
                Ok(()) => {
                    tracing::info!(%domain, "federated aggregator: domain processed");
                }
                Err(e) => {
                    // Log but continue – don't let one failure block the rest.
                    tracing::warn!(%domain, error = %e, "federated aggregator: failed to process domain");
                }
            }
        }

        Ok(())
    }
}

async fn process_domain(
    domain: &str,
    classifier: &RuleBasedClassifier,
    scorer: &ConfidenceScorer,
    classify_ctx: &ClassifyContext,
    ctx: &AppContext,
) -> anyhow::Result<()> {
    // 1. Classify
    let classification = classifier.classify(domain, classify_ctx).await?;

    // 2. Score
    let confidence = scorer.score(&classification);

    // 3. Upsert discovery_candidates
    let candidate_id = sqlx::query_scalar::<_, i64>(
        r#"INSERT INTO discovery_candidates
               (domain, source, confidence_score, classification, source_metadata, status)
           VALUES (
               $1,
               'federated'::crawler_source,
               $2,
               $3::jsonb,
               $4::jsonb,
               'pending'::discovery_candidate_status
           )
           ON CONFLICT (domain) DO UPDATE SET
               confidence_score = GREATEST(discovery_candidates.confidence_score, $2),
               updated_at       = NOW()
           RETURNING id"#,
    )
    .bind(domain)
    .bind(confidence)
    .bind(
        classification
            .category_guess
            .as_deref()
            .unwrap_or("unknown"),
    )
    .bind(&classification.evidence)
    .fetch_one(&ctx.db)
    .await?;

    // 4. Update federated_aggregates with candidate link and advance status
    sqlx::query(
        r#"UPDATE federated_aggregates
           SET status                 = 'reviewing'::federated_aggregate_status,
               discovery_candidate_id = $2,
               updated_at             = NOW()
           WHERE domain = $1"#,
    )
    .bind(domain)
    .bind(candidate_id)
    .execute(&ctx.db)
    .await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::classifier::Classification;

    fn make_classification(keyword: f64, structure: f64) -> Classification {
        Classification {
            keyword_score: keyword,
            structure_score: structure,
            link_graph_score: 0.0,
            category_guess: Some("online_casino".to_string()),
            evidence: serde_json::json!({}),
        }
    }

    #[test]
    fn scorer_produces_expected_range() {
        let scorer = ConfidenceScorer::default();
        let c = make_classification(0.8, 0.9);
        let score = scorer.score(&c);
        assert!(
            score > 0.0 && score <= 1.0,
            "score {score} should be in (0,1]"
        );
    }

    #[test]
    fn classifier_low_score_stays_low() {
        let scorer = ConfidenceScorer::default();
        let c = make_classification(0.0, 0.0);
        let score = scorer.score(&c);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn federated_aggregator_type_accessible() {
        let _ = std::any::type_name::<FederatedAggregator>();
    }
}
