use bb_common::enums::{EnrollmentTier, EventType};
use bb_common::models::ReportingConfig;

use super::AgentEvent;

/// Filters events based on the enrollment tier's privacy settings.
///
/// - **Self-enrolled**: Strip domain, drop metadata. Only keep aggregated counts.
/// - **Partner**: Keep domain if `domain_details` is true.
/// - **Authority**: Full detail (audit-grade).
pub struct PrivacyFilter {
    tier: EnrollmentTier,
    reporting_config: ReportingConfig,
}

impl PrivacyFilter {
    pub fn new(tier: EnrollmentTier, reporting_config: ReportingConfig) -> Self {
        Self {
            tier,
            reporting_config,
        }
    }

    /// Filter an event according to privacy settings.
    /// Returns `None` if the event should be dropped entirely.
    pub fn filter(&self, event: &AgentEvent) -> Option<AgentEvent> {
        match self.tier {
            EnrollmentTier::SelfEnrolled => self.filter_self_tier(event),
            EnrollmentTier::Partner => self.filter_partner_tier(event),
            EnrollmentTier::Authority => Some(event.clone()), // Full detail
        }
    }

    fn filter_self_tier(&self, event: &AgentEvent) -> Option<AgentEvent> {
        // If blocked_attempt_counts is false and this is a block event, drop it
        if !self.reporting_config.blocked_attempt_counts && event.event_type == EventType::Block {
            return None;
        }

        let mut filtered = event.clone();

        // Strip domain info unless explicitly opted in
        if !self.reporting_config.domain_details {
            filtered.domain = None;
        }

        // Always strip metadata for self tier
        filtered.metadata = serde_json::json!({});

        Some(filtered)
    }

    fn filter_partner_tier(&self, event: &AgentEvent) -> Option<AgentEvent> {
        // If blocked_attempt_counts is false and this is a block event, drop it
        if !self.reporting_config.blocked_attempt_counts && event.event_type == EventType::Block {
            return None;
        }

        let mut filtered = event.clone();

        // Keep domain only if domain_details is true
        if !self.reporting_config.domain_details {
            filtered.domain = None;
        }

        // Redact detailed metadata unless domain_details is on (used as proxy for detail level)
        if !self.reporting_config.domain_details {
            filtered.metadata = serde_json::json!({});
        }

        Some(filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bb_common::enums::{EventCategory, EventSeverity, EventType};

    fn make_block_event() -> AgentEvent {
        AgentEvent {
            id: None,
            event_type: EventType::Block,
            category: EventCategory::Dns,
            severity: EventSeverity::Info,
            domain: Some("bet365.com".to_string()),
            plugin_id: "dns.resolver".to_string(),
            metadata: serde_json::json!({"upstream_ns": "8.8.8.8", "latency_ms": 12}),
            timestamp: chrono::Utc::now(),
            reported: false,
        }
    }

    #[test]
    fn test_self_tier_strips_domain_and_metadata() {
        let filter = PrivacyFilter::new(EnrollmentTier::SelfEnrolled, ReportingConfig::default());
        let event = make_block_event();
        let filtered = filter.filter(&event).expect("should not be dropped");

        assert!(
            filtered.domain.is_none(),
            "Self tier should strip domain by default"
        );
        assert_eq!(
            filtered.metadata,
            serde_json::json!({}),
            "Self tier should strip metadata"
        );
    }

    #[test]
    fn test_self_tier_drops_block_when_counts_disabled() {
        let config = ReportingConfig {
            blocked_attempt_counts: false,
            ..Default::default()
        };
        let filter = PrivacyFilter::new(EnrollmentTier::SelfEnrolled, config);
        let event = make_block_event();
        assert!(
            filter.filter(&event).is_none(),
            "Should drop block events when counts are disabled"
        );
    }

    #[test]
    fn test_partner_tier_keeps_domain_when_opted_in() {
        let config = ReportingConfig {
            domain_details: true,
            ..Default::default()
        };
        let filter = PrivacyFilter::new(EnrollmentTier::Partner, config);
        let event = make_block_event();
        let filtered = filter.filter(&event).expect("should not be dropped");

        assert_eq!(filtered.domain, Some("bet365.com".to_string()));
    }

    #[test]
    fn test_partner_tier_strips_domain_by_default() {
        let filter = PrivacyFilter::new(EnrollmentTier::Partner, ReportingConfig::default());
        let event = make_block_event();
        let filtered = filter.filter(&event).expect("should not be dropped");
        assert!(filtered.domain.is_none());
    }

    #[test]
    fn test_authority_tier_preserves_everything() {
        let filter = PrivacyFilter::new(EnrollmentTier::Authority, ReportingConfig::default());
        let event = make_block_event();
        let filtered = filter.filter(&event).expect("should not be dropped");

        assert_eq!(filtered.domain, Some("bet365.com".to_string()));
        assert!(filtered.metadata.get("upstream_ns").is_some());
        assert!(filtered.metadata.get("latency_ms").is_some());
    }

    #[test]
    fn test_tamper_event_always_passes_self_tier() {
        let filter = PrivacyFilter::new(EnrollmentTier::SelfEnrolled, ReportingConfig::default());
        let event = AgentEvent::tamper_detected("dns.hosts", "HOSTS file modified");
        let filtered = filter.filter(&event).expect("tamper events should pass");

        assert_eq!(filtered.event_type, EventType::TamperDetected);
        // Metadata still stripped for self tier
        assert_eq!(filtered.metadata, serde_json::json!({}));
    }
}
