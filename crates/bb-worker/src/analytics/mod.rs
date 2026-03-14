pub mod trends;

use std::sync::Arc;

use crate::scheduler::{AppContext, JobScheduler};

/// Register all analytics jobs with the scheduler.
///
/// Currently schedules:
/// - Trend computation: runs at the top of every hour.
///
/// # Errors
/// Returns an error if a job cannot be registered.
pub async fn register_jobs(scheduler: &JobScheduler, ctx: Arc<AppContext>) -> anyhow::Result<()> {
    scheduler
        .add_job(
            "trend_computation",
            "0 0 * * * *",
            ctx,
            |ctx| async move {
                if let Err(e) = trends::compute_trends(&ctx.db).await {
                    tracing::error!(error = %e, "trend computation failed");
                }
            },
        )
        .await?;

    Ok(())
}
