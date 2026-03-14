# Phase 2 Sub-Plan 2: Intelligence Pipeline (Discovery + Federated Reporting)

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Build the automated domain discovery pipeline (crawlers, classifier, scoring, review queue) and the federated reporting system (agent-side anonymized reports, ingestion, aggregation, auto-promotion). Both pipelines converge on a shared review queue.
**Architecture:** Discovery runs in `bb-worker` as background jobs. Federated reporting has agent-side components in `bb-agent-core` and server-side ingestion in `bb-api` + aggregation in `bb-worker`. Shared review queue with admin UI.
**Tech Stack:** Rust, reqwest, scraper, tokio-cron-scheduler, axum, sqlx, PostgreSQL, TypeScript/Next.js
**Depends on:** Phase 1 complete (agent core, API, blocklist, events, web dashboard)

**Reference Docs:**
- Phase 2 Design (sections 3-4): `docs/plans/2026-03-13-phase2-design.md`
- Master Plan: `docs/superpowers/plans/2026-03-13-phase2-master-plan.md`
- Existing enums: `crates/bb-common/src/enums.rs`
- Event system: `crates/bb-agent-core/src/events/`
- API routes: `crates/bb-api/src/routes/mod.rs`
- API services: `crates/bb-api/src/services/mod.rs`

---

## File Structure

```
crates/
  bb-common/src/
    enums.rs                              # Add DiscoveryCandidateStatus, CrawlerSource enums
    models/
      discovery_candidate.rs              # DiscoveryCandidate model
      federated_report.rs                 # FederatedReport, FederatedAggregate models
  bb-worker/src/
    main.rs                               # Extend with job scheduler
    scheduler.rs                          # Tokio-cron job scheduler framework
    discovery/
      mod.rs                              # Discovery pipeline orchestrator
      crawler.rs                          # DomainCrawler trait + CrawlerScheduler
      crawlers/
        mod.rs                            # Crawler registry
        affiliate.rs                      # Gambling affiliate directory crawler
        registry.rs                       # License registry crawler (UKGC, MGA, Curacao)
        whois.rs                          # WHOIS pattern crawler
        dns_zone.rs                       # DNS zone monitoring (.bet, .casino, .poker)
        search.rs                         # Search engine results crawler
      classifier.rs                       # ContentClassifier trait + RuleBasedClassifier
      scorer.rs                           # ConfidenceScorer (combines classifier outputs)
      queue.rs                            # Review queue job processing
    federated/
      mod.rs                              # Federated aggregation pipeline
      aggregator.rs                       # Dedup, threshold, classifier routing
      promoter.rs                         # Auto-promotion logic
  bb-agent-core/src/
    federated/
      mod.rs                              # FederatedReporter (batching, submission)
      anonymizer.rs                       # k-anonymity: rotating tokens, temporal bucketing
  bb-api/src/
    routes/
      review_queue.rs                     # Review queue CRUD endpoints
      federated.rs                        # Federated report ingestion (IP-stripped)
    services/
      review_queue_service.rs             # Review queue business logic
      federated_service.rs                # Federated report storage + aggregation triggers
    middleware/
      ip_strip.rs                         # Middleware to strip source IP before handler
migrations/
  NNNN_discovery_candidates.sql           # discovery_candidates table
  NNNN_federated_reports.sql              # federated_reports + federated_aggregates tables
web/src/app/admin/review-queue/
  page.tsx                                # Review queue list page
  [id]/page.tsx                           # Review queue detail/action page
```

---

## Chunk 1: Enums, Models, and Database Schema (~120 lines)

### Task 1: Discovery and federated enums

**Crate:** `bb-common`
**File:** `crates/bb-common/src/enums.rs`

- [ ] **Step 1:** Add `DiscoveryCandidateStatus` enum: `Pending`, `Approved`, `Rejected`, `Deferred`
- [ ] **Step 2:** Add `CrawlerSource` enum: `Affiliate`, `LicenseRegistry`, `WhoisPattern`, `DnsZone`, `SearchEngine`, `Federated`
- [ ] **Step 3:** Add `FederatedAggregateStatus` enum: `Collecting`, `ThresholdMet`, `Reviewing`, `Promoted`, `Rejected`
- [ ] **Step 4:** Write unit tests verifying serde round-trip for all three enums

### Task 2: Discovery candidate model

**File:** `crates/bb-common/src/models/discovery_candidate.rs`

- [ ] **Step 1:** Create `DiscoveryCandidate` struct with fields: `id: i64`, `domain: String`, `source: CrawlerSource`, `source_metadata: serde_json::Value`, `confidence_score: f64`, `classification: serde_json::Value`, `status: DiscoveryCandidateStatus`, `reviewed_by: Option<i64>`, `reviewed_at: Option<DateTime<Utc>>`, `created_at: DateTime<Utc>`
- [ ] **Step 2:** Register in `crates/bb-common/src/models/mod.rs`
- [ ] **Step 3:** Write tests for default values and serialization

### Task 3: Federated report and aggregate models

**File:** `crates/bb-common/src/models/federated_report.rs`

- [ ] **Step 1:** Create `FederatedReport` struct: `id: i64`, `domain: String`, `reporter_token: String`, `heuristic_score: f64`, `category_guess: Option<String>`, `reported_at: DateTime<Utc>`, `batch_id: Uuid`, `created_at: DateTime<Utc>`
- [ ] **Step 2:** Create `FederatedAggregate` struct: `id: i64`, `domain: String`, `unique_reporters: i32`, `avg_heuristic_score: f64`, `first_reported_at: DateTime<Utc>`, `last_reported_at: DateTime<Utc>`, `status: FederatedAggregateStatus`, `discovery_candidate_id: Option<i64>`, `created_at: DateTime<Utc>`, `updated_at: DateTime<Utc>`
- [ ] **Step 3:** Register in models mod.rs, write serde tests

### Task 4: Database migrations

- [ ] **Step 1:** Create `discovery_candidates` migration matching the schema from the design doc (see `docs/plans/2026-03-13-phase2-design.md` section 3). Include indexes on `status`, `domain`, and unique index on `(domain, source)`
- [ ] **Step 2:** Create `federated_reports` migration with indexes on `domain`. Include `federated_aggregates` table with unique index on `domain` and FK to `discovery_candidates`
- [ ] **Step 3:** Run migrations, verify with `sqlx` offline mode check

---

## Chunk 2: Crawler Framework and First Crawler (~130 lines)

### Task 5: Worker job scheduler

**Crate:** `bb-worker`
**File:** `crates/bb-worker/src/scheduler.rs`

- [ ] **Step 1:** Add deps to `crates/bb-worker/Cargo.toml`: `tokio-cron-scheduler`, `reqwest` (with `rustls-tls`), `scraper`, `sqlx`, `serde_json`, `chrono`, `uuid`, `tracing`
- [ ] **Step 2:** Create `JobScheduler` struct wrapping `tokio_cron_scheduler::JobScheduler`. Methods: `new()`, `add_job(name, cron_expr, handler)`, `start()`. Each handler is an `async fn` receiving a shared `AppContext` (DB pool, config)
- [ ] **Step 3:** Create `AppContext` struct holding `sqlx::PgPool`, rate limiter config, and `reqwest::Client`
- [ ] **Step 4:** Update `crates/bb-worker/src/main.rs`: initialize DB pool, build `AppContext`, create `JobScheduler`, register discovery jobs, call `start()`, await shutdown signal
- [ ] **Step 5:** Write test: scheduler can register and trigger a no-op job

### Task 6: Crawler trait and rate limiter

**File:** `crates/bb-worker/src/discovery/crawler.rs`

- [ ] **Step 1:** Define `DomainCrawler` trait:

```rust
#[async_trait]
pub trait DomainCrawler: Send + Sync {
    fn name(&self) -> &str;
    fn source(&self) -> CrawlerSource;
    async fn crawl(&self, ctx: &CrawlContext) -> Result<Vec<CrawlResult>, CrawlError>;
}

pub struct CrawlContext {
    pub http: reqwest::Client,
    pub rate_limiter: RateLimiter,
    pub last_run: Option<DateTime<Utc>>,
}

pub struct CrawlResult {
    pub domain: String,
    pub source_metadata: serde_json::Value,
}
```

- [ ] **Step 2:** Implement `RateLimiter` using a token-bucket algorithm (wrap `governor` crate). Config: `requests_per_second: u32`, `burst_size: u32`. Method: `async fn acquire(&self)` blocks until a token is available
- [ ] **Step 3:** Create `CrawlerScheduler` that holds `Vec<Box<dyn DomainCrawler>>`, iterates through each crawler on its schedule, stores `CrawlResult`s into `discovery_candidates` table with status `Pending`
- [ ] **Step 4:** Write tests: rate limiter enforces delay; CrawlerScheduler deduplicates domains already in the DB (uses `ON CONFLICT DO NOTHING` on `(domain, source)` unique index)

### Task 7: Affiliate directory crawler

**File:** `crates/bb-worker/src/discovery/crawlers/affiliate.rs`

- [ ] **Step 1:** Implement `AffiliateCrawler` struct with config: `seed_urls: Vec<String>`, `max_depth: u32`, `link_selector: String` (CSS selector for extracting links)
- [ ] **Step 2:** Implement `DomainCrawler` for `AffiliateCrawler`: fetch each seed URL with `reqwest`, parse HTML with `scraper`, extract outbound domains using the configured CSS selector, deduplicate, return as `CrawlResult`s
- [ ] **Step 3:** Handle errors gracefully: HTTP timeouts (30s), non-200 responses (skip with warning), parse failures (skip with warning). Use `tracing::warn!` for each
- [ ] **Step 4:** Write tests using `wiremock` to mock HTTP responses. Test: correct domains extracted from sample HTML; rate limiter respected between requests; non-200 responses skipped

### Task 8: Discovery module wiring

**File:** `crates/bb-worker/src/discovery/mod.rs`

- [ ] **Step 1:** Create `DiscoveryPipeline` struct that holds a `CrawlerScheduler`, a `ContentClassifier`, and a `ConfidenceScorer`
- [ ] **Step 2:** Implement `run_cycle()`: execute crawler, classify results, score, insert/update `discovery_candidates`
- [ ] **Step 3:** Register `DiscoveryPipeline::run_cycle` as a cron job in `main.rs` (default: every 6 hours)
- [ ] **Step 4:** Write integration test: mock crawler returns domains, pipeline stores them in DB with correct status

---

## Chunk 3: Content Classifier and Confidence Scoring (~110 lines)

### Task 9: Content classifier trait and rule-based implementation

**File:** `crates/bb-worker/src/discovery/classifier.rs`

- [ ] **Step 1:** Define `ContentClassifier` trait:

```rust
#[async_trait]
pub trait ContentClassifier: Send + Sync {
    async fn classify(&self, domain: &str, ctx: &ClassifyContext) -> Result<Classification, ClassifyError>;
}

pub struct ClassifyContext {
    pub http: reqwest::Client,
}

pub struct Classification {
    pub keyword_score: f64,       // 0.0-1.0
    pub structure_score: f64,     // 0.0-1.0
    pub link_graph_score: f64,    // 0.0-1.0
    pub category_guess: Option<GamblingCategory>,
    pub evidence: serde_json::Value,
}
```

- [ ] **Step 2:** Implement `RuleBasedClassifier`. The `classify` method: (a) fetch the domain's homepage via HTTP, (b) run keyword analysis, (c) run HTML structure analysis, (d) run link graph analysis, (e) combine into `Classification`
- [ ] **Step 3:** Implement keyword density analysis: define gambling keyword lists (weighted), count occurrences normalized by total word count. Keywords: "bet", "wager", "odds", "casino", "poker", "slots", "deposit bonus", "free spins", "responsible gambling", "18+", etc. Higher weight for domain-specific terms ("deposit bonus" > "bet")
- [ ] **Step 4:** Implement HTML structure analysis: look for gambling-specific elements via CSS selectors -- betting slip forms, odds tables (`<table>` with numeric patterns like "2.5", "1/4"), deposit forms (input fields near "deposit"/"withdraw" text), responsible gambling footer links
- [ ] **Step 5:** Implement link graph analysis: extract all outbound links, count how many link to known gambling domains (query `blocklist_entries` table). Score = `gambling_links / total_links`, capped at 1.0
- [ ] **Step 6:** Write tests for each sub-analyzer with sample HTML fixtures. Test keyword scorer with known gambling page HTML vs. non-gambling page. Test structure scorer detects odds tables. Test link graph scorer against known domain list

### Task 10: Confidence scoring engine

**File:** `crates/bb-worker/src/discovery/scorer.rs`

- [ ] **Step 1:** Implement `ConfidenceScorer` struct with configurable weights: `keyword_weight: f64` (default 0.4), `structure_weight: f64` (default 0.3), `link_graph_weight: f64` (default 0.3)
- [ ] **Step 2:** `score(classification: &Classification) -> f64`: weighted average of the three sub-scores, clamped to 0.0-1.0
- [ ] **Step 3:** Implement threshold logic: `auto_discard_threshold: f64` (default 0.3), `review_threshold: f64` (default 0.5), `high_priority_threshold: f64` (default 0.85). Method `categorize(score: f64) -> ScoreCategory` returning `Discard`, `StandardReview`, or `HighPriority`
- [ ] **Step 4:** Write tests: weighted scoring is correct; boundary cases (all zeros, all ones, mixed); threshold categorization works at boundaries

---

## Chunk 4: Review Queue (API + Service + Admin UI) (~130 lines)

### Task 11: Review queue service

**File:** `crates/bb-api/src/services/review_queue_service.rs`

- [ ] **Step 1:** Create `ReviewQueueService` with methods: `list(filters, pagination) -> PaginatedResponse<DiscoveryCandidate>`, `get(id) -> DiscoveryCandidate`, `approve(id, reviewer_id, category) -> Result`, `reject(id, reviewer_id) -> Result`, `defer(id, reviewer_id) -> Result`, `bulk_approve(ids, reviewer_id, category) -> Result`, `bulk_reject(ids, reviewer_id) -> Result`
- [ ] **Step 2:** `approve()` logic: update `discovery_candidates` status to `Approved`, set `reviewed_by`/`reviewed_at`, insert domain into `blocklist_entries` with source `Automated` and the given category
- [ ] **Step 3:** `reject()` logic: update status to `Rejected`, set reviewer fields. Optionally add to an allowlist table to prevent re-crawling
- [ ] **Step 4:** `defer()` logic: update status to `Deferred`, set reviewer fields
- [ ] **Step 5:** Implement `ListFilters`: `status: Option<DiscoveryCandidateStatus>`, `source: Option<CrawlerSource>`, `min_confidence: Option<f64>`, `search: Option<String>` (domain substring match), `sort_by: Option<String>` (default: `confidence_score DESC`)
- [ ] **Step 6:** Register in `crates/bb-api/src/services/mod.rs`
- [ ] **Step 7:** Write unit tests with mock DB: list filters, approve inserts into blocklist, reject updates status, bulk operations

### Task 12: Review queue API routes

**File:** `crates/bb-api/src/routes/review_queue.rs`

- [ ] **Step 1:** Create request/response types: `ReviewQueueFilters` (query params), `ApproveRequest { category: GamblingCategory }`, `BulkActionRequest { ids: Vec<i64>, category: Option<GamblingCategory> }`
- [ ] **Step 2:** Implement handlers (all require `RequireAdmin`): `list_review_queue(State, Query<Filters>, Pagination)`, `get_review_item(State, Path<id>)`, `approve_item(State, Path<id>, Json<ApproveRequest>)`, `reject_item(State, Path<id>)`, `defer_item(State, Path<id>)`, `bulk_approve(State, Json<BulkActionRequest>)`, `bulk_reject(State, Json<BulkActionRequest>)`
- [ ] **Step 3:** Register routes in `crates/bb-api/src/routes/mod.rs` under `/v1/admin/review-queue`
- [ ] **Step 4:** Write integration tests: list with filters returns correct items; approve creates blocklist entry; reject updates status; unauthorized requests return 403

### Task 13: Review queue admin UI

**Directory:** `web/src/app/admin/review-queue/`

- [ ] **Step 1:** Create `page.tsx`: paginated table of discovery candidates. Columns: domain, source, confidence score, category guess, status, created_at. Filters: status dropdown, source dropdown, min confidence slider, domain search. Sort by confidence (default), date
- [ ] **Step 2:** Add row actions: Approve (opens category picker modal), Reject, Defer. Bulk select with checkbox column + bulk Approve/Reject buttons
- [ ] **Step 3:** Create `[id]/page.tsx`: detail view showing full classification evidence (keyword matches, structure hits, link graph), source metadata, and action buttons
- [ ] **Step 4:** Add "Review Queue" link to admin sidebar navigation
- [ ] **Step 5:** Write component tests: table renders, filters work, approve action calls API

---

## Chunk 5: Additional Crawlers (~100 lines)

### Task 14: License registry crawler

**File:** `crates/bb-worker/src/discovery/crawlers/registry.rs`

- [ ] **Step 1:** Implement `RegistryCrawler` struct with config: `registries: Vec<RegistryConfig>` where `RegistryConfig` has `name`, `url`, `parser` (enum: `HtmlTable`, `JsonApi`, `Csv`)
- [ ] **Step 2:** Implement `DomainCrawler` trait: fetch registry page, parse based on parser type, extract licensed operator domains
- [ ] **Step 3:** Add initial registry configs for UKGC (HTML table parser), MGA (HTML), Curacao (HTML)
- [ ] **Step 4:** Write tests with mock HTML fixtures for each registry format

### Task 15: WHOIS pattern crawler

**File:** `crates/bb-worker/src/discovery/crawlers/whois.rs`

- [ ] **Step 1:** Implement `WhoisCrawler` struct with config: `known_registrants: Vec<String>` (email patterns, org names of known gambling operators), `tlds: Vec<String>`
- [ ] **Step 2:** Implement `DomainCrawler`: query WHOIS data for recently registered domains matching known registrant patterns. Use a WHOIS API service (configurable endpoint) rather than raw WHOIS protocol
- [ ] **Step 3:** Write tests with mock WHOIS API responses

### Task 16: DNS zone monitoring crawler

**File:** `crates/bb-worker/src/discovery/crawlers/dns_zone.rs`

- [ ] **Step 1:** Implement `DnsZoneCrawler` for gambling-specific TLDs: `.bet`, `.casino`, `.poker`, `.games`, `.bingo`
- [ ] **Step 2:** Implement `DomainCrawler`: fetch zone files (where available via CZDS) or use DNS enumeration services. Track new domains since last run via `last_run` in `CrawlContext`
- [ ] **Step 3:** Write tests with mock zone file data

### Task 17: Search engine crawler

**File:** `crates/bb-worker/src/discovery/crawlers/search.rs`

- [ ] **Step 1:** Implement `SearchCrawler` with config: `queries: Vec<String>` (gambling keyword queries), `engine: SearchEngine` (enum: `Google`, `Bing`, `DuckDuckGo`), `results_per_query: u32`
- [ ] **Step 2:** Implement `DomainCrawler`: execute search queries via search API (SerpApi or similar), extract result domains, deduplicate against existing blocklist
- [ ] **Step 3:** Rate limit aggressively (1 query per 10 seconds default) to avoid API bans
- [ ] **Step 4:** Write tests with mock search API responses

### Task 18: Crawler registry and wiring

**File:** `crates/bb-worker/src/discovery/crawlers/mod.rs`

- [ ] **Step 1:** Create `build_crawlers(config: &WorkerConfig) -> Vec<Box<dyn DomainCrawler>>` that instantiates enabled crawlers from config
- [ ] **Step 2:** Wire all crawlers into `DiscoveryPipeline` in `main.rs`
- [ ] **Step 3:** Write integration test: all crawlers can be instantiated and return empty results when mocked endpoints return no data

---

## Chunk 6: Agent-Side Federated Reporting (~110 lines)

### Task 19: Rotating token anonymizer

**Crate:** `bb-agent-core`
**File:** `crates/bb-agent-core/src/federated/anonymizer.rs`

- [ ] **Step 1:** Implement `TokenRotator` struct. Stores a seed (random bytes generated once, persisted to agent's local config). Generates a pseudonym token by hashing `HMAC-SHA256(seed, date_string)` where `date_string` is the current UTC date (`YYYY-MM-DD`). Token rotates daily automatically
- [ ] **Step 2:** Implement `TemporalBucketer`: method `bucket(timestamp: DateTime<Utc>) -> DateTime<Utc>` that rounds down to the nearest hour boundary
- [ ] **Step 3:** Write tests: same seed + same day = same token; same seed + different day = different token; temporal bucketing rounds correctly (e.g., 14:37 -> 14:00)

### Task 20: Federated reporter

**File:** `crates/bb-agent-core/src/federated/mod.rs`

- [ ] **Step 1:** Create `FederatedReport` struct (agent-side): `domain: String`, `heuristic_score: f64`, `category_guess: Option<String>`, `reporter_token: String`, `reported_at: DateTime<Utc>` (bucketed), `batch_id: Uuid`
- [ ] **Step 2:** Implement `FederatedReporter` struct with config: `batch_interval: Duration` (default 6 hours), `min_batch_size: usize` (default 1), `enabled: bool`
- [ ] **Step 3:** Implement report collection: `add_report(domain, heuristic_score, category_guess)` -- applies token rotation and temporal bucketing, queues report in memory buffer
- [ ] **Step 4:** Implement batch submission: `flush() -> Result<(), FederatedError>` -- collects buffered reports, assigns a common `batch_id` (UUIDv4), serializes as JSON array, POSTs to `POST /v1/federated/reports`
- [ ] **Step 5:** Implement periodic flush: `run(interval: Duration)` spawns a tokio task that calls `flush()` every `batch_interval`
- [ ] **Step 6:** Wire into agent event pipeline: when `EventEmitter` emits a `DnsHeuristicMatch` event (new `EventType` variant needed if not present), the `FederatedReporter` picks it up
- [ ] **Step 7:** Write tests: reports are anonymized (no device_id in output); batch submission serializes correctly; flush clears the buffer; reports accumulate between flushes

---

## Chunk 7: Federated Ingestion API (~80 lines)

### Task 21: IP-stripping middleware

**File:** `crates/bb-api/src/middleware/ip_strip.rs`

- [ ] **Step 1:** Implement `StripSourceIp` Axum middleware layer. Before the handler runs, remove `X-Forwarded-For`, `X-Real-Ip`, and `Forwarded` headers from the request. Do not store the connection IP in any request extension
- [ ] **Step 2:** Ensure the middleware is applied only to federated report routes (not globally)
- [ ] **Step 3:** Write test: request with `X-Forwarded-For` header passes through middleware with header removed; non-federated routes still have headers

### Task 22: Federated ingestion endpoint

**File:** `crates/bb-api/src/routes/federated.rs`

- [ ] **Step 1:** Define `IngestReportRequest`: `reports: Vec<ReportPayload>` where `ReportPayload` has `domain: String`, `reporter_token: String`, `heuristic_score: f64`, `category_guess: Option<String>`, `reported_at: String` (ISO 8601), `batch_id: Uuid`
- [ ] **Step 2:** Implement `POST /v1/federated/reports` handler (unauthenticated -- agents submit without device auth to preserve anonymity). Validate payload (domain format, score range 0.0-1.0, batch_id present). Return 202 Accepted
- [ ] **Step 3:** Apply `StripSourceIp` middleware to this route group in `crates/bb-api/src/routes/mod.rs`
- [ ] **Step 4:** Write tests: valid payload returns 202; invalid domain format returns 400; missing fields return 422; IP headers are stripped

### Task 23: Federated storage service

**File:** `crates/bb-api/src/services/federated_service.rs`

- [ ] **Step 1:** Implement `FederatedService` with method `ingest(reports: Vec<ReportPayload>) -> Result<usize>`. Insert each report into `federated_reports` table. Return count of inserted reports
- [ ] **Step 2:** On each ingest, update `federated_aggregates`: upsert by domain, increment `unique_reporters` (use `SELECT COUNT(DISTINCT reporter_token)` from `federated_reports`), update `avg_heuristic_score`, update `last_reported_at`
- [ ] **Step 3:** If `unique_reporters` crosses the threshold (configurable, default 5), update aggregate status to `ThresholdMet`
- [ ] **Step 4:** Register in `crates/bb-api/src/services/mod.rs`
- [ ] **Step 5:** Write tests: ingest stores reports; aggregate counts are correct; threshold transition fires

---

## Chunk 8: Aggregation Pipeline and Auto-Promotion (~100 lines)

### Task 24: Aggregation worker job

**File:** `crates/bb-worker/src/federated/aggregator.rs`

- [ ] **Step 1:** Implement `FederatedAggregator` as a scheduled job (cron: every 15 minutes). Queries `federated_aggregates` where `status = 'threshold_met'`
- [ ] **Step 2:** For each threshold-met domain: run through `ContentClassifier` (same classifier from chunk 3). Update `classification` in the linked `discovery_candidates` row (create one if not exists via upsert with source `Federated`)
- [ ] **Step 3:** Run through `ConfidenceScorer`. Update `confidence_score` on the `discovery_candidates` row
- [ ] **Step 4:** Update aggregate status to `Reviewing`. The domain now appears in the admin review queue
- [ ] **Step 5:** Write tests: threshold-met aggregates are classified and scored; discovery_candidate is created/linked; status transitions correctly

### Task 25: Auto-promotion logic

**File:** `crates/bb-worker/src/federated/promoter.rs`

- [ ] **Step 1:** Implement `AutoPromoter` as a scheduled job (cron: every 30 minutes). Config: `enabled: bool` (default false), `min_unique_reporters: i32` (default 10), `min_confidence: f64` (default 0.95), `max_domain_age_days: i32` (default 30)
- [ ] **Step 2:** Query: `federated_aggregates` with status `Reviewing`, joined to `discovery_candidates` where `confidence_score >= min_confidence` and `unique_reporters >= min_unique_reporters`
- [ ] **Step 3:** For each qualifying domain: check WHOIS domain age (query cached WHOIS data or fetch). If age > `max_domain_age_days`, skip (needs human review). Check domain is not on allowlist
- [ ] **Step 4:** Auto-promote: insert into `blocklist_entries` with source `Federated`, update `discovery_candidates` status to `Approved` (reviewer = system), update `federated_aggregates` status to `Promoted`
- [ ] **Step 5:** Log all auto-promotions with `tracing::info!` including domain, score, reporter count
- [ ] **Step 6:** Write tests: domain meeting all criteria is promoted; domain with low reporters is skipped; domain with high age is skipped; domain on allowlist is skipped; disabled promoter does nothing

### Task 26: Federated pipeline wiring

**File:** `crates/bb-worker/src/federated/mod.rs`

- [ ] **Step 1:** Create `FederatedPipeline` struct holding `FederatedAggregator` and `AutoPromoter`
- [ ] **Step 2:** Register both as cron jobs in `main.rs` scheduler
- [ ] **Step 3:** Write integration test: end-to-end flow -- reports ingested via API, aggregator classifies threshold-met domains, promoter auto-promotes qualifying domains to blocklist

### Task 27: Review queue integration

- [ ] **Step 1:** Update `ReviewQueueService.list()` to include federated-sourced candidates (source = `Federated`). Add `source` filter to distinguish discovery vs. federated items
- [ ] **Step 2:** Update review queue admin UI to show federated metadata: unique reporter count, avg heuristic score, first/last reported dates
- [ ] **Step 3:** When a federated candidate is approved/rejected via review queue, update both `discovery_candidates` and `federated_aggregates` status
- [ ] **Step 4:** Write tests: federated candidates appear in review queue; approval updates both tables; rejection updates both tables

---

## Definition of Done

- [ ] All crawlers run on schedule without crashing (affiliate, registry, WHOIS, DNS zone, search)
- [ ] Rule-based classifier produces reasonable scores on known gambling vs non-gambling sites
- [ ] Review queue shows candidates from both discovery and federated pipelines
- [ ] Admin can approve/reject/defer/bulk-action items in review queue
- [ ] Approved domains flow into `blocklist_entries` and will sync to agents
- [ ] Agents generate anonymized federated reports (rotating tokens, temporal bucketing, no device_id)
- [ ] Federated ingestion endpoint strips IP and stores reports
- [ ] Aggregation pipeline deduplicates, counts unique reporters, triggers classification at threshold
- [ ] Auto-promotion (when enabled) promotes high-confidence domains without human review
- [ ] All new code has unit tests; integration tests cover the full pipeline
- [ ] CI passes with all new tests
