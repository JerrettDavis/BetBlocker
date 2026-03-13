use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::{
    EnrollmentStatus, EnrollmentTier, ReportingLevel, TamperResponse, UnenrollmentPolicyType,
    VpnDetectionMode,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enrollment {
    pub id: i64,
    pub public_id: Uuid,
    pub device_id: i64,
    pub account_id: i64,
    pub enrolled_by: i64,
    pub tier: EnrollmentTier,
    pub status: EnrollmentStatus,
    pub protection_config: ProtectionConfig,
    pub reporting_config: ReportingConfig,
    pub unenrollment_policy: UnenrollmentPolicy,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectionConfig {
    pub dns_blocking: bool,
    pub app_blocking: bool,
    pub browser_blocking: bool,
    pub vpn_detection: VpnDetectionMode,
    pub tamper_response: TamperResponse,
}

impl Default for ProtectionConfig {
    fn default() -> Self {
        Self {
            dns_blocking: true,
            app_blocking: false,
            browser_blocking: false,
            vpn_detection: VpnDetectionMode::Alert,
            tamper_response: TamperResponse::Log,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportingConfig {
    pub level: ReportingLevel,
    pub blocked_attempt_counts: bool,
    pub domain_details: bool,
    pub tamper_alerts: bool,
}

impl Default for ReportingConfig {
    fn default() -> Self {
        Self {
            level: ReportingLevel::Aggregated,
            blocked_attempt_counts: true,
            domain_details: false,
            tamper_alerts: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnenrollmentPolicy {
    #[serde(rename = "type")]
    pub policy_type: UnenrollmentPolicyType,
    pub cooldown_hours: Option<i32>,
    pub requires_approval_from: Option<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protection_config_default_has_dns_enabled() {
        let config = ProtectionConfig::default();
        assert!(config.dns_blocking);
        assert!(!config.app_blocking);
        assert!(!config.browser_blocking);
    }

    #[test]
    fn protection_config_roundtrips_json() {
        let config = ProtectionConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let roundtripped: ProtectionConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.dns_blocking, roundtripped.dns_blocking);
        assert_eq!(config.vpn_detection, roundtripped.vpn_detection);
    }

    #[test]
    fn unenrollment_policy_self_tier() {
        let policy = UnenrollmentPolicy {
            policy_type: UnenrollmentPolicyType::TimeDelayed,
            cooldown_hours: Some(48),
            requires_approval_from: None,
        };
        let json = serde_json::to_string(&policy).unwrap();
        assert!(json.contains("time_delayed"));
        assert!(json.contains("48"));
    }
}
