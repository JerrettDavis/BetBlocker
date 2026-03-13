# ADR-007: Event Reporting & Privacy Model

## Status
Proposed

## Date
2026-03-12

## Context

BetBlocker generates events on enrolled devices: blocked DNS queries, detected gambling apps, bypass attempts, tamper detection alerts, and more. These events serve three purposes:

1. **Accountability.** Partners and authorities need to see that blocking is working and whether bypass attempts have occurred. This is the core value of the partner and authority tiers.
2. **Federated intelligence.** Unknown domains flagged by heuristics on one device should improve the blocklist for all devices. This requires some data to flow from agents to the central API.
3. **Operational monitoring.** The platform needs to know if agents are healthy, blocklists are current, and the system is functioning.

However, BetBlocker is explicitly not spyware. The vision document states non-negotiable boundaries:

- No keylogging or screen capture
- No full browsing history collection
- No microphone, camera, or location access
- No data sold or shared with third parties

The tension is real: effective accountability requires knowing what was blocked, but privacy requires not building a comprehensive activity profile. The reporting model must navigate this tension with explicit, granular controls tied to the enrollment tier.

## Decision

### Event Taxonomy

Every reportable action on a device is classified into one of these event types:

```rust
pub enum EventType {
    // --- Blocking events ---
    /// A DNS query was blocked because the domain is on the blocklist
    DnsBlocked {
        domain: String,
        category: GamblingCategory,
        rule_id: Uuid,
    },

    /// A DNS query matched a heuristic but was not on the blocklist
    DnsHeuristicMatch {
        domain: String,
        heuristic_score: f32,
        category_guess: GamblingCategory,
    },

    /// An installed app was detected as a gambling application
    AppDetected {
        app_id: AppIdentifier,
        action: AppAction, // Blocked launch, prevented install, flagged
    },

    /// Browser content was blocked or filtered
    ContentBlocked {
        // NOTE: No URL. Only the domain and content type.
        domain: String,
        content_type: ContentBlockType, // Ad, iframe, affiliate link, search result
    },

    // --- Bypass/tamper events ---
    /// DNS settings were changed (potential bypass attempt)
    DnsConfigChanged {
        change_type: DnsChangeType, // Server changed, DoH enabled, etc.
        auto_remediated: bool,
    },

    /// VPN or proxy was activated (potential bypass)
    VpnDetected {
        vpn_type: VpnType, // VPN app, SOCKS proxy, Tor, HTTP proxy
    },

    /// Agent process was killed and restarted
    AgentRestarted {
        killed_by: Option<String>, // Process name if identifiable
        downtime_seconds: u32,
    },

    /// Binary integrity check failed
    BinaryTampered {
        expected_hash: String,
        actual_hash: String,
        auto_repaired: bool,
    },

    /// Browser extension was removed or disabled
    ExtensionRemoved {
        browser: String,
    },

    // --- Lifecycle events ---
    /// Enrollment created or modified
    EnrollmentChanged {
        change: EnrollmentChange, // Created, tier changed, unenroll requested, etc.
    },

    /// Agent started or stopped
    AgentLifecycle {
        action: LifecycleAction, // Started, stopped gracefully, crashed
    },

    /// Heartbeat (periodic health report, not stored as event but mentioned for completeness)
    Heartbeat,
}
```

### What Is Never Collected

These are hard architectural constraints, not configurable settings:

| Data Type | Collected? | Rationale |
|-----------|-----------|-----------|
| Full browsing history | NEVER | BetBlocker blocks gambling, it does not surveil browsing. |
| URL paths (anything after the domain) | NEVER | `example.com/gambling-page` is reported as `example.com`, nothing more. |
| Page content or screenshots | NEVER | No content exfiltration of any kind. |
| Keystrokes | NEVER | No keylogging under any circumstances. |
| Location data | NEVER | GPS/IP geolocation is never accessed or inferred. |
| Camera or microphone | NEVER | No permissions requested, no APIs called. |
| Contact lists, messages, call logs | NEVER | No access to personal communications. |
| Non-gambling app usage | NEVER | Only gambling-matched apps are reported. General app usage is not tracked. |
| Device identifiers beyond device UUID | NEVER | No IMEI, MAC address, advertising ID, or hardware serial number. |

These constraints are enforced at the type system level: the `EventType` enum does not have variants that could carry this data. The agent binary physically cannot collect what the types do not express.

### Reporting Levels

Events are filtered before leaving the device based on the enrollment's reporting policy:

```rust
pub enum ReportingLevel {
    /// Only heartbeats and critical tamper alerts. No blocking event details.
    /// Use case: Self-enrolled user who wants protection but no record.
    Minimal,

    /// Block counts per category per time period. No domain names.
    /// Example: "12 casino blocks today, 3 sports betting blocks today"
    /// Use case: Self-enrolled user who wants to track their own progress.
    Aggregate,

    /// Block counts + category + domain names for blocked attempts.
    /// Example: "blocked bet365.com (sports betting) at 14:32"
    /// Use case: Partner accountability with mutual consent.
    Detailed,

    /// Everything in Detailed + tamper events + bypass attempts + timestamps.
    /// Example: "blocked bet365.com at 14:32, VPN detected at 14:35, DNS change at 14:40"
    /// Use case: Authority/court-mandated compliance monitoring.
    FullAudit,
}
```

**Default by enrollment tier:**

| Tier | Default Level | User Can Change To | Notes |
|------|--------------|-------------------|-------|
| Self | Aggregate | Minimal or Detailed | User has full control |
| Partner | Aggregate | Detailed (with mutual consent) | Partner sees Aggregate by default. Both parties must consent to Detailed. |
| Authority | FullAudit | Cannot reduce | Mandated by enrollment authority. User was informed at enrollment. |

### Event Lifecycle on the Device

```
Event occurs (e.g., DNS query blocked)
  |
  v
Event created in memory with full detail
  |
  v
Privacy filter applied based on enrollment's ReportingLevel:
  - Minimal: event dropped (not stored, not reported)
  - Aggregate: domain stripped, only category + timestamp retained
  - Detailed: domain retained, URL path stripped (enforced by type system)
  - FullAudit: full event retained
  |
  v
Filtered event written to local event buffer (encrypted, ring buffer, max 10,000 events)
  |
  v
Event reporter batches events and sends to API on schedule:
  - Normal: every 15 minutes
  - Authority tier: every 5 minutes
  - Tamper events (Level 2+): immediately
  |
  v
API ingests events, stores in TimescaleDB, applies per-enrollment visibility rules for dashboard queries
```

**Critical design point:** The privacy filter runs ON THE DEVICE before events are stored locally or transmitted. The API never receives data that exceeds the enrollment's reporting level. This is privacy-by-design: even if the API is compromised, it cannot extract data that was never sent.

### Local Event Buffer

Events are temporarily stored on the device in an encrypted ring buffer:

- **Capacity:** 10,000 events (approximately 5-10 MB depending on event size)
- **Encryption:** AES-256-GCM with a key derived from the device's hardware-bound master key
- **Ring behavior:** Oldest events are overwritten when buffer is full
- **Persistence:** Buffer survives agent restarts (stored on disk in the agent's protected data directory)
- **Flush on sync:** Successfully reported events are removed from the buffer

The buffer exists so events generated while offline are not lost. When connectivity is restored, the event reporter drains the buffer.

### Federated Report Anonymization

Federated reports (heuristic matches on unknown domains sent to the central review queue) undergo additional anonymization beyond the standard privacy filter:

```rust
pub struct FederatedReport {
    // What IS included:
    pub domain: String,              // The domain that triggered the heuristic
    pub heuristic_score: f32,        // How confident the heuristic is
    pub category_guess: GamblingCategory, // Best guess at gambling category
    pub timestamp: DateTime<Utc>,    // When the match occurred (rounded to nearest hour)

    // What is NOT included:
    // - Device ID (not present)
    // - Account ID (not present)
    // - Enrollment ID (not present)
    // - IP address (not present -- submitted via anonymous endpoint)
    // - API instance URL (not present -- self-hosted reports are forwarded without origin)
    // - Any other event context (not present)
}
```

**Anonymization steps:**

1. **Device identity stripped.** The agent generates a fresh, random session token for each batch of federated reports. This token is not linked to the device certificate or any persistent identity. The API endpoint for federated reports does not require device authentication.

2. **Timestamp coarsened.** The exact timestamp is rounded to the nearest hour. This prevents correlation attacks where an observer with network logs could match a specific report to a specific device's DNS query time.

3. **Batching with delay.** Federated reports are not sent immediately. They are batched and sent at random intervals (1-4 hours after the match). This prevents timing correlation.

4. **k-anonymity threshold.** The API does not act on a federated report until at least k=5 independent reports for the same domain have been received. This ensures that no single agent's report can cause a blocklist change, and that the domain is genuinely being encountered by multiple users.

5. **No round-trip.** The agent sends federated reports but never receives feedback about whether its reports were accepted or rejected. This prevents an attacker from using the federated report endpoint as an oracle to test whether specific domains are on the blocklist.

### Event Visibility in Web Platform

The web platform's dashboard enforces the same visibility rules as the API:

| Dashboard | Viewer | Visible Events | Detail Level |
|-----------|--------|----------------|--------------|
| User dashboard | Device owner | Own events | Per-enrollment ReportingLevel |
| Partner dashboard | Designated partner | Supervised device events | Aggregate (default), Detailed (with consent) |
| Authority dashboard | Authority rep / org member | Mandated device events | FullAudit |
| Admin panel | Platform admin | Aggregate platform stats | No per-device visibility (admin sees counts, not individual events) |

**Platform admins cannot see individual user events.** The admin panel shows aggregate metrics (total blocks across all devices, category breakdowns, system health) but cannot drill down to a specific device or user. This is a deliberate architectural constraint to prevent insider abuse.

### Data Retention

| Data Type | Retention (Hosted) | Retention (Self-Hosted) | Rationale |
|-----------|-------------------|------------------------|-----------|
| Detailed events | 90 days | Configurable (default 90 days) | Balance between accountability history and data minimization |
| Aggregate events | 1 year | Configurable (default 1 year) | Trend analysis for user progress |
| Tamper events | 2 years | Configurable (default 2 years) | Compliance evidence for authority tier |
| Federated reports | 30 days (after processing) | 30 days | Only needed until review decision |
| Heartbeat status | 7 days (only latest matters) | 7 days | Operational monitoring only |

**Right to deletion:** A user who unenrolls (after completing the unenrollment policy for their tier) can request deletion of all their events. The API performs a hard delete from TimescaleDB. Federated reports that have already been anonymized and submitted cannot be attributed back to the user and are not affected.

## Alternatives Considered

### Full Event Collection with Server-Side Filtering

**Pros:** Maximum flexibility. Collect everything, filter on display. If the privacy policy changes, historical data is available.

**Rejected because:** This is architecturally hostile to privacy. If the data exists on the server, it can be subpoenaed, breached, or misused by insiders. Privacy-by-design means the data never leaves the device if the reporting level says it shouldn't. "We could see it but choose not to" is categorically weaker than "we cannot see it because it was never sent."

### Differential Privacy for Federated Reports

**Considered:** Adding calibrated noise to federated reports (e.g., randomized response for whether a domain was actually matched).

**Deferred because:**
- Differential privacy is most valuable when the data is rich enough to re-identify individuals. Federated reports contain only a domain name and a confidence score -- there is no personal data to protect with DP.
- The k-anonymity threshold (k=5) provides sufficient protection: a single report cannot trigger a blocklist change, and the report cannot be attributed to any specific agent.
- If federated reports expand in scope in the future (e.g., including page content heuristics), differential privacy will be reconsidered.

### No Reporting for Self-Enrolled (Privacy Maximalism)

**Considered:** Self-enrolled devices do not report any events to the API. All data stays local.

**Rejected because:**
- Users want to track their own progress. "You blocked 47 gambling sites this week" is a motivational metric that requires server-side aggregation for cross-device views.
- Heartbeats are essential for operational health. Without heartbeats, the API cannot detect that a device is offline or compromised.
- The `Minimal` reporting level achieves near-zero data collection while still maintaining basic operational telemetry.

### Blockchain-Based Audit Trail for Authority Tier

**Considered:** Storing authority-tier audit events on a blockchain for tamper-proof, independently verifiable compliance records.

**Rejected because:**
- Blockchain adds enormous complexity for a feature that serves a small subset of users (authority tier).
- TimescaleDB with append-only tables and cryptographic hash chains (each event references the hash of the previous event) provides equivalent tamper evidence without the operational overhead of a blockchain node.
- If a court or institution requires independently verifiable audit trails, the hash chain can be anchored to a public blockchain periodically (e.g., daily Merkle root published to a Bitcoin OP_RETURN), but this is a Phase 3+ consideration.

## Consequences

### What becomes easier

- **Privacy compliance.** GDPR, CCPA, and similar regulations are satisfied by design. Data minimization is architectural, not procedural. The system cannot over-collect because the types do not permit it.
- **User trust.** The privacy model is explainable: "BetBlocker blocks gambling domains. It reports what it blocked based on the level you chose. It never tracks your browsing, keystrokes, location, or anything else." This is verifiable by inspecting the open-source agent code.
- **Federated intelligence without surveillance.** The anonymization pipeline ensures that contributing to the community blocklist does not compromise any individual user's privacy.
- **Audit compliance.** Authority tier's FullAudit level with hash-chained events provides the evidence trail that courts and institutions require, without requiring special infrastructure.

### What becomes harder

- **Retroactive analysis.** If a user later wants to see detailed event history but was previously on `Minimal` reporting, the data does not exist. There is no way to "go back" and see what was blocked. Mitigation: clearly communicate this at enrollment time. "Choosing Minimal means no history will be recorded."
- **Debugging.** When a user reports "BetBlocker isn't blocking X," and they are on `Minimal` reporting, the support team has no server-side data to investigate. Mitigation: the agent's local event buffer retains the last 10,000 events locally (encrypted), and the user can choose to export and share them for debugging.
- **Partner frustration.** A partner who wants to see detailed blocking data for their supervised device must obtain the user's consent to upgrade from `Aggregate` to `Detailed`. If the user refuses, the partner sees only counts. Mitigation: this is by design. The partner enrolled them to provide accountability, not surveillance. If the partner needs more visibility, they should discuss it with the user.
- **Federated report latency.** The anonymization pipeline (random delay, batching, k-anonymity threshold) means a new gambling domain takes hours to days to appear in the review queue. Mitigation: the automated discovery pipeline (Phase 2) handles time-sensitive discoveries. Federated reports are for long-tail domains that the pipeline misses.

## Implementation Notes

### Phase 1 Deliverables

- [ ] Event type system (`EventType` enum with all blocking and tamper event variants)
- [ ] On-device privacy filter applying `ReportingLevel` before storage and transmission
- [ ] Encrypted local event buffer (AES-256-GCM, ring buffer, 10,000 events)
- [ ] Event reporter with batched transmission (15min normal, 5min authority, immediate tamper)
- [ ] API event ingestion endpoint with enrollment-scoped authorization
- [ ] TimescaleDB event storage with time-based partitioning
- [ ] User dashboard showing events at their enrollment's reporting level
- [ ] Partner dashboard showing aggregate blocking data for supervised devices
- [ ] Data retention job (background worker deletes expired events)

### Phase 2 Additions

- [ ] Federated report anonymization pipeline
- [ ] k-anonymity threshold enforcement in API
- [ ] Federated report review queue in admin panel
- [ ] Authority dashboard with FullAudit view
- [ ] Hash-chained audit trail for authority tier events
- [ ] Event export functionality (user can export their own data)

### Phase 3 Additions

- [ ] Consent management UI for partner-tier reporting level upgrades
- [ ] Right-to-deletion workflow (unenroll + delete all events)
- [ ] Audit trail anchoring to external notarization service (optional)
- [ ] Advanced analytics: trend visualization, category breakdowns, progress tracking

### Event Schema (TimescaleDB)

```sql
CREATE TABLE events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id UUID NOT NULL REFERENCES devices(id),
    enrollment_id UUID NOT NULL REFERENCES enrollments(id),
    event_type TEXT NOT NULL,
    category TEXT,
    domain TEXT,                    -- NULL for non-domain events or Aggregate level
    detail JSONB,                   -- Event-specific data, filtered by reporting level
    reporting_level TEXT NOT NULL,  -- Level at which this event was recorded
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    prev_event_hash BYTEA,         -- SHA-256 of previous event (authority tier only, for hash chain)
    event_hash BYTEA               -- SHA-256 of this event's content
);

SELECT create_hypertable('events', 'created_at');

CREATE INDEX idx_events_device ON events (device_id, created_at DESC);
CREATE INDEX idx_events_enrollment ON events (enrollment_id, created_at DESC);

-- Retention policy: auto-drop chunks older than configured retention
SELECT add_retention_policy('events', INTERVAL '90 days');
```

### Privacy Audit Checklist

- [ ] Verify no URL paths appear in any event type (only domains)
- [ ] Verify no device identifiers beyond UUID appear in events
- [ ] Verify federated reports contain no device/account/enrollment identifiers
- [ ] Verify privacy filter runs before local storage (not just before transmission)
- [ ] Verify admin panel cannot drill down to individual device events
- [ ] Verify right-to-deletion removes events from TimescaleDB and all backups within 30 days
- [ ] Verify `Minimal` reporting level produces no event records (only heartbeats)
- [ ] Verify `Aggregate` reporting level contains no domain names
- [ ] External privacy audit before launch (engaged third-party auditor)
