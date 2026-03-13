# ADR-006: Deployment Architecture -- Hosted vs Self-Hosted Parity

## Status
Proposed

## Date
2026-03-12

## Context

BetBlocker's growth strategy depends on self-hosted deployments building trust and community, while the hosted platform generates revenue. The vision document states the core principle: "One artifact, two deployment models. The hosted platform runs the exact same containers self-hosters run."

This creates tension:

1. The hosted platform has features that self-hosted does not need (Stripe billing, marketing pages, the automated discovery pipeline, staffed review queue).
2. Self-hosted instances need features the hosted platform takes for granted (community blocklist feed, self-service setup, offline operation).
3. Agent binaries must trust the API they connect to. A self-hosted agent must trust the self-hosted API, not the betblocker.com API. But the binary signing and blocklist signing keys differ between hosted and self-hosted.
4. Self-hosted operators should be able to contribute federated intelligence back to the community without exposing their users' data.

The deployment architecture must resolve these tensions without maintaining two separate codebases.

## Decision

### Single Codebase, Feature Flags via Environment

All features exist in a single codebase. Hosted-only features are gated by environment variables, not build-time flags. The same Docker image runs in both contexts.

**Feature flag hierarchy:**

```env
# Deployment mode (affects which features are active)
BETBLOCKER_DEPLOYMENT=hosted|self-hosted

# Feature flags (auto-set by DEPLOYMENT, individually overridable)
BETBLOCKER_BILLING_ENABLED=true|false          # Stripe integration
BETBLOCKER_DISCOVERY_PIPELINE=true|false       # Automated gambling site discovery
BETBLOCKER_MARKETING_PAGES=true|false          # Landing pages, pricing, etc.
BETBLOCKER_COMMUNITY_FEED_URL=https://feed.betblocker.org/v1  # Community blocklist feed
BETBLOCKER_FEDERATED_REPORT_UPSTREAM=https://api.betblocker.org/v1/reports  # Optional upstream for federated reports
BETBLOCKER_TELEMETRY_ENABLED=false             # Never true by default for self-hosted
```

**Defaults by deployment mode:**

| Feature | `hosted` | `self-hosted` |
|---------|----------|---------------|
| Billing (Stripe) | Enabled | Disabled |
| Discovery pipeline | Enabled | Disabled |
| Marketing pages | Enabled | Disabled |
| Community feed sync | N/A (IS the source) | Enabled (pulls from betblocker.org) |
| Federated report upstream | N/A (IS the receiver) | Disabled (opt-in) |
| Telemetry | Enabled (anonymized) | Disabled |
| Self-service setup wizard | Disabled | Enabled |
| Authority tier verification | BetBlocker team | Operator self-service |

### Configuration Hierarchy

Configuration is resolved in order of precedence (highest first):

```
1. Enrollment-level overrides (per-device, set by API)
   Example: This specific device uses DoH upstream 1.1.1.1

2. Organization-level settings (per-org, set by authority)
   Example: All devices in this court program use 15-minute heartbeats

3. Instance-level settings (per-deployment, set by operator)
   Example: This self-hosted instance uses a custom upstream DNS

4. Platform defaults (compiled into the binary)
   Example: Default heartbeat interval is 1 hour
```

The agent resolves configuration by merging these layers. The API serves the merged configuration to the agent during sync, so the agent does not need to understand the hierarchy -- it receives a flat configuration object.

### Container Topology

The same six containers run in both modes:

```yaml
# docker-compose.yml (self-hosted)
services:
  api:
    image: ghcr.io/betblocker/betblocker-api:latest
    environment:
      BETBLOCKER_DEPLOYMENT: self-hosted
      DATABASE_URL: postgres://betblocker:${DB_PASSWORD}@db:5432/betblocker
      REDIS_URL: redis://cache:6379
      TIMESCALE_URL: postgres://betblocker:${DB_PASSWORD}@analytics:5432/betblocker_analytics
    ports:
      - "443:8443"
    depends_on:
      - db
      - cache

  web:
    image: ghcr.io/betblocker/betblocker-web:latest
    environment:
      BETBLOCKER_DEPLOYMENT: self-hosted
      API_URL: http://api:8443
    ports:
      - "80:3000"

  worker:
    image: ghcr.io/betblocker/betblocker-worker:latest
    environment:
      BETBLOCKER_DEPLOYMENT: self-hosted
      DATABASE_URL: postgres://betblocker:${DB_PASSWORD}@db:5432/betblocker
      REDIS_URL: redis://cache:6379
    depends_on:
      - db
      - cache

  db:
    image: postgres:16-alpine
    volumes:
      - betblocker-db:/var/lib/postgresql/data
    environment:
      POSTGRES_DB: betblocker
      POSTGRES_USER: betblocker
      POSTGRES_PASSWORD: ${DB_PASSWORD}

  cache:
    image: redis:7-alpine
    volumes:
      - betblocker-cache:/data

  analytics:
    image: timescale/timescaledb:latest-pg16
    volumes:
      - betblocker-analytics:/var/lib/postgresql/data
    environment:
      POSTGRES_DB: betblocker_analytics
      POSTGRES_USER: betblocker
      POSTGRES_PASSWORD: ${DB_PASSWORD}

volumes:
  betblocker-db:
  betblocker-cache:
  betblocker-analytics:
  betblocker-keys:
```

**Hosted platform** uses the same images but with Kubernetes manifests, managed databases (RDS, ElastiCache, Timescale Cloud), and additional infrastructure (CDN, load balancer, monitoring).

### First-Run Setup (Self-Hosted)

On first `docker compose up`, the API container detects that it has no initialization state and enters setup mode:

```
1. Generate cryptographic material:
   - Root CA keypair (Ed25519) -> stored in betblocker-keys volume
   - Device CA keypair (Ed25519, signed by Root CA) -> stored in betblocker-keys volume
   - Blocklist Signing keypair (Ed25519, signed by Root CA) -> stored in betblocker-keys volume
   - JWT Signing keypair (Ed25519) -> stored in betblocker-keys volume
   - API TLS certificate (self-signed or Let's Encrypt via ACME) -> stored in betblocker-keys volume

2. Run database migrations

3. Create initial admin account:
   - Email and password from BETBLOCKER_ADMIN_EMAIL / BETBLOCKER_ADMIN_PASSWORD env vars
   - Or interactive setup via web UI if env vars not set

4. Seed blocklist:
   - If BETBLOCKER_COMMUNITY_FEED_URL is set: pull initial blocklist from community feed
   - Otherwise: use compiled-in seed list

5. Display setup summary:
   - API URL, admin credentials, CA fingerprint
   - Backup instructions for betblocker-keys volume
```

### Agent Binary Signing for Self-Hosted

This is the hardest problem in hosted/self-hosted parity. The agent binary must trust its API, and the API's identity is bound to cryptographic keys that differ between deployments.

**Solution: Trust-on-first-use (TOFU) with pinned configuration.**

1. **Hosted agents** have the BetBlocker hosted public keys compiled into the binary. They trust betblocker.com and only betblocker.com. These are the standard download from the website.

2. **Self-hosted agents** are built from the same source but with a different configuration step. The self-hosted operator runs:

```bash
# Generate a customized agent configuration package
docker compose exec api betblocker-cli agent-config export \
  --api-url https://my-betblocker.example.com \
  --output agent-config.json
```

This produces a signed configuration file containing:
- The self-hosted instance's API URL
- The instance's Blocklist Signing public key
- The instance's Device CA public key
- The instance's API TLS certificate fingerprint
- A configuration signature (signed by the instance's Root CA)

3. The agent binary is the same for all deployments. On first launch, if no configuration is present, the agent prompts for either:
   - An enrollment token (which encodes the API URL and fetches the configuration automatically), OR
   - A path to the `agent-config.json` file (for air-gapped setups)

4. Once configured, the agent pins the API's public keys and refuses to connect to any other API. Re-pointing an agent to a different API requires re-enrollment.

**Why not compile per-operator binaries:**
- Compiling Rust binaries per self-hosted operator is logistically impossible at scale.
- Binary signing (Authenticode, Apple notarization) is done once by BetBlocker, not per operator.
- The TOFU approach means the same signed binary works for all deployments, with trust established at enrollment time.

### Blocklist Feed Architecture

```
+---------------------------+
|  BetBlocker Hosted API    |
|  (source of truth for     |
|   curated blocklist)      |
+-----+---------------------+
      |
      | Publishes to:
      v
+---------------------------+
|  Community Feed           |
|  feed.betblocker.org      |
|  (public, signed, free)   |
+-----+---------------------+
      |
      | Consumed by:
      v
+---------------------------+     +---------------------------+
|  Self-Hosted Instance A   |     |  Self-Hosted Instance B   |
|  (syncs community feed)   |     |  (syncs community feed    |
|  (adds local entries)     |     |   + own curated entries)  |
+---------------------------+     +---------------------------+
      |                                 |
      | Agents sync from               | Agents sync from
      | Instance A's API               | Instance B's API
      v                                v
  [Agents]                          [Agents]
```

**Community feed properties:**
- Published every 6 hours by the hosted platform
- Contains all entries with confidence >= 0.9 and source in {Curated, AutomatedApproved, FederatedApproved}
- Signed with the BetBlocker community feed signing key (Ed25519)
- Uses the same delta sync protocol as agent-to-API sync (ADR-003)
- Publicly accessible, no authentication required (the blocklist itself is not secret)
- Available at `https://feed.betblocker.org/v1/sync?from_version=N`

**Self-hosted blocklist layering:**

Self-hosted instances maintain two blocklist sources:
1. **Community feed entries** (synced from betblocker.org, read-only locally)
2. **Local entries** (added by the self-hosted operator via admin panel)

The agent receives a unified blocklist from its API. It does not know which entries are community vs local. The API merges them and signs the combined list with the instance's Blocklist Signing Key.

### Federated Report Contribution (Self-Hosted)

Self-hosted operators can opt in to contributing federated reports upstream:

```env
BETBLOCKER_FEDERATED_REPORT_UPSTREAM=https://api.betblocker.org/v1/reports
BETBLOCKER_FEDERATED_REPORT_API_KEY=<issued by BetBlocker>
```

When enabled:
1. Agents submit federated reports to their own self-hosted API (as normal).
2. The self-hosted worker periodically batches these reports and forwards them upstream to the BetBlocker hosted API.
3. Forwarded reports are fully anonymized: no device ID, no instance ID, just domain + heuristic score + category + timestamp.
4. The upstream API deduplicates and processes them alongside reports from hosted agents.

This creates a network effect: more self-hosted instances contributing reports improves the community feed for everyone.

## Alternatives Considered

### Separate Codebases (Fork for Self-Hosted)

**Pros:** Maximum freedom to diverge. Self-hosted can have a simpler architecture.

**Rejected because:** Forks drift. Within 6 months, the self-hosted fork would be missing features, security patches, and blocklist improvements. Maintaining two codebases doubles engineering effort. The whole point of the "one artifact" principle is to avoid this.

### Build-Time Feature Flags (Compile Two Binaries)

**Pros:** Dead code elimination. The self-hosted binary would be slightly smaller without Stripe integration code.

**Rejected because:**
- Two binaries means two CI pipelines, two sets of container images, two release processes.
- The code size difference is trivial (Stripe client is ~1,000 lines; the entire API binary is ~20 MB).
- Runtime feature flags allow operators to enable/disable features without rebuilding, which is important for edge cases (e.g., a self-hosted operator who WANTS billing for their therapy practice).

### Agent Compiled Per Operator (Custom Binaries)

**Pros:** Maximum trust anchoring. Each operator's agents would have their public keys baked in, like the hosted binary.

**Rejected because:**
- Rust compilation takes 10-20 minutes per target. With 9 targets, that is 90-180 minutes per operator per release. Not scalable.
- Binary signing must be done by BetBlocker (Apple notarization, WHQL). Per-operator signing is legally and logistically impossible.
- The TOFU approach achieves equivalent security: trust is established at enrollment time and pinned thereafter. The only attack window is the initial enrollment, which requires physical access to the device and the enrollment token.

### No Community Feed (Self-Hosted is Fully Independent)

**Pros:** Simpler. Self-hosted operators curate their own blocklist.

**Rejected because:** Curating a gambling blocklist is a full-time job. Self-hosted operators (therapy practices, small courts) do not have the resources. The community feed is the primary value proposition of the self-hosted deployment: you get a professionally curated blocklist for free.

## Consequences

### What becomes easier

- **Single release process.** One set of container images is built, tested, signed, and published. Self-hosted operators pull the same images from the same registry.
- **Self-hosted onboarding.** `docker compose up` with a `.env` file is the entire setup. No compilation, no custom builds, no complex configuration.
- **Feature experimentation.** New features can be developed behind feature flags, tested on hosted, and then enabled for self-hosted when stable.
- **Community contribution.** Self-hosted instances that opt into federated reporting improve the blocklist for everyone, creating a virtuous cycle.

### What becomes harder

- **Feature flag discipline.** Every hosted-only feature must be cleanly gated. A missed flag check could expose billing UI to self-hosted operators or, worse, break the self-hosted deployment. Mitigation: integration tests that run the full test suite with `BETBLOCKER_DEPLOYMENT=self-hosted`.
- **Key management UX for self-hosted.** Operators must safeguard their `betblocker-keys` volume. If lost, all devices must re-enroll. Mitigation: the setup wizard prominently warns about backup, and the CLI provides a `betblocker-cli backup-keys` command.
- **Agent trust bootstrapping.** The TOFU model means the first enrollment is a trust decision. If an attacker intercepts the enrollment token and the API URL, they could point the agent at a malicious API. Mitigation: enrollment tokens are 256-bit random, 24-hour expiry, single-use. The attack requires both the token AND the ability to MITM the HTTPS connection to the legitimate API.
- **Version compatibility.** Self-hosted operators may run older versions of the API while their agents auto-update (or vice versa). The API must maintain backward compatibility for at least 2 major versions. Mitigation: API versioning (`/api/v1/`, `/api/v2/`) with graceful deprecation.

## Implementation Notes

### Phase 1 Deliverables

- [ ] docker-compose.yml with all 6 containers
- [ ] Environment-based feature flag system (`BETBLOCKER_DEPLOYMENT` + individual overrides)
- [ ] First-run setup wizard (generate keys, run migrations, create admin, seed blocklist)
- [ ] `betblocker-cli agent-config export` for self-hosted agent configuration
- [ ] Community feed endpoint on hosted platform (public, signed, delta sync)
- [ ] Community feed consumer in self-hosted worker (configurable sync interval)
- [ ] Hosted Kubernetes manifests (Helm chart)
- [ ] Agent TOFU enrollment with API URL pinning
- [ ] Integration test suite running with both `hosted` and `self-hosted` deployment modes

### Phase 2 Additions

- [ ] Helm chart for self-hosted Kubernetes deployments
- [ ] Federated report upstream contribution (opt-in for self-hosted)
- [ ] `betblocker-cli backup-keys` and `betblocker-cli restore-keys` commands
- [ ] Operator dashboard showing community feed sync status and local entry count
- [ ] Automated health checks (is the community feed reachable? are certificates expiring?)

### Versioning Policy

- Container images are tagged with semver: `ghcr.io/betblocker/betblocker-api:1.2.3`
- `latest` tag always points to the most recent stable release
- Self-hosted operators can pin to a specific version or use `latest`
- The API advertises its version in response headers; the agent warns if the API is older than the agent's minimum compatible version
- Breaking changes to the agent-API protocol require a new API version (`/api/v2/`) and a migration period of at least 6 months
