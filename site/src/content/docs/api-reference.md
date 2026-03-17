---
title: API Reference
description: Complete REST API documentation
---


Base URL: `https://your-server/v1`

All requests and responses use JSON. All authenticated endpoints require a `Bearer` token in the `Authorization` header. Every response includes an `X-Request-Id` header for tracing.

**Authentication:** obtain a token via `POST /v1/auth/login`. Access tokens expire after 1 hour (configurable). Use the refresh token to obtain a new access token without re-authenticating.

---

## Common Response Shapes

### Error

```json
{
  "error": "string",
  "message": "human-readable detail"
}
```

### Pagination (where applicable)

Query params: `?page=1&per_page=20`

```json
{
  "data": [...],
  "page": 1,
  "per_page": 20,
  "total": 143
}
```

---

## Health

### `GET /health`

Auth required: No

Returns service health. Use this for monitoring and uptime checks.

**Response 200:**
```json
{
  "status": "ok",
  "version": "1.2.3",
  "db": "ok",
  "cache": "ok"
}
```

---

## Authentication

### `POST /v1/auth/register`

Auth required: No

Create a new account.

**Request:**
```json
{
  "email": "user@example.com",
  "password": "strong-password",
  "display_name": "Alex"
}
```

**Response 201:**
```json
{
  "account_id": "01jwxyz...",
  "email": "user@example.com",
  "display_name": "Alex"
}
```

---

### `POST /v1/auth/login`

Auth required: No

**Request:**
```json
{
  "email": "user@example.com",
  "password": "strong-password"
}
```

**Response 200:**
```json
{
  "access_token": "eyJ...",
  "refresh_token": "eyJ...",
  "expires_in": 3600
}
```

---

### `POST /v1/auth/refresh`

Auth required: No (uses refresh token in body)

**Request:**
```json
{
  "refresh_token": "eyJ..."
}
```

**Response 200:** Same shape as `/login`.

---

### `POST /v1/auth/logout`

Auth required: Yes

Revokes the current refresh token.

**Response 204:** No body.

---

### `POST /v1/auth/forgot-password`

Auth required: No

Sends a password reset email if the address is registered. Always returns 200 (no account enumeration).

**Request:**
```json
{ "email": "user@example.com" }
```

**Response 200:** `{ "message": "If that address is registered, a reset email has been sent." }`

---

### `POST /v1/auth/reset-password`

Auth required: No

**Request:**
```json
{
  "token": "reset-token-from-email",
  "new_password": "new-strong-password"
}
```

**Response 200:** `{ "message": "Password updated." }`

---

## Accounts

### `GET /v1/accounts/me`

Auth required: Yes

Returns the authenticated user's account.

**Response 200:**
```json
{
  "id": "01jwxyz...",
  "email": "user@example.com",
  "display_name": "Alex",
  "created_at": "2026-03-15T10:00:00Z"
}
```

---

### `PATCH /v1/accounts/me`

Auth required: Yes

Update display name or email.

**Request (all fields optional):**
```json
{
  "display_name": "New Name",
  "email": "new@example.com"
}
```

**Response 200:** Updated account object.

---

### `GET /v1/accounts/{id}`

Auth required: Yes

Returns another account's public profile. Used by partners to look up linked accounts.

**Response 200:** Account object (subset — no email for non-self).

---

## Devices

### `POST /v1/devices`

Auth required: Yes

Register a new device. Called by the agent during enrollment.

**Request:**
```json
{
  "enrollment_token": "tok_...",
  "platform": "windows",
  "hostname": "alex-laptop",
  "agent_version": "1.2.3",
  "client_cert_csr": "-----BEGIN CERTIFICATE REQUEST-----..."
}
```

**Response 201:**
```json
{
  "device_id": "01jwxyz...",
  "client_cert": "-----BEGIN CERTIFICATE-----...",
  "blocklist_version": 42,
  "config": { ... }
}
```

---

### `GET /v1/devices`

Auth required: Yes

List all devices belonging to the authenticated account.

**Response 200:**
```json
{
  "data": [
    {
      "id": "01jwxyz...",
      "hostname": "alex-laptop",
      "platform": "windows",
      "status": "active",
      "last_heartbeat": "2026-03-15T10:55:00Z",
      "enrollment_id": "01jwxyz...",
      "agent_version": "1.2.3"
    }
  ]
}
```

---

### `GET /v1/devices/{id}`

Auth required: Yes

**Response 200:** Single device object with full detail.

---

### `DELETE /v1/devices/{id}`

Auth required: Yes

Remove a device record. Does not unenroll — use `POST /v1/enrollments/{id}/unenroll` to initiate the unenrollment flow.

**Response 204:** No body.

---

### `POST /v1/devices/{id}/heartbeat`

Auth required: Yes

Called by the agent on a regular interval (default: every 5 minutes). Records the device as alive and returns any pending configuration changes.

**Request:**
```json
{
  "blocklist_version": 42,
  "agent_version": "1.2.3",
  "status": "healthy",
  "integrity_ok": true
}
```

**Response 200:**
```json
{
  "blocklist_version": 43,
  "config_changed": false,
  "commands": []
}
```

`commands` may contain server-initiated actions (e.g., `force_blocklist_update`).

---

### `GET /v1/devices/{id}/config`

Auth required: Yes

Returns the full configuration for the device including blocklist server URL, heartbeat interval, and protection settings.

**Response 200:**
```json
{
  "blocklist_url": "https://your-server/v1/blocklist/delta",
  "heartbeat_interval_secs": 300,
  "protection": {
    "dns_blocking": true,
    "app_blocking": false,
    "bypass_detection": true
  },
  "reporting": {
    "send_block_events": true,
    "send_tamper_events": true
  }
}
```

---

## Enrollments

### `POST /v1/enrollments`

Auth required: Yes

Create a new enrollment (generates an enrollment token for use during agent setup).

**Request:**
```json
{
  "device_id": "01jwxyz...",
  "tier": "partner",
  "unenroll_delay_hours": 48,
  "partner_account_id": "01jwxyz..."
}
```

`tier` values: `self`, `partner`, `authority`.

**Response 201:**
```json
{
  "id": "01jwxyz...",
  "token": "tok_...",
  "qr_url": "https://your-server/v1/enrollments/01jwxyz.../qr",
  "tier": "partner",
  "status": "pending"
}
```

---

### `GET /v1/enrollments`

Auth required: Yes

List enrollments for the authenticated account (as enrollee or as partner/authority).

**Response 200:** Paginated list of enrollment objects.

---

### `GET /v1/enrollments/{id}`

Auth required: Yes

**Response 200:** Full enrollment object including protection config and unenrollment policy.

---

### `PATCH /v1/enrollments/{id}`

Auth required: Yes (enrollment authority only)

Update enrollment configuration (protection settings, reporting settings).

**Request (all fields optional):**
```json
{
  "unenroll_delay_hours": 72,
  "protection": {
    "bypass_detection": true
  }
}
```

**Response 200:** Updated enrollment object.

---

### `POST /v1/enrollments/{id}/unenroll`

Auth required: Yes

Initiate an unenrollment request. For `self` tier, starts the time-delay timer. For `partner`/`authority` tier, creates a pending request requiring approval.

**Response 202:**
```json
{
  "status": "pending_approval",
  "eligible_at": null,
  "message": "Unenrollment request sent to your accountability partner."
}
```

For self tier:
```json
{
  "status": "pending_delay",
  "eligible_at": "2026-03-17T10:00:00Z"
}
```

---

### `POST /v1/enrollments/{id}/approve-unenroll`

Auth required: Yes (partner or authority role on this enrollment)

Approve a pending unenrollment request. Immediately completes unenrollment.

**Response 200:** `{ "status": "unenrolled" }`

---

## Partners

### `POST /v1/partners/invite`

Auth required: Yes

Send an accountability partner invitation by email.

**Request:**
```json
{ "email": "partner@example.com" }
```

**Response 202:** `{ "message": "Invitation sent." }`

---

### `GET /v1/partners`

Auth required: Yes

List accepted and pending partner relationships.

**Response 200:**
```json
{
  "data": [
    {
      "id": "01jwxyz...",
      "partner_account_id": "01jwxyz...",
      "partner_display_name": "Sam",
      "status": "accepted",
      "created_at": "2026-03-01T00:00:00Z"
    }
  ]
}
```

---

### `POST /v1/partners/{id}/accept`

Auth required: Yes

Accept an incoming partner invitation.

**Response 200:** Partner relationship object.

---

### `DELETE /v1/partners/{id}`

Auth required: Yes

Remove a partner relationship. Does not unenroll devices.

**Response 204:** No body.

---

## Organizations

Organizations group devices and partners (used by therapy practices, court programs, families).

### `POST /v1/organizations`

Auth required: Yes

**Request:**
```json
{
  "name": "Recovery Support Group",
  "description": "optional"
}
```

**Response 201:** Organization object with `id`.

---

### `GET /v1/organizations`

Auth required: Yes

List organizations the authenticated account belongs to.

---

### `GET /v1/organizations/{id}`

Auth required: Yes (member)

---

### `PATCH /v1/organizations/{id}`

Auth required: Yes (org admin)

Update name or description.

---

### `DELETE /v1/organizations/{id}`

Auth required: Yes (org owner)

---

### `POST /v1/organizations/{id}/members`

Auth required: Yes (org admin)

Invite a member by email.

**Request:**
```json
{ "email": "member@example.com", "role": "member" }
```

`role` values: `owner`, `admin`, `member`.

---

### `GET /v1/organizations/{id}/members`

Auth required: Yes (org member)

---

### `PATCH /v1/organizations/{id}/members/{member_id}`

Auth required: Yes (org admin)

Update a member's role.

---

### `DELETE /v1/organizations/{id}/members/{member_id}`

Auth required: Yes (org admin)

Remove a member.

---

### `POST /v1/organizations/{id}/devices`

Auth required: Yes (org admin)

Assign a device to the organization.

**Request:** `{ "device_id": "01jwxyz..." }`

---

### `GET /v1/organizations/{id}/devices`

Auth required: Yes (org member)

---

### `DELETE /v1/organizations/{id}/devices/{device_id}`

Auth required: Yes (org admin)

---

### `POST /v1/organizations/{id}/tokens`

Auth required: Yes (org admin)

Create an enrollment token tied to the organization (used for bulk device enrollment).

**Request:**
```json
{
  "label": "Staff laptop enrollment",
  "tier": "authority",
  "max_uses": 100
}
```

**Response 201:** `{ "id": "...", "token": "tok_...", "qr_url": "..." }`

---

### `GET /v1/organizations/{id}/tokens`

Auth required: Yes (org admin)

---

### `DELETE /v1/organizations/{id}/tokens/{token_id}`

Auth required: Yes (org admin)

Revoke a token immediately. Devices already enrolled remain enrolled.

---

### `GET /v1/organizations/{id}/tokens/{token_id}/qr`

Auth required: Yes (org admin)

Returns a QR code PNG for the enrollment token. Useful for printing or displaying on screen.

---

## Enrollment by Token

### `POST /v1/enroll/{token_public_id}`

Auth required: Yes

Redeem an organization enrollment token. Called by the agent during device setup as an alternative to the standard enrollment flow.

**Request:** Same as `POST /v1/devices` (device registration payload).

**Response 201:** Device and enrollment objects.

---

## Blocklist

### `GET /v1/blocklist/version`

Auth required: No

Returns the current blocklist version.

**Response 200:**
```json
{
  "version": 42,
  "entry_count": 14892,
  "updated_at": "2026-03-15T06:00:00Z",
  "signature": "base64-encoded-ed25519-signature"
}
```

---

### `GET /v1/blocklist/delta`

Auth required: No

Query params:
- `from_version` (required): integer, the client's current blocklist version

Returns only the changes since `from_version`. Agents call this on heartbeat when the version has changed.

**Response 200:**
```json
{
  "from_version": 41,
  "to_version": 42,
  "added": ["newgamblingsite.com", "..."],
  "removed": ["cleanedup.com"],
  "signature": "base64-encoded-ed25519-signature"
}
```

The signature covers the full serialised delta payload. Agents must verify this signature against the configured blocklist signing public key before applying updates.

---

### `POST /v1/blocklist/report`

Auth required: Yes

Submit a domain for blocklist review. Used by agents when heuristic matching flags an unknown domain.

**Request:**
```json
{
  "domain": "suspectdomain.com",
  "reason": "heuristic_match",
  "context": "optional additional detail"
}
```

**Response 202:** `{ "message": "Report queued for review." }`

---

## Analytics

All analytics endpoints respect enrollment-tier visibility rules. Partners see aggregated data unless the account has granted detailed access.

### `GET /v1/analytics/timeseries`

Auth required: Yes

Query params: `device_id`, `enrollment_id`, `from` (ISO 8601), `to` (ISO 8601), `interval` (`1h`, `1d`, `1w`)

**Response 200:**
```json
{
  "series": [
    { "timestamp": "2026-03-15T00:00:00Z", "block_count": 12, "tamper_alerts": 0 }
  ]
}
```

---

### `GET /v1/analytics/trends`

Auth required: Yes

Returns week-over-week and month-over-month change percentages.

**Response 200:**
```json
{
  "block_count_wow": -15.2,
  "block_count_mom": -42.0,
  "active_devices": 3
}
```

---

### `GET /v1/analytics/summary`

Auth required: Yes

Returns totals for the current account.

**Response 200:**
```json
{
  "total_blocks_all_time": 8420,
  "active_devices": 2,
  "enrollments_active": 2,
  "last_block_at": "2026-03-14T22:10:00Z"
}
```

---

### `GET /v1/analytics/heatmap`

Auth required: Yes

Returns block counts by hour-of-day and day-of-week for the requested period.

Query params: `from`, `to`, `device_id` (optional)

**Response 200:**
```json
{
  "heatmap": [
    { "day": 1, "hour": 22, "count": 7 }
  ]
}
```

---

### `GET /v1/analytics/export/csv`

Auth required: Yes

Download event data as CSV. Query params same as `timeseries`.

**Response 200:** `Content-Type: text/csv`

---

### `GET /v1/analytics/export/pdf`

Auth required: Yes

Download a summary report as PDF.

**Response 200:** `Content-Type: application/pdf`

---

### `GET /v1/analytics/org/{org_id}/summary`

Auth required: Yes (org member)

Aggregate summary across all devices in the organisation.

---

## Events

### `POST /v1/events`

Auth required: Yes

Batch ingest events from the agent. Agents buffer events locally and submit in batches.

**Request:**
```json
{
  "device_id": "01jwxyz...",
  "events": [
    {
      "event_type": "block",
      "domain": "gamblingsite.com",
      "timestamp": "2026-03-15T10:00:00Z",
      "layer": "dns"
    },
    {
      "event_type": "tamper_attempt",
      "detail": "dns_config_change",
      "timestamp": "2026-03-15T10:05:00Z"
    }
  ]
}
```

`event_type` values: `block`, `bypass_attempt`, `tamper_attempt`, `enrollment_change`, `heartbeat_missed`

**Response 202:** `{ "accepted": 2 }`

---

### `GET /v1/events`

Auth required: Yes

Query stored events.

Query params: `device_id`, `enrollment_id`, `event_type`, `from`, `to`, `page`, `per_page`

**Response 200:** Paginated list of event objects.

---

### `GET /v1/events/summary`

Auth required: Yes

Counts by event type for the current account.

**Response 200:**
```json
{
  "block": 8420,
  "tamper_attempt": 3,
  "bypass_attempt": 1
}
```

---

## Admin — Blocklist Management

All `/v1/admin/*` endpoints require an account with the `admin` role.

### `POST /v1/admin/blocklist/entries`

Create a blocklist entry.

**Request:**
```json
{
  "domain": "gamblingsite.com",
  "reason": "manual",
  "category": "sports_betting",
  "notes": "submitted via community report"
}
```

**Response 201:** Entry object with `id`.

---

### `GET /v1/admin/blocklist/entries`

List all blocklist entries. Query params: `category`, `reason`, `page`, `per_page`

---

### `PATCH /v1/admin/blocklist/entries/{id}`

Update a blocklist entry.

---

### `DELETE /v1/admin/blocklist/entries/{id}`

Remove a blocklist entry. Takes effect on next blocklist compilation.

**Response 204:** No body.

---

### `GET /v1/admin/blocklist/review-queue`

List domains pending human review (submitted via `POST /v1/blocklist/report` or federated reports).

---

### `POST /v1/admin/blocklist/review-queue/{domain}/resolve`

Accept or reject a domain from the review queue.

**Request:**
```json
{
  "action": "approve",
  "category": "casino",
  "notes": "verified gambling site"
}
```

`action` values: `approve`, `reject`

**Response 200:** `{ "status": "resolved" }`

---

## Admin — Review Queue

### `GET /v1/admin/review-queue`

List all items in the review queue.

---

### `POST /v1/admin/review-queue/bulk-approve`

**Request:** `{ "ids": ["01jwxyz...", "..."] }`

---

### `POST /v1/admin/review-queue/bulk-reject`

**Request:** `{ "ids": ["01jwxyz...", "..."] }`

---

### `GET /v1/admin/review-queue/{id}`

Get a single review queue item.

---

### `POST /v1/admin/review-queue/{id}/approve`

**Request:** `{ "notes": "optional" }`

---

### `POST /v1/admin/review-queue/{id}/reject`

**Request:** `{ "notes": "optional" }`

---

### `POST /v1/admin/review-queue/{id}/defer`

Move item to deferred for later review.

**Request:** `{ "defer_until": "2026-03-22T00:00:00Z", "notes": "optional" }`

---

## Admin — App Signatures

App signatures are the database of gambling app identifiers (package names, bundle IDs, code signing certificates) used by the application blocking layer.

### `POST /v1/admin/app-signatures`

**Request:**
```json
{
  "platform": "android",
  "identifier": "com.gambling.app",
  "identifier_type": "package_name",
  "display_name": "Gambling App",
  "category": "casino"
}
```

---

### `GET /v1/admin/app-signatures`

List app signatures. Query params: `platform`, `category`, `page`, `per_page`

---

### `GET /v1/admin/app-signatures/{id}`

---

### `PUT /v1/admin/app-signatures/{id}`

Full replacement of a signature record.

---

### `DELETE /v1/admin/app-signatures/{id}`

**Response 204:** No body.

---

## Federated

### `POST /v1/federated/reports`

Auth required: No (source IP is stripped before processing)

Submit domain reports from a self-hosted instance back to the central community feed (opt-in). Only called by the worker when `BETBLOCKER_FEDERATED_REPORT_UPSTREAM` is configured.

**Request:**
```json
{
  "api_key": "your-api-key",
  "reports": [
    {
      "domain": "unknownsite.com",
      "report_type": "heuristic_match",
      "confidence": 0.87
    }
  ]
}
```

**Response 202:** `{ "accepted": 1 }`

---

## Tor Exit Nodes

### `GET /v1/tor-exits`

Auth required: No

Returns the current list of known Tor exit node IP addresses, updated automatically by the worker. Used by agents for bypass detection.

**Response 200:**
```json
{
  "updated_at": "2026-03-15T04:00:00Z",
  "exits": ["185.220.101.0", "..."]
}
```

---

## Billing (Hosted Only)

These endpoints are only available when `BB_BILLING_ENABLED=true`. They are disabled on all self-hosted instances.

### `POST /v1/billing/subscribe`

Start a Stripe subscription.

### `GET /v1/billing/status`

Returns current subscription status.

### `POST /v1/billing/webhook`

Stripe webhook receiver. Must be registered in the Stripe dashboard.

### `POST /v1/billing/cancel`

Cancel the current subscription.
