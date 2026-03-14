pub mod classifier;
pub mod crawler;
pub mod crawlers;
pub mod scorer;

use crate::discovery::classifier::{ClassifyContext, ContentClassifier, RuleBasedClassifier};
use crate::discovery::crawler::CrawlerScheduler;
use crate::discovery::crawlers::{WorkerConfig, build_crawlers};
use crate::discovery::scorer::ConfidenceScorer;
use crate::scheduler::AppContext;

/// Top-level discovery pipeline.
///
/// Holds the crawlers, classifier, and scorer.
pub struct DiscoveryPipeline {
    crawler_scheduler: CrawlerScheduler,
    classifier: RuleBasedClassifier,
    scorer: ConfidenceScorer,
}

impl DiscoveryPipeline {
    /// Build the default pipeline with all registered crawlers.
    #[must_use]
    pub fn new() -> Self {
        let config = WorkerConfig::default();
        let crawlers = build_crawlers(&config);

        Self {
            crawler_scheduler: CrawlerScheduler::new(crawlers),
            classifier: RuleBasedClassifier::new(),
            scorer: ConfidenceScorer::default(),
        }
    }

    /// Execute one full discovery cycle: crawl, classify, score, and persist
    /// results.
    ///
    /// For each newly inserted domain candidate, the classifier fetches the
    /// page content and extracts keyword/structure/link-graph signals.  The
    /// confidence scorer then converts those signals into a single score in
    /// 0.0–1.0 which is written back to `discovery_candidates`.
    #[allow(unused)]
    pub async fn run_cycle(ctx: &AppContext) -> anyhow::Result<()> {
        tracing::info!("discovery pipeline: starting cycle");

        let pipeline = Self::new();
        let inserted = pipeline
            .crawler_scheduler
            .run_all(&ctx.db, &ctx.http)
            .await?;

        tracing::info!(inserted, "discovery pipeline: crawl complete, classifying candidates");

        // Fetch newly inserted candidates that have not yet been scored.
        let candidates: Vec<(String,)> = sqlx::query_as(
            r"SELECT domain FROM discovery_candidates
              WHERE confidence_score = 0.0 OR confidence_score IS NULL
              LIMIT 200",
        )
        .fetch_all(&ctx.db)
        .await?;

        let classify_ctx = ClassifyContext {
            http: ctx.http.clone(),
        };

        let mut classified = 0usize;
        for (domain,) in &candidates {
            match pipeline.classifier.classify(domain, &classify_ctx).await {
                Ok(classification) => {
                    let score = pipeline.scorer.score(&classification);
                    let category = classification.category_guess.as_deref().unwrap_or("");
                    let evidence = classification.evidence.clone();

                    let update_result = sqlx::query(
                        r"UPDATE discovery_candidates
                          SET confidence_score = $1,
                              category_guess   = NULLIF($2, ''),
                              classifier_evidence = $3
                          WHERE domain = $4",
                    )
                    .bind(score)
                    .bind(category)
                    .bind(evidence)
                    .bind(domain.as_str())
                    .execute(&ctx.db)
                    .await;

                    match update_result {
                        Ok(_) => {
                            classified += 1;
                            tracing::debug!(
                                domain = %domain,
                                score,
                                "classified candidate"
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                domain = %domain,
                                error = %e,
                                "failed to update candidate with classification"
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        domain = %domain,
                        error = %e,
                        "classification failed; leaving score at 0.0"
                    );
                }
            }
        }

        tracing::info!(inserted, classified, "discovery pipeline: cycle complete");

        Ok(())
    }
}

impl Default for DiscoveryPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_constructs() {
        let pipeline = DiscoveryPipeline::new();
        // Ensure the default pipeline has at least one crawler.
        let _ = pipeline.crawler_scheduler;
    }
}
