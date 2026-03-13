use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use tokio::time::{interval, MissedTickBehavior};

use bb_common::enums::EnrollmentTier;
use bb_common::models::ReportingConfig;

use crate::comms::client::{ApiClient, ApiClientError};
use crate::events::privacy::PrivacyFilter;
use crate::events::store::EventStore;

/// Batches local events and reports them to the API with privacy filtering.
///
/// Runs on a periodic schedule (default 5 minutes). Events are filtered
/// according to the enrollment tier's privacy settings before transmission.
pub struct EventReporter {
    api_client: Arc<ApiClient>,
    device_id: String,
    privacy_filter: PrivacyFilter,
    /// Maximum events per batch.
    batch_size: usize,
    /// How often to check for unreported events.
    report_interval: Duration,
    /// Batch sequence counter.
    batch_sequence: u64,
}

/// Errors from event reporting.
#[derive(Debug, thiserror::Error)]
pub enum ReporterError {
    #[error("API error: {0}")]
    ApiError(#[from] ApiClientError),

    #[error("Event store error: {0}")]
    StoreError(#[from] rusqlite::Error),

    #[error("No events to report")]
    NoEvents,
}

impl EventReporter {
    pub fn new(
        api_client: Arc<ApiClient>,
        device_id: String,
        tier: EnrollmentTier,
        reporting_config: ReportingConfig,
    ) -> Self {
        Self {
            api_client,
            device_id,
            privacy_filter: PrivacyFilter::new(tier, reporting_config),
            batch_size: 100,
            report_interval: Duration::from_secs(300), // 5 minutes
            batch_sequence: 0,
        }
    }

    /// Override the report interval.
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.report_interval = interval;
        self
    }

    /// Override the batch size.
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Run the event reporting loop until shutdown signal is received.
    pub async fn run(&mut self, store: &EventStore, mut shutdown: watch::Receiver<bool>) {
        let mut ticker = interval(self.report_interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    match self.report_batch(store).await {
                        Ok(count) => {
                            if count > 0 {
                                tracing::debug!(count, "Reported events batch");
                            }
                        }
                        Err(ReporterError::NoEvents) => {}
                        Err(e) => {
                            tracing::warn!(error = %e, "Event reporting failed, will retry next cycle");
                        }
                    }
                }
                _ = shutdown.changed() => {
                    tracing::info!("Event reporter shutting down");
                    // Final flush attempt
                    if let Err(e) = self.report_batch(store).await {
                        tracing::debug!(error = %e, "Final event flush failed");
                    }
                    break;
                }
            }
        }
    }

    /// Report a single batch of unreported events.
    ///
    /// Queries up to `batch_size` unreported events, applies privacy filter,
    /// serializes to protobuf, sends to API, and marks reported on success.
    pub async fn report_batch(&mut self, store: &EventStore) -> Result<usize, ReporterError> {
        let events = store.unreported(self.batch_size)?;
        if events.is_empty() {
            return Err(ReporterError::NoEvents);
        }

        // Apply privacy filter
        let filtered: Vec<_> = events
            .iter()
            .filter_map(|e| self.privacy_filter.filter(e))
            .collect();

        if filtered.is_empty() {
            // All events were filtered out; still mark as reported
            let ids: Vec<i64> = events.iter().filter_map(|e| e.id).collect();
            store.mark_reported(&ids)?;
            return Ok(0);
        }

        // Convert to protobuf
        let proto_events: Vec<bb_proto::events::EventRecord> = filtered
            .iter()
            .map(|e| {
                let occurred_at = e.timestamp.timestamp_millis() as u64;
                bb_proto::events::EventRecord {
                    event_type: serde_json::to_string(&e.event_type)
                        .unwrap_or_default()
                        .trim_matches('"')
                        .to_string(),
                    category: serde_json::to_string(&e.category)
                        .unwrap_or_default()
                        .trim_matches('"')
                        .to_string(),
                    severity: serde_json::to_string(&e.severity)
                        .unwrap_or_default()
                        .trim_matches('"')
                        .to_string(),
                    metadata: e.metadata.to_string().into_bytes(),
                    occurred_at,
                }
            })
            .collect();

        let batch = bb_proto::events::EventBatch {
            device_id: self.device_id.clone(),
            batch_sequence: self.batch_sequence,
            events: proto_events,
        };

        let path = format!("/api/v1/devices/{}/events", self.device_id);
        let response: bb_proto::events::EventBatchResponse =
            self.api_client.post_proto(&path, &batch).await?;

        if response.acknowledged {
            let ids: Vec<i64> = events.iter().filter_map(|e| e.id).collect();
            store.mark_reported(&ids)?;
            self.batch_sequence += 1;
            Ok(ids.len())
        } else {
            tracing::warn!("Event batch not acknowledged by server");
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reporter_creation() {
        let client = Arc::new(ApiClient::new_insecure("http://localhost:1".to_string()));
        let reporter = EventReporter::new(
            client,
            "test-device".to_string(),
            EnrollmentTier::SelfEnrolled,
            ReportingConfig::default(),
        );
        assert_eq!(reporter.batch_size, 100);
        assert_eq!(reporter.report_interval, Duration::from_secs(300));
    }

    #[test]
    fn test_reporter_with_overrides() {
        let client = Arc::new(ApiClient::new_insecure("http://localhost:1".to_string()));
        let reporter = EventReporter::new(
            client,
            "test-device".to_string(),
            EnrollmentTier::Partner,
            ReportingConfig::default(),
        )
        .with_interval(Duration::from_secs(60))
        .with_batch_size(50);

        assert_eq!(reporter.batch_size, 50);
        assert_eq!(reporter.report_interval, Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_report_batch_empty_store() {
        let client = Arc::new(ApiClient::new_insecure("http://localhost:1".to_string()));
        let mut reporter = EventReporter::new(
            client,
            "test-device".to_string(),
            EnrollmentTier::SelfEnrolled,
            ReportingConfig::default(),
        );

        let store = EventStore::in_memory().expect("store");
        let result = reporter.report_batch(&store).await;
        assert!(matches!(result, Err(ReporterError::NoEvents)));
    }
}
