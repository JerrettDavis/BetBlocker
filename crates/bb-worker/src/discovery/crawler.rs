use std::num::NonZeroU32;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use governor::{Quota, RateLimiter as GovernorLimiter, clock, middleware, state};
use sqlx::PgPool;

use bb_common::enums::CrawlerSource;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum CrawlError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("rate limited")]
    #[allow(dead_code)]
    RateLimited,

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Contextual data passed to every crawler invocation.
pub struct CrawlContext {
    pub http: reqwest::Client,
    pub rate_limiter: RateLimiter,
    #[allow(dead_code)]
    pub last_run: Option<DateTime<Utc>>,
}

/// A single domain discovered by a crawler.
#[derive(Debug, Clone)]
pub struct CrawlResult {
    pub domain: String,
    pub source_metadata: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Common interface every domain-discovery crawler must implement.
#[async_trait]
pub trait DomainCrawler: Send + Sync {
    /// Human-readable crawler name (used for logging / metrics).
    fn name(&self) -> &str;

    /// Which `CrawlerSource` variant this crawler maps to.
    fn source(&self) -> CrawlerSource;

    /// Execute one crawl cycle, returning discovered domains.
    async fn crawl(&self, ctx: &CrawlContext) -> Result<Vec<CrawlResult>, CrawlError>;
}

// ---------------------------------------------------------------------------
// Rate limiter
// ---------------------------------------------------------------------------

type DirectLimiter = GovernorLimiter<
    state::NotKeyed,
    state::InMemoryState,
    clock::DefaultClock,
    middleware::NoOpMiddleware,
>;

/// Token-bucket rate limiter wrapping the `governor` crate.
pub struct RateLimiter {
    inner: Arc<DirectLimiter>,
}

impl RateLimiter {
    /// Create a new rate limiter.
    ///
    /// * `requests_per_second` – sustained request rate.
    /// * `burst_size`          – maximum burst above the sustained rate.
    ///
    /// # Panics
    /// Panics if either parameter is zero.
    #[must_use]
    pub fn new(requests_per_second: u32, burst_size: u32) -> Self {
        let rps = NonZeroU32::new(requests_per_second).expect("requests_per_second must be > 0");
        let burst = NonZeroU32::new(burst_size).expect("burst_size must be > 0");
        let quota = Quota::per_second(rps).allow_burst(burst);
        Self {
            inner: Arc::new(GovernorLimiter::direct(quota)),
        }
    }

    /// Wait until a token is available, then consume it.
    pub async fn acquire(&self) {
        self.inner.until_ready().await;
    }
}

impl Clone for RateLimiter {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

// ---------------------------------------------------------------------------
// Crawler Scheduler – stores crawl results into the database
// ---------------------------------------------------------------------------

/// Orchestrates running a set of crawlers and persisting results.
pub struct CrawlerScheduler {
    crawlers: Vec<Box<dyn DomainCrawler>>,
}

impl CrawlerScheduler {
    #[must_use]
    pub fn new(crawlers: Vec<Box<dyn DomainCrawler>>) -> Self {
        Self { crawlers }
    }

    /// Execute every registered crawler and upsert results into
    /// `discovery_candidates`.
    ///
    /// Uses `ON CONFLICT (domain, source) DO NOTHING` so duplicate
    /// discoveries are silently ignored.
    #[allow(unused)]
    pub async fn run_all(&self, db: &PgPool, http: &reqwest::Client) -> anyhow::Result<usize> {
        let mut total_inserted: usize = 0;

        for crawler in &self.crawlers {
            let ctx = CrawlContext {
                http: http.clone(),
                rate_limiter: RateLimiter::new(2, 5),
                last_run: None, // TODO: track per-crawler last-run timestamps
            };

            match crawler.crawl(&ctx).await {
                Ok(results) => {
                    tracing::info!(
                        crawler = crawler.name(),
                        count = results.len(),
                        "crawl completed"
                    );
                    for result in &results {
                        let source_str = serde_json::to_string(&crawler.source())
                            .unwrap_or_default()
                            .trim_matches('"')
                            .to_string();
                        let inserted = Self::upsert_candidate(
                            db,
                            &result.domain,
                            &source_str,
                            &result.source_metadata,
                        )
                        .await;
                        match inserted {
                            Ok(true) => total_inserted += 1,
                            Ok(false) => {} // duplicate, skip
                            Err(e) => {
                                tracing::warn!(
                                    crawler = crawler.name(),
                                    domain = %result.domain,
                                    error = %e,
                                    "failed to upsert candidate"
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        crawler = crawler.name(),
                        error = %e,
                        "crawler failed"
                    );
                }
            }
        }

        Ok(total_inserted)
    }

    #[allow(unused)]
    async fn upsert_candidate(
        db: &PgPool,
        domain: &str,
        source: &str,
        metadata: &serde_json::Value,
    ) -> anyhow::Result<bool> {
        let result = sqlx::query(
            r"
            INSERT INTO discovery_candidates (domain, source, source_metadata)
            VALUES ($1, $2::crawler_source, $3)
            ON CONFLICT (domain, source) DO NOTHING
            ",
        )
        .bind(domain)
        .bind(source)
        .bind(metadata)
        .execute(db)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limiter_constructs() {
        let rl = RateLimiter::new(10, 20);
        // Cloning should share the inner Arc.
        let rl2 = rl.clone();
        assert!(Arc::ptr_eq(&rl.inner, &rl2.inner));
    }

    #[tokio::test]
    async fn rate_limiter_acquire_returns() {
        let rl = RateLimiter::new(100, 100);
        // Should not hang – we have plenty of burst capacity.
        rl.acquire().await;
    }

    #[test]
    #[should_panic(expected = "requests_per_second must be > 0")]
    fn rate_limiter_rejects_zero_rps() {
        let _ = RateLimiter::new(0, 5);
    }

    #[test]
    fn crawl_result_debug() {
        let r = CrawlResult {
            domain: "test.com".to_string(),
            source_metadata: serde_json::json!({}),
        };
        // Ensure Debug is derived.
        let _ = format!("{r:?}");
    }
}
