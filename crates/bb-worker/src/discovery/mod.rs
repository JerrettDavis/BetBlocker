pub mod classifier;
pub mod crawler;
pub mod crawlers;
pub mod scorer;

use crate::discovery::crawler::CrawlerScheduler;
use crate::discovery::crawlers::{WorkerConfig, build_crawlers};
use crate::scheduler::AppContext;

/// Top-level discovery pipeline.
///
/// Holds the crawlers (and, in future sprints, the classifier and scorer).
pub struct DiscoveryPipeline {
    crawler_scheduler: CrawlerScheduler,
}

impl DiscoveryPipeline {
    /// Build the default pipeline with all registered crawlers.
    #[must_use]
    pub fn new() -> Self {
        let config = WorkerConfig::default();
        let crawlers = build_crawlers(&config);

        Self {
            crawler_scheduler: CrawlerScheduler::new(crawlers),
        }
    }

    /// Execute one full discovery cycle: crawl, (classify – stub), (score –
    /// stub), and persist results.
    #[allow(unused)]
    pub async fn run_cycle(ctx: &AppContext) -> anyhow::Result<()> {
        tracing::info!("discovery pipeline: starting cycle");

        let pipeline = Self::new();
        let inserted = pipeline
            .crawler_scheduler
            .run_all(&ctx.db, &ctx.http)
            .await?;

        tracing::info!(inserted, "discovery pipeline: cycle complete");

        // TODO (SP2-T9): classify candidates
        // TODO (SP2-T10): score candidates

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
