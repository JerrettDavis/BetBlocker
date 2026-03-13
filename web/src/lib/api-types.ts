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

export interface ReviewQueueItem {
  domain: string;
  report_count: number;
  first_reported_at: string;
  last_reported_at: string;
  aggregated_confidence: number;
  top_heuristic_matches: string[];
  sample_context: Record<string, unknown>;
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
