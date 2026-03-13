# ADR-003: Blocklist Sync Protocol

## Status
Proposed

## Date
2026-03-12

## Context

Every BetBlocker agent maintains a local copy of the gambling blocklist. This blocklist is the core data asset of the platform -- it determines what gets blocked. The sync protocol must satisfy these requirements:

1. **Efficiency.** The full blocklist may contain 100,000+ entries. Agents on mobile networks cannot download the full list on every sync. Delta updates are essential.

2. **Tamper-proofness.** If an attacker can modify the blocklist in transit or at rest on the device, they can whitelist gambling sites. The blocklist must be cryptographically signed and verified by the agent.

3. **Offline operation.** The agent must block gambling even when the device has no internet connection. The local blocklist cache must be self-contained and functional indefinitely.

4. **Versioning.** The API produces blocklist versions. Agents request deltas from their current version. The protocol must handle version gaps (agent was offline for weeks), version resets (blocklist was rebuilt), and rollbacks.

5. **Federated contribution.** Agents report unknown domains that triggered heuristic matches. These reports flow to a central review queue, and approved entries are promoted to the blocklist. The sync protocol must support this bidirectional flow.

6. **Self-hosted parity.** Self-hosted instances maintain their own blocklist but can optionally subscribe to the community feed. The sync protocol must work for both API-to-agent sync and community-feed-to-self-hosted-API sync.

## Decision

### Blocklist Data Model

```rust
pub struct BlocklistEntry {
    /// Unique entry ID (UUID v7 for time-ordered creation)
    pub id: Uuid,

    /// The domain or pattern to block
    pub rule: BlockRule,

    /// How this entry was sourced
    pub source: EntrySource,

    /// Category for reporting granularity
    pub category: GamblingCategory,

    /// Confidence score (0.0 - 1.0). Curated entries are 1.0.
    /// Automated discoveries start lower and are promoted after review.
    pub confidence: f32,

    /// When this entry was added to the blocklist
    pub created_at: DateTime<Utc>,

    /// When this entry was last verified as still a gambling site
    pub last_verified_at: DateTime<Utc>,
}

pub enum BlockRule {
    /// Exact domain match: "example-casino.com"
    ExactDomain(String),

    /// Wildcard: "*.example-casino.com" (blocks all subdomains)
    WildcardDomain(String),

    /// Regex pattern for complex matching (used sparingly, expensive)
    Pattern(String),
}

pub enum EntrySource {
    /// Manually curated by BetBlocker team
    Curated,
    /// Discovered by automated pipeline and approved by reviewer
    AutomatedApproved,
    /// Contributed by federated agent report and approved by reviewer
    FederatedApproved,
    /// From community feed (self-hosted instances)
    CommunityFeed,
}

pub enum GamblingCategory {
    Casino,
    SportsBetting,
    Poker,
    Lottery,
    BinaryOptions,
    CryptoGambling,
    SocialCasino,
    AffiliateMarketing,
    GamblingNews,
    Other,
}
```

### Delta Sync Format

The agent stores its current blocklist version (a monotonically increasing `u64`). On sync, it sends this version to the API, which responds with one of:

**1. Delta response** (common case: agent is slightly behind)

```json
{
  "type": "delta",
  "from_version": 1042,
  "to_version": 1047,
  "additions": [
    { "id": "...", "rule": { "type": "exact", "domain": "new-casino.com" }, "category": "casino", "confidence": 1.0 }
  ],
  "removals": ["entry-uuid-1", "entry-uuid-2"],
  "modifications": [
    { "id": "entry-uuid-3", "confidence": 1.0 }
  ],
  "signature": "base64-encoded-ed25519-signature",
  "signed_hash": "sha256-of-full-blocklist-at-to_version"
}
```

**2. Full sync response** (agent is too far behind, or first sync)

```json
{
  "type": "full",
  "version": 1047,
  "entries": [ ... ],
  "signature": "base64-encoded-ed25519-signature",
  "content_hash": "sha256-of-entries-array"
}
```

**3. No-change response** (agent is already current)

```json
{
  "type": "current",
  "version": 1047
}
```

The API maintains a delta log (last 100 versions). If the agent's version is within the delta log window, it receives a delta. If it is outside the window (or version 0 for first sync), it receives a full sync.

### Wire Format

- **Transport**: HTTPS with mTLS (agent presents device certificate, API presents server certificate with pinned public key).
- **Serialization**: The sync payload is serialized using `postcard` (compact binary format) for bandwidth efficiency, with a JSON fallback for debugging. The `Content-Type` header indicates the format.
- **Compression**: The payload is compressed with `zstd` at level 3 (good compression ratio, fast decompression). A full blocklist of 100K entries compresses from ~8 MB JSON to ~400 KB postcard+zstd.

### Cryptographic Signing

Every blocklist version is signed by the API's blocklist signing key (Ed25519).

**Key hierarchy:**

```
BetBlocker Root CA (Ed25519, offline, HSM-stored)
  |
  +-- Blocklist Signing Key (Ed25519, rotated annually)
  |     Signs: blocklist versions, delta payloads
  |
  +-- API TLS Certificate (X.509, rotated quarterly)
  |     Used for: mTLS server identity
  |
  +-- Agent Signing Key (Ed25519, per-platform, rotated per release)
        Signs: agent binaries
```

**Verification on the agent:**

1. Agent has the Blocklist Signing Key's public key embedded in its binary (compiled in, not configurable).
2. On receiving a sync response, agent verifies the Ed25519 signature over the payload.
3. For delta responses, agent applies the delta to its local blocklist, computes the SHA-256 hash of the resulting full list, and verifies it matches `signed_hash`.
4. If verification fails, agent rejects the update and retains its current blocklist. It logs a tamper alert event and retries on the next sync cycle.

**Self-hosted key management:**

- Self-hosted operators generate their own Blocklist Signing Key during setup.
- The agent binary for self-hosted is compiled with the operator's public key, OR the operator configures the key via a signed configuration file that is itself signed by a bootstrap key.
- If the self-hosted instance subscribes to the community feed, it verifies the community feed's signature (BetBlocker's public key) and re-signs entries with its own key before distributing to its agents.

### Local Cache

The agent stores the blocklist locally in a purpose-built format:

```
blocklist.bbcache
  - Header: magic bytes, format version, blocklist version, entry count, hash
  - Index: sorted domain array for binary search (exact matches)
  - Patterns: compiled regex set (regex automaton serialized to disk)
  - Metadata: per-entry category and confidence (for reporting)
  - Signature: Ed25519 signature over header + index + patterns + metadata
```

**Properties:**

- The cache file is memory-mapped for zero-copy reads. `check_domain()` performs a binary search on the memory-mapped index without deserialization.
- The cache is validated on agent startup by verifying its signature. If invalid, the agent falls back to a compiled-in seed list (top 10,000 known gambling domains, embedded in the binary) and requests a full sync.
- The cache file is protected by OS-level file permissions (owned by SYSTEM/root, not readable by unprivileged users) and, where available, by the kernel minifilter (Windows) or immutable file attributes (Linux).

### Sync Schedule

| Scenario | Interval | Rationale |
|----------|----------|-----------|
| Normal operation | Every 4 hours | Balance between freshness and battery/bandwidth |
| After failed sync | Exponential backoff: 15min, 30min, 1h, 2h, 4h | Avoid hammering a down API |
| After enrollment change | Immediate | New enrollment may change blocklist configuration |
| On agent startup | Immediate (if last sync > 1 hour ago) | Catch up after being offline |
| Mobile on cellular | Every 8 hours | Reduce cellular data usage |
| Mobile on WiFi | Every 4 hours (same as normal) | No bandwidth concern |

The agent uses a jittered schedule (random offset within +/- 30 minutes) to avoid thundering herd on the API when many agents have the same sync interval.

### Federated Report Flow

```
Agent detects unknown domain via heuristic
  |
  v
Agent sends FederatedReport to API:
  { domain, heuristic_score, category_guess, timestamp }
  (No user identity. No browsing context. Just the domain.)
  |
  v
API ingests report into federated_reports table:
  - Deduplicated by domain
  - Aggregated: report_count, avg_heuristic_score, first_seen, last_seen
  - Source tracked as "federated" (no individual agent attribution)
  |
  v
When report_count >= threshold (configurable, default 5):
  Report promoted to review queue
  |
  v
Reviewer (human or automated classifier) evaluates:
  - Approve: entry added to blocklist with source=FederatedApproved
  - Reject: domain added to false_positive_allowlist
  - Defer: needs more data, stays in queue
  |
  v
Approved entry included in next blocklist version
  |
  v
Delta sync distributes to all agents
```

**Privacy constraint:** Federated reports contain only the domain name, a heuristic confidence score, a category guess, and a timestamp. They do NOT contain: the reporting device's identity, the user's identity, the URL path, any browsing context, or any enrollment information. Reports are submitted via a separate API endpoint that does not require device authentication (uses a rotating anonymous token scoped to the reporting session).

### Community Feed (Self-Hosted)

The community feed is a public HTTPS endpoint that serves the BetBlocker-curated blocklist (source = Curated + AutomatedApproved + FederatedApproved entries with confidence >= 0.9).

- Self-hosted APIs poll the community feed on a configurable schedule (default: daily).
- The feed uses the same delta sync protocol (the self-hosted API acts as an "agent" from the community feed's perspective).
- Self-hosted operators can add their own entries locally. These local entries are not shared back to the community feed.
- Optionally, self-hosted operators can enable federated report contribution, which sends anonymized heuristic reports to the central BetBlocker API.

## Alternatives Considered

### Full Sync Only (No Deltas)

**Pros:** Dramatically simpler. No delta log, no version tracking, no hash verification of applied deltas.

**Rejected because:** At 100K+ entries, a full sync is ~400 KB compressed. On a 4-hour schedule across thousands of agents, this is significant bandwidth for both the API and mobile users. More importantly, full sync means the agent must rebuild its memory-mapped cache file on every sync, causing a brief blocking gap during reconstruction.

### CRDTs for Blocklist Convergence

**Pros:** Eventual consistency without version ordering, handles concurrent modifications gracefully, natural fit for federated systems.

**Rejected because:** The blocklist is authoritative, not collaborative. There is one source of truth (the API's reviewed, signed blocklist). Agents do not modify the blocklist locally. CRDTs add complexity (tombstones, vector clocks) for a problem that doesn't exist: the sync is strictly one-directional (API to agent), and the API is the sole writer.

### BitTorrent / P2P Distribution

**Pros:** Reduces API bandwidth costs, scales naturally with agent count.

**Rejected because:** P2P distribution leaks which devices are BetBlocker agents to network observers. The privacy cost is too high. Additionally, P2P requires open ports or NAT traversal, which is blocked on many networks and all mobile carriers. CDN distribution (CloudFront/Cloudflare) provides the same scalability benefit without the privacy cost.

### JSON-only Wire Format

**Pros:** Human-readable, easier to debug, universal client support.

**Rejected as default because:** Binary format (`postcard` + `zstd`) reduces bandwidth by ~10x compared to JSON. Since both endpoints are Rust, there is no interoperability concern. JSON is kept as a debug-mode fallback (enabled by API header), not the default.

## Consequences

### What becomes easier

- **Efficient mobile sync.** Delta updates on a 4-hour schedule use negligible bandwidth (~1-5 KB typical delta).
- **Offline resilience.** Memory-mapped cache works indefinitely without connectivity. The compiled-in seed list provides baseline protection even if the cache is corrupted.
- **Tamper detection.** Any modification to the local cache is detected on next startup (signature verification fails). Any modification to the sync payload is detected on receipt.
- **Self-hosted independence.** Self-hosted instances are fully functional without the community feed. The feed is additive, not required.

### What becomes harder

- **Delta log management.** The API must maintain a rolling window of deltas. If the blocklist changes frequently (e.g., during a bulk import), the delta log grows and older agents fall off the window, triggering full syncs. Mitigation: compact deltas that affect the same entries, and keep the window at 100 versions.
- **Cache format versioning.** If the `.bbcache` format changes between agent versions, old caches must be migrated or discarded. Mitigation: version number in the header, and the agent can always fall back to a full sync to rebuild.
- **Signing key rotation.** When the Blocklist Signing Key is rotated, agents with the old public key cannot verify new blocklist versions. The agent binary must be updated with the new public key. Mitigation: embed both the current and next public key in the binary (key rollover period), and time key rotation to coincide with agent releases.

## Implementation Notes

### Phase 1 Deliverables

- Full sync and delta sync endpoints on the API
- Ed25519 signing of blocklist versions
- Agent-side signature verification and cache management
- Memory-mapped `.bbcache` file with binary search index
- Compiled-in seed list (top 10K gambling domains from public lists)
- 4-hour sync schedule with jitter and exponential backoff

### Phase 2 Additions

- Federated report endpoint (anonymous submission)
- Review queue in admin panel
- Community feed endpoint for self-hosted instances
- Automated classifier for federated report pre-screening

### API Endpoints

```
POST /api/v1/blocklist/sync
  Request:  { current_version: u64 }
  Response: DeltaResponse | FullSyncResponse | CurrentResponse
  Auth:     Device certificate (mTLS)

POST /api/v1/blocklist/report
  Request:  { domain: String, heuristic_score: f32, category_guess: String }
  Response: { accepted: bool }
  Auth:     Anonymous rotating token (no device identity)

GET /api/v1/feed/community
  Request:  ?from_version=N
  Response: Same format as /blocklist/sync
  Auth:     API key (issued to self-hosted operators)
```

### Seed List Maintenance

The compiled-in seed list is generated during the build process from a curated snapshot. It is updated with each agent release. The seed list is a last resort; under normal operation, the agent syncs a full blocklist on first startup and never uses the seed list again.
