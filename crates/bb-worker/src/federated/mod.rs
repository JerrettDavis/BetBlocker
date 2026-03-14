pub mod aggregator;
pub mod promoter;

use std::sync::Arc;

use crate::scheduler::{AppContext, JobScheduler};

use self::aggregator::FederatedAggregator;
use self::promoter::AutoPromoter;

/// Top-level struct that owns the federated pipeline components.
pub struct FederatedPipeline {
    promoter: AutoPromoter,
}

impl FederatedPipeline {
    /// Build the default federated pipeline.
    pub fn new() -> Self {
        Self {
            promoter: AutoPromoter::with_defaults(),
        }
    }
}

impl Default for FederatedPipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Register both federated jobs with the scheduler.
///
/// * `federated_aggregator` – runs every 15 minutes.
/// * `federated_promoter`   – runs every 30 minutes.
pub async fn register_jobs(
    scheduler: &JobScheduler,
    ctx: Arc<AppContext>,
) -> anyhow::Result<()> {
    // Aggregator: every 15 minutes
    scheduler
        .add_job(
            "federated_aggregator",
            "0 */15 * * * *",
            Arc::clone(&ctx),
            |ctx| async move {
                if let Err(e) = FederatedAggregator::run(&ctx).await {
                    tracing::error!(error = %e, "federated aggregator failed");
                }
            },
        )
        .await?;

    // Promoter: every 30 minutes
    let pipeline = Arc::new(FederatedPipeline::new());
    scheduler
        .add_job(
            "federated_promoter",
            "0 */30 * * * *",
            Arc::clone(&ctx),
            move |ctx| {
                let pipeline = Arc::clone(&pipeline);
                async move {
                    if let Err(e) = pipeline.promoter.run(&ctx).await {
                        tracing::error!(error = %e, "federated promoter failed");
                    }
                }
            },
        )
        .await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_constructs() {
        let pipeline = FederatedPipeline::new();
        // Verify that the promoter defaults to disabled.
        assert!(!pipeline.promoter.config.enabled);
    }

    #[test]
    fn default_pipeline_is_same_as_new() {
        let _pipeline = FederatedPipeline::default();
    }
}
