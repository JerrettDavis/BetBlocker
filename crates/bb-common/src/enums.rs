use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountRole {
    User,
    Partner,
    Authority,
    Admin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Windows,
    Macos,
    Linux,
    Android,
    Ios,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceStatus {
    Pending,
    Active,
    Offline,
    Unenrolling,
    Unenrolled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnrollmentTier {
    #[serde(rename = "self")]
    SelfEnrolled,
    Partner,
    Authority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnrollmentStatus {
    Pending,
    Active,
    UnenrollRequested,
    UnenrollApproved,
    Unenrolling,
    Unenrolled,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnenrollmentPolicyType {
    TimeDelayed,
    PartnerApproval,
    AuthorityApproval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnenrollRequestStatus {
    Pending,
    Approved,
    Denied,
    Expired,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartnerRelationshipStatus {
    Pending,
    Active,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartnerRole {
    AccountabilityPartner,
    Therapist,
    AuthorityRep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationType {
    Family,
    TherapyPractice,
    CourtProgram,
    Employer,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrgMemberRole {
    Owner,
    Admin,
    Member,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlocklistSource {
    Curated,
    Automated,
    Federated,
    Community,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlocklistEntryStatus {
    PendingReview,
    Active,
    Inactive,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GamblingCategory {
    OnlineCasino,
    SportsBetting,
    Poker,
    Lottery,
    Bingo,
    FantasySports,
    CryptoGambling,
    Affiliate,
    PaymentProcessor,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Block,
    BypassAttempt,
    TamperDetected,
    TamperSelfHealed,
    VpnDetected,
    EnrollmentCreated,
    EnrollmentModified,
    UnenrollRequested,
    UnenrollCompleted,
    Heartbeat,
    AgentStarted,
    AgentUpdated,
    BlocklistUpdated,
    /// A blocked application was detected running on the device.
    AppDetected,
    /// A blocked application process was killed by the agent.
    AppBlocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventCategory {
    Dns,
    App,
    Browser,
    Tamper,
    Enrollment,
    Heartbeat,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportingLevel {
    None,
    Aggregated,
    Detailed,
    FullAudit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VpnDetectionMode {
    Disabled,
    Log,
    Alert,
    Block,
    Lockdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TamperResponse {
    Log,
    AlertUser,
    AlertPartner,
    AlertAuthority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionPlan {
    Free,
    Standard,
    PartnerTier,
    Institutional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Trialing,
    Active,
    PastDue,
    Cancelled,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FederatedReportStatus {
    Pending,
    Promoted,
    Rejected,
    Duplicate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockingLayer {
    Dns,
    Application,
    Browser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockDecision {
    Allow,
    Block,
}

// ── SP2: Discovery Pipeline ─────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryCandidateStatus {
    Pending,
    Approved,
    Rejected,
    Deferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlerSource {
    Affiliate,
    LicenseRegistry,
    WhoisPattern,
    DnsZone,
    SearchEngine,
    Federated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FederatedAggregateStatus {
    Collecting,
    ThresholdMet,
    Reviewing,
    Promoted,
    Rejected,
}

// ── SP3: App Blocking ───────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppSignatureStatus {
    Active,
    Inactive,
    PendingReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppSignaturePlatform {
    Windows,
    Macos,
    Linux,
    Android,
    Ios,
    All,
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! serde_roundtrip {
        ($name:ident, $ty:ty, $variant:expr, $expected_json:expr) => {
            #[test]
            fn $name() {
                let val: $ty = $variant;
                let json = serde_json::to_string(&val).unwrap();
                assert_eq!(json, $expected_json);
                let back: $ty = serde_json::from_str(&json).unwrap();
                assert_eq!(val, back);
            }
        };
    }

    // SP2 enums
    serde_roundtrip!(discovery_candidate_status_pending, DiscoveryCandidateStatus, DiscoveryCandidateStatus::Pending, "\"pending\"");
    serde_roundtrip!(discovery_candidate_status_deferred, DiscoveryCandidateStatus, DiscoveryCandidateStatus::Deferred, "\"deferred\"");
    serde_roundtrip!(crawler_source_license_registry, CrawlerSource, CrawlerSource::LicenseRegistry, "\"license_registry\"");
    serde_roundtrip!(crawler_source_search_engine, CrawlerSource, CrawlerSource::SearchEngine, "\"search_engine\"");
    serde_roundtrip!(federated_aggregate_status_threshold_met, FederatedAggregateStatus, FederatedAggregateStatus::ThresholdMet, "\"threshold_met\"");
    serde_roundtrip!(federated_aggregate_status_collecting, FederatedAggregateStatus, FederatedAggregateStatus::Collecting, "\"collecting\"");

    // VpnDetectionMode
    serde_roundtrip!(vpn_detection_mode_block, VpnDetectionMode, VpnDetectionMode::Block, "\"block\"");
    serde_roundtrip!(vpn_detection_mode_lockdown, VpnDetectionMode, VpnDetectionMode::Lockdown, "\"lockdown\"");

    // SP3 enums
    serde_roundtrip!(app_signature_status_active, AppSignatureStatus, AppSignatureStatus::Active, "\"active\"");
    serde_roundtrip!(app_signature_status_pending_review, AppSignatureStatus, AppSignatureStatus::PendingReview, "\"pending_review\"");
    serde_roundtrip!(app_signature_platform_all, AppSignaturePlatform, AppSignaturePlatform::All, "\"all\"");
    serde_roundtrip!(app_signature_platform_macos, AppSignaturePlatform, AppSignaturePlatform::Macos, "\"macos\"");
}
