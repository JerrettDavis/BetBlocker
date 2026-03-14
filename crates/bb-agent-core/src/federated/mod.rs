//! Agent-side federated reporting with k-anonymity.
//!
//! Agents observe domains that trigger heuristic scoring (e.g. gambling-like
//! behaviour) and report them to the BetBlocker API in batches.  To preserve
//! k-anonymity, no device ID or account ID is ever included in the report;
//! instead a daily-rotating pseudonym token is used (see [`anonymizer`]).
//!
//! # Usage
//!
//! ```no_run
//! use std::sync::Arc;
//! use std::time::Duration;
//! use bb_agent_core::federated::{FederatedReporter, ReporterConfig};
//! use bb_agent_core::comms::ApiClient;
//!
//! # async fn example() {
//! let api = Arc::new(ApiClient::new_insecure("https://api.example.com".into()));
//! let config = ReporterConfig::default();
//! let reporter = FederatedReporter::new(api, config);
//!
//! reporter.add_report("example-casino.com", 0.9, Some("online_casino".into())).await;
//! reporter.flush().await.ok();
//! # }
//! ```

pub mod anonymizer;

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::comms::client::{ApiClient, ApiClientError};

use anonymizer::{TemporalBucketer, TokenRotator};

// ── Error type ──────────────────────────────────────────────────────────────

/// Errors produced by federated reporting.
#[derive(Debug, thiserror::Error)]
pub enum FederatedError {
    #[error("HTTP error while submitting batch: {0}")]
    Http(#[from] ApiClientError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Federated reporting is disabled")]
    Disabled,

    #[error("Batch too small: need at least {min} reports, have {have}")]
    BatchTooSmall { min: usize, have: usize },
}

// ── Report struct ────────────────────────────────────────────────────────────

/// A batch of anonymized domain reports submitted by the agent.
///
/// This wrapper matches the `IngestReportRequest` expected by the API.
#[derive(Debug, Serialize, Deserialize)]
struct FederatedReportBatch {
    pub reports: Vec<FederatedReport>,
}

/// A single anonymized domain report contributed by the agent.
///
/// **Never** includes a device ID or account ID — this is a deliberate
/// k-anonymity requirement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedReport {
    /// The domain being reported (e.g. `"example-casino.com"`).
    pub domain: String,

    /// Heuristic suspicion score in the range `[0.0, 1.0]`.
    pub heuristic_score: f64,

    /// Optional category guess (e.g. `"online_casino"`, `"sports_betting"`).
    pub category_guess: Option<String>,

    /// Daily-rotating pseudonym token — never traceable to a specific device.
    pub reporter_token: String,

    /// Observation time bucketed to the nearest hour (UTC).
    pub reported_at: DateTime<Utc>,

    /// UUID shared by all reports in the same flush batch, enabling the server
    /// to deduplicate or rate-limit entire batches.
    pub batch_id: Uuid,
}

// ── Reporter config ──────────────────────────────────────────────────────────

/// Configuration for the [`FederatedReporter`].
#[derive(Debug, Clone)]
pub struct ReporterConfig {
    /// How often the reporter will automatically flush its buffer.
    ///
    /// Defaults to 6 hours.
    pub batch_interval: Duration,

    /// Minimum number of queued reports before a flush is allowed.
    ///
    /// Defaults to `1` (flush on every non-empty buffer).
    pub min_batch_size: usize,

    /// Whether federated reporting is active.
    ///
    /// When `false`, [`FederatedReporter::add_report`] is a no-op and
    /// [`FederatedReporter::flush`] returns [`FederatedError::Disabled`].
    pub enabled: bool,
}

impl Default for ReporterConfig {
    fn default() -> Self {
        Self {
            batch_interval: Duration::from_secs(6 * 3600), // 6 hours
            min_batch_size: 1,
            enabled: true,
        }
    }
}

// ── Reporter ─────────────────────────────────────────────────────────────────

/// Collects domain-heuristic observations from the agent and submits them to
/// the BetBlocker API in anonymized batches.
pub struct FederatedReporter {
    api_client: Arc<ApiClient>,
    config: ReporterConfig,
    token_rotator: TokenRotator,
    buffer: Mutex<Vec<PendingReport>>,
}

/// Internal pending report before batch ID assignment.
#[derive(Clone)]
struct PendingReport {
    domain: String,
    heuristic_score: f64,
    category_guess: Option<String>,
    reporter_token: String,
    reported_at: DateTime<Utc>,
}

impl FederatedReporter {
    /// Create a new reporter.
    ///
    /// `seed` should be a 32-byte value generated once and persisted in the
    /// agent's local configuration.  Call [`TokenRotator::generate_seed`] to
    /// produce a cryptographically random seed.
    pub fn new(api_client: Arc<ApiClient>, config: ReporterConfig) -> Self {
        let seed = TokenRotator::generate_seed(32);
        Self::with_seed(api_client, config, seed)
    }

    /// Create a reporter with an explicit seed (useful in tests and when
    /// loading a persisted seed from disk).
    pub fn with_seed(api_client: Arc<ApiClient>, config: ReporterConfig, seed: Vec<u8>) -> Self {
        Self {
            api_client,
            config,
            token_rotator: TokenRotator::new(seed),
            buffer: Mutex::new(Vec::new()),
        }
    }

    /// Queue an anonymized report for the given domain.
    ///
    /// Applies the daily-rotating token and rounds the timestamp to the
    /// nearest hour before storing.  If the reporter is disabled this is a
    /// no-op.
    pub async fn add_report(
        &self,
        domain: impl Into<String>,
        heuristic_score: f64,
        category_guess: Option<String>,
    ) {
        if !self.config.enabled {
            return;
        }

        let reporter_token = self.token_rotator.current_token();
        let reported_at = TemporalBucketer::bucket(Utc::now());

        let report = PendingReport {
            domain: domain.into(),
            heuristic_score,
            category_guess,
            reporter_token,
            reported_at,
        };

        self.buffer.lock().await.push(report);
    }

    /// Flush all buffered reports to the API as a single batch.
    ///
    /// Assigns a fresh UUIDv4 `batch_id` to every report in the batch,
    /// serializes as a JSON array, and POSTs to `POST /v1/federated/reports`.
    ///
    /// On success the buffer is cleared.  Returns [`FederatedError::Disabled`]
    /// if reporting is disabled, or [`FederatedError::BatchTooSmall`] if the
    /// buffer has fewer than `min_batch_size` entries.
    pub async fn flush(&self) -> Result<(), FederatedError> {
        if !self.config.enabled {
            return Err(FederatedError::Disabled);
        }

        let pending: Vec<PendingReport> = {
            let mut guard = self.buffer.lock().await;
            if guard.len() < self.config.min_batch_size {
                return Err(FederatedError::BatchTooSmall {
                    min: self.config.min_batch_size,
                    have: guard.len(),
                });
            }
            std::mem::take(&mut *guard)
        };

        if pending.is_empty() {
            return Err(FederatedError::BatchTooSmall {
                min: self.config.min_batch_size,
                have: 0,
            });
        }

        let batch_id = Uuid::new_v4();

        let reports: Vec<FederatedReport> = pending
            .into_iter()
            .map(|p| FederatedReport {
                domain: p.domain,
                heuristic_score: p.heuristic_score,
                category_guess: p.category_guess,
                reporter_token: p.reporter_token,
                reported_at: p.reported_at,
                batch_id,
            })
            .collect();

        let body = serde_json::to_vec(&FederatedReportBatch { reports: reports.clone() })?;

        self.api_client
            .post_raw("/v1/federated/reports", "application/json", body)
            .await
            .map(|_| ())?;

        tracing::info!(
            batch_id = %batch_id,
            count = reports.len(),
            "Submitted federated report batch"
        );

        Ok(())
    }

    /// Spawn a background tokio task that calls [`flush`](Self::flush) every
    /// `batch_interval`.
    ///
    /// The returned [`tokio::task::JoinHandle`] can be aborted to stop the loop.
    pub fn run(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let interval_duration = self.config.batch_interval;
        tokio::spawn(async move {
            let mut ticker =
                tokio::time::interval(interval_duration);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            // Skip the immediate first tick so we don't flush on startup.
            ticker.tick().await;

            loop {
                ticker.tick().await;
                match self.flush().await {
                    Ok(()) => {}
                    Err(FederatedError::BatchTooSmall { .. }) => {
                        // Not enough reports yet — silently skip.
                    }
                    Err(FederatedError::Disabled) => {
                        tracing::debug!("Federated reporter disabled; stopping run loop");
                        break;
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Federated report flush failed; will retry");
                    }
                }
            }
        })
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn fixed_seed() -> Vec<u8> {
        b"test-seed-fixed-32-bytes-padding".to_vec()
    }

    fn make_reporter(base_url: String) -> FederatedReporter {
        let api = Arc::new(ApiClient::new_insecure(base_url));
        FederatedReporter::with_seed(api, ReporterConfig::default(), fixed_seed())
    }

    // ── Anonymisation ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn report_contains_no_device_id() {
        // Build a FederatedReport directly and verify JSON has no identifying fields.
        let fed = FederatedReport {
            domain: "casino.example.com".into(),
            heuristic_score: 0.85,
            category_guess: Some("online_casino".into()),
            reporter_token: "aabbccdd".repeat(8),
            reported_at: Utc::now(),
            batch_id: Uuid::new_v4(),
        };
        let fed_json = serde_json::to_string(&fed).expect("serialise federated report");
        assert!(
            !fed_json.contains("device_id"),
            "FederatedReport must not contain device_id: {fed_json}"
        );
        assert!(
            !fed_json.contains("account_id"),
            "FederatedReport must not contain account_id: {fed_json}"
        );
    }

    #[tokio::test]
    async fn report_token_is_non_empty_hex() {
        let reporter = make_reporter("http://localhost:1".into());
        reporter
            .add_report("poker.example.com", 0.7, None)
            .await;

        let guard = reporter.buffer.lock().await;
        let report = guard.first().expect("one report");
        assert_eq!(report.reporter_token.len(), 64, "token should be 64 hex chars");
        assert!(
            report.reporter_token.chars().all(|c| c.is_ascii_hexdigit()),
            "token should be hex"
        );
    }

    #[tokio::test]
    async fn reported_at_is_on_hour_boundary() {
        let reporter = make_reporter("http://localhost:1".into());
        reporter
            .add_report("slots.example.com", 0.9, None)
            .await;

        let guard = reporter.buffer.lock().await;
        let report = guard.first().expect("one report");
        assert_eq!(
            report.reported_at.minute(),
            0,
            "reported_at minutes should be 0"
        );
        assert_eq!(
            report.reported_at.second(),
            0,
            "reported_at seconds should be 0"
        );
    }

    // ── Batch serialisation ──────────────────────────────────────────────────

    #[tokio::test]
    async fn batch_submission_serialises_correctly() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/federated/reports"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b""))
            .expect(1)
            .mount(&server)
            .await;

        let reporter = make_reporter(server.uri());
        reporter
            .add_report("example-casino.com", 0.95, Some("online_casino".into()))
            .await;
        reporter
            .add_report("bet365-mirror.net", 0.8, Some("sports_betting".into()))
            .await;

        reporter.flush().await.expect("flush should succeed");

        // Wiremock automatically verifies the mock was called exactly once
        // when the MockServer is dropped.
    }

    #[tokio::test]
    async fn flush_clears_buffer() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/federated/reports"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b""))
            .mount(&server)
            .await;

        let reporter = make_reporter(server.uri());
        reporter.add_report("casino.com", 0.9, None).await;
        assert_eq!(reporter.buffer.lock().await.len(), 1, "buffer should have 1 report before flush");

        reporter.flush().await.expect("flush should succeed");

        assert_eq!(
            reporter.buffer.lock().await.len(),
            0,
            "buffer should be empty after flush"
        );
    }

    #[tokio::test]
    async fn reports_accumulate_between_flushes() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/federated/reports"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b""))
            .mount(&server)
            .await;

        let reporter = make_reporter(server.uri());

        // Add reports in two separate batches
        reporter.add_report("site-a.com", 0.6, None).await;
        reporter.add_report("site-b.com", 0.7, None).await;
        assert_eq!(reporter.buffer.lock().await.len(), 2, "should have 2 reports");

        reporter.add_report("site-c.com", 0.8, None).await;
        assert_eq!(reporter.buffer.lock().await.len(), 3, "should have 3 reports before flush");

        reporter.flush().await.expect("flush should succeed");
        assert_eq!(reporter.buffer.lock().await.len(), 0, "buffer cleared after flush");

        // Add more after flush
        reporter.add_report("site-d.com", 0.9, None).await;
        assert_eq!(reporter.buffer.lock().await.len(), 1, "should accumulate again after flush");
    }

    #[tokio::test]
    async fn flush_returns_disabled_when_disabled() {
        let api = Arc::new(ApiClient::new_insecure("http://localhost:1".into()));
        let config = ReporterConfig {
            enabled: false,
            ..Default::default()
        };
        let reporter = FederatedReporter::with_seed(api, config, fixed_seed());
        // add_report should be a no-op
        reporter.add_report("casino.com", 0.9, None).await;
        assert_eq!(reporter.buffer.lock().await.len(), 0, "disabled reporter should not buffer");
        let err = reporter.flush().await.expect_err("flush should fail when disabled");
        assert!(matches!(err, FederatedError::Disabled));
    }

    #[tokio::test]
    async fn flush_returns_batch_too_small_when_buffer_empty() {
        let api = Arc::new(ApiClient::new_insecure("http://localhost:1".into()));
        let config = ReporterConfig {
            min_batch_size: 3,
            ..Default::default()
        };
        let reporter = FederatedReporter::with_seed(api, config, fixed_seed());
        reporter.add_report("casino.com", 0.9, None).await;
        reporter.add_report("casino2.com", 0.8, None).await;
        // only 2 reports, min is 3
        let err = reporter.flush().await.expect_err("should fail with batch too small");
        assert!(matches!(err, FederatedError::BatchTooSmall { min: 3, have: 2 }));
        // buffer should NOT have been cleared
        assert_eq!(reporter.buffer.lock().await.len(), 2, "buffer should be untouched on BatchTooSmall");
    }

    #[tokio::test]
    async fn all_reports_in_batch_share_same_batch_id() {
        let server = MockServer::start().await;
        let captured_body: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let body_store = captured_body.clone();

        Mock::given(method("POST"))
            .and(path("/v1/federated/reports"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b""))
            .mount(&server)
            .await;

        // We'll check via an independent approach: just verify flush succeeds
        // and that JSON has consistent batch_id.
        // Use a secondary check by inspecting via wiremock received_requests.
        let reporter = make_reporter(server.uri());
        reporter.add_report("a.com", 0.5, None).await;
        reporter.add_report("b.com", 0.6, None).await;
        reporter.flush().await.expect("flush ok");

        let requests = server.received_requests().await.expect("requests");
        assert_eq!(requests.len(), 1);
        let body = &requests[0].body;
        let batch: FederatedReportBatch =
            serde_json::from_slice(body).expect("parse batch JSON");
        let reports = batch.reports;
        assert_eq!(reports.len(), 2);
        let id0 = reports[0].batch_id;
        let id1 = reports[1].batch_id;
        assert_eq!(id0, id1, "all reports in one batch must share batch_id");
        drop(captured_body); // silence unused warning
        drop(body_store);
    }
}
