// Pedantic clippy: allow common lints at crate level.
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::must_use_candidate,
    clippy::module_name_repetitions,
    clippy::needless_raw_string_hashes,
    clippy::redundant_closure_for_method_calls,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::collapsible_if,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::unused_async,
    clippy::unused_self,
    clippy::single_match_else,
    clippy::match_same_arms,
    clippy::struct_excessive_bools,
    clippy::struct_field_names,
    clippy::let_and_return,
    clippy::map_unwrap_or,
    clippy::unnecessary_wraps,
    clippy::unnecessary_literal_bound,
    clippy::format_push_string,
    clippy::return_self_not_must_use,
    clippy::needless_pass_by_value,
    clippy::expect_used
)]

mod analytics;
mod discovery;
mod federated;
mod scheduler;
mod tor_exits;

use std::sync::Arc;

use scheduler::{AppContext, JobScheduler};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("BetBlocker Worker starting...");

    // ── Database ────────────────────────────────────────────────────────
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    tracing::info!("connected to database");

    // ── HTTP client ─────────────────────────────────────────────────────
    let http = reqwest::Client::builder()
        .user_agent("BetBlocker-Worker/0.1")
        .build()?;

    let ctx = Arc::new(AppContext { db, http });

    // ── Scheduler ───────────────────────────────────────────────────────
    let sched = JobScheduler::new().await?;

    // Register analytics jobs (trend computation every hour).
    analytics::register_jobs(&sched, Arc::clone(&ctx)).await?;

    // Run discovery pipeline every 6 hours.
    sched
        .add_job(
            "discovery_pipeline",
            "0 0 */6 * * *",
            Arc::clone(&ctx),
            |ctx| async move {
                if let Err(e) = discovery::DiscoveryPipeline::run_cycle(&ctx).await {
                    tracing::error!(error = %e, "discovery pipeline failed");
                }
            },
        )
        .await?;

    // Register federated ingestion jobs (aggregator every 15 min, promoter every 30 min).
    federated::register_jobs(&sched, Arc::clone(&ctx)).await?;

    // Tor exit node refresh every 6 hours.
    sched
        .add_job(
            "tor_exit_refresh",
            "0 0 */6 * * *",
            Arc::clone(&ctx),
            |ctx| async move {
                if let Err(e) = tor_exits::TorExitNodeRefreshJob::run(&ctx).await {
                    tracing::error!(error = %e, "tor exit node refresh failed");
                }
            },
        )
        .await?;

    sched.start().await?;
    tracing::info!("scheduler started – waiting for Ctrl-C");

    tokio::signal::ctrl_c().await?;
    tracing::info!("shutting down");

    Ok(())
}
