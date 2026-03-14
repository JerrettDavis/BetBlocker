pub mod classifier;
pub mod crawler;
pub mod crawlers;
pub mod scorer;

use crate::discovery::crawler::CrawlerScheduler;
use crate::discovery::crawlers::affiliate::AffiliateCrawler;
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
        let affiliate = AffiliateCrawler::new(
            vec![
                "https://www.askgamblers.com/online-casinos/all".to_string(),
                "https://www.casinomeister.com/".to_string(),
            ],
            1,
            "a".to_string(),
        );

        let crawlers: Vec<Box<dyn crawler::DomainCrawler>> = vec![Box::new(affiliate)];

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
