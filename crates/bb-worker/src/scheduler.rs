use std::sync::Arc;

use sqlx::PgPool;
use tokio_cron_scheduler::{Job, JobScheduler as TokioCronScheduler};

/// Shared application context available to all scheduled jobs.
#[derive(Clone)]
pub struct AppContext {
    pub db: PgPool,
    pub http: reqwest::Client,
}

/// Thin wrapper around `tokio_cron_scheduler::JobScheduler`.
pub struct JobScheduler {
    inner: TokioCronScheduler,
}

impl JobScheduler {
    /// Create a new scheduler instance.
    ///
    /// # Errors
    /// Returns an error if the underlying scheduler fails to initialise.
    pub async fn new() -> anyhow::Result<Self> {
        let inner = TokioCronScheduler::new().await?;
        Ok(Self { inner })
    }

    /// Register a recurring job.
    ///
    /// * `name`      – human-readable label used for logging.
    /// * `cron_expr` – six-field cron expression (sec min hour dom mon dow).
    /// * `ctx`       – shared application context.
    /// * `handler`   – async function executed on each tick.
    ///
    /// # Errors
    /// Returns an error if the cron expression is invalid or the job cannot be
    /// added to the scheduler.
    pub async fn add_job<F, Fut>(
        &self,
        name: &str,
        cron_expr: &str,
        ctx: Arc<AppContext>,
        handler: F,
    ) -> anyhow::Result<()>
    where
        F: Fn(Arc<AppContext>) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let job_name = name.to_string();
        let job = Job::new_async(cron_expr, move |_uuid, _lock| {
            let ctx = Arc::clone(&ctx);
            let handler = handler.clone();
            let job_name = job_name.clone();
            Box::pin(async move {
                tracing::info!(job = %job_name, "job tick");
                handler(ctx).await;
            })
        })?;
        self.inner.add(job).await?;
        tracing::info!(job = %name, cron = %cron_expr, "registered job");
        Ok(())
    }

    /// Start the scheduler (non-blocking).
    ///
    /// # Errors
    /// Returns an error if the scheduler fails to start.
    pub async fn start(&self) -> anyhow::Result<()> {
        self.inner.start().await?;
        Ok(())
    }
}
