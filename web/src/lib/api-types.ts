// --- Enums ---

export type AccountRole = 'user' | 'partner' | 'authority' | 'admin';
export type SubscriptionTier = 'free' | 'standard' | 'partner_tier' | 'institutional';
export type DevicePlatform = 'windows' | 'macos' | 'linux' | 'android' | 'ios';
export type DeviceStatus = 'pending' | 'active' | 'offline' | 'unenrolling' | 'unenrolled';
export type EnrollmentTier = 'self' | 'partner' | 'authority';
export type EnrollmentStatus =
  | 'pending'
  | 'active'
  | 'unenroll_requested'
  | 'unenroll_approved'
  | 'unenrolling'
  | 'unenrolled'
  | 'expired';
export type VpnDetection = 'disabled' | 'log' | 'alert' | 'lockdown';
export type TamperResponse = 'log' | 'alert_user' | 'alert_partner' | 'alert_authority';
export type ReportingLevel = 'none' | 'aggregated' | 'detailed' | 'full_audit';
export type UnenrollmentPolicyType = 'time_delayed' | 'partner_approval' | 'authority_approval';
export type EventType =
  | 'block'
  | 'bypass_attempt'
  | 'tamper_detected'
  | 'tamper_self_healed'
  | 'vpn_detected'
  | 'enrollment_created'
  | 'enrollment_modified'
  | 'unenroll_requested'
  | 'unenroll_completed'
  | 'heartbeat'
  | 'agent_started'
  | 'agent_updated'
  | 'blocklist_updated';
export type EventCategory =
  | 'dns'
  | 'app'
  | 'browser'
  | 'tamper'
  | 'enrollment'
  | 'heartbeat'
  | 'system';
export type EventSeverity = 'info' | 'warning' | 'critical';
export type BlocklistCategory =
  | 'online_casino'
  | 'sports_betting'
  | 'poker'
  | 'lottery'
  | 'bingo'
  | 'fantasy_sports'
  | 'crypto_gambling'
  | 'affiliate'
  | 'payment_processor'
  | 'other';
export type BlocklistSource = 'curated' | 'automated' | 'federated' | 'community';
export type BlocklistEntryStatus = 'pending_review' | 'active' | 'inactive' | 'rejected';
export type PartnerStatus = 'pending' | 'active' | 'revoked';
export type PartnerRole = 'accountability_partner' | 'therapist' | 'authority_rep';
export type OrganizationType = 'family' | 'clinical' | 'enterprise' | 'government';
export type OrgMemberRole = 'owner' | 'admin' | 'member';

// --- Models ---

export interface Account {
  id: string;
  email: string;
  display_name: string;
  role: AccountRole;
  email_verified: boolean;
  mfa_enabled: boolean;
  timezone: string;
  locale: string;
  organization_id: string | null;
  subscription_tier: SubscriptionTier;
  created_at: string;
  updated_at: string;
}

export interface Device {
  id: string;
  account_id: string;
  name: string;
  platform: DevicePlatform;
  os_version: string;
  agent_version: string;
  hostname: string;
  status: DeviceStatus;
  blocklist_version: number;
  last_heartbeat_at: string | null;
  certificate_fingerprint: string | null;
  enrollment_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface ProtectionConfig {
  dns_blocking: boolean;
  app_blocking: boolean;
  browser_blocking: boolean;
  vpn_detection: VpnDetection;
  tamper_response: TamperResponse;
}

export interface ReportingConfig {
  level: ReportingLevel;
  blocked_attempt_counts: boolean;
  domain_details: boolean;
  tamper_alerts: boolean;
}

export interface UnenrollmentPolicy {
  type: UnenrollmentPolicyType;
  cooldown_hours: number | null;
  requires_approval_from: string | null;
}

export interface UnenrollmentRequest {
  requested_at: string;
  requested_by: string;
  reason: string | null;
  eligible_at: string | null;
  approved_at: string | null;
  approved_by: string | null;
}

export interface Enrollment {
  id: string;
  device_id: string;
  account_id: string;
  enrolled_by: string;
  tier: EnrollmentTier;
  status: EnrollmentStatus;
  protection_config: ProtectionConfig;
  reporting_config: ReportingConfig;
  unenrollment_policy: UnenrollmentPolicy;
  unenrollment_request: UnenrollmentRequest | null;
  created_at: string;
  updated_at: string;
  expires_at: string | null;
}

export interface Event {
  id: string;
  device_id: string;
  enrollment_id: string;
  type: EventType;
  category: EventCategory;
  severity: EventSeverity;
  payload: Record<string, unknown>;
  occurred_at: string;
  received_at: string;
}

export interface BlocklistEntry {
  id: string;
  domain: string | null;
  pattern: string | null;
  category: BlocklistCategory;
  source: BlocklistSource;
  confidence: number;
  status: BlocklistEntryStatus;
  added_by: string | null;
  reviewed_by: string | null;
  evidence_url: string | null;
  tags: string[];
  blocklist_version_added: number;
  blocklist_version_removed: number | null;
  created_at: string;
  updated_at: string;
}

export interface PartnerPermissions {
  view_reports: boolean;
  approve_unenrollment: boolean;
  modify_enrollment: boolean;
}

export interface Partner {
  id: string;
  account_id: string;
  partner_account_id: string;
  status: PartnerStatus;
  role: PartnerRole;
  permissions: PartnerPermissions;
  invited_by: string;
  invited_at: string;
  accepted_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface Organization {
  id: string;
  name: string;
  org_type: OrganizationType;
  owner_id: string;
  default_protection_config: Record<string, unknown> | null;
  default_reporting_config: Record<string, unknown> | null;
  default_unenrollment_policy: Record<string, unknown> | null;
  created_at: string;
  updated_at: string;
}

export interface OrgMember {
  id: number;
  organization_id: string;
  account_id: string;
  role: OrgMemberRole;
  display_name: string | null;
  email: string | null;
  invited_by: number | null;
  joined_at: string;
}

export type AppSignatureStatus = 'active' | 'inactive' | 'pending_review';
export type AppSignaturePlatform = 'windows' | 'macos' | 'linux' | 'android' | 'ios' | 'all';

export interface EnrollmentToken {
  id: number;
  public_id: string;
  organization_id: string;
  label: string | null;
  max_uses: number | null;
  uses_count: number;
  expires_at: string | null;
  created_at: string;
}

export interface OrgDevice {
  id: number;
  organization_id: string;
  device_id: number;
  assigned_by: number | null;
  assigned_at: string;
}

export interface AppSignature {
  id: string;
  name: string;
  package_names: string[];
  executable_names: string[];
  cert_hashes: string[];
  display_name_patterns: string[];
  platforms: AppSignaturePlatform[];
  category: BlocklistCategory;
  status: AppSignatureStatus;
  confidence: number;
  source: BlocklistSource;
  evidence_url: string | null;
  tags: string[];
  created_at: string;
  updated_at: string;
}

export interface ReviewQueueItem {
  domain: string;
  report_count: number;
  first_reported_at: string;
  last_reported_at: string;
  aggregated_confidence: number;
  top_heuristic_matches: string[];
  sample_context: Record<string, unknown>;
}

/** Alias for ReviewQueueItem — represents a discovery candidate awaiting review. */
export type DiscoveryCandidate = ReviewQueueItem;

export interface ReviewQueueFilters {
  status?: string;
  source?: string;
  min_confidence?: number;
  domain?: string;
  sort?: 'confidence' | 'date';
  page?: number;
  per_page?: number;
}

export interface EventSummary {
  enrollment_id: string;
  device_id: string;
  period: string;
  from: string;
  to: string;
  summary: {
    total_blocks: number;
    total_bypass_attempts: number;
    total_tamper_events: number;
    categories: Record<string, number>;
  };
  timeseries: Array<{
    period_start: string;
    blocks: number;
    bypass_attempts: number;
    tamper_events: number;
  }>;
}

// --- Analytics types ---

/** A single point in a timeseries dataset. */
export interface TimeseriesPoint {
  /** ISO-8601 bucket timestamp (hourly or daily). */
  bucket: string;
  event_type: string;
  count: number;
}

/** A pre-computed trend metric for a device. */
export interface TrendMetric {
  metric_name: string;
  metric_value: Record<string, unknown>;
  computed_at: string;
}

/** A single cell in an activity heatmap (hour x day-of-week). */
export interface HeatmapCell {
  hour_of_day: number;
  day_of_week: number;
  event_count: number;
}

/** Full heatmap dataset. */
export interface HeatmapData {
  heatmap: HeatmapCell[];
}

/** Summary statistics for analytics dashboard. */
export interface AnalyticsSummary {
  total_events: number;
  total_blocks: number;
  total_bypass_attempts: number;
  total_tamper_events: number;
  unique_event_types: number;
}

/** Timeseries response envelope from GET /v1/analytics/timeseries. */
export interface TimeseriesResponse {
  period: string;
  data: Array<{
    timestamp: string;
    device_id: number;
    event_type: string;
    event_count: number;
  }>;
}

/** Trends response envelope from GET /v1/analytics/trends. */
export interface TrendsResponse {
  trends: Array<{
    id: number;
    device_id: number;
    metric_name: string;
    metric_value: Record<string, unknown>;
    computed_at: string;
    period_start: string;
    period_end: string;
  }>;
}

// --- API response envelopes ---

export interface ApiMeta {
  request_id: string;
  timestamp: string;
}

export interface ApiResponse<T> {
  data: T;
  meta: ApiMeta;
}

export interface PaginatedResponse<T> {
  data: T[];
  meta: ApiMeta;
  pagination: {
    total: number;
    page: number;
    per_page: number;
    total_pages: number;
  };
}

export interface ApiError {
  error: {
    code: string;
    message: string;
    details?: {
      fields?: Record<string, string[]>;
    };
  };
  meta: ApiMeta;
}

// --- Auth response types ---

export interface AuthTokens {
  access_token: string;
  refresh_token: string;
  expires_in: number;
}

export interface LoginResponse {
  account: Pick<
    Account,
    'id' | 'email' | 'display_name' | 'role' | 'email_verified' | 'mfa_enabled'
  >;
  access_token: string;
  refresh_token: string;
  expires_in: number;
}

export interface RegisterResponse {
  account: Pick<
    Account,
    'id' | 'email' | 'display_name' | 'role' | 'email_verified' | 'created_at'
  >;
  access_token: string;
  refresh_token: string;
  expires_in: number;
}
