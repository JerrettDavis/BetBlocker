# ADR-004: Authentication & Authorization Model

## Status
Proposed

## Date
2026-03-12

## Context

BetBlocker has two distinct authentication domains that interact:

1. **Human users** accessing the web platform (dashboards, management panels). These are accounts with one of several roles: individual user, accountability partner, authority representative, or platform admin.

2. **Devices** (endpoint agents) communicating with the API for blocklist sync, event reporting, configuration pull, and heartbeats. Devices are not humans; they authenticate differently and have a different permission model.

The authorization model is driven by the Core Invariant: **the enrollment authority determines the unenrollment authority, the reporting visibility, and the bypass protection level.** This means permissions are not just role-based -- they are scoped to the enrollment relationship between a device and its authority.

Specific challenges:

- A partner must be able to view reports for devices they supervise, but not devices enrolled by other partners or self-enrolled.
- An authority (court/institution) must have full audit visibility over their enrolled devices, but cannot see self-enrolled devices that happen to belong to the same user.
- A user can self-enroll some devices and have other devices partner-enrolled, each with different visibility rules.
- Device authentication must survive the agent being offline (cached credentials), resist credential extraction by the device user, and support revocation.
- Self-hosted operators need the same auth model but without the hosted platform's identity provider.

## Decision

### Human Authentication: JWT + Refresh Tokens

**Registration and login** use email + password (bcrypt-hashed, minimum 12 characters) with mandatory email verification. OAuth providers (Google, Apple) are supported as optional convenience but are not required.

**Token structure:**

```
Access Token (JWT, short-lived):
  Header: { alg: "EdDSA", typ: "JWT" }
  Payload: {
    sub: "account-uuid",
    iss: "betblocker-api",
    iat: 1741737600,
    exp: 1741741200,        // 1 hour lifetime
    roles: ["user"],
    org_id: "org-uuid" | null,
    tier: "self" | "partner" | "authority" | "admin"
  }
  Signature: Ed25519 (same key family as blocklist signing, different key)

Refresh Token (opaque, long-lived):
  Stored in: HttpOnly, Secure, SameSite=Strict cookie
  Lifetime: 30 days, sliding window (reissued on use)
  Storage: Hashed in PostgreSQL, associated with account + device fingerprint
  Revocation: Explicit logout, password change, or admin action revokes all refresh tokens
```

**Token rotation:** Every time a refresh token is used to obtain a new access token, the old refresh token is invalidated and a new one is issued. This detects token theft: if an attacker uses a stolen refresh token, the legitimate user's next refresh will fail (their token was already consumed), alerting them to compromise.

**Multi-factor authentication (MFA):** Required for partner and authority tier accounts. TOTP (RFC 6238) is the primary method. WebAuthn/passkeys are supported as a second option. MFA is optional for self-enrolled users but encouraged.

### Device Authentication: Mutual TLS with Device Certificates

Devices do NOT use JWT. Instead, they authenticate via mTLS with a device-specific X.509 certificate.

**Enrollment flow (device certificate issuance):**

```
1. User initiates enrollment on web platform
   -> API creates Enrollment record (tier, config, authority)
   -> API generates enrollment_token (one-time, 24h expiry, cryptographically random)

2. User installs agent on device, enters enrollment_token
   -> Agent generates Ed25519 keypair locally
   -> Agent sends CSR (Certificate Signing Request) to API with enrollment_token
   -> API validates token, creates Device record, issues X.509 certificate:
        Subject: CN=device-uuid
        Issuer: BetBlocker Device CA
        Extensions:
          enrollment_id: enrollment-uuid
          tier: self|partner|authority
          issued_at: timestamp
        Validity: 1 year (auto-renewed before expiry)
   -> API returns certificate + enrollment configuration
   -> Agent stores private key in hardware-bound storage:
        Windows: TPM 2.0 via NCrypt
        macOS: Secure Enclave via Keychain
        Linux: TPM 2.0 via tpm2-tss, fallback to encrypted file
        Android: Android Keystore (hardware-backed on supported devices)
        iOS: Secure Enclave via Keychain

3. All subsequent API calls use mTLS:
   -> Agent presents device certificate
   -> API verifies certificate chain (Device CA -> Root CA)
   -> API extracts device-uuid from Subject, looks up enrollment
   -> Request is authorized based on enrollment tier and permissions
```

**Why mTLS instead of API keys:**

- The private key never leaves the device (hardware-bound). An API key is a bearer token that can be exfiltrated by a motivated user inspecting the agent's configuration.
- Certificate revocation is authoritative: the API maintains a CRL (Certificate Revocation List) and checks it on every request. A revoked device is immediately locked out.
- mTLS provides bidirectional authentication: the agent also verifies the API's certificate (pinned public key), preventing MITM attacks even if the device's trust store is compromised.

**Certificate renewal:**

- Certificates are valid for 1 year.
- The agent initiates renewal 30 days before expiry.
- Renewal requires the existing valid certificate (mutual authentication) -- no enrollment token needed.
- If a certificate expires (device was offline for >1 year), re-enrollment is required.

### Authorization Model: Enrollment-Scoped RBAC

Permissions are not just role-based; they are scoped to enrollment relationships.

**Roles:**

| Role | Description | Assigned To |
|------|-------------|-------------|
| `user` | Individual using BetBlocker | Every account |
| `partner` | Accountability partner supervising enrolled devices | Invited by user or self-designated |
| `authority` | Institutional representative (court, therapy program) | Verified by BetBlocker team |
| `admin` | BetBlocker platform administrator | Internal team only |

**Enrollment-scoped permissions:**

```rust
pub struct EnrollmentPermissions {
    /// Who can unenroll this device
    pub unenroll: UnenrollPolicy,

    /// What event data is visible, and to whom
    pub reporting: ReportingPolicy,

    /// What tamper resistance level is enforced
    pub protection: ProtectionLevel,
}

pub enum UnenrollPolicy {
    /// Self-enrolled: user can unenroll after a time delay
    SelfWithDelay { delay_hours: u32 },

    /// Partner-enrolled: requires partner approval
    PartnerApproval { partner_account_id: Uuid },

    /// Authority-enrolled: requires authority approval + audit log
    AuthorityApproval { authority_account_id: Uuid, org_id: Uuid },
}

pub enum ReportingPolicy {
    /// User controls their own reporting level
    UserControlled { level: ReportingLevel },

    /// Partner sees aggregated data by default, detailed with mutual consent
    PartnerVisible {
        partner_account_id: Uuid,
        detail_level: ReportingLevel,
        user_consented_to_detail: bool,
    },

    /// Authority sees full audit log (mandated)
    AuthorityAudit {
        authority_account_id: Uuid,
        org_id: Uuid,
    },
}

pub enum ReportingLevel {
    /// Block counts only (X blocks today)
    Aggregate,
    /// Block counts + categories (5 casino blocks, 2 sports betting)
    CategoryBreakdown,
    /// Block counts + categories + domain names
    Detailed,
    /// Full audit: timestamps, domains, bypass attempts, tamper events
    FullAudit,
}
```

**Permission checks:**

Every API endpoint that accesses enrollment-scoped data (device status, events, configuration) performs a two-step authorization:

1. **Role check:** Does the caller have the `partner` or `authority` role?
2. **Enrollment scope check:** Is the caller the designated partner/authority for THIS specific enrollment?

```rust
// Pseudocode for an API endpoint
async fn get_device_events(
    auth: AuthenticatedAccount,  // From JWT
    device_id: Uuid,
) -> Result<Vec<Event>, ApiError> {
    let device = db.get_device(device_id)?;
    let enrollment = db.get_enrollment(device.enrollment_id)?;

    // Check: is this caller authorized to see this device's events?
    match &enrollment.permissions.reporting {
        ReportingPolicy::UserControlled { .. } => {
            // Only the device owner can see their own events
            require(auth.account_id == device.owner_account_id)?;
        }
        ReportingPolicy::PartnerVisible { partner_account_id, .. } => {
            // Owner or designated partner can see events
            require(
                auth.account_id == device.owner_account_id
                || auth.account_id == *partner_account_id
            )?;
        }
        ReportingPolicy::AuthorityAudit { authority_account_id, org_id } => {
            // Owner, authority rep, or any member of authority org
            require(
                auth.account_id == device.owner_account_id
                || auth.account_id == *authority_account_id
                || db.is_org_member(auth.account_id, *org_id)?
            )?;
        }
    }

    // Return events filtered to the appropriate detail level
    let level = enrollment.effective_reporting_level_for(&auth);
    db.get_events(device_id, level)
}
```

### Enrollment Authority Chain Enforcement

The Core Invariant requires that enrollment tier escalation (self -> partner -> authority) can only be performed by the higher authority, and de-escalation follows the unenrollment policy.

**Rules:**

1. A self-enrolled device can be escalated to partner-enrolled if both the user AND the partner confirm. The partner becomes the unenrollment authority.
2. A partner-enrolled device can be escalated to authority-enrolled if the authority initiates and the partner consents (or the authority has legal mandate, verified by BetBlocker team).
3. De-escalation (authority -> partner -> self) requires the current authority's approval.
4. The enrollment tier is recorded in the device certificate's extensions. If the tier changes, the device certificate is re-issued.
5. The API enforces these transitions as a state machine:

```
Self ---[user + partner consent]---> Partner
Partner ---[authority mandate]------> Authority
Authority ---[authority approval]---> Partner
Partner ---[partner approval]-------> Self
Self ---[time delay]----------------> Unenrolled
```

No transition can skip a level (authority cannot directly de-escalate to self; it must go through partner first, or the partner role can be vacated which automatically de-escalates to self).

### Self-Hosted Authentication

Self-hosted instances use the same authentication model with these differences:

- **No OAuth providers** by default (operator can configure their own OIDC provider via environment variables).
- **Device CA is operator-generated** during `docker compose up` first run. The setup script generates a Root CA and Device CA keypair, stores them in a Docker volume.
- **No BetBlocker team verification** for authority tier. Self-hosted operators are their own authority; they can designate authority accounts directly.
- **JWT signing key** is operator-generated (Ed25519 keypair in Docker volume).

## Alternatives Considered

### API Keys for Device Authentication

**Pros:** Simpler to implement, no PKI infrastructure needed, familiar pattern.

**Rejected because:**
- API keys are bearer tokens stored in the agent's configuration. A motivated user with local admin access can extract the key, use it from another device, or revoke it and claim the agent "malfunctioned."
- API keys cannot be hardware-bound. The key exists as bytes in a file or environment variable.
- Revocation requires the agent to check a revocation endpoint, which fails when offline. Certificate revocation can be cached (CRL with validity period).

### OAuth2 Device Authorization Grant for Devices

**Pros:** Standard protocol (RFC 8628), no certificate infrastructure, works well for headless devices.

**Rejected because:**
- Device Authorization Grant issues bearer tokens (access + refresh), which have the same exfiltration problem as API keys.
- The grant flow requires the device to poll the authorization server, adding latency to enrollment.
- Does not provide mutual authentication (server cannot verify device identity without additional mechanisms).

### Passwordless-Only (WebAuthn/Passkeys)

**Considered for human authentication.**

**Deferred because:**
- WebAuthn adoption is growing but not universal. Requiring passkeys would exclude users without compatible devices or browsers.
- The target audience (people struggling with gambling addiction) may not have the technical sophistication to set up passkeys. Email + password is the lowest-friction path.
- WebAuthn is supported as a second-factor and will be promoted as the primary method once adoption reaches critical mass.

### Session-Based Auth (No JWT)

**Pros:** Simpler, no token expiry management, server-side revocation is immediate.

**Rejected because:**
- Session-based auth requires server-side session storage, which complicates horizontal scaling. The API is designed to be stateless (see vision document).
- JWTs with short expiry (1 hour) and refresh token rotation provide near-equivalent security with stateless verification.
- The refresh token IS effectively a session identifier, stored server-side, giving us the revocation benefits of sessions.

## Consequences

### What becomes easier

- **Stateless API scaling.** JWT verification requires only the public key, not a database lookup. The API can scale horizontally without shared session state.
- **Hardware-bound device identity.** Device certificates with TPM/Secure Enclave storage make credential extraction extremely difficult, even for a motivated local user with admin access.
- **Enrollment-scoped authorization.** The permission model directly encodes the Core Invariant. Every authorization check is explicit about which enrollment relationship is being evaluated.
- **Offline device operation.** The device certificate is valid for 1 year. The agent can authenticate and operate normally even if it hasn't contacted the API in months.

### What becomes harder

- **PKI infrastructure.** Running a Device CA requires key management, CRL distribution, certificate renewal automation, and disaster recovery for the CA private key. Mitigation: use a proven library (rcgen for certificate generation, rustls for TLS), automate renewal, and store CA keys in HSM (hosted) or encrypted Docker volume (self-hosted).
- **Enrollment UX.** The enrollment token flow (web -> token -> paste into agent) is less seamless than "sign in with Google on the agent." Mitigation: support QR code scanning on mobile (agent camera captures token), and deep links on desktop (betblocker://enroll?token=...).
- **Certificate expiry edge cases.** If a device is offline for >1 year, its certificate expires and it must re-enroll. The user may not understand why. Mitigation: agent warns the user 30 days before expiry, and re-enrollment preserves the existing enrollment configuration.
- **Self-hosted CA management.** Operators must safeguard their CA private key. If it's lost, all devices must re-enroll. Mitigation: the setup script generates a CA backup file and prominently warns the operator to store it safely.

## Implementation Notes

### Phase 1 Deliverables

- Email + password registration and login
- JWT access tokens (1h) + refresh tokens (30d) with rotation
- Device enrollment via one-time token
- Device certificate issuance (self-signed Device CA for Phase 1; HSM-backed in production)
- mTLS for all agent-API communication
- Enrollment-scoped permission checks on all device-related endpoints
- Self-enrollment and partner-enrollment flows
- Refresh token revocation on logout and password change

### Phase 2 Additions

- MFA (TOTP) for partner and authority accounts
- WebAuthn/passkey support as optional second factor
- OAuth2 provider integration (Google, Apple)
- Authority tier enrollment with verification workflow
- Organization management (authority accounts grouped by org)

### Phase 3 Additions

- SSO/OIDC integration for institutional deployments
- Certificate auto-renewal with hardware key attestation
- Bulk device enrollment for authority tier

### Security Hardening Checklist

- [ ] Rate limiting on login (5 attempts per minute per IP, 20 per hour per account)
- [ ] Bcrypt cost factor >= 12
- [ ] JWT signed with Ed25519 (not HMAC -- asymmetric signing prevents API key compromise from forging tokens)
- [ ] Refresh tokens stored as SHA-256 hashes in database (not plaintext)
- [ ] Device certificate private keys marked as non-exportable in hardware keystore
- [ ] CRL checked on every mTLS handshake (cached for 1 hour)
- [ ] Enrollment tokens are 256-bit cryptographically random, single-use, 24-hour expiry
- [ ] All password reset flows require email verification
- [ ] Account lockout after 10 failed login attempts (30-minute cooldown)
