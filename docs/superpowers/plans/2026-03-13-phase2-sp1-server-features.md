# Phase 2, Sub-Plan 1: Server-Side Features (Organizations + Advanced Reporting)

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement organization CRUD, member/device management, bulk enrollment tokens, TimescaleDB continuous aggregates, analytics API, trend analysis, report export, and enhanced dashboard UI.

**Depends on:** Phase 1 complete (accounts, devices, enrollments, events, partners, web dashboard)
**Blocks:** Nothing (independent of other Phase 2 sub-plans)
**Estimated tasks:** 22

**Reference Files:**
- Design doc: `docs/plans/2026-03-13-phase2-design.md` (sections 5, 6)
- Master plan: `docs/superpowers/plans/2026-03-13-phase2-master-plan.md`
- Existing org model: `crates/bb-common/src/models/organization.rs`
- Enums: `crates/bb-common/src/enums.rs` (OrganizationType, OrgMemberRole)
- Route patterns: `crates/bb-api/src/routes/mod.rs`, `crates/bb-api/src/routes/partners.rs`
- Service patterns: `crates/bb-api/src/services/partner_service.rs`
- API types: `web/src/lib/api-types.ts`
- API client: `web/src/lib/api-client.ts`
- Existing migrations: `migrations/0005_create_organizations.sql`, `migrations/0006_create_organization_members.sql`
- Events table: `migrations/0015_create_events.sql` (partitioned by month)

**Key Patterns (follow Phase 1 conventions):**
- Routes: Axum handlers using `State(state): State<AppState>`, `AuthenticatedAccount` extractor, `Pagination` extractor
- Services: Free functions taking `&PgPool`, returning `Result<T, ApiError>`, using `sqlx::query_as` with `FromRow` structs
- Models: `bb-common/src/models/*.rs` with `Serialize, Deserialize, Debug, Clone`
- Migrations: sequential `NNNN_description.sql` files in `migrations/`
- Web: Next.js App Router pages in `web/src/app/(dashboard)/`

---

## Chunk 1: Organization CRUD (Tasks 1-4)

### Task 1: Database migrations for org enhancements

- [ ] **1a.** Create `migrations/0022_extend_organizations.sql`: add `owner_id BIGINT NOT NULL REFERENCES accounts(id)`, `default_protection_config JSONB`, `default_reporting_config JSONB`, `default_unenrollment_policy JSONB` columns to `organizations` table. The existing table (migration 0005) has `id, public_id, name, type, created_at, updated_at` but lacks `owner_id` and config columns.
- [ ] **1b.** Create `migrations/0023_create_organization_devices.sql`: `organization_devices` table with `id BIGSERIAL PRIMARY KEY`, `organization_id BIGINT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE`, `device_id BIGINT NOT NULL REFERENCES devices(id) ON DELETE CASCADE`, `assigned_by BIGINT REFERENCES accounts(id)`, `assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`, `UNIQUE(organization_id, device_id)`.
- [ ] **1c.** Create `migrations/0024_create_enrollment_tokens.sql`: `enrollment_tokens` table with `id BIGSERIAL PRIMARY KEY`, `public_id UUID NOT NULL UNIQUE DEFAULT gen_random_uuid()`, `organization_id BIGINT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE`, `created_by BIGINT NOT NULL REFERENCES accounts(id)`, `label VARCHAR(200)`, `protection_config JSONB NOT NULL`, `reporting_config JSONB NOT NULL`, `unenrollment_policy JSONB NOT NULL`, `max_uses INT`, `uses_count INT NOT NULL DEFAULT 0`, `expires_at TIMESTAMPTZ`, `created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`.
- [ ] **1d.** Add `invited_by BIGINT REFERENCES accounts(id)` column to `organization_members` table via `migrations/0025_extend_org_members.sql` (missing from migration 0006).
- [ ] **1e.** Run migrations locally and verify schema: `cargo sqlx migrate run`.

### Task 2: Organization models and enums

- [ ] **2a.** Update `crates/bb-common/src/models/organization.rs`: add `owner_id`, `default_protection_config: Option<serde_json::Value>`, `default_reporting_config: Option<serde_json::Value>`, `default_unenrollment_policy: Option<serde_json::Value>` fields to match extended schema.
- [ ] **2b.** Create `crates/bb-common/src/models/org_member.rs`: `OrgMember` struct with `id, organization_id, account_id, role: OrgMemberRole, invited_by: Option<i64>, joined_at`. Register in `models/mod.rs`.
- [ ] **2c.** Create `crates/bb-common/src/models/org_device.rs`: `OrgDevice` struct with `id, organization_id, device_id, assigned_by: Option<i64>, assigned_at`. Register in `models/mod.rs`.
- [ ] **2d.** Create `crates/bb-common/src/models/enrollment_token.rs`: `EnrollmentToken` struct with all DB columns. Register in `models/mod.rs`.
- [ ] **2e.** Update `crates/bb-common/src/enums.rs`: add `Owner` variant to `OrgMemberRole` (currently only `Admin, Member`).

### Task 3: Organization service layer

- [ ] **3a.** Create `crates/bb-api/src/services/organization_service.rs`. Register in `services/mod.rs`. Follow the `partner_service.rs` pattern with `FromRow` row structs and free functions.
- [ ] **3b.** Implement `create_organization(db, name, org_type, owner_id) -> Result<OrgRow>`: INSERT into `organizations`, then INSERT owner into `organization_members` with role `owner` in a single transaction.
- [ ] **3c.** Implement `get_organization(db, public_id) -> Result<OrgRow>`: SELECT by `public_id`.
- [ ] **3d.** Implement `list_organizations_for_account(db, account_id, pagination) -> Result<(Vec<OrgRow>, i64)>`: JOIN through `organization_members` to find all orgs the account belongs to.
- [ ] **3e.** Implement `update_organization(db, org_id, name?, org_type?, default_configs?) -> Result<OrgRow>`: partial UPDATE.
- [ ] **3f.** Implement `delete_organization(db, org_id) -> Result<()>`: CASCADE delete (DB handles member/device/token cleanup).
- [ ] **3g.** Implement `check_org_permission(db, org_id, account_id, required_role) -> Result<OrgMemberRow>`: verify the account has the required role (owner > admin > member) in the org. Used as a guard in all org routes.
- [ ] **3h.** Write tests in `crates/bb-api/src/services/organization_service.rs` (or a `tests/` module) for each function using `sqlx::test` with a test database.

### Task 4: Organization API routes

- [ ] **4a.** Create `crates/bb-api/src/routes/organizations.rs`. Register in `routes/mod.rs`.
- [ ] **4b.** Define request structs: `CreateOrgRequest { name, org_type }`, `UpdateOrgRequest { name?, org_type?, default_protection_config?, default_reporting_config?, default_unenrollment_policy? }`.
- [ ] **4c.** Implement route handlers following `partners.rs` pattern:
  - `POST /v1/organizations` -> `create_org` (authenticated, creates org with caller as owner)
  - `GET /v1/organizations` -> `list_orgs` (authenticated, returns caller's orgs)
  - `GET /v1/organizations/{id}` -> `get_org` (authenticated, check membership)
  - `PATCH /v1/organizations/{id}` -> `update_org` (require admin+ role)
  - `DELETE /v1/organizations/{id}` -> `delete_org` (require owner role)
- [ ] **4d.** Register routes in `crates/bb-api/src/routes/mod.rs` under `/v1/organizations` nest, following the existing pattern.
- [ ] **4e.** Write integration tests for all CRUD endpoints.
- [ ] **4f.** Verify: `cargo test` passes, `cargo clippy` clean.

---

## Chunk 2: Member Management (Tasks 5-7)

### Task 5: Member invitation service

- [ ] **5a.** Add to `organization_service.rs`: `invite_member(db, org_id, email, role, invited_by) -> Result<OrgMemberRow>`. Look up account by email, INSERT into `organization_members` with the specified role. Return error if already a member.
- [ ] **5b.** Add `list_members(db, org_id, pagination) -> Result<(Vec<OrgMemberRow>, i64)>`: JOIN with `accounts` to include `display_name`, `email`, `public_id`.
- [ ] **5c.** Add `update_member_role(db, org_id, member_account_id, new_role, caller_account_id) -> Result<OrgMemberRow>`: validate caller has higher role than target. Prevent demoting the sole owner.
- [ ] **5d.** Add `remove_member(db, org_id, member_account_id, caller_account_id) -> Result<()>`: DELETE from `organization_members`. Prevent removing sole owner. Also unassign the member's devices from the org.
- [ ] **5e.** Write tests for invite, list, update-role, remove scenarios.

### Task 6: Member API routes

- [ ] **6a.** Add member sub-routes to `organizations.rs`:
  - `POST /v1/organizations/{id}/members` -> `invite_member` (require admin+)
  - `GET /v1/organizations/{id}/members` -> `list_members` (require member+)
  - `PATCH /v1/organizations/{id}/members/{member_id}` -> `update_member_role` (require admin+)
  - `DELETE /v1/organizations/{id}/members/{member_id}` -> `remove_member` (require admin+)
- [ ] **6b.** Define `InviteMemberRequest { email, role }` and `UpdateMemberRoleRequest { role }` structs.
- [ ] **6c.** Register nested routes in the org router in `routes/mod.rs`.
- [ ] **6d.** Write integration tests for member invitation, role change, and removal.

### Task 7: Organization + member TypeScript types and API client

- [ ] **7a.** Add to `web/src/lib/api-types.ts`:
  - `OrganizationType = 'family' | 'therapy' | 'court' | 'enterprise'`
  - `OrgMemberRole = 'owner' | 'admin' | 'member'`
  - `Organization` interface (id, name, org_type, owner_id, default configs, created_at, updated_at)
  - `OrgMember` interface (id, account_id, display_name, email, role, joined_at)
- [ ] **7b.** Add `organizations` namespace to `web/src/lib/api-client.ts` with methods: `create`, `list`, `get`, `update`, `delete`, `inviteMember`, `listMembers`, `updateMemberRole`, `removeMember`. Follow existing `partners` namespace pattern.
- [ ] **7c.** Verify: TypeScript compiles with `npx tsc --noEmit`.

---

## Chunk 3: Device Assignment + Bulk Tokens (Tasks 8-10)

### Task 8: Device assignment service and routes

- [ ] **8a.** Add to `organization_service.rs`: `assign_device(db, org_id, device_id, assigned_by) -> Result<OrgDeviceRow>`. Verify device belongs to a member of the org. INSERT into `organization_devices`.
- [ ] **8b.** Add `unassign_device(db, org_id, device_id) -> Result<()>`: DELETE from `organization_devices`.
- [ ] **8c.** Add `list_org_devices(db, org_id, pagination) -> Result<(Vec<DeviceRow>, i64)>`: JOIN `organization_devices` with `devices` to return full device info.
- [ ] **8d.** Add `apply_org_defaults_to_enrollment(db, org_id, enrollment_id) -> Result<()>`: update enrollment's protection/reporting/unenrollment configs from org defaults where not already overridden.
- [ ] **8e.** Add device sub-routes to `organizations.rs`:
  - `POST /v1/organizations/{id}/devices` -> `assign_device` (admin+, body: `{ device_id }`)
  - `DELETE /v1/organizations/{id}/devices/{device_id}` -> `unassign_device` (admin+)
  - `GET /v1/organizations/{id}/devices` -> `list_org_devices` (member+)
- [ ] **8f.** Write tests for assign, unassign, list, and default-application scenarios.

### Task 9: Bulk enrollment token service and routes

- [ ] **9a.** Add to `organization_service.rs` (or create `enrollment_token_service.rs`):
  - `create_enrollment_token(db, org_id, created_by, label, configs, max_uses, expires_at) -> Result<TokenRow>`
  - `list_enrollment_tokens(db, org_id, pagination) -> Result<(Vec<TokenRow>, i64)>`
  - `get_enrollment_token(db, public_id) -> Result<TokenRow>` (public, for token redemption)
  - `revoke_enrollment_token(db, token_id) -> Result<()>` (set expires_at to now)
  - `redeem_enrollment_token(db, public_id, device_id) -> Result<Enrollment>`: increment `uses_count`, check `max_uses` and `expires_at`, create enrollment with token's configs, assign device to org.
- [ ] **9b.** Add routes:
  - `POST /v1/organizations/{id}/tokens` -> `create_token` (admin+)
  - `GET /v1/organizations/{id}/tokens` -> `list_tokens` (admin+)
  - `DELETE /v1/organizations/{id}/tokens/{token_id}` -> `revoke_token` (admin+)
  - `POST /v1/enroll/{token_public_id}` -> `redeem_token` (authenticated, public endpoint)
- [ ] **9c.** Write tests: create, list, redeem (success, expired, max-uses-exceeded), revoke.

### Task 10: QR code generation and org UI types

- [ ] **10a.** Add `qrcode` crate dependency to `crates/bb-api/Cargo.toml`. Implement `GET /v1/organizations/{id}/tokens/{token_id}/qr` -> returns PNG image of QR code encoding the enrollment URL `https://{base}/enroll/{token_public_id}`. Use `qrcode` + `image` crates; return `Content-Type: image/png`.
- [ ] **10b.** Add to `web/src/lib/api-types.ts`: `EnrollmentToken` interface, `OrgDevice` interface.
- [ ] **10c.** Add to `web/src/lib/api-client.ts`: `organizations.createToken`, `listTokens`, `revokeToken`, `getTokenQr` (returns blob URL), `assignDevice`, `unassignDevice`, `listDevices`, `redeemToken`.
- [ ] **10d.** Verify: `cargo test`, `cargo clippy`, `npx tsc --noEmit` all pass.

---

## Chunk 4: Organization UI (Tasks 11-12)

### Task 11: Organization management pages

- [ ] **11a.** Create `web/src/app/(dashboard)/organizations/page.tsx`: list user's orgs with create-org button. Use `organizations.list()` API call. Show name, type, member count, device count in a table.
- [ ] **11b.** Create `web/src/app/(dashboard)/organizations/[id]/page.tsx`: org detail page with tabs (Overview, Members, Devices, Tokens, Settings). Default to Overview tab showing summary stats.
- [ ] **11c.** Create `web/src/app/(dashboard)/organizations/[id]/members/page.tsx`: member list with invite form (email + role select), role-change dropdown, remove button. Follow the `partners` page pattern.
- [ ] **11d.** Create `web/src/app/(dashboard)/organizations/[id]/devices/page.tsx`: device list showing assigned devices with status, platform, last heartbeat. Add "assign device" modal that lists unassigned devices from the user's device list.
- [ ] **11e.** Create `web/src/app/(dashboard)/organizations/[id]/tokens/page.tsx`: token list with create form (label, max uses, expiry), QR code display (inline `<img>` from QR endpoint), copy-link button, revoke action.
- [ ] **11f.** Create `web/src/app/(dashboard)/organizations/[id]/settings/page.tsx`: form to edit org name, type, and default enrollment configs (protection config, reporting config, unenrollment policy). Reuse existing config form components from enrollment pages if available.

### Task 12: Organization navigation and layout

- [ ] **12a.** Create `web/src/app/(dashboard)/organizations/[id]/layout.tsx`: tab navigation layout for org sub-pages (Overview, Members, Devices, Tokens, Settings).
- [ ] **12b.** Add "Organizations" link to the dashboard sidebar navigation in `web/src/app/(dashboard)/layout.tsx`.
- [ ] **12c.** Verify: all org pages render, navigation works, API calls succeed against running backend.

---

## Chunk 5: TimescaleDB Aggregates + Analytics API (Tasks 13-16)

### Task 13: TimescaleDB continuous aggregate migrations

- [ ] **13a.** Create `migrations/0026_enable_timescaledb.sql`: `CREATE EXTENSION IF NOT EXISTS timescaledb;`. Convert the existing `events` partitioned table to a TimescaleDB hypertable: `SELECT create_hypertable('events', 'created_at', migrate_data => true, if_not_exists => true);`. Note: this replaces the manual monthly partitions with TimescaleDB's automatic chunking.
- [ ] **13b.** Create `migrations/0027_create_hourly_block_stats.sql`: continuous aggregate materialized view:
```sql
CREATE MATERIALIZED VIEW hourly_block_stats
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 hour', created_at) AS bucket,
    device_id,
    event_type,
    COUNT(*) AS event_count
FROM events
WHERE event_type IN ('block', 'bypass_attempt', 'tamper', 'app_block', 'vpn_detected')
GROUP BY bucket, device_id, event_type;

SELECT add_continuous_aggregate_policy('hourly_block_stats',
    start_offset => INTERVAL '3 hours',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour');
```
- [ ] **13c.** Create `migrations/0028_create_daily_block_stats.sql`: daily rollup continuous aggregate over `hourly_block_stats` using `time_bucket('1 day', bucket)` with `SUM(event_count)`.
- [ ] **13d.** Create `migrations/0029_create_analytics_trends.sql`: `analytics_trends` table with `id BIGSERIAL PRIMARY KEY`, `device_id BIGINT NOT NULL`, `metric_name TEXT NOT NULL`, `metric_value JSONB NOT NULL`, `computed_at TIMESTAMPTZ NOT NULL`, `period_start TIMESTAMPTZ NOT NULL`, `period_end TIMESTAMPTZ NOT NULL`. Add index on `(device_id, metric_name, computed_at DESC)`.
- [ ] **13e.** Run migrations, verify views are created and policies active.

### Task 14: Analytics models

- [ ] **14a.** Create `crates/bb-common/src/models/analytics.rs`: define `HourlyBlockStat { bucket, device_id, event_type, event_count }`, `DailyBlockStat { day, device_id, event_type, event_count }`, `AnalyticsTrend { id, device_id, metric_name, metric_value, computed_at, period_start, period_end }`. Register in `models/mod.rs`.

### Task 15: Analytics service with enrollment visibility

- [ ] **15a.** Create `crates/bb-api/src/services/analytics_service.rs`. Register in `services/mod.rs`.
- [ ] **15b.** Implement `get_hourly_stats(db, device_id, from, to) -> Result<Vec<HourlyStatRow>>`: query `hourly_block_stats` with time range filter.
- [ ] **15c.** Implement `get_daily_stats(db, device_id, from, to) -> Result<Vec<DailyStatRow>>`: query `daily_block_stats`.
- [ ] **15d.** Implement `get_trends(db, device_id, metric_names) -> Result<Vec<TrendRow>>`: query `analytics_trends` for latest computed values.
- [ ] **15e.** Implement `enforce_enrollment_visibility(db, caller_account_id, device_id) -> Result<ReportingConfig>`: check if caller owns the device, is a partner with `view_reports` permission, or is an org admin for the device's org. Return the effective `ReportingConfig` that governs what data the caller can see. Return 403 if no access.
- [ ] **15f.** Write tests for visibility enforcement: owner sees all, partner sees per config, org admin sees per config, stranger gets 403.

### Task 16: Analytics API routes

- [ ] **16a.** Create `crates/bb-api/src/routes/analytics.rs`. Register in `routes/mod.rs`.
- [ ] **16b.** Implement routes:
  - `GET /v1/analytics/timeseries` -> `get_timeseries`: query params `device_id, period (hour|day), from, to`. Enforce visibility, then query hourly or daily stats.
  - `GET /v1/analytics/trends` -> `get_trends`: query params `device_id, metrics (comma-separated)`. Return latest trend values.
  - `GET /v1/analytics/summary` -> `get_summary`: query params `device_id, from, to`. Return aggregate totals (total blocks, bypass attempts, tamper events, per-category breakdown).
  - `GET /v1/analytics/heatmap` -> `get_heatmap`: query params `device_id, from, to`. Return hour-of-day x day-of-week matrix of block counts, computed from hourly stats.
- [ ] **16c.** Register under `/v1/analytics` in `routes/mod.rs`.
- [ ] **16d.** Write integration tests for each endpoint with visibility checks.
- [ ] **16e.** Verify: `cargo test`, `cargo clippy` clean.

---

## Chunk 6: Trend Analysis Engine (Tasks 17-18)

### Task 17: bb-worker job framework

- [ ] **17a.** Add dependencies to `crates/bb-worker/Cargo.toml`: `sqlx`, `tokio-cron-scheduler` (or `apalis`), `serde_json`, `chrono`, `tracing`.
- [ ] **17b.** Update `crates/bb-worker/src/main.rs`: initialize DB pool, configure scheduler, register analytics jobs. Structure:
```rust
mod analytics;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let db = PgPool::connect(&std::env::var("DATABASE_URL")?).await?;
    let sched = JobScheduler::new().await?;
    analytics::register_jobs(&sched, db.clone()).await?;
    sched.start().await?;
    tokio::signal::ctrl_c().await?;
    Ok(())
}
```
- [ ] **17c.** Create `crates/bb-worker/src/analytics/mod.rs`: `register_jobs` function that schedules the trend analysis job to run every hour.

### Task 18: Trend analysis computations

- [ ] **18a.** Create `crates/bb-worker/src/analytics/trends.rs` with `compute_trends(db: &PgPool) -> Result<()>`. This iterates all active devices and computes the following metrics, storing results in `analytics_trends`:
- [ ] **18b.** Implement `compute_peak_hour(db, device_id, period_start, period_end)`: query `hourly_block_stats`, find the hour with the highest block count. Store as `metric_name = "peak_hour"`, `metric_value = {"hour": 22, "count": 15}`.
- [ ] **18c.** Implement `compute_day_of_week_pattern(db, device_id, period_start, period_end)`: query `daily_block_stats`, aggregate by day-of-week. Store as `metric_name = "dow_pattern"`, `metric_value = {"mon": 5, "tue": 3, ...}`.
- [ ] **18d.** Implement `compute_category_distribution(db, device_id, period_start, period_end)`: query events with category metadata from `events.metadata->>'category'`, count per category. Store as `metric_name = "category_dist"`, `metric_value = {"sports_betting": 40, "casino": 25, ...}`.
- [ ] **18e.** Implement `compute_streak(db, device_id)`: find the longest consecutive run of days with zero block events (from `daily_block_stats`). Store as `metric_name = "clean_streak"`, `metric_value = {"current_days": 12, "longest_days": 30, "streak_start": "2026-03-01"}`.
- [ ] **18f.** Implement `compute_weekly_trend(db, device_id)`: compare this week's total blocks to last week's. Store as `metric_name = "weekly_trend"`, `metric_value = {"this_week": 20, "last_week": 35, "change_pct": -42.8}`.
- [ ] **18g.** Write unit tests for each computation using test fixtures. Test edge cases: no data, single day, all-zero days.
- [ ] **18h.** Verify: `cargo test -p bb-worker` passes.

---

## Chunk 7: Report Export + Dashboard UI (Tasks 19-22)

### Task 19: PDF/CSV export service

- [ ] **19a.** Add dependencies to `crates/bb-api/Cargo.toml`: `genpdf` (PDF generation), `csv` (CSV writing).
- [ ] **19b.** Create `crates/bb-api/src/services/export_service.rs`. Register in `services/mod.rs`.
- [ ] **19c.** Implement `generate_csv_report(db, device_id, from, to, reporting_config) -> Result<Vec<u8>>`: query daily stats and trends, write CSV with columns: date, total_blocks, bypass_attempts, tamper_events, top_category. Respect `reporting_config` visibility (if `domain_details` is false, omit category breakdown).
- [ ] **19d.** Implement `generate_pdf_report(db, device_id, from, to, reporting_config) -> Result<Vec<u8>>`: use `genpdf` to produce a formatted report with header (device name, date range), summary stats table, daily counts table, trend highlights (streak, peak hour, weekly change). Include org name if device is assigned to an org.
- [ ] **19e.** Add export routes to `analytics.rs`:
  - `GET /v1/analytics/export/csv` -> query params `device_id, from, to`. Returns `Content-Type: text/csv`, `Content-Disposition: attachment; filename="report.csv"`.
  - `GET /v1/analytics/export/pdf` -> same params. Returns `Content-Type: application/pdf`.
  Both enforce enrollment visibility before generating.
- [ ] **19f.** Write tests: CSV output parses correctly, PDF is non-empty and starts with `%PDF`.

### Task 20: Analytics TypeScript types and API client

- [ ] **20a.** Add to `web/src/lib/api-types.ts`:
  - `TimeseriesPoint { bucket: string; event_type: EventType; count: number }`
  - `TrendMetric { metric_name: string; metric_value: Record<string, unknown>; computed_at: string }`
  - `HeatmapData { hour: number; day_of_week: number; count: number }[]`
  - `AnalyticsSummary { total_blocks, total_bypass_attempts, total_tamper_events, categories: Record<string, number>, period: { from, to } }`
- [ ] **20b.** Add `analytics` namespace to `web/src/lib/api-client.ts`: `timeseries(params)`, `trends(params)`, `summary(params)`, `heatmap(params)`, `exportCsv(params)` (returns Blob), `exportPdf(params)` (returns Blob). For export methods, use `fetch` directly with `responseType: 'blob'`.
- [ ] **20c.** Verify: `npx tsc --noEmit` passes.

### Task 21: Enhanced dashboard analytics pages

- [ ] **21a.** Install charting library: `cd web && npm install recharts` (lightweight, React-native charts). Also install `react-calendar-heatmap` for the GitHub-style heatmap.
- [ ] **21b.** Create `web/src/app/(dashboard)/reports/analytics/page.tsx`: analytics dashboard with date range selector (default: last 30 days), device selector dropdown.
- [ ] **21c.** Add time-series line chart component (`web/src/components/analytics/TimeseriesChart.tsx`): uses `recharts` `<LineChart>` to plot block counts over time. Supports hourly and daily granularity toggle. Separate lines per event type (blocks, bypass attempts, tamper).
- [ ] **21d.** Add heatmap component (`web/src/components/analytics/ActivityHeatmap.tsx`): uses `react-calendar-heatmap` to show daily blocking activity as a GitHub-contribution-style calendar. Color intensity based on block count.
- [ ] **21e.** Add trend cards component (`web/src/components/analytics/TrendCards.tsx`): display current streak, peak hour, weekly trend (up/down arrow with percentage), category distribution (horizontal bar chart or pie chart).
- [ ] **21f.** Add category breakdown chart (`web/src/components/analytics/CategoryChart.tsx`): `recharts` `<PieChart>` or `<BarChart>` showing distribution of block categories.
- [ ] **21g.** Add export buttons to the analytics page: "Export CSV" and "Export PDF" that trigger downloads via the API client blob methods.

### Task 22: Final integration and verification

- [ ] **22a.** Add org-scoped analytics: on the org detail page Overview tab, show aggregate analytics across all org devices. Implement `GET /v1/analytics/org-summary?organization_id={id}&from=...&to=...` route that sums stats across all devices in the org (respecting caller's visibility).
- [ ] **22b.** Update `web/src/app/(dashboard)/organizations/[id]/page.tsx` Overview tab to display org-level analytics charts (aggregate timeseries, combined heatmap, per-device comparison table).
- [ ] **22c.** End-to-end test: create org, invite member, assign device, generate events, verify aggregates populate, verify analytics API returns data, verify export downloads work, verify dashboard renders charts.
- [ ] **22d.** Run full test suite: `cargo test --workspace`, `cd web && npm test`, `cargo clippy --workspace -- -D warnings`.
- [ ] **22e.** Commit with message: `feat(phase2-sp1): organizations, analytics, reporting, export, dashboard`

---

## Definition of Done

- [ ] Organization CRUD works: create, list, get, update, delete with role-based access
- [ ] Member management: invite, list, role change, remove with role hierarchy enforcement
- [ ] Device assignment: assign/unassign devices to orgs, org default configs applied
- [ ] Bulk enrollment tokens: create, list, revoke, redeem, QR code generation
- [ ] TimescaleDB continuous aggregates: hourly and daily rollups auto-refreshing
- [ ] Analytics API: timeseries, trends, summary, heatmap endpoints with enrollment visibility
- [ ] Trend analysis: peak hour, day-of-week, category distribution, streaks, weekly trend computed hourly
- [ ] Export: CSV and PDF reports downloadable with visibility enforcement
- [ ] Dashboard UI: time-series charts, heatmaps, trend cards, category breakdown, export buttons
- [ ] Organization UI: org list, detail with tabs, member management, device assignment, tokens with QR
- [ ] All tests pass, clippy clean, TypeScript compiles
