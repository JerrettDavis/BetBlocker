# BetBlocker Central API Specification

**Version:** 1.0.0-draft
**Date:** 2026-03-12
**Status:** Draft
**Base URL:** `https://api.betblocker.com/v1` (hosted) or `https://<self-hosted>/v1`
**Protocol:** HTTPS only (TLS 1.3 minimum)

---

## Table of Contents

1. [Conventions](#1-conventions)
2. [Authentication Scheme](#2-authentication-scheme)
3. [Shared Data Models](#3-shared-data-models)
4. [Endpoint Groups](#4-endpoint-groups)
   - 4.1 [Authentication](#41-authentication)
   - 4.2 [Accounts](#42-accounts)
   - 4.3 [Devices](#43-devices)
   - 4.4 [Enrollments](#44-enrollments)
   - 4.5 [Blocklist](#45-blocklist)
   - 4.6 [Events](#46-events)
   - 4.7 [Organizations](#47-organizations-phase-2)
   - 4.8 [Billing](#48-billing-hosted-only)
   - 4.9 [Partners](#49-partners)
5. [Error Handling](#5-error-handling)
6. [Rate Limiting](#6-rate-limiting)
7. [Pagination](#7-pagination)
8. [Versioning](#8-versioning)

---

## 1. Conventions

### Request Format

- All request bodies are JSON (`Content-Type: application/json`).
- Path parameters use snake_case: `/devices/:device_id`.
- Query parameters use snake_case: `?from_version=42`.
- All timestamps are ISO 8601 in UTC: `2026-03-12T14:30:00Z`.
- UUIDs are v7 (time-ordered) unless otherwise noted.

### Response Envelope

Every successful response uses a consistent envelope:

```json
{
  "data": { ... },
  "meta": {
    "request_id": "req_abc123",
    "timestamp": "2026-03-12T14:30:00Z"
  }
}
```

Paginated list responses:

```json
{
  "data": [ ... ],
  "meta": {
    "request_id": "req_abc123",
    "timestamp": "2026-03-12T14:30:00Z"
  },
  "pagination": {
    "total": 142,
    "page": 1,
    "per_page": 50,
    "total_pages": 3
  }
}
```

### Error Envelope

```json
{
  "error": {
    "code": "ENROLLMENT_NOT_FOUND",
    "message": "No enrollment found with the given ID.",
    "details": { ... }
  },
  "meta": {
    "request_id": "req_abc123",
    "timestamp": "2026-03-12T14:30:00Z"
  }
}
```

### Auth Header Conventions

| Auth Type | Header |
|-----------|--------|
| User JWT | `Authorization: Bearer <jwt>` |
| Device mTLS | Mutual TLS client certificate (certificate CN = device ID) |
| Device Token | `X-Device-Token: <token>` (fallback for platforms without mTLS) |
| Admin JWT | `Authorization: Bearer <jwt>` (JWT contains `role: admin` claim) |

---

## 2. Authentication Scheme

### JWT Structure

Access tokens are short-lived (15 minutes). Refresh tokens are long-lived (30 days) and stored hashed in the database.

**Access Token Claims:**

```json
{
  "sub": "acc_uuid",
  "email": "user@example.com",
  "role": "user | partner | authority | admin",
  "iat": 1710254400,
  "exp": 1710255300,
  "jti": "tok_uuid"
}
```

**Refresh Token:** Opaque 256-bit random string, stored as SHA-256 hash in PostgreSQL. Bound to a specific user agent and IP prefix for rotation detection.

### Device Authentication

Devices authenticate via one of two mechanisms:

1. **mTLS (preferred):** Device presents a client certificate issued during registration. Certificate CN contains the device ID. Certificate is signed by the BetBlocker device CA.
2. **Device Token (fallback):** For platforms where mTLS is impractical (some mobile contexts), a long-lived opaque token is issued at registration and stored in hardware-backed keystore. Sent via `X-Device-Token` header.

Both mechanisms bind the device identity to the request. The API validates device ownership against the authenticated account (for user-initiated requests) or directly against the device record (for agent-initiated requests like heartbeat).

---

## 3. Shared Data Models

### 3.1 Account

```json
{
  "id": "acc_01H...",
  "email": "user@example.com",
  "display_name": "Jane Doe",
  "role": "user",
  "email_verified": true,
  "mfa_enabled": false,
  "timezone": "America/New_York",
  "locale": "en-US",
  "organization_id": null,
  "subscription_tier": "standard",
  "created_at": "2026-03-12T14:30:00Z",
  "updated_at": "2026-03-12T14:30:00Z"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | `string (UUID)` | Unique account identifier, prefixed `acc_` |
| `email` | `string` | Unique email address |
| `display_name` | `string` | User-chosen display name (2-100 chars) |
| `role` | `enum` | One of: `user`, `partner`, `authority`, `admin` |
| `email_verified` | `boolean` | Whether email has been verified |
| `mfa_enabled` | `boolean` | Whether MFA is configured |
| `timezone` | `string` | IANA timezone identifier |
| `locale` | `string` | BCP 47 locale tag |
| `organization_id` | `string (UUID) \| null` | Associated organization, if any |
| `subscription_tier` | `enum` | One of: `free`, `standard`, `partner_tier`, `institutional` |
| `created_at` | `string (datetime)` | Account creation timestamp |
| `updated_at` | `string (datetime)` | Last modification timestamp |

### 3.2 Device

```json
{
  "id": "dev_01H...",
  "account_id": "acc_01H...",
  "name": "Jane's MacBook Pro",
  "platform": "macos",
  "os_version": "15.3.1",
  "agent_version": "1.2.0",
  "hostname": "janes-mbp.local",
  "status": "active",
  "blocklist_version": 1247,
  "last_heartbeat_at": "2026-03-12T14:25:00Z",
  "certificate_fingerprint": "sha256:ab12cd34...",
  "enrollment_id": "enr_01H...",
  "created_at": "2026-03-12T10:00:00Z",
  "updated_at": "2026-03-12T14:25:00Z"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | `string (UUID)` | Unique device identifier, prefixed `dev_` |
| `account_id` | `string (UUID)` | Owning account |
| `name` | `string` | User-friendly device name (1-100 chars) |
| `platform` | `enum` | One of: `windows`, `macos`, `linux`, `android`, `ios` |
| `os_version` | `string` | OS version string |
| `agent_version` | `string` | Semver of installed agent |
| `hostname` | `string` | Device hostname |
| `status` | `enum` | One of: `pending`, `active`, `offline`, `unenrolling`, `unenrolled` |
| `blocklist_version` | `integer` | Latest blocklist version confirmed by device |
| `last_heartbeat_at` | `string (datetime) \| null` | Last successful heartbeat |
| `certificate_fingerprint` | `string \| null` | SHA-256 fingerprint of device mTLS cert |
| `enrollment_id` | `string (UUID) \| null` | Currently active enrollment |
| `created_at` | `string (datetime)` | Registration timestamp |
| `updated_at` | `string (datetime)` | Last modification timestamp |

### 3.3 Enrollment

```json
{
  "id": "enr_01H...",
  "device_id": "dev_01H...",
  "account_id": "acc_01H...",
  "enrolled_by": "acc_01H...",
  "tier": "partner",
  "status": "active",
  "protection_config": {
    "dns_blocking": true,
    "app_blocking": true,
    "browser_blocking": false,
    "vpn_detection": "alert",
    "tamper_response": "alert_partner"
  },
  "reporting_config": {
    "level": "aggregated",
    "blocked_attempt_counts": true,
    "domain_details": false,
    "tamper_alerts": true
  },
  "unenrollment_policy": {
    "type": "partner_approval",
    "cooldown_hours": null,
    "requires_approval_from": "acc_01H..."
  },
  "unenrollment_request": null,
  "created_at": "2026-03-12T10:00:00Z",
  "updated_at": "2026-03-12T10:00:00Z",
  "expires_at": null
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | `string (UUID)` | Unique enrollment identifier, prefixed `enr_` |
| `device_id` | `string (UUID)` | The enrolled device |
| `account_id` | `string (UUID)` | The device owner |
| `enrolled_by` | `string (UUID)` | Account that created the enrollment (self, partner, or authority) |
| `tier` | `enum` | One of: `self`, `partner`, `authority` |
| `status` | `enum` | One of: `pending`, `active`, `unenroll_requested`, `unenroll_approved`, `unenrolling`, `unenrolled`, `expired` |
| `protection_config` | `ProtectionConfig` | What blocking layers are active and how bypass attempts are handled |
| `reporting_config` | `ReportingConfig` | What data is visible and to whom |
| `unenrollment_policy` | `UnenrollmentPolicy` | Rules governing how unenrollment works |
| `unenrollment_request` | `UnenrollmentRequest \| null` | Present when unenrollment has been requested |
| `created_at` | `string (datetime)` | Enrollment creation timestamp |
| `updated_at` | `string (datetime)` | Last modification timestamp |
| `expires_at` | `string (datetime) \| null` | Optional expiration (authority tier may set) |

**ProtectionConfig:**

| Field | Type | Description |
|-------|------|-------------|
| `dns_blocking` | `boolean` | DNS/network layer active |
| `app_blocking` | `boolean` | Application blocking active (Phase 2) |
| `browser_blocking` | `boolean` | Browser content blocking active (Phase 3) |
| `vpn_detection` | `enum` | `disabled`, `log`, `alert`, `lockdown` |
| `tamper_response` | `enum` | `log`, `alert_user`, `alert_partner`, `alert_authority` |

**ReportingConfig:**

| Field | Type | Description |
|-------|------|-------------|
| `level` | `enum` | `none`, `aggregated`, `detailed`, `full_audit` |
| `blocked_attempt_counts` | `boolean` | Include count of blocked attempts |
| `domain_details` | `boolean` | Include specific blocked domains |
| `tamper_alerts` | `boolean` | Report tamper detection events |

**UnenrollmentPolicy:**

| Field | Type | Description |
|-------|------|-------------|
| `type` | `enum` | `time_delayed`, `partner_approval`, `authority_approval` |
| `cooldown_hours` | `integer \| null` | Hours to wait before completing unenrollment (self tier, 24-72) |
| `requires_approval_from` | `string (UUID) \| null` | Account that must approve (partner/authority tiers) |

**UnenrollmentRequest:**

| Field | Type | Description |
|-------|------|-------------|
| `requested_at` | `string (datetime)` | When unenrollment was requested |
| `requested_by` | `string (UUID)` | Who requested it |
| `reason` | `string \| null` | Optional reason |
| `eligible_at` | `string (datetime) \| null` | When time-delayed unenrollment completes |
| `approved_at` | `string (datetime) \| null` | When approval was granted |
| `approved_by` | `string (UUID) \| null` | Who approved |

### 3.4 Event

```json
{
  "id": "evt_01H...",
  "device_id": "dev_01H...",
  "enrollment_id": "enr_01H...",
  "type": "block",
  "category": "dns",
  "severity": "info",
  "payload": {
    "domain": "example-casino.com",
    "query_type": "A",
    "source_app": "com.google.chrome",
    "blocklist_rule_id": "blk_01H..."
  },
  "occurred_at": "2026-03-12T14:30:00Z",
  "received_at": "2026-03-12T14:30:01Z"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | `string (UUID)` | Unique event identifier, prefixed `evt_` |
| `device_id` | `string (UUID)` | Source device |
| `enrollment_id` | `string (UUID)` | Associated enrollment |
| `type` | `enum` | See Event Types below |
| `category` | `enum` | `dns`, `app`, `browser`, `tamper`, `enrollment`, `heartbeat`, `system` |
| `severity` | `enum` | `info`, `warning`, `critical` |
| `payload` | `object` | Type-specific structured data |
| `occurred_at` | `string (datetime)` | When event occurred on device |
| `received_at` | `string (datetime)` | When API received event |

**Event Types:**

| Type | Description | Category |
|------|-------------|----------|
| `block` | Gambling domain/app/content blocked | `dns`, `app`, `browser` |
| `bypass_attempt` | User attempted to bypass blocking | `dns`, `app`, `browser` |
| `tamper_detected` | Agent tampering detected | `tamper` |
| `tamper_self_healed` | Agent recovered from tamper attempt | `tamper` |
| `vpn_detected` | VPN/proxy/Tor activity detected | `dns` |
| `enrollment_created` | New enrollment activated | `enrollment` |
| `enrollment_modified` | Enrollment config changed | `enrollment` |
| `unenroll_requested` | Unenrollment requested | `enrollment` |
| `unenroll_completed` | Unenrollment completed | `enrollment` |
| `heartbeat` | Periodic status report | `heartbeat` |
| `agent_started` | Agent process started | `system` |
| `agent_updated` | Agent updated to new version | `system` |
| `blocklist_updated` | Blocklist synced to new version | `system` |

### 3.5 BlocklistEntry

```json
{
  "id": "blk_01H...",
  "domain": "example-casino.com",
  "pattern": null,
  "category": "online_casino",
  "source": "curated",
  "confidence": 1.0,
  "status": "active",
  "added_by": "acc_01H...",
  "reviewed_by": "acc_01H...",
  "evidence_url": "https://...",
  "tags": ["casino", "slots", "uk-licensed"],
  "blocklist_version_added": 1200,
  "blocklist_version_removed": null,
  "created_at": "2026-03-12T10:00:00Z",
  "updated_at": "2026-03-12T10:00:00Z"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | `string (UUID)` | Unique entry identifier, prefixed `blk_` |
| `domain` | `string \| null` | Exact domain to block (mutually exclusive with `pattern`) |
| `pattern` | `string \| null` | Glob/regex pattern for wildcard blocking |
| `category` | `enum` | `online_casino`, `sports_betting`, `poker`, `lottery`, `bingo`, `fantasy_sports`, `crypto_gambling`, `affiliate`, `payment_processor`, `other` |
| `source` | `enum` | `curated` (manual), `automated` (discovery pipeline), `federated` (agent report), `community` (public list import) |
| `confidence` | `float` | 0.0-1.0 confidence score. Curated entries are 1.0. |
| `status` | `enum` | `pending_review`, `active`, `inactive`, `rejected` |
| `added_by` | `string (UUID) \| null` | Account that added the entry |
| `reviewed_by` | `string (UUID) \| null` | Account that reviewed/approved |
| `evidence_url` | `string \| null` | URL to evidence supporting the classification |
| `tags` | `string[]` | Freeform classification tags |
| `blocklist_version_added` | `integer` | Blocklist version when entry was activated |
| `blocklist_version_removed` | `integer \| null` | Blocklist version when entry was deactivated |
| `created_at` | `string (datetime)` | Creation timestamp |
| `updated_at` | `string (datetime)` | Last modification timestamp |

### 3.6 Organization

```json
{
  "id": "org_01H...",
  "name": "Recovery Center of Austin",
  "type": "therapy_practice",
  "owner_id": "acc_01H...",
  "member_count": 12,
  "device_count": 34,
  "settings": {
    "default_enrollment_tier": "authority",
    "default_protection_config": { ... },
    "default_reporting_config": { ... }
  },
  "created_at": "2026-03-12T10:00:00Z",
  "updated_at": "2026-03-12T10:00:00Z"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | `string (UUID)` | Unique org identifier, prefixed `org_` |
| `name` | `string` | Organization display name (2-200 chars) |
| `type` | `enum` | `therapy_practice`, `court_program`, `family`, `employer`, `other` |
| `owner_id` | `string (UUID)` | Account that owns the org |
| `member_count` | `integer` | Number of member accounts |
| `device_count` | `integer` | Number of devices under org enrollments |
| `settings` | `OrgSettings` | Default config for enrollments created through this org |
| `created_at` | `string (datetime)` | Creation timestamp |
| `updated_at` | `string (datetime)` | Last modification timestamp |

### 3.7 Partner

```json
{
  "id": "ptr_01H...",
  "account_id": "acc_01H...",
  "partner_account_id": "acc_01H...",
  "status": "active",
  "role": "accountability_partner",
  "permissions": {
    "view_reports": true,
    "approve_unenrollment": true,
    "modify_enrollment": false
  },
  "invited_by": "acc_01H...",
  "invited_at": "2026-03-12T10:00:00Z",
  "accepted_at": "2026-03-12T11:00:00Z",
  "created_at": "2026-03-12T10:00:00Z",
  "updated_at": "2026-03-12T11:00:00Z"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | `string (UUID)` | Unique partner relationship identifier, prefixed `ptr_` |
| `account_id` | `string (UUID)` | The user who has the partner |
| `partner_account_id` | `string (UUID)` | The partner's account |
| `status` | `enum` | `pending`, `active`, `revoked` |
| `role` | `enum` | `accountability_partner`, `therapist`, `authority_rep` |
| `permissions` | `PartnerPermissions` | What the partner can do |
| `invited_by` | `string (UUID)` | Who initiated the relationship |
| `invited_at` | `string (datetime)` | Invitation timestamp |
| `accepted_at` | `string (datetime) \| null` | Acceptance timestamp |
| `created_at` | `string (datetime)` | Creation timestamp |
| `updated_at` | `string (datetime)` | Last modification timestamp |

**PartnerPermissions:**

| Field | Type | Description |
|-------|------|-------------|
| `view_reports` | `boolean` | Can view enrollment reports |
| `approve_unenrollment` | `boolean` | Can approve unenrollment requests |
| `modify_enrollment` | `boolean` | Can modify enrollment protection/reporting config |

---

## 4. Endpoint Groups

### 4.1 Authentication

All auth endpoints are unauthenticated (no JWT required) unless noted.

---

#### POST /auth/register

Create a new account.

**Auth:** None

**Request Body:**

```json
{
  "email": "user@example.com",
  "password": "strongPassw0rd!",
  "display_name": "Jane Doe",
  "timezone": "America/New_York",
  "locale": "en-US"
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `email` | `string` | Yes | Valid email, max 255 chars, unique |
| `password` | `string` | Yes | Min 12 chars, must contain uppercase, lowercase, digit, and special char |
| `display_name` | `string` | Yes | 2-100 chars |
| `timezone` | `string` | No | Valid IANA timezone. Default: `UTC` |
| `locale` | `string` | No | Valid BCP 47 tag. Default: `en-US` |

**Response: `201 Created`**

```json
{
  "data": {
    "account": {
      "id": "acc_01H...",
      "email": "user@example.com",
      "display_name": "Jane Doe",
      "role": "user",
      "email_verified": false,
      "created_at": "2026-03-12T14:30:00Z"
    },
    "access_token": "eyJ...",
    "refresh_token": "rtk_...",
    "expires_in": 900
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 400 | `VALIDATION_ERROR` | Invalid input (details include field-level errors) |
| 409 | `EMAIL_ALREADY_EXISTS` | An account with this email already exists |
| 429 | `RATE_LIMIT_EXCEEDED` | Too many registration attempts |

**Rate Limit:** 5 requests per IP per hour.

**Notes:** A verification email is sent asynchronously. The account is functional immediately but certain features (partner invitations) require verified email.

---

#### POST /auth/login

Authenticate and receive tokens.

**Auth:** None

**Request Body:**

```json
{
  "email": "user@example.com",
  "password": "strongPassw0rd!",
  "mfa_code": "123456"
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `email` | `string` | Yes | Valid email |
| `password` | `string` | Yes | Non-empty |
| `mfa_code` | `string` | Conditional | Required if account has MFA enabled. 6-digit TOTP code. |

**Response: `200 OK`**

```json
{
  "data": {
    "account": {
      "id": "acc_01H...",
      "email": "user@example.com",
      "display_name": "Jane Doe",
      "role": "user",
      "email_verified": true,
      "mfa_enabled": false
    },
    "access_token": "eyJ...",
    "refresh_token": "rtk_...",
    "expires_in": 900
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `INVALID_CREDENTIALS` | Email or password is incorrect |
| 401 | `MFA_REQUIRED` | Account has MFA enabled; `mfa_code` must be provided |
| 401 | `MFA_INVALID` | The provided MFA code is incorrect or expired |
| 403 | `ACCOUNT_LOCKED` | Too many failed attempts; account temporarily locked |
| 429 | `RATE_LIMIT_EXCEEDED` | Too many login attempts |

**Rate Limit:** 10 requests per email per 15 minutes. 30 requests per IP per 15 minutes. After 5 consecutive failures for an email, lock account for 15 minutes.

**Notes:** The error response for `INVALID_CREDENTIALS` intentionally does not distinguish between "email not found" and "wrong password" to prevent user enumeration.

---

#### POST /auth/refresh

Exchange a valid refresh token for a new access token. Implements refresh token rotation: the old refresh token is invalidated and a new one is issued.

**Auth:** None (refresh token in body)

**Request Body:**

```json
{
  "refresh_token": "rtk_..."
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `refresh_token` | `string` | Yes | Non-empty |

**Response: `200 OK`**

```json
{
  "data": {
    "access_token": "eyJ...",
    "refresh_token": "rtk_...",
    "expires_in": 900
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `INVALID_REFRESH_TOKEN` | Token is invalid, expired, or already used |
| 401 | `TOKEN_FAMILY_REVOKED` | Reuse of a rotated token detected; entire token family revoked (potential theft) |

**Rate Limit:** 20 requests per account per hour.

**Notes:** If a previously rotated refresh token is reused, this indicates potential token theft. The API revokes the entire token family (all refresh tokens for that account) and returns `TOKEN_FAMILY_REVOKED`, forcing re-authentication.

---

#### POST /auth/logout

Revoke the current refresh token.

**Auth:** User JWT

**Request Body:**

```json
{
  "refresh_token": "rtk_..."
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `refresh_token` | `string` | Yes | Non-empty |

**Response: `204 No Content`**

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `UNAUTHORIZED` | Invalid or expired access token |

**Rate Limit:** Standard (see section 6).

---

#### POST /auth/forgot-password

Request a password reset email.

**Auth:** None

**Request Body:**

```json
{
  "email": "user@example.com"
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `email` | `string` | Yes | Valid email |

**Response: `202 Accepted`**

```json
{
  "data": {
    "message": "If an account with that email exists, a reset link has been sent."
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 429 | `RATE_LIMIT_EXCEEDED` | Too many reset requests |

**Rate Limit:** 3 requests per email per hour. 10 requests per IP per hour.

**Notes:** Always returns 202 regardless of whether the email exists, to prevent user enumeration. The reset token is a 256-bit random value, valid for 1 hour, stored as SHA-256 hash in the database.

---

#### POST /auth/reset-password

Reset password using a token received via email.

**Auth:** None

**Request Body:**

```json
{
  "token": "rst_...",
  "new_password": "newStrongPassw0rd!"
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `token` | `string` | Yes | Non-empty |
| `new_password` | `string` | Yes | Same password requirements as registration |

**Response: `200 OK`**

```json
{
  "data": {
    "message": "Password has been reset. Please log in with your new password."
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 400 | `VALIDATION_ERROR` | New password does not meet requirements |
| 401 | `INVALID_RESET_TOKEN` | Token is invalid, expired, or already used |
| 429 | `RATE_LIMIT_EXCEEDED` | Too many reset attempts |

**Rate Limit:** 5 requests per IP per hour.

**Notes:** On successful reset, all existing refresh tokens for the account are revoked (forces re-login on all sessions). The reset token is single-use.

---

### 4.2 Accounts

---

#### GET /accounts/me

Get the currently authenticated user's full profile.

**Auth:** User JWT

**Response: `200 OK`**

```json
{
  "data": {
    "id": "acc_01H...",
    "email": "user@example.com",
    "display_name": "Jane Doe",
    "role": "user",
    "email_verified": true,
    "mfa_enabled": false,
    "timezone": "America/New_York",
    "locale": "en-US",
    "organization_id": null,
    "subscription_tier": "standard",
    "created_at": "2026-03-12T14:30:00Z",
    "updated_at": "2026-03-12T14:30:00Z"
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `UNAUTHORIZED` | Invalid or expired access token |

**Rate Limit:** Standard.

---

#### PATCH /accounts/me

Update the currently authenticated user's profile. Partial update: only provided fields are changed.

**Auth:** User JWT

**Request Body:**

```json
{
  "display_name": "Jane D.",
  "timezone": "Europe/London",
  "locale": "en-GB",
  "current_password": "oldPassw0rd!",
  "new_password": "newStrongPassw0rd!"
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `display_name` | `string` | No | 2-100 chars |
| `timezone` | `string` | No | Valid IANA timezone |
| `locale` | `string` | No | Valid BCP 47 tag |
| `current_password` | `string` | Conditional | Required when changing password or email |
| `new_password` | `string` | No | Same requirements as registration |
| `email` | `string` | No | Valid email, unique. Requires `current_password`. Triggers re-verification. |

**Response: `200 OK`**

```json
{
  "data": {
    "id": "acc_01H...",
    "email": "user@example.com",
    "display_name": "Jane D.",
    "role": "user",
    "email_verified": true,
    "mfa_enabled": false,
    "timezone": "Europe/London",
    "locale": "en-GB",
    "organization_id": null,
    "subscription_tier": "standard",
    "created_at": "2026-03-12T14:30:00Z",
    "updated_at": "2026-03-12T15:00:00Z"
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 400 | `VALIDATION_ERROR` | Invalid input |
| 401 | `UNAUTHORIZED` | Invalid or expired access token |
| 401 | `INCORRECT_PASSWORD` | `current_password` is wrong |
| 409 | `EMAIL_ALREADY_EXISTS` | The new email is already in use |

**Rate Limit:** 10 requests per account per hour.

---

#### GET /accounts/:id

View another account's profile. Only accessible to partners who have an active relationship with the target account, authority representatives with active enrollment oversight, or admins.

**Auth:** User JWT (partner, authority, or admin role)

**Path Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `id` | `string (UUID)` | Target account ID |

**Response: `200 OK`**

Returns a filtered view of the Account model. Partners see `id`, `display_name`, `email_verified`, and `created_at`. Authority representatives see the same plus `email`. Admins see the full model.

```json
{
  "data": {
    "id": "acc_01H...",
    "display_name": "John Smith",
    "email_verified": true,
    "created_at": "2026-03-12T14:30:00Z"
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `UNAUTHORIZED` | Invalid or expired access token |
| 403 | `FORBIDDEN` | No active partner/authority relationship with this account |
| 404 | `ACCOUNT_NOT_FOUND` | Account does not exist |

**Rate Limit:** Standard.

---

### 4.3 Devices

---

#### POST /devices

Register a new device. Called by the agent during initial setup.

**Auth:** User JWT

**Request Body:**

```json
{
  "name": "Jane's MacBook Pro",
  "platform": "macos",
  "os_version": "15.3.1",
  "agent_version": "1.2.0",
  "hostname": "janes-mbp.local",
  "hardware_id": "hw_sha256_...",
  "csr": "-----BEGIN CERTIFICATE REQUEST-----\n..."
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `name` | `string` | Yes | 1-100 chars |
| `platform` | `enum` | Yes | One of: `windows`, `macos`, `linux`, `android`, `ios` |
| `os_version` | `string` | Yes | 1-50 chars |
| `agent_version` | `string` | Yes | Valid semver |
| `hostname` | `string` | Yes | 1-255 chars |
| `hardware_id` | `string` | Yes | SHA-256 hash of hardware identifiers (prevents duplicate registrations) |
| `csr` | `string` | No | PEM-encoded certificate signing request for mTLS. If omitted, a device token is issued instead. |

**Response: `201 Created`**

```json
{
  "data": {
    "device": {
      "id": "dev_01H...",
      "account_id": "acc_01H...",
      "name": "Jane's MacBook Pro",
      "platform": "macos",
      "os_version": "15.3.1",
      "agent_version": "1.2.0",
      "hostname": "janes-mbp.local",
      "status": "pending",
      "enrollment_id": null,
      "created_at": "2026-03-12T14:30:00Z"
    },
    "certificate": "-----BEGIN CERTIFICATE-----\n...",
    "device_token": null,
    "api_endpoints": {
      "heartbeat": "/v1/devices/dev_01H.../heartbeat",
      "config": "/v1/devices/dev_01H.../config",
      "events": "/v1/events",
      "blocklist": "/v1/blocklist"
    }
  }
}
```

If `csr` was provided, `certificate` contains the signed device certificate and `device_token` is null. If `csr` was omitted, `certificate` is null and `device_token` contains the opaque token.

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 400 | `VALIDATION_ERROR` | Invalid input |
| 401 | `UNAUTHORIZED` | Invalid or expired access token |
| 409 | `DEVICE_ALREADY_REGISTERED` | A device with this `hardware_id` is already registered to this account |
| 422 | `INVALID_CSR` | The CSR is malformed or contains invalid data |

**Rate Limit:** 5 devices per account per hour.

---

#### GET /devices

List all devices for the authenticated user. Partners and authorities can also see devices they have enrollment oversight for.

**Auth:** User JWT

**Query Parameters:**

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `status` | `enum` | (all) | Filter by status: `pending`, `active`, `offline`, `unenrolling`, `unenrolled` |
| `platform` | `enum` | (all) | Filter by platform |
| `page` | `integer` | 1 | Page number |
| `per_page` | `integer` | 50 | Items per page (max 100) |

**Response: `200 OK`**

```json
{
  "data": [
    {
      "id": "dev_01H...",
      "account_id": "acc_01H...",
      "name": "Jane's MacBook Pro",
      "platform": "macos",
      "status": "active",
      "agent_version": "1.2.0",
      "blocklist_version": 1247,
      "last_heartbeat_at": "2026-03-12T14:25:00Z",
      "enrollment_id": "enr_01H...",
      "created_at": "2026-03-12T10:00:00Z"
    }
  ],
  "pagination": {
    "total": 3,
    "page": 1,
    "per_page": 50,
    "total_pages": 1
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `UNAUTHORIZED` | Invalid or expired access token |

**Rate Limit:** Standard.

---

#### GET /devices/:id

Get full detail for a specific device.

**Auth:** User JWT (device owner, partner with relationship, authority with enrollment oversight, or admin)

**Path Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `id` | `string (UUID)` | Device ID |

**Response: `200 OK`**

Returns the full Device model as described in section 3.2.

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `UNAUTHORIZED` | Invalid or expired access token |
| 403 | `FORBIDDEN` | Not authorized to view this device |
| 404 | `DEVICE_NOT_FOUND` | Device does not exist |

**Rate Limit:** Standard.

---

#### DELETE /devices/:id

Begin device unenrollment. This does not immediately delete the device; it triggers the enrollment's unenrollment policy (time delay or approval). The device transitions to `unenrolling` status.

**Auth:** User JWT (device owner or admin)

**Path Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `id` | `string (UUID)` | Device ID |

**Request Body (optional):**

```json
{
  "reason": "Switching to a new device."
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `reason` | `string` | No | Max 500 chars |

**Response: `200 OK`**

```json
{
  "data": {
    "device": {
      "id": "dev_01H...",
      "status": "unenrolling"
    },
    "unenrollment": {
      "type": "time_delayed",
      "eligible_at": "2026-03-14T14:30:00Z",
      "message": "Unenrollment will complete after 48-hour cooling-off period."
    }
  }
}
```

For partner/authority enrollments:

```json
{
  "data": {
    "device": {
      "id": "dev_01H...",
      "status": "unenrolling"
    },
    "unenrollment": {
      "type": "partner_approval",
      "requires_approval_from": "acc_01H...",
      "message": "Your accountability partner has been notified and must approve this request."
    }
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `UNAUTHORIZED` | Invalid or expired access token |
| 403 | `FORBIDDEN` | Not authorized to unenroll this device |
| 404 | `DEVICE_NOT_FOUND` | Device does not exist |
| 409 | `ALREADY_UNENROLLING` | An unenrollment request is already in progress |
| 409 | `NO_ACTIVE_ENROLLMENT` | Device has no active enrollment |

**Rate Limit:** 3 requests per device per day.

---

#### POST /devices/:id/heartbeat

Agent sends periodic heartbeat with status information. Used for device health monitoring and dead-man's switch alerting.

**Auth:** Device cert or device token

**Path Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `id` | `string (UUID)` | Device ID (must match authenticated device) |

**Request Body:**

```json
{
  "agent_version": "1.2.0",
  "os_version": "15.3.1",
  "blocklist_version": 1247,
  "uptime_seconds": 86400,
  "blocking_active": true,
  "integrity_check": {
    "binary_hash": "sha256:...",
    "config_hash": "sha256:...",
    "valid": true
  },
  "stats": {
    "blocks_since_last_heartbeat": 14,
    "dns_queries_since_last_heartbeat": 4821
  }
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `agent_version` | `string` | Yes | Valid semver |
| `os_version` | `string` | Yes | 1-50 chars |
| `blocklist_version` | `integer` | Yes | Non-negative |
| `uptime_seconds` | `integer` | Yes | Non-negative |
| `blocking_active` | `boolean` | Yes | Whether all blocking layers are functioning |
| `integrity_check` | `IntegrityCheck` | Yes | Agent self-integrity verification |
| `stats` | `HeartbeatStats` | No | Optional runtime statistics |

**Response: `200 OK`**

```json
{
  "data": {
    "ack": true,
    "server_time": "2026-03-12T14:30:00Z",
    "next_heartbeat_seconds": 300,
    "commands": [
      {
        "type": "update_blocklist",
        "params": { "target_version": 1250 }
      }
    ]
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `ack` | `boolean` | Heartbeat acknowledged |
| `server_time` | `string (datetime)` | Server timestamp for clock drift detection |
| `next_heartbeat_seconds` | `integer` | Recommended interval before next heartbeat |
| `commands` | `Command[]` | Pending commands for the agent to execute |

**Command Types:**

| Type | Description |
|------|-------------|
| `update_blocklist` | Agent should sync to target blocklist version |
| `update_agent` | Agent update available |
| `refresh_config` | Enrollment config has changed; agent should fetch `/config` |
| `revoke_certificate` | Device certificate has been revoked; agent should re-register |

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `DEVICE_UNAUTHORIZED` | Invalid device certificate or token |
| 403 | `DEVICE_ID_MISMATCH` | Authenticated device does not match path parameter |
| 404 | `DEVICE_NOT_FOUND` | Device does not exist |

**Rate Limit:** 120 requests per device per hour (minimum interval ~30 seconds).

---

#### GET /devices/:id/config

Get the full active configuration for a device, including enrollment settings, protection config, and reporting config. Called by the agent on startup and when instructed via heartbeat command.

**Auth:** Device cert or device token

**Path Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `id` | `string (UUID)` | Device ID (must match authenticated device) |

**Response: `200 OK`**

```json
{
  "data": {
    "device_id": "dev_01H...",
    "enrollment": {
      "id": "enr_01H...",
      "tier": "partner",
      "status": "active",
      "protection_config": {
        "dns_blocking": true,
        "app_blocking": true,
        "browser_blocking": false,
        "vpn_detection": "alert",
        "tamper_response": "alert_partner"
      },
      "reporting_config": {
        "level": "aggregated",
        "blocked_attempt_counts": true,
        "domain_details": false,
        "tamper_alerts": true
      }
    },
    "blocklist": {
      "current_version": 1250,
      "download_url": "/v1/blocklist/delta?from_version=1247"
    },
    "heartbeat": {
      "interval_seconds": 300,
      "missed_threshold": 3
    },
    "agent_update": {
      "latest_version": "1.3.0",
      "download_url": "https://cdn.betblocker.com/agent/1.3.0/macos/betblocker-agent",
      "signature": "sha256:...",
      "mandatory": false
    }
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `DEVICE_UNAUTHORIZED` | Invalid device certificate or token |
| 403 | `DEVICE_ID_MISMATCH` | Authenticated device does not match path parameter |
| 404 | `DEVICE_NOT_FOUND` | Device does not exist |

**Rate Limit:** 30 requests per device per hour.

---

### 4.4 Enrollments

---

#### POST /enrollments

Create a new enrollment. Can be self-enrollment (user enrolls their own device) or partner/authority-initiated (partner enrolls a user's device with the user's prior consent via partner relationship).

**Auth:** User JWT

**Request Body:**

```json
{
  "device_id": "dev_01H...",
  "tier": "partner",
  "protection_config": {
    "dns_blocking": true,
    "app_blocking": false,
    "browser_blocking": false,
    "vpn_detection": "alert",
    "tamper_response": "alert_partner"
  },
  "reporting_config": {
    "level": "aggregated",
    "blocked_attempt_counts": true,
    "domain_details": false,
    "tamper_alerts": true
  },
  "unenrollment_policy": {
    "type": "partner_approval",
    "cooldown_hours": null,
    "requires_approval_from": "acc_01H..."
  },
  "expires_at": null
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `device_id` | `string (UUID)` | Yes | Must be a registered device |
| `tier` | `enum` | Yes | `self`, `partner`, `authority` |
| `protection_config` | `ProtectionConfig` | No | Defaults per tier if omitted |
| `reporting_config` | `ReportingConfig` | No | Defaults per tier if omitted |
| `unenrollment_policy` | `UnenrollmentPolicy` | No | Defaults per tier if omitted |
| `expires_at` | `string (datetime)` | No | Optional expiration |

**Tier Defaults:**

| Tier | dns_blocking | vpn_detection | tamper_response | reporting level | unenrollment type | cooldown_hours |
|------|-------------|---------------|-----------------|-----------------|-------------------|----------------|
| `self` | true | `log` | `log` | `none` | `time_delayed` | 48 |
| `partner` | true | `alert` | `alert_partner` | `aggregated` | `partner_approval` | n/a |
| `authority` | true | `lockdown` | `alert_authority` | `full_audit` | `authority_approval` | n/a |

**Response: `201 Created`**

Returns the full Enrollment model as described in section 3.3.

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 400 | `VALIDATION_ERROR` | Invalid input |
| 401 | `UNAUTHORIZED` | Invalid or expired access token |
| 403 | `FORBIDDEN` | Not authorized to enroll this device (e.g., partner tier requires active partner relationship) |
| 404 | `DEVICE_NOT_FOUND` | Device does not exist |
| 409 | `DEVICE_ALREADY_ENROLLED` | Device already has an active enrollment |
| 422 | `INVALID_TIER_CONFIG` | Configuration is not valid for the selected tier (e.g., self tier cannot set `authority_approval` unenrollment) |

**Rate Limit:** 10 requests per account per hour.

**Notes:**

- For `self` tier: the authenticated user must own the device.
- For `partner` tier: the authenticated user must have an active partner relationship with the device owner, and the partner relationship must include `approve_unenrollment` permission.
- For `authority` tier: the authenticated user must be an authority representative with appropriate organization membership.
- `cooldown_hours` for self tier must be between 24 and 72 (inclusive).

---

#### GET /enrollments

List enrollments visible to the authenticated user. Users see their own enrollments. Partners see enrollments where they are the enrolled_by or approval authority. Admins see all.

**Auth:** User JWT

**Query Parameters:**

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `status` | `enum` | (all) | Filter by enrollment status |
| `tier` | `enum` | (all) | Filter by tier |
| `device_id` | `string (UUID)` | (all) | Filter by specific device |
| `page` | `integer` | 1 | Page number |
| `per_page` | `integer` | 50 | Items per page (max 100) |

**Response: `200 OK`**

```json
{
  "data": [
    { "...Enrollment object..." }
  ],
  "pagination": {
    "total": 5,
    "page": 1,
    "per_page": 50,
    "total_pages": 1
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `UNAUTHORIZED` | Invalid or expired access token |

**Rate Limit:** Standard.

---

#### GET /enrollments/:id

Get full detail for a specific enrollment.

**Auth:** User JWT (enrollment owner, enrolled_by account, approval authority, or admin)

**Path Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `id` | `string (UUID)` | Enrollment ID |

**Response: `200 OK`**

Returns the full Enrollment model as described in section 3.3.

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `UNAUTHORIZED` | Invalid or expired access token |
| 403 | `FORBIDDEN` | Not authorized to view this enrollment |
| 404 | `ENROLLMENT_NOT_FOUND` | Enrollment does not exist |

**Rate Limit:** Standard.

---

#### PATCH /enrollments/:id

Modify an active enrollment's configuration. Who can modify depends on the tier:

- **Self tier:** The enrolled user can modify protection and reporting config.
- **Partner tier:** The partner (enrolled_by) can modify protection and reporting config. The enrolled user can request changes, which the partner must approve (out of scope for Phase 1).
- **Authority tier:** Only the authority representative can modify.

**Auth:** User JWT

**Path Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `id` | `string (UUID)` | Enrollment ID |

**Request Body:**

```json
{
  "protection_config": {
    "vpn_detection": "lockdown"
  },
  "reporting_config": {
    "domain_details": true
  },
  "expires_at": "2027-03-12T00:00:00Z"
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `protection_config` | `Partial<ProtectionConfig>` | No | Partial update; only provided fields change |
| `reporting_config` | `Partial<ReportingConfig>` | No | Partial update |
| `unenrollment_policy` | `Partial<UnenrollmentPolicy>` | No | Only modifiable by enrolled_by (partner/authority). Self tier can change `cooldown_hours` within 24-72 range. |
| `expires_at` | `string (datetime) \| null` | No | Set or clear expiration |

**Response: `200 OK`**

Returns the updated full Enrollment model.

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 400 | `VALIDATION_ERROR` | Invalid input |
| 401 | `UNAUTHORIZED` | Invalid or expired access token |
| 403 | `FORBIDDEN` | Not authorized to modify this enrollment |
| 404 | `ENROLLMENT_NOT_FOUND` | Enrollment does not exist |
| 409 | `ENROLLMENT_NOT_ACTIVE` | Enrollment is not in `active` status |
| 422 | `INVALID_TIER_CONFIG` | Configuration change is invalid for the enrollment tier |

**Rate Limit:** 10 requests per enrollment per hour.

---

#### POST /enrollments/:id/unenroll

Request unenrollment. Behavior depends on the enrollment tier:

- **Self tier:** Starts the cooldown timer. After `cooldown_hours`, the enrollment completes unenrollment automatically.
- **Partner tier:** Notifies the partner and waits for approval via `POST /enrollments/:id/approve-unenroll`.
- **Authority tier:** Notifies the authority representative and waits for approval.

**Auth:** User JWT (enrollment owner)

**Path Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `id` | `string (UUID)` | Enrollment ID |

**Request Body:**

```json
{
  "reason": "I have been in recovery for 2 years and feel confident."
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `reason` | `string` | No | Max 1000 chars |

**Response: `200 OK`**

```json
{
  "data": {
    "enrollment": {
      "id": "enr_01H...",
      "status": "unenroll_requested",
      "unenrollment_request": {
        "requested_at": "2026-03-12T14:30:00Z",
        "requested_by": "acc_01H...",
        "reason": "I have been in recovery for 2 years and feel confident.",
        "eligible_at": "2026-03-14T14:30:00Z",
        "approved_at": null,
        "approved_by": null
      }
    },
    "message": "Unenrollment request submitted. The 48-hour cooling-off period begins now."
  }
}
```

For partner/authority tiers, `eligible_at` is null and `message` indicates that approval is required.

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `UNAUTHORIZED` | Invalid or expired access token |
| 403 | `FORBIDDEN` | Not authorized to request unenrollment for this enrollment |
| 404 | `ENROLLMENT_NOT_FOUND` | Enrollment does not exist |
| 409 | `ENROLLMENT_NOT_ACTIVE` | Enrollment is not in `active` status |
| 409 | `UNENROLL_ALREADY_REQUESTED` | An unenrollment request is already pending |

**Rate Limit:** 3 requests per enrollment per day.

---

#### POST /enrollments/:id/approve-unenroll

Approve a pending unenrollment request. Only callable by the account designated in `unenrollment_policy.requires_approval_from`.

**Auth:** User JWT (partner or authority representative)

**Path Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `id` | `string (UUID)` | Enrollment ID |

**Request Body:**

```json
{
  "approved": true,
  "note": "Recovery progress has been excellent. Approved."
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `approved` | `boolean` | Yes | `true` to approve, `false` to deny |
| `note` | `string` | No | Max 1000 chars. Visible to the enrolled user. |

**Response: `200 OK`**

When approved:

```json
{
  "data": {
    "enrollment": {
      "id": "enr_01H...",
      "status": "unenroll_approved",
      "unenrollment_request": {
        "requested_at": "2026-03-12T14:30:00Z",
        "requested_by": "acc_01H...",
        "reason": "I have been in recovery for 2 years.",
        "eligible_at": null,
        "approved_at": "2026-03-12T16:00:00Z",
        "approved_by": "acc_01H..."
      }
    },
    "message": "Unenrollment approved. The device will be unenrolled shortly."
  }
}
```

When denied:

```json
{
  "data": {
    "enrollment": {
      "id": "enr_01H...",
      "status": "active",
      "unenrollment_request": null
    },
    "message": "Unenrollment request denied."
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `UNAUTHORIZED` | Invalid or expired access token |
| 403 | `FORBIDDEN` | Not the designated approval authority for this enrollment |
| 404 | `ENROLLMENT_NOT_FOUND` | Enrollment does not exist |
| 409 | `NO_PENDING_UNENROLL` | No unenrollment request is pending |

**Rate Limit:** Standard.

---

### 4.5 Blocklist

---

#### GET /blocklist/version

Get the current blocklist version number and metadata. Used by agents to determine if they need an update.

**Auth:** Device cert/token or User JWT

**Response: `200 OK`**

```json
{
  "data": {
    "version": 1250,
    "entry_count": 48732,
    "last_updated_at": "2026-03-12T12:00:00Z",
    "signature": "sha256:...",
    "size_bytes": 1048576
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `version` | `integer` | Monotonically increasing version number |
| `entry_count` | `integer` | Total active entries |
| `last_updated_at` | `string (datetime)` | When the blocklist was last compiled |
| `signature` | `string` | Cryptographic signature for integrity verification |
| `size_bytes` | `integer` | Full blocklist size in bytes |

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `UNAUTHORIZED` | Invalid authentication |

**Rate Limit:** 60 requests per device per hour.

---

#### GET /blocklist/delta

Get incremental blocklist updates since a given version. Returns only the additions and removals between `from_version` and the current version.

**Auth:** Device cert/token or User JWT

**Query Parameters:**

| Param | Type | Required | Validation |
|-------|------|----------|------------|
| `from_version` | `integer` | Yes | Must be > 0 and <= current version. If too old, returns `FULL_SYNC_REQUIRED`. |

**Response: `200 OK`**

```json
{
  "data": {
    "from_version": 1247,
    "to_version": 1250,
    "additions": [
      {
        "domain": "new-casino-site.com",
        "pattern": null,
        "category": "online_casino"
      },
      {
        "domain": null,
        "pattern": "*.gambling-affiliate-network.net",
        "category": "affiliate"
      }
    ],
    "removals": [
      {
        "domain": "false-positive-site.com"
      }
    ],
    "signature": "sha256:...",
    "full_sync_url": "/v1/blocklist/full"
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `from_version` | `integer` | The version the delta is computed from |
| `to_version` | `integer` | The version the delta brings you to |
| `additions` | `DeltaEntry[]` | Entries added since `from_version` |
| `removals` | `DeltaEntry[]` | Entries removed since `from_version` |
| `signature` | `string` | Signature covering the resulting full blocklist at `to_version` |
| `full_sync_url` | `string` | URL to download the full blocklist (fallback) |

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 400 | `VALIDATION_ERROR` | Missing or invalid `from_version` |
| 401 | `UNAUTHORIZED` | Invalid authentication |
| 410 | `FULL_SYNC_REQUIRED` | `from_version` is too old; delta is unavailable. Agent must download full blocklist. |

**Rate Limit:** 30 requests per device per hour.

**Notes:** The API retains delta history for the last 100 versions. Agents that fall further behind must perform a full sync.

---

#### POST /blocklist/report

Submit a federated report from an agent. When the agent's heuristic engine encounters a domain it suspects is gambling-related but is not in the blocklist, it submits a report here for central review.

**Auth:** Device cert/token

**Request Body:**

```json
{
  "reports": [
    {
      "domain": "suspicious-gambling-site.com",
      "detected_via": "heuristic",
      "heuristic_score": 0.87,
      "context": {
        "matched_keywords": ["casino", "slots", "bonus"],
        "redirect_chain": ["ad-network.com", "suspicious-gambling-site.com"],
        "tls_cert_org": "Casino Holdings Ltd"
      },
      "occurred_at": "2026-03-12T14:25:00Z"
    }
  ]
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `reports` | `FederatedReport[]` | Yes | 1-50 reports per request |
| `reports[].domain` | `string` | Yes | Valid domain name |
| `reports[].detected_via` | `enum` | Yes | `heuristic`, `redirect`, `content_match`, `user_report` |
| `reports[].heuristic_score` | `float` | No | 0.0-1.0 |
| `reports[].context` | `object` | No | Freeform supporting evidence |
| `reports[].occurred_at` | `string (datetime)` | Yes | When the detection occurred |

**Response: `202 Accepted`**

```json
{
  "data": {
    "accepted": 1,
    "duplicates": 0,
    "message": "Reports queued for review."
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 400 | `VALIDATION_ERROR` | Invalid input |
| 401 | `DEVICE_UNAUTHORIZED` | Invalid device authentication |
| 422 | `REPORTING_DISABLED` | The enrollment's reporting config does not allow federated reports |

**Rate Limit:** 60 requests per device per hour. Max 50 reports per request.

**Notes:** Reports are deduplicated by domain. The device ID and enrollment ID are recorded for provenance but are not exposed in the admin review queue (to preserve privacy). Reports from multiple devices increase the confidence score.

---

#### Admin Blocklist Endpoints

The following endpoints are restricted to admin users for managing the blocklist.

---

##### POST /admin/blocklist/entries

Create a new blocklist entry (curated).

**Auth:** Admin JWT

**Request Body:**

```json
{
  "domain": "new-gambling-site.com",
  "pattern": null,
  "category": "online_casino",
  "evidence_url": "https://...",
  "tags": ["casino", "uk-licensed"],
  "notes": "Licensed UK gambling operator launched 2026-02."
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `domain` | `string` | Conditional | Required if `pattern` is null. Valid domain. |
| `pattern` | `string` | Conditional | Required if `domain` is null. Valid glob pattern. |
| `category` | `enum` | Yes | Valid blocklist category |
| `evidence_url` | `string` | No | Valid URL |
| `tags` | `string[]` | No | Max 20 tags, each max 50 chars |
| `notes` | `string` | No | Max 2000 chars |

**Response: `201 Created`**

Returns the full BlocklistEntry model. Entry is created with `status: active` and `confidence: 1.0` for curated entries.

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 400 | `VALIDATION_ERROR` | Invalid input |
| 401 | `UNAUTHORIZED` | Not authenticated |
| 403 | `FORBIDDEN` | Not an admin |
| 409 | `ENTRY_ALREADY_EXISTS` | Domain or pattern already exists in the blocklist |

**Rate Limit:** 100 requests per admin per hour.

---

##### GET /admin/blocklist/entries

List and search blocklist entries.

**Auth:** Admin JWT

**Query Parameters:**

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `search` | `string` | (none) | Search domain/pattern text |
| `category` | `enum` | (all) | Filter by category |
| `source` | `enum` | (all) | Filter by source |
| `status` | `enum` | (all) | Filter by status |
| `page` | `integer` | 1 | Page number |
| `per_page` | `integer` | 50 | Items per page (max 200) |

**Response: `200 OK`**

Paginated list of BlocklistEntry objects.

**Rate Limit:** Standard.

---

##### PATCH /admin/blocklist/entries/:id

Update a blocklist entry.

**Auth:** Admin JWT

**Path Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `id` | `string (UUID)` | Blocklist entry ID |

**Request Body:**

```json
{
  "category": "sports_betting",
  "status": "inactive",
  "tags": ["sports", "decommissioned"]
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `category` | `enum` | No | Valid blocklist category |
| `status` | `enum` | No | `active`, `inactive` |
| `tags` | `string[]` | No | Max 20 tags |
| `evidence_url` | `string` | No | Valid URL |
| `notes` | `string` | No | Max 2000 chars |

**Response: `200 OK`**

Returns updated BlocklistEntry.

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `UNAUTHORIZED` | Not authenticated |
| 403 | `FORBIDDEN` | Not an admin |
| 404 | `ENTRY_NOT_FOUND` | Entry does not exist |

**Rate Limit:** 100 requests per admin per hour.

---

##### DELETE /admin/blocklist/entries/:id

Soft-delete a blocklist entry (sets status to `inactive` and records `blocklist_version_removed`).

**Auth:** Admin JWT

**Path Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `id` | `string (UUID)` | Blocklist entry ID |

**Response: `200 OK`**

Returns updated BlocklistEntry with `status: inactive`.

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `UNAUTHORIZED` | Not authenticated |
| 403 | `FORBIDDEN` | Not an admin |
| 404 | `ENTRY_NOT_FOUND` | Entry does not exist |

**Rate Limit:** 100 requests per admin per hour.

---

##### GET /admin/blocklist/review-queue

List federated reports pending review.

**Auth:** Admin JWT

**Query Parameters:**

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `min_reports` | `integer` | 1 | Minimum number of agent reports for a domain |
| `min_confidence` | `float` | 0.0 | Minimum aggregated confidence score |
| `sort` | `enum` | `confidence_desc` | Sort order: `confidence_desc`, `reports_desc`, `oldest_first` |
| `page` | `integer` | 1 | Page number |
| `per_page` | `integer` | 50 | Items per page (max 200) |

**Response: `200 OK`**

```json
{
  "data": [
    {
      "domain": "suspicious-site.com",
      "report_count": 14,
      "first_reported_at": "2026-03-10T08:00:00Z",
      "last_reported_at": "2026-03-12T14:00:00Z",
      "aggregated_confidence": 0.91,
      "top_heuristic_matches": ["casino", "slots", "deposit bonus"],
      "sample_context": {
        "tls_cert_org": "Casino Holdings Ltd",
        "redirect_chains": [["ad.net", "suspicious-site.com"]]
      }
    }
  ],
  "pagination": { "..." }
}
```

**Rate Limit:** Standard.

---

##### POST /admin/blocklist/review-queue/:domain/resolve

Resolve a review queue item by promoting it to the blocklist or rejecting it.

**Auth:** Admin JWT

**Path Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `domain` | `string` | The domain under review |

**Request Body:**

```json
{
  "action": "promote",
  "category": "online_casino",
  "tags": ["casino"],
  "notes": "Confirmed gambling site after manual review."
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `action` | `enum` | Yes | `promote` (add to blocklist) or `reject` (dismiss reports) |
| `category` | `enum` | Conditional | Required if `action` is `promote` |
| `tags` | `string[]` | No | Max 20 tags |
| `notes` | `string` | No | Max 2000 chars |

**Response: `200 OK`**

If promoted, returns the created BlocklistEntry. If rejected, returns confirmation.

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `UNAUTHORIZED` | Not authenticated |
| 403 | `FORBIDDEN` | Not an admin |
| 404 | `DOMAIN_NOT_IN_QUEUE` | No pending reports for this domain |
| 409 | `ENTRY_ALREADY_EXISTS` | Domain is already in the blocklist (for promote action) |

**Rate Limit:** 100 requests per admin per hour.

---

### 4.6 Events

---

#### POST /events

Agent submits a batch of events. Events are written to TimescaleDB for time-series analysis.

**Auth:** Device cert/token

**Request Body:**

```json
{
  "events": [
    {
      "type": "block",
      "category": "dns",
      "severity": "info",
      "payload": {
        "domain": "example-casino.com",
        "query_type": "A",
        "source_app": "com.google.chrome"
      },
      "occurred_at": "2026-03-12T14:25:00Z"
    },
    {
      "type": "tamper_detected",
      "category": "tamper",
      "severity": "critical",
      "payload": {
        "component": "dns_resolver",
        "detail": "DNS config changed externally"
      },
      "occurred_at": "2026-03-12T14:26:00Z"
    }
  ]
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `events` | `EventInput[]` | Yes | 1-100 events per batch |
| `events[].type` | `enum` | Yes | Valid event type (see section 3.4) |
| `events[].category` | `enum` | Yes | Valid event category |
| `events[].severity` | `enum` | Yes | `info`, `warning`, `critical` |
| `events[].payload` | `object` | Yes | Type-specific structured data, max 4KB per event |
| `events[].occurred_at` | `string (datetime)` | Yes | Must be within last 7 days and not in the future |

**Response: `202 Accepted`**

```json
{
  "data": {
    "accepted": 2,
    "rejected": 0,
    "errors": []
  }
}
```

If some events fail validation, they are reported individually:

```json
{
  "data": {
    "accepted": 1,
    "rejected": 1,
    "errors": [
      {
        "index": 1,
        "code": "INVALID_EVENT_TYPE",
        "message": "Unknown event type: 'foo'"
      }
    ]
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 400 | `VALIDATION_ERROR` | Entire batch is invalid (e.g., empty events array) |
| 401 | `DEVICE_UNAUTHORIZED` | Invalid device authentication |
| 413 | `PAYLOAD_TOO_LARGE` | Total request body exceeds 512KB |

**Rate Limit:** 120 requests per device per hour. Max 100 events per request.

**Notes:** Events are subject to the enrollment's `reporting_config`. The API filters out events that exceed the configured reporting level before storage. For example, if `domain_details` is false, the `payload.domain` field is stripped before persistence and replaced with a category-level aggregate.

---

#### GET /events

Query events with filtering. Visibility is governed by the enrollment's reporting config and the requester's relationship to the enrollment.

**Auth:** User JWT

**Query Parameters:**

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `device_id` | `string (UUID)` | (all visible) | Filter by device |
| `enrollment_id` | `string (UUID)` | (all visible) | Filter by enrollment |
| `type` | `enum` | (all) | Filter by event type |
| `category` | `enum` | (all) | Filter by event category |
| `severity` | `enum` | (all) | Filter by severity |
| `from` | `string (datetime)` | 7 days ago | Start of time range |
| `to` | `string (datetime)` | now | End of time range |
| `page` | `integer` | 1 | Page number |
| `per_page` | `integer` | 50 | Items per page (max 200) |

**Response: `200 OK`**

```json
{
  "data": [
    {
      "id": "evt_01H...",
      "device_id": "dev_01H...",
      "enrollment_id": "enr_01H...",
      "type": "block",
      "category": "dns",
      "severity": "info",
      "payload": {
        "domain": "example-casino.com",
        "query_type": "A"
      },
      "occurred_at": "2026-03-12T14:25:00Z",
      "received_at": "2026-03-12T14:25:01Z"
    }
  ],
  "pagination": { "..." }
}
```

**Visibility Rules:**

- **Enrollment owner (self tier, reporting = none):** No events visible via API. Only local device display.
- **Enrollment owner (self tier, reporting = aggregated):** Only aggregated counts, no domain details.
- **Partner:** Sees events according to enrollment's `reporting_config`. `aggregated` = counts only. `detailed` = domain details included.
- **Authority:** Full access to all events per `full_audit` reporting level.
- **Admin:** Full access to all events.

When the requester's access level is `aggregated`, the `payload` field is replaced with:

```json
{
  "payload": {
    "aggregated": true,
    "count": 14,
    "categories": { "online_casino": 10, "sports_betting": 4 }
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 400 | `VALIDATION_ERROR` | Invalid query parameters |
| 401 | `UNAUTHORIZED` | Invalid or expired access token |
| 403 | `FORBIDDEN` | Not authorized to view events for this device/enrollment |

**Rate Limit:** Standard.

---

#### GET /events/summary

Get aggregated event summary per enrollment. Useful for dashboard widgets.

**Auth:** User JWT

**Query Parameters:**

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `enrollment_id` | `string (UUID)` | (all visible) | Filter by enrollment |
| `device_id` | `string (UUID)` | (all visible) | Filter by device |
| `period` | `enum` | `day` | Aggregation period: `hour`, `day`, `week`, `month` |
| `from` | `string (datetime)` | 30 days ago | Start of time range |
| `to` | `string (datetime)` | now | End of time range |

**Response: `200 OK`**

```json
{
  "data": {
    "enrollment_id": "enr_01H...",
    "device_id": "dev_01H...",
    "period": "day",
    "from": "2026-02-12T00:00:00Z",
    "to": "2026-03-12T23:59:59Z",
    "summary": {
      "total_blocks": 342,
      "total_bypass_attempts": 2,
      "total_tamper_events": 0,
      "categories": {
        "online_casino": 210,
        "sports_betting": 98,
        "affiliate": 34
      }
    },
    "timeseries": [
      {
        "period_start": "2026-03-11T00:00:00Z",
        "blocks": 18,
        "bypass_attempts": 0,
        "tamper_events": 0
      },
      {
        "period_start": "2026-03-12T00:00:00Z",
        "blocks": 24,
        "bypass_attempts": 1,
        "tamper_events": 0
      }
    ]
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 400 | `VALIDATION_ERROR` | Invalid query parameters |
| 401 | `UNAUTHORIZED` | Invalid or expired access token |
| 403 | `FORBIDDEN` | Not authorized to view events for this enrollment |

**Rate Limit:** 30 requests per account per hour (aggregation queries are expensive).

**Notes:** The same visibility rules as `GET /events` apply. If the requester only has `aggregated` access, domain-level breakdowns are omitted and only category-level counts are shown.

---

### 4.7 Organizations (Phase 2+)

These endpoints are specified now but will return `501 Not Implemented` until Phase 2. The interface is defined to allow frontend development to proceed.

---

#### POST /organizations

Create a new organization.

**Auth:** User JWT

**Request Body:**

```json
{
  "name": "Recovery Center of Austin",
  "type": "therapy_practice",
  "settings": {
    "default_enrollment_tier": "authority",
    "default_protection_config": { "..." },
    "default_reporting_config": { "..." }
  }
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `name` | `string` | Yes | 2-200 chars |
| `type` | `enum` | Yes | `therapy_practice`, `court_program`, `family`, `employer`, `other` |
| `settings` | `OrgSettings` | No | Default enrollment configuration |

**Response: `201 Created`**

Returns the full Organization model. The creating account becomes the owner.

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 400 | `VALIDATION_ERROR` | Invalid input |
| 401 | `UNAUTHORIZED` | Invalid or expired access token |
| 501 | `NOT_IMPLEMENTED` | Feature not yet available (Phase 1) |

---

#### GET /organizations

List organizations the authenticated user belongs to.

**Auth:** User JWT

**Response: `200 OK`**

Paginated list of Organization objects.

---

#### GET /organizations/:id

Get full detail for an organization.

**Auth:** User JWT (must be a member)

**Path Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `id` | `string (UUID)` | Organization ID |

**Response: `200 OK`**

Returns full Organization model.

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `UNAUTHORIZED` | Invalid or expired access token |
| 403 | `FORBIDDEN` | Not a member of this organization |
| 404 | `ORGANIZATION_NOT_FOUND` | Organization does not exist |

---

#### PATCH /organizations/:id

Update organization details.

**Auth:** User JWT (must be owner or admin of the org)

**Request Body:**

```json
{
  "name": "Austin Recovery Center",
  "settings": { "..." }
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `name` | `string` | No | 2-200 chars |
| `type` | `enum` | No | Valid org type |
| `settings` | `OrgSettings` | No | Partial update |

**Response: `200 OK`**

Returns updated Organization model.

---

#### DELETE /organizations/:id

Delete an organization. All member associations are removed. Active enrollments created through this org are not affected (they persist independently).

**Auth:** User JWT (must be owner)

**Path Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `id` | `string (UUID)` | Organization ID |

**Response: `204 No Content`**

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `UNAUTHORIZED` | Invalid or expired access token |
| 403 | `FORBIDDEN` | Not the organization owner |
| 404 | `ORGANIZATION_NOT_FOUND` | Organization does not exist |

---

#### POST /organizations/:id/members

Add a member to the organization.

**Auth:** User JWT (owner or admin of the org)

**Request Body:**

```json
{
  "account_id": "acc_01H...",
  "role": "member"
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `account_id` | `string (UUID)` | Yes | Must be a valid account |
| `role` | `enum` | Yes | `owner`, `admin`, `member` |

**Response: `201 Created`**

```json
{
  "data": {
    "organization_id": "org_01H...",
    "account_id": "acc_01H...",
    "role": "member",
    "added_at": "2026-03-12T14:30:00Z"
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `UNAUTHORIZED` | Invalid or expired access token |
| 403 | `FORBIDDEN` | Not authorized to manage members |
| 404 | `ACCOUNT_NOT_FOUND` | Target account does not exist |
| 409 | `ALREADY_MEMBER` | Account is already a member |

---

#### GET /organizations/:id/members

List organization members.

**Auth:** User JWT (must be a member)

**Response: `200 OK`**

Paginated list of member objects with `account_id`, `role`, and `added_at`.

---

#### DELETE /organizations/:id/members/:account_id

Remove a member from the organization.

**Auth:** User JWT (owner or admin of the org)

**Path Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `id` | `string (UUID)` | Organization ID |
| `account_id` | `string (UUID)` | Account to remove |

**Response: `204 No Content`**

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `UNAUTHORIZED` | Invalid or expired access token |
| 403 | `FORBIDDEN` | Not authorized to manage members |
| 403 | `CANNOT_REMOVE_OWNER` | Cannot remove the organization owner |
| 404 | `MEMBER_NOT_FOUND` | Account is not a member |

---

#### POST /organizations/:id/devices

Assign a device to an organization (for group management).

**Auth:** User JWT (owner or admin of the org)

**Request Body:**

```json
{
  "device_id": "dev_01H..."
}
```

**Response: `200 OK`**

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 403 | `FORBIDDEN` | Not authorized |
| 404 | `DEVICE_NOT_FOUND` | Device does not exist |
| 409 | `DEVICE_ALREADY_ASSIGNED` | Device is already assigned to an organization |

---

#### GET /organizations/:id/devices

List devices assigned to an organization.

**Auth:** User JWT (must be a member)

**Response: `200 OK`**

Paginated list of Device objects.

---

### 4.8 Billing (Hosted Only)

These endpoints are only available on the hosted platform. Self-hosted deployments return `404` for all billing routes (the router does not register them when `BILLING_ENABLED=false`).

---

#### POST /billing/subscribe

Create a new Stripe subscription.

**Auth:** User JWT

**Request Body:**

```json
{
  "plan": "standard",
  "payment_method_id": "pm_..."
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `plan` | `enum` | Yes | `standard`, `partner_tier`, `institutional` |
| `payment_method_id` | `string` | Yes | Stripe payment method ID (from Stripe.js on the frontend) |

**Response: `201 Created`**

```json
{
  "data": {
    "subscription_id": "sub_...",
    "plan": "standard",
    "status": "active",
    "current_period_start": "2026-03-12T00:00:00Z",
    "current_period_end": "2026-04-12T00:00:00Z",
    "price_cents": 1000,
    "currency": "usd",
    "cancel_at_period_end": false
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 400 | `VALIDATION_ERROR` | Invalid input |
| 401 | `UNAUTHORIZED` | Invalid or expired access token |
| 402 | `PAYMENT_FAILED` | Stripe declined the payment method |
| 409 | `ALREADY_SUBSCRIBED` | Account already has an active subscription |

**Rate Limit:** 5 requests per account per hour.

---

#### GET /billing/status

Get current subscription status.

**Auth:** User JWT

**Response: `200 OK`**

```json
{
  "data": {
    "has_subscription": true,
    "subscription": {
      "subscription_id": "sub_...",
      "plan": "standard",
      "status": "active",
      "current_period_start": "2026-03-12T00:00:00Z",
      "current_period_end": "2026-04-12T00:00:00Z",
      "price_cents": 1000,
      "currency": "usd",
      "cancel_at_period_end": false,
      "created_at": "2026-03-12T14:30:00Z"
    },
    "invoices": [
      {
        "id": "inv_...",
        "amount_cents": 1000,
        "currency": "usd",
        "status": "paid",
        "period_start": "2026-03-12T00:00:00Z",
        "period_end": "2026-04-12T00:00:00Z",
        "pdf_url": "https://..."
      }
    ]
  }
}
```

When no subscription exists:

```json
{
  "data": {
    "has_subscription": false,
    "subscription": null,
    "invoices": []
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `UNAUTHORIZED` | Invalid or expired access token |

**Rate Limit:** Standard.

---

#### POST /billing/webhook

Stripe webhook handler. Receives Stripe events for subscription lifecycle management.

**Auth:** None (verified via Stripe webhook signature in `Stripe-Signature` header)

**Request Body:** Raw Stripe event payload (not JSON-parsed by application; signature verification requires raw body).

**Handled Events:**

| Stripe Event | Action |
|-------------|--------|
| `customer.subscription.created` | Record new subscription |
| `customer.subscription.updated` | Update subscription status/plan |
| `customer.subscription.deleted` | Mark subscription as cancelled |
| `invoice.payment_succeeded` | Record successful payment |
| `invoice.payment_failed` | Notify user, grace period begins |
| `customer.subscription.trial_will_end` | Send trial ending notification |

**Response: `200 OK`**

```json
{
  "received": true
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 400 | `INVALID_SIGNATURE` | Stripe signature verification failed |

**Rate Limit:** None (Stripe controls the rate).

**Notes:** The webhook endpoint must be idempotent. Stripe may deliver the same event multiple times. Events are deduplicated by Stripe event ID.

---

#### POST /billing/cancel

Cancel the current subscription. The subscription remains active until the end of the current billing period (`cancel_at_period_end = true`).

**Auth:** User JWT

**Request Body:**

```json
{
  "reason": "No longer needed.",
  "feedback": "too_expensive"
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `reason` | `string` | No | Max 500 chars |
| `feedback` | `enum` | No | `too_expensive`, `not_useful`, `switching_service`, `other` |

**Response: `200 OK`**

```json
{
  "data": {
    "subscription_id": "sub_...",
    "status": "active",
    "cancel_at_period_end": true,
    "current_period_end": "2026-04-12T00:00:00Z",
    "message": "Your subscription will remain active until 2026-04-12. You will not be charged again."
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `UNAUTHORIZED` | Invalid or expired access token |
| 404 | `NO_ACTIVE_SUBSCRIPTION` | No subscription to cancel |
| 409 | `ALREADY_CANCELLING` | Subscription is already set to cancel at period end |

**Rate Limit:** 3 requests per account per day.

**Notes:** Cancelling a subscription does not affect active enrollments. Devices remain enrolled and blocking continues. After the subscription lapses, the account is downgraded to the free tier. If the free tier does not support the account's current device count, a grace period applies before enforcement.

---

### 4.9 Partners

---

#### POST /partners/invite

Send an accountability partner invitation. The invitee receives an email with a link to accept.

**Auth:** User JWT

**Request Body:**

```json
{
  "email": "partner@example.com",
  "role": "accountability_partner",
  "permissions": {
    "view_reports": true,
    "approve_unenrollment": true,
    "modify_enrollment": false
  },
  "message": "Hi, I would like you to be my accountability partner for gambling blocking."
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `email` | `string` | Yes | Valid email |
| `role` | `enum` | Yes | `accountability_partner`, `therapist`, `authority_rep` |
| `permissions` | `PartnerPermissions` | No | Defaults: `view_reports: true`, `approve_unenrollment: true`, `modify_enrollment: false` |
| `message` | `string` | No | Max 500 chars. Included in the invitation email. |

**Response: `201 Created`**

```json
{
  "data": {
    "id": "ptr_01H...",
    "account_id": "acc_01H...",
    "partner_email": "partner@example.com",
    "partner_account_id": null,
    "status": "pending",
    "role": "accountability_partner",
    "permissions": {
      "view_reports": true,
      "approve_unenrollment": true,
      "modify_enrollment": false
    },
    "invited_at": "2026-03-12T14:30:00Z",
    "expires_at": "2026-03-19T14:30:00Z"
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 400 | `VALIDATION_ERROR` | Invalid input |
| 401 | `UNAUTHORIZED` | Invalid or expired access token |
| 403 | `EMAIL_NOT_VERIFIED` | Inviter must have a verified email |
| 409 | `PARTNER_ALREADY_INVITED` | An active or pending partner relationship already exists with this email |
| 422 | `CANNOT_INVITE_SELF` | Cannot invite yourself as a partner |

**Rate Limit:** 10 invitations per account per day.

**Notes:** The invitation is valid for 7 days. If the invitee does not have a BetBlocker account, they will be prompted to register when accepting. The `partner_account_id` is populated when the invitation is accepted.

---

#### POST /partners/accept

Accept a partner invitation. The invitee calls this endpoint with the invitation token received via email.

**Auth:** User JWT (the invitee must be logged in)

**Request Body:**

```json
{
  "token": "inv_..."
}
```

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `token` | `string` | Yes | Non-empty invitation token |

**Response: `200 OK`**

```json
{
  "data": {
    "id": "ptr_01H...",
    "account_id": "acc_01H...",
    "partner_account_id": "acc_01H...",
    "status": "active",
    "role": "accountability_partner",
    "permissions": {
      "view_reports": true,
      "approve_unenrollment": true,
      "modify_enrollment": false
    },
    "invited_at": "2026-03-12T14:30:00Z",
    "accepted_at": "2026-03-12T16:00:00Z"
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `UNAUTHORIZED` | Invalid or expired access token |
| 401 | `INVALID_INVITATION_TOKEN` | Token is invalid, expired, or already used |
| 409 | `INVITATION_ALREADY_ACCEPTED` | This invitation has already been accepted |
| 422 | `CANNOT_ACCEPT_OWN_INVITATION` | The invitee cannot be the same as the inviter |

**Rate Limit:** Standard.

---

#### GET /partners

List all partner relationships for the authenticated user. Returns both relationships where the user is the account holder and where the user is the partner.

**Auth:** User JWT

**Query Parameters:**

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `status` | `enum` | (all) | Filter by status: `pending`, `active`, `revoked` |
| `role` | `enum` | (all) | Filter by relationship direction: `my_partners` (I invited them), `partner_of` (they invited me) |
| `page` | `integer` | 1 | Page number |
| `per_page` | `integer` | 50 | Items per page (max 100) |

**Response: `200 OK`**

```json
{
  "data": [
    {
      "id": "ptr_01H...",
      "account_id": "acc_01H...",
      "partner_account_id": "acc_01H...",
      "partner_display_name": "John Smith",
      "status": "active",
      "role": "accountability_partner",
      "direction": "my_partners",
      "permissions": {
        "view_reports": true,
        "approve_unenrollment": true,
        "modify_enrollment": false
      },
      "invited_at": "2026-03-12T14:30:00Z",
      "accepted_at": "2026-03-12T16:00:00Z"
    }
  ],
  "pagination": { "..." }
}
```

The `direction` field indicates the relationship direction:
- `my_partners`: Partners I invited (they oversee me)
- `partner_of`: Users who invited me (I oversee them)

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `UNAUTHORIZED` | Invalid or expired access token |

**Rate Limit:** Standard.

---

#### DELETE /partners/:id

Remove a partner relationship. Either party can initiate removal.

**Auth:** User JWT (either the account holder or the partner)

**Path Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `id` | `string (UUID)` | Partner relationship ID |

**Response: `200 OK`**

```json
{
  "data": {
    "id": "ptr_01H...",
    "status": "revoked",
    "revoked_at": "2026-03-12T18:00:00Z",
    "revoked_by": "acc_01H...",
    "affected_enrollments": [
      {
        "enrollment_id": "enr_01H...",
        "action": "downgraded_to_self",
        "message": "Enrollment downgraded to self tier. Unenrollment policy changed to 48-hour time delay."
      }
    ]
  }
}
```

**Errors:**

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 401 | `UNAUTHORIZED` | Invalid or expired access token |
| 403 | `FORBIDDEN` | Not a party to this partner relationship |
| 404 | `PARTNER_NOT_FOUND` | Partner relationship does not exist |
| 409 | `ALREADY_REVOKED` | Partner relationship is already revoked |

**Rate Limit:** 5 requests per account per day.

**Notes:** When a partner relationship is removed, any enrollments that reference that partner for unenrollment approval are automatically downgraded:
- The enrollment tier changes from `partner` to `self`.
- The unenrollment policy changes to `time_delayed` with a 48-hour cooldown.
- The enrolled user is notified of the change.
- All pending unenrollment requests requiring partner approval are automatically approved.

---

## 5. Error Handling

### Standard Error Codes

These error codes can be returned by any endpoint.

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 400 | `VALIDATION_ERROR` | Request body or query parameters failed validation. `details` contains field-level errors. |
| 401 | `UNAUTHORIZED` | Missing or invalid authentication token |
| 401 | `TOKEN_EXPIRED` | JWT has expired; use refresh token to obtain a new one |
| 403 | `FORBIDDEN` | Authenticated but insufficient permissions |
| 404 | `NOT_FOUND` | Resource does not exist |
| 405 | `METHOD_NOT_ALLOWED` | HTTP method not supported for this path |
| 409 | `CONFLICT` | Request conflicts with current state of the resource |
| 413 | `PAYLOAD_TOO_LARGE` | Request body exceeds size limit (1MB default) |
| 422 | `UNPROCESSABLE_ENTITY` | Request is syntactically valid but semantically invalid |
| 429 | `RATE_LIMIT_EXCEEDED` | Too many requests. `Retry-After` header indicates when to retry. |
| 500 | `INTERNAL_ERROR` | Unexpected server error. Request ID is logged for debugging. |
| 501 | `NOT_IMPLEMENTED` | Feature is spec'd but not yet available |
| 503 | `SERVICE_UNAVAILABLE` | Server is temporarily unavailable (maintenance, overload) |

### Validation Error Details

When `VALIDATION_ERROR` is returned, the `details` field contains structured field-level errors:

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Request validation failed.",
    "details": {
      "fields": {
        "email": ["must be a valid email address"],
        "password": [
          "must be at least 12 characters",
          "must contain at least one uppercase letter"
        ]
      }
    }
  }
}
```

### Rate Limit Headers

All responses include rate limit headers:

```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 97
X-RateLimit-Reset: 1710255300
Retry-After: 60
```

| Header | Description |
|--------|-------------|
| `X-RateLimit-Limit` | Maximum requests allowed in the current window |
| `X-RateLimit-Remaining` | Requests remaining in the current window |
| `X-RateLimit-Reset` | Unix timestamp when the window resets |
| `Retry-After` | Seconds to wait before retrying (only on 429 responses) |

---

## 6. Rate Limiting

Rate limits are applied per authentication principal (account ID, device ID, or IP for unauthenticated endpoints).

### Default Tiers

| Context | Limit | Window |
|---------|-------|--------|
| **Standard (authenticated user)** | 200 requests | 15 minutes |
| **Device agent** | 120 requests | 1 hour |
| **Unauthenticated** | 30 requests per IP | 15 minutes |
| **Admin** | 500 requests | 15 minutes |
| **Webhook (Stripe)** | Unlimited | n/a |

### Per-Endpoint Overrides

Certain endpoints have stricter limits as documented in their individual specifications. The most restrictive limit applies.

### Burst Handling

The rate limiter uses a sliding window algorithm with burst allowance. A client may use up to 20% of their window limit in a single second without being throttled, to accommodate legitimate burst patterns (e.g., page load triggering multiple API calls).

---

## 7. Pagination

All list endpoints support cursor-based or offset-based pagination.

### Offset Pagination (default)

Query parameters:

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `page` | `integer` | 1 | Page number (1-indexed) |
| `per_page` | `integer` | 50 | Items per page |

Maximum `per_page` values vary by endpoint (documented per endpoint, typically 100-200).

Response includes a `pagination` object:

```json
{
  "pagination": {
    "total": 142,
    "page": 1,
    "per_page": 50,
    "total_pages": 3
  }
}
```

### Cursor Pagination (events and high-volume endpoints)

For endpoints dealing with high-volume time-series data (events), cursor-based pagination is also supported:

| Param | Type | Description |
|-------|------|-------------|
| `cursor` | `string` | Opaque cursor from previous response |
| `limit` | `integer` | Items to return (max 200) |

Response:

```json
{
  "pagination": {
    "next_cursor": "eyJ...",
    "has_more": true
  }
}
```

---

## 8. Versioning

The API is versioned via URL path prefix: `/v1/...`.

### Compatibility Policy

- **Minor additions** (new optional fields in responses, new endpoints) are made without version bump.
- **Breaking changes** (field removal, type changes, behavior changes) require a version bump (`/v2/...`).
- Previous API versions are supported for at least 12 months after a new version is released.
- The `API-Version` response header indicates the exact version: `API-Version: 1.0.0`.

### Deprecation

Deprecated endpoints return a `Deprecation` header:

```
Deprecation: true
Sunset: 2027-06-01
Link: <https://api.betblocker.com/v2/equivalent>; rel="successor-version"
```

---

## Appendix A: OpenAPI 3.0 Generation Notes

This specification is structured to map directly to OpenAPI 3.0:

- **Shared Data Models** (section 3) map to `components/schemas`.
- **Endpoint Groups** (section 4) map to `paths`, grouped by `tags`.
- **Error Codes** (section 5) map to `components/responses`.
- **Auth Requirements** map to `components/securitySchemes` with three schemes:
  - `bearerAuth` (HTTP bearer with JWT)
  - `deviceCert` (mutual TLS)
  - `deviceToken` (API key in `X-Device-Token` header)
- **Rate Limit** annotations use the `x-ratelimit` extension.

The canonical OpenAPI YAML will be generated from this specification and maintained alongside it.

---

## Appendix B: WebSocket Interface (Future)

A WebSocket endpoint at `/v1/ws` is planned for real-time push to the web dashboard:

- **Connection auth:** JWT passed as query parameter or in first frame.
- **Channels:** `device:{device_id}:status`, `enrollment:{enrollment_id}:events`, `account:{account_id}:notifications`.
- **Message format:** JSON with `type`, `channel`, `data`, and `timestamp` fields.
- **Guaranteed ordering:** Messages within a channel are ordered by server-assigned sequence numbers.

Full WebSocket specification will be added in a separate document when the real-time push feature is implemented.
