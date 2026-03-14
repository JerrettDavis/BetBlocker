import type {
  Account,
  AnalyticsSummary,
  ApiResponse,
  AppSignature,
  AuthTokens,
  BlocklistEntry,
  Device,
  Enrollment,
  EnrollmentToken,
  Event,
  EventSummary,
  HeatmapData,
  LoginResponse,
  OrgDevice,
  OrgMember,
  OrgMemberRole,
  Organization,
  PaginatedResponse,
  Partner,
  PartnerPermissions,
  ProtectionConfig,
  RegisterResponse,
  ReportingConfig,
  ReviewQueueItem,
  TimeseriesResponse,
  TrendsResponse,
  UnenrollmentPolicy,
} from './api-types';
import { API_BASE_URL } from './constants';

export class ApiClientError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
    public details?: Record<string, unknown>,
  ) {
    super(message);
    this.name = 'ApiClientError';
  }
}

let accessToken: string | null = null;

export function setAccessToken(token: string | null) {
  accessToken = token;
}

export function getAccessToken(): string | null {
  return accessToken;
}

async function request<T>(
  method: string,
  path: string,
  body?: unknown,
  opts?: { noAuth?: boolean },
): Promise<T> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };

  if (!opts?.noAuth && accessToken) {
    headers['Authorization'] = `Bearer ${accessToken}`;
  }

  const res = await fetch(`${API_BASE_URL}${path}`, {
    method,
    headers,
    body: body ? JSON.stringify(body) : undefined,
    credentials: 'include',
  });

  if (res.status === 204) {
    return undefined as T;
  }

  const json = await res.json();

  if (!res.ok) {
    throw new ApiClientError(
      res.status,
      json.error?.code ?? 'UNKNOWN_ERROR',
      json.error?.message ?? 'An unexpected error occurred',
      json.error?.details,
    );
  }

  return json as T;
}

// --- Auth ---

export const auth = {
  register(data: {
    email: string;
    password: string;
    display_name: string;
    timezone?: string;
    locale?: string;
  }) {
    return request<ApiResponse<RegisterResponse>>('POST', '/auth/register', data, {
      noAuth: true,
    });
  },

  login(data: { email: string; password: string; mfa_code?: string }) {
    return request<ApiResponse<LoginResponse>>('POST', '/auth/login', data, { noAuth: true });
  },

  refresh() {
    return request<ApiResponse<AuthTokens>>('POST', '/auth/refresh', undefined, { noAuth: true });
  },

  logout(refresh_token: string) {
    return request<void>('POST', '/auth/logout', { refresh_token });
  },

  forgotPassword(email: string) {
    return request<ApiResponse<{ message: string }>>(
      'POST',
      '/auth/forgot-password',
      { email },
      { noAuth: true },
    );
  },

  resetPassword(token: string, new_password: string) {
    return request<ApiResponse<{ message: string }>>(
      'POST',
      '/auth/reset-password',
      { token, new_password },
      { noAuth: true },
    );
  },
};

// --- Accounts ---

export const accounts = {
  me() {
    return request<ApiResponse<Account>>('GET', '/accounts/me');
  },

  updateMe(
    data: Partial<Pick<Account, 'display_name' | 'timezone' | 'locale'>> & {
      current_password?: string;
      new_password?: string;
      email?: string;
    },
  ) {
    return request<ApiResponse<Account>>('PATCH', '/accounts/me', data);
  },
};

// --- Devices ---

export const devices = {
  list(params?: { status?: string; platform?: string; page?: number; per_page?: number }) {
    const qs = new URLSearchParams();
    if (params?.status) qs.set('status', params.status);
    if (params?.platform) qs.set('platform', params.platform);
    if (params?.page) qs.set('page', String(params.page));
    if (params?.per_page) qs.set('per_page', String(params.per_page));
    const query = qs.toString();
    return request<PaginatedResponse<Device>>('GET', `/devices${query ? `?${query}` : ''}`);
  },

  get(id: string) {
    return request<ApiResponse<Device>>('GET', `/devices/${id}`);
  },

  delete(id: string, reason?: string) {
    return request<
      ApiResponse<{
        device: Pick<Device, 'id' | 'status'>;
        unenrollment: Record<string, unknown>;
      }>
    >('DELETE', `/devices/${id}`, reason ? { reason } : undefined);
  },
};

// --- Enrollments ---

export const enrollments = {
  list(params?: {
    status?: string;
    tier?: string;
    device_id?: string;
    page?: number;
    per_page?: number;
  }) {
    const qs = new URLSearchParams();
    if (params?.status) qs.set('status', params.status);
    if (params?.tier) qs.set('tier', params.tier);
    if (params?.device_id) qs.set('device_id', params.device_id);
    if (params?.page) qs.set('page', String(params.page));
    if (params?.per_page) qs.set('per_page', String(params.per_page));
    const query = qs.toString();
    return request<PaginatedResponse<Enrollment>>(
      'GET',
      `/enrollments${query ? `?${query}` : ''}`,
    );
  },

  get(id: string) {
    return request<ApiResponse<Enrollment>>('GET', `/enrollments/${id}`);
  },

  create(data: {
    device_id: string;
    tier: string;
    protection_config?: Partial<ProtectionConfig>;
    reporting_config?: Partial<ReportingConfig>;
    unenrollment_policy?: Partial<UnenrollmentPolicy>;
    expires_at?: string | null;
  }) {
    return request<ApiResponse<Enrollment>>('POST', '/enrollments', data);
  },

  update(
    id: string,
    data: {
      protection_config?: Partial<ProtectionConfig>;
      reporting_config?: Partial<ReportingConfig>;
      unenrollment_policy?: Partial<UnenrollmentPolicy>;
      expires_at?: string | null;
    },
  ) {
    return request<ApiResponse<Enrollment>>('PATCH', `/enrollments/${id}`, data);
  },

  requestUnenroll(id: string, reason?: string) {
    return request<ApiResponse<{ enrollment: Enrollment; message: string }>>(
      'POST',
      `/enrollments/${id}/unenroll`,
      reason ? { reason } : undefined,
    );
  },

  approveUnenroll(id: string, data: { approved: boolean; note?: string }) {
    return request<ApiResponse<{ enrollment: Enrollment; message: string }>>(
      'POST',
      `/enrollments/${id}/approve-unenroll`,
      data,
    );
  },
};

// --- Partners ---

export const partners = {
  list(params?: { status?: string; role?: string; page?: number; per_page?: number }) {
    const qs = new URLSearchParams();
    if (params?.status) qs.set('status', params.status);
    if (params?.role) qs.set('role', params.role);
    if (params?.page) qs.set('page', String(params.page));
    if (params?.per_page) qs.set('per_page', String(params.per_page));
    const query = qs.toString();
    return request<PaginatedResponse<Partner>>('GET', `/partners${query ? `?${query}` : ''}`);
  },

  invite(data: {
    email: string;
    role: string;
    permissions?: Partial<PartnerPermissions>;
    message?: string;
  }) {
    return request<ApiResponse<Partner>>('POST', '/partners/invite', data);
  },

  accept(token: string) {
    return request<ApiResponse<Partner>>('POST', '/partners/accept', { token });
  },

  remove(id: string) {
    return request<
      ApiResponse<{ id: string; status: string; affected_enrollments: unknown[] }>
    >('DELETE', `/partners/${id}`);
  },
};

// --- Organizations ---

export const organizations = {
  create(data: { name: string; org_type: string }) {
    return request<ApiResponse<Organization>>('POST', '/organizations', data);
  },

  list(params?: { page?: number; per_page?: number }) {
    const qs = new URLSearchParams();
    if (params?.page) qs.set('page', String(params.page));
    if (params?.per_page) qs.set('per_page', String(params.per_page));
    const query = qs.toString();
    return request<PaginatedResponse<Organization>>(
      'GET',
      `/organizations${query ? `?${query}` : ''}`,
    );
  },

  get(id: string) {
    return request<ApiResponse<Organization>>('GET', `/organizations/${id}`);
  },

  update(
    id: string,
    data: {
      name?: string;
      org_type?: string;
      default_protection_config?: Record<string, unknown>;
      default_reporting_config?: Record<string, unknown>;
      default_unenrollment_policy?: Record<string, unknown>;
    },
  ) {
    return request<ApiResponse<Organization>>('PATCH', `/organizations/${id}`, data);
  },

  delete(id: string) {
    return request<ApiResponse<{ deleted: boolean; id: string }>>('DELETE', `/organizations/${id}`);
  },

  inviteMember(orgId: string, data: { email: string; role: OrgMemberRole }) {
    return request<ApiResponse<OrgMember>>('POST', `/organizations/${orgId}/members`, data);
  },

  listMembers(orgId: string, params?: { page?: number; per_page?: number }) {
    const qs = new URLSearchParams();
    if (params?.page) qs.set('page', String(params.page));
    if (params?.per_page) qs.set('per_page', String(params.per_page));
    const query = qs.toString();
    return request<PaginatedResponse<OrgMember>>(
      'GET',
      `/organizations/${orgId}/members${query ? `?${query}` : ''}`,
    );
  },

  updateMemberRole(orgId: string, memberId: string, data: { role: OrgMemberRole }) {
    return request<ApiResponse<OrgMember>>(
      'PATCH',
      `/organizations/${orgId}/members/${memberId}`,
      data,
    );
  },

  removeMember(orgId: string, memberId: string) {
    return request<ApiResponse<{ deleted: boolean; organization_id: string; account_id: string }>>(
      'DELETE',
      `/organizations/${orgId}/members/${memberId}`,
    );
  },

  // Device assignment
  assignDevice(orgId: string, data: { device_id: number }) {
    return request<ApiResponse<OrgDevice>>('POST', `/organizations/${orgId}/devices`, data);
  },

  listDevices(orgId: string, params?: { page?: number; per_page?: number }) {
    const qs = new URLSearchParams();
    if (params?.page) qs.set('page', String(params.page));
    if (params?.per_page) qs.set('per_page', String(params.per_page));
    const query = qs.toString();
    return request<PaginatedResponse<OrgDevice>>(
      'GET',
      `/organizations/${orgId}/devices${query ? `?${query}` : ''}`,
    );
  },

  unassignDevice(orgId: string, deviceId: number) {
    return request<ApiResponse<{ deleted: boolean; organization_id: string; device_id: number }>>(
      'DELETE',
      `/organizations/${orgId}/devices/${deviceId}`,
    );
  },

  // Enrollment tokens
  createToken(
    orgId: string,
    data: {
      label?: string;
      protection_config: Record<string, unknown>;
      reporting_config: Record<string, unknown>;
      unenrollment_policy: Record<string, unknown>;
      max_uses?: number;
      expires_at?: string;
    },
  ) {
    return request<ApiResponse<EnrollmentToken>>(
      'POST',
      `/organizations/${orgId}/tokens`,
      data,
    );
  },

  listTokens(orgId: string, params?: { page?: number; per_page?: number }) {
    const qs = new URLSearchParams();
    if (params?.page) qs.set('page', String(params.page));
    if (params?.per_page) qs.set('per_page', String(params.per_page));
    const query = qs.toString();
    return request<PaginatedResponse<EnrollmentToken>>(
      'GET',
      `/organizations/${orgId}/tokens${query ? `?${query}` : ''}`,
    );
  },

  revokeToken(orgId: string, tokenId: number) {
    return request<ApiResponse<{ revoked: boolean; token_id: number; organization_id: string }>>(
      'DELETE',
      `/organizations/${orgId}/tokens/${tokenId}`,
    );
  },

  getTokenQrUrl(orgId: string, tokenId: number): string {
    return `${API_BASE_URL}/organizations/${orgId}/tokens/${tokenId}/qr`;
  },
};

// --- Blocklist Admin ---

export const blocklist = {
  listEntries(params?: {
    search?: string;
    category?: string;
    source?: string;
    status?: string;
    page?: number;
    per_page?: number;
  }) {
    const qs = new URLSearchParams();
    if (params?.search) qs.set('search', params.search);
    if (params?.category) qs.set('category', params.category);
    if (params?.source) qs.set('source', params.source);
    if (params?.status) qs.set('status', params.status);
    if (params?.page) qs.set('page', String(params.page));
    if (params?.per_page) qs.set('per_page', String(params.per_page));
    const query = qs.toString();
    return request<PaginatedResponse<BlocklistEntry>>(
      'GET',
      `/admin/blocklist/entries${query ? `?${query}` : ''}`,
    );
  },

  createEntry(data: {
    domain?: string;
    pattern?: string;
    category: string;
    evidence_url?: string;
    tags?: string[];
    notes?: string;
  }) {
    return request<ApiResponse<BlocklistEntry>>('POST', '/admin/blocklist/entries', data);
  },

  updateEntry(
    id: string,
    data: {
      category?: string;
      status?: string;
      tags?: string[];
      evidence_url?: string;
      notes?: string;
    },
  ) {
    return request<ApiResponse<BlocklistEntry>>('PATCH', `/admin/blocklist/entries/${id}`, data);
  },

  deleteEntry(id: string) {
    return request<ApiResponse<BlocklistEntry>>('DELETE', `/admin/blocklist/entries/${id}`);
  },

  reviewQueue(params?: {
    min_reports?: number;
    min_confidence?: number;
    sort?: string;
    page?: number;
    per_page?: number;
  }) {
    const qs = new URLSearchParams();
    if (params?.min_reports) qs.set('min_reports', String(params.min_reports));
    if (params?.min_confidence) qs.set('min_confidence', String(params.min_confidence));
    if (params?.sort) qs.set('sort', params.sort);
    if (params?.page) qs.set('page', String(params.page));
    if (params?.per_page) qs.set('per_page', String(params.per_page));
    const query = qs.toString();
    return request<PaginatedResponse<ReviewQueueItem>>(
      'GET',
      `/admin/blocklist/review-queue${query ? `?${query}` : ''}`,
    );
  },

  resolveReview(
    domain: string,
    data: { action: 'promote' | 'reject'; category?: string; tags?: string[]; notes?: string },
  ) {
    return request<ApiResponse<BlocklistEntry | { message: string }>>(
      'POST',
      `/admin/blocklist/review-queue/${encodeURIComponent(domain)}/resolve`,
      data,
    );
  },
};

// ---------------------------------------------------------------------------
// Helper for binary/blob downloads
// ---------------------------------------------------------------------------

async function requestBlob(path: string): Promise<Blob> {
  const headers: Record<string, string> = {};
  if (accessToken) {
    headers['Authorization'] = `Bearer ${accessToken}`;
  }

  const res = await fetch(`${API_BASE_URL}${path}`, {
    method: 'GET',
    headers,
    credentials: 'include',
  });

  if (!res.ok) {
    let code = 'UNKNOWN_ERROR';
    let message = 'An unexpected error occurred';
    try {
      const json = await res.json();
      code = json.error?.code ?? code;
      message = json.error?.message ?? message;
    } catch {
      // ignore JSON parse failure
    }
    throw new ApiClientError(res.status, code, message);
  }

  return res.blob();
}

// --- Enrollment (token redemption) ---

export const enroll = {
  redeem(tokenPublicId: string, data: { device_id: number }) {
    return request<
      ApiResponse<{
        redeemed: boolean;
        token_public_id: string;
        organization_id: number;
        device_id: number;
      }>
    >('POST', `/enroll/${tokenPublicId}`, data);
  },
};

// --- Admin App Signatures ---

export const adminAppSignatures = {
  create(data: {
    name: string;
    package_names?: string[];
    executable_names?: string[];
    cert_hashes?: string[];
    display_name_patterns?: string[];
    platforms?: string[];
    category: string;
    status?: string;
    confidence?: number;
    source?: string;
    evidence_url?: string;
    tags?: string[];
  }) {
    return request<ApiResponse<AppSignature>>('POST', '/admin/app-signatures', data);
  },

  list(params?: {
    search?: string;
    category?: string;
    platform?: string;
    status?: string;
    page?: number;
    per_page?: number;
  }) {
    const qs = new URLSearchParams();
    if (params?.search) qs.set('search', params.search);
    if (params?.category) qs.set('category', params.category);
    if (params?.platform) qs.set('platform', params.platform);
    if (params?.status) qs.set('status', params.status);
    if (params?.page) qs.set('page', String(params.page));
    if (params?.per_page) qs.set('per_page', String(params.per_page));
    const query = qs.toString();
    return request<PaginatedResponse<AppSignature>>(
      'GET',
      `/admin/app-signatures${query ? `?${query}` : ''}`,
    );
  },

  get(id: string) {
    return request<ApiResponse<AppSignature>>('GET', `/admin/app-signatures/${id}`);
  },

  update(
    id: string,
    data: {
      name?: string;
      package_names?: string[];
      executable_names?: string[];
      cert_hashes?: string[];
      display_name_patterns?: string[];
      platforms?: string[];
      category?: string;
      status?: string;
      confidence?: number;
      source?: string;
      evidence_url?: string;
      tags?: string[];
    },
  ) {
    return request<ApiResponse<AppSignature>>('PUT', `/admin/app-signatures/${id}`, data);
  },

  delete(id: string) {
    return request<ApiResponse<{ deleted: boolean; id: string }>>(
      'DELETE',
      `/admin/app-signatures/${id}`,
    );
  },
};

// --- Events ---

export const events = {
  list(params?: {
    device_id?: string;
    enrollment_id?: string;
    type?: string;
    category?: string;
    severity?: string;
    from?: string;
    to?: string;
    page?: number;
    per_page?: number;
  }) {
    const qs = new URLSearchParams();
    if (params?.device_id) qs.set('device_id', params.device_id);
    if (params?.enrollment_id) qs.set('enrollment_id', params.enrollment_id);
    if (params?.type) qs.set('type', params.type);
    if (params?.category) qs.set('category', params.category);
    if (params?.severity) qs.set('severity', params.severity);
    if (params?.from) qs.set('from', params.from);
    if (params?.to) qs.set('to', params.to);
    if (params?.page) qs.set('page', String(params.page));
    if (params?.per_page) qs.set('per_page', String(params.per_page));
    const query = qs.toString();
    return request<PaginatedResponse<Event>>('GET', `/events${query ? `?${query}` : ''}`);
  },

  summary(params?: {
    enrollment_id?: string;
    device_id?: string;
    period?: 'hour' | 'day' | 'week' | 'month';
    from?: string;
    to?: string;
  }) {
    const qs = new URLSearchParams();
    if (params?.enrollment_id) qs.set('enrollment_id', params.enrollment_id);
    if (params?.device_id) qs.set('device_id', params.device_id);
    if (params?.period) qs.set('period', params.period);
    if (params?.from) qs.set('from', params.from);
    if (params?.to) qs.set('to', params.to);
    const query = qs.toString();
    return request<ApiResponse<EventSummary>>(
      'GET',
      `/events/summary${query ? `?${query}` : ''}`,
    );
  },
};

// --- Analytics ---

export const analyticsApi = {
  timeseries(params: {
    device_id: number;
    period?: 'hourly' | 'daily';
    from: string;
    to: string;
  }) {
    const qs = new URLSearchParams();
    qs.set('device_id', String(params.device_id));
    if (params.period) qs.set('period', params.period);
    qs.set('from', params.from);
    qs.set('to', params.to);
    return request<ApiResponse<TimeseriesResponse>>('GET', `/v1/analytics/timeseries?${qs}`);
  },

  trends(params: { device_id: number; metrics?: string }) {
    const qs = new URLSearchParams();
    qs.set('device_id', String(params.device_id));
    if (params.metrics) qs.set('metrics', params.metrics);
    return request<ApiResponse<TrendsResponse>>('GET', `/v1/analytics/trends?${qs}`);
  },

  summary(params: { device_id: number; from: string; to: string }) {
    const qs = new URLSearchParams();
    qs.set('device_id', String(params.device_id));
    qs.set('from', params.from);
    qs.set('to', params.to);
    return request<ApiResponse<AnalyticsSummary>>('GET', `/v1/analytics/summary?${qs}`);
  },

  heatmap(params: { device_id: number; from: string; to: string }) {
    const qs = new URLSearchParams();
    qs.set('device_id', String(params.device_id));
    qs.set('from', params.from);
    qs.set('to', params.to);
    return request<ApiResponse<HeatmapData>>('GET', `/v1/analytics/heatmap?${qs}`);
  },

  exportCsv(params: { device_id: number; from: string; to: string }): Promise<Blob> {
    const qs = new URLSearchParams();
    qs.set('device_id', String(params.device_id));
    qs.set('from', params.from);
    qs.set('to', params.to);
    return requestBlob(`/v1/analytics/export/csv?${qs}`);
  },

  exportPdf(params: { device_id: number; from: string; to: string }): Promise<Blob> {
    const qs = new URLSearchParams();
    qs.set('device_id', String(params.device_id));
    qs.set('from', params.from);
    qs.set('to', params.to);
    return requestBlob(`/v1/analytics/export/pdf?${qs}`);
  },
};
