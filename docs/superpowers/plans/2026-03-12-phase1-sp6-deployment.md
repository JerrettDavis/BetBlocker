# Phase 1 Sub-Plan 6: Deployment

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Production-ready Docker deployment for both hosted and self-hosted models, CI/CD pipeline for build+test+sign+publish.
**Architecture:** Multi-stage Docker builds producing minimal images. Single docker-compose.yml for self-hosted. Helm chart for hosted Kubernetes. GitHub Actions for CI/CD.
**Tech Stack:** Docker, docker-compose, Helm, GitHub Actions, musl (static Rust binaries)
**Depends on:** Sub-Plans 2, 3, 4, 5
**Reference Docs:**
- Master plan: `docs/superpowers/plans/2026-03-12-phase1-master-plan.md`
- Repo structure: `docs/architecture/repo-structure.md`
- Deployment ADR: `docs/architecture/adrs/ADR-006-deployment-architecture-hosted-vs-self-hosted-parity.md`
- API spec: `docs/architecture/api-spec.md`
- Threat model: `docs/architecture/threat-model.md`

---

## File Structure

```
betblocker/
├── deploy/
│   ├── docker/
│   │   ├── Dockerfile.api
│   │   ├── Dockerfile.worker
│   │   ├── Dockerfile.web
│   │   └── Dockerfile.agent-linux
│   ├── docker-compose.yml            # Production / self-hosted full stack
│   ├── docker-compose.dev.yml        # Dev overrides (already exists from SP1)
│   └── helm/
│       └── betblocker/
│           ├── Chart.yaml
│           ├── values.yaml
│           └── templates/
│               ├── _helpers.tpl
│               ├── api-deployment.yaml
│               ├── api-service.yaml
│               ├── api-hpa.yaml
│               ├── worker-deployment.yaml
│               ├── web-deployment.yaml
│               ├── web-service.yaml
│               ├── web-ingress.yaml
│               ├── secrets.yaml
│               └── configmap.yaml
├── scripts/
│   └── setup.sh                      # Self-hosted first-run setup
├── tools/
│   └── signing/
│       ├── sign-binary.sh            # Ed25519 binary signing
│       └── verify-binary.sh          # Ed25519 signature verification
└── .github/
    └── workflows/
        ├── pr.yml                    # Already exists from SP1, enhanced here
        ├── merge.yml                 # Build + push + E2E on merge to main
        └── release.yml               # Build + sign + publish on version tag
```

---

## Chunk 1: Rust Server Dockerfiles

### Task 1: Create Dockerfile.api

**Files:**
- Create: `deploy/docker/Dockerfile.api`

- [ ] **Step 1: Write multi-stage Dockerfile for bb-api**

The API Dockerfile uses a two-stage build: a Rust builder stage targeting musl for a fully static binary, and a `FROM scratch` final stage containing only the binary and CA certificates. Target image size: ~15MB.

```dockerfile
# deploy/docker/Dockerfile.api
# ---------------------------------------------------------
# Stage 1: Build static Rust binary
# ---------------------------------------------------------
FROM rust:1.85-alpine AS builder

RUN apk add --no-cache musl-dev protobuf-dev

# Install the musl target
RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /build

# Copy workspace manifests first for dependency caching
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY .cargo .cargo
COPY crates/bb-common/Cargo.toml crates/bb-common/Cargo.toml
COPY crates/bb-proto/Cargo.toml crates/bb-proto/Cargo.toml
COPY crates/bb-api/Cargo.toml crates/bb-api/Cargo.toml
COPY crates/bb-worker/Cargo.toml crates/bb-worker/Cargo.toml
COPY crates/bb-agent-core/Cargo.toml crates/bb-agent-core/Cargo.toml
COPY crates/bb-agent-plugins/Cargo.toml crates/bb-agent-plugins/Cargo.toml
COPY crates/bb-agent-linux/Cargo.toml crates/bb-agent-linux/Cargo.toml
COPY crates/bb-cli/Cargo.toml crates/bb-cli/Cargo.toml

# Create dummy source files so cargo can resolve the workspace and cache deps
RUN mkdir -p crates/bb-common/src && echo "" > crates/bb-common/src/lib.rs && \
    mkdir -p crates/bb-proto/src && echo "" > crates/bb-proto/src/lib.rs && \
    mkdir -p crates/bb-proto/proto && touch crates/bb-proto/proto/device.proto && \
    mkdir -p crates/bb-api/src && echo "fn main() {}" > crates/bb-api/src/main.rs && \
    mkdir -p crates/bb-worker/src && echo "fn main() {}" > crates/bb-worker/src/main.rs && \
    mkdir -p crates/bb-agent-core/src && echo "" > crates/bb-agent-core/src/lib.rs && \
    mkdir -p crates/bb-agent-plugins/src && echo "" > crates/bb-agent-plugins/src/lib.rs && \
    mkdir -p crates/bb-agent-linux/src && echo "fn main() {}" > crates/bb-agent-linux/src/main.rs && \
    mkdir -p crates/bb-cli/src && echo "fn main() {}" > crates/bb-cli/src/main.rs

# Build dependencies only (this layer is cached unless Cargo.toml/Cargo.lock change)
RUN cargo build --release --target x86_64-unknown-linux-musl -p bb-api 2>/dev/null || true

# Copy real source code
COPY crates crates
COPY migrations migrations

# Touch source files to invalidate the binary cache but keep dependency cache
RUN find crates -name "*.rs" -exec touch {} +

# Build the actual binary
RUN cargo build --release --target x86_64-unknown-linux-musl -p bb-api

# Strip the binary for minimal size
RUN strip /build/target/x86_64-unknown-linux-musl/release/betblocker-api

# ---------------------------------------------------------
# Stage 2: Minimal runtime image
# ---------------------------------------------------------
FROM scratch

# Import CA certificates for outbound TLS (Stripe, community feed, etc.)
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt

# Import the static binary
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/betblocker-api /betblocker-api

# Copy migrations for the binary to run at startup if needed
COPY --from=builder /build/migrations /migrations

# Non-root user: use numeric UID since scratch has no /etc/passwd
USER 65534:65534

EXPOSE 8443

# Health check is handled by the orchestrator (docker-compose / k8s)
# The binary exposes GET /healthz for probing

ENTRYPOINT ["/betblocker-api"]
```

- [ ] **Step 2: Verify build produces static binary**

Build locally and confirm:
```bash
docker build -f deploy/docker/Dockerfile.api -t betblocker-api:local .
docker images betblocker-api:local  # Should be ~15-20MB
# Verify static linking:
docker run --rm betblocker-api:local --version
```

---

### Task 2: Create Dockerfile.worker

**Files:**
- Create: `deploy/docker/Dockerfile.worker`

- [ ] **Step 1: Write multi-stage Dockerfile for bb-worker**

Identical pattern to the API Dockerfile, targeting the `betblocker-worker` binary.

```dockerfile
# deploy/docker/Dockerfile.worker
# ---------------------------------------------------------
# Stage 1: Build static Rust binary
# ---------------------------------------------------------
FROM rust:1.85-alpine AS builder

RUN apk add --no-cache musl-dev protobuf-dev

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /build

# Copy workspace manifests first for dependency caching
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY .cargo .cargo
COPY crates/bb-common/Cargo.toml crates/bb-common/Cargo.toml
COPY crates/bb-proto/Cargo.toml crates/bb-proto/Cargo.toml
COPY crates/bb-api/Cargo.toml crates/bb-api/Cargo.toml
COPY crates/bb-worker/Cargo.toml crates/bb-worker/Cargo.toml
COPY crates/bb-agent-core/Cargo.toml crates/bb-agent-core/Cargo.toml
COPY crates/bb-agent-plugins/Cargo.toml crates/bb-agent-plugins/Cargo.toml
COPY crates/bb-agent-linux/Cargo.toml crates/bb-agent-linux/Cargo.toml
COPY crates/bb-cli/Cargo.toml crates/bb-cli/Cargo.toml

# Create dummy source files for dependency caching
RUN mkdir -p crates/bb-common/src && echo "" > crates/bb-common/src/lib.rs && \
    mkdir -p crates/bb-proto/src && echo "" > crates/bb-proto/src/lib.rs && \
    mkdir -p crates/bb-proto/proto && touch crates/bb-proto/proto/device.proto && \
    mkdir -p crates/bb-api/src && echo "fn main() {}" > crates/bb-api/src/main.rs && \
    mkdir -p crates/bb-worker/src && echo "fn main() {}" > crates/bb-worker/src/main.rs && \
    mkdir -p crates/bb-agent-core/src && echo "" > crates/bb-agent-core/src/lib.rs && \
    mkdir -p crates/bb-agent-plugins/src && echo "" > crates/bb-agent-plugins/src/lib.rs && \
    mkdir -p crates/bb-agent-linux/src && echo "fn main() {}" > crates/bb-agent-linux/src/main.rs && \
    mkdir -p crates/bb-cli/src && echo "fn main() {}" > crates/bb-cli/src/main.rs

RUN cargo build --release --target x86_64-unknown-linux-musl -p bb-worker 2>/dev/null || true

COPY crates crates
COPY migrations migrations

RUN find crates -name "*.rs" -exec touch {} +

RUN cargo build --release --target x86_64-unknown-linux-musl -p bb-worker

RUN strip /build/target/x86_64-unknown-linux-musl/release/betblocker-worker

# ---------------------------------------------------------
# Stage 2: Minimal runtime image
# ---------------------------------------------------------
FROM scratch

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/betblocker-worker /betblocker-worker
COPY --from=builder /build/migrations /migrations

USER 65534:65534

# Worker has no exposed ports -- it pulls jobs from Redis
# Health check via /healthz on an internal metrics port
EXPOSE 9090

ENTRYPOINT ["/betblocker-worker"]
```

---

## Chunk 2: Web and Agent Dockerfiles

### Task 3: Create Dockerfile.web

**Files:**
- Create: `deploy/docker/Dockerfile.web`

- [ ] **Step 1: Write multi-stage Dockerfile for the Next.js web app**

Three-stage build: install dependencies, build standalone output, copy to minimal Node.js runtime.

```dockerfile
# deploy/docker/Dockerfile.web
# Build context: web/ directory
# ---------------------------------------------------------
# Stage 1: Install dependencies
# ---------------------------------------------------------
FROM node:22-alpine AS deps

WORKDIR /app

COPY package.json package-lock.json ./
RUN npm ci --ignore-scripts

# ---------------------------------------------------------
# Stage 2: Build the Next.js application
# ---------------------------------------------------------
FROM node:22-alpine AS builder

WORKDIR /app

COPY --from=deps /app/node_modules ./node_modules
COPY . .

# Next.js standalone output produces a self-contained server
ENV NEXT_TELEMETRY_DISABLED=1
RUN npm run build

# ---------------------------------------------------------
# Stage 3: Minimal runtime
# ---------------------------------------------------------
FROM node:22-alpine AS runner

WORKDIR /app

ENV NODE_ENV=production
ENV NEXT_TELEMETRY_DISABLED=1

# Create non-root user
RUN addgroup --system --gid 1001 betblocker && \
    adduser --system --uid 1001 betblocker

# Copy standalone output
COPY --from=builder /app/.next/standalone ./
COPY --from=builder /app/.next/static ./.next/static
COPY --from=builder /app/public ./public

# Set ownership
RUN chown -R betblocker:betblocker /app

USER betblocker

EXPOSE 3000

ENV PORT=3000
ENV HOSTNAME="0.0.0.0"

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:3000/api/health || exit 1

CMD ["node", "server.js"]
```

- [ ] **Step 2: Ensure Next.js config enables standalone output**

In `web/next.config.ts`, verify or add:
```typescript
const nextConfig = {
  output: 'standalone',
  // ... other config
};
```

---

### Task 4: Create Dockerfile.agent-linux

**Files:**
- Create: `deploy/docker/Dockerfile.agent-linux`

- [ ] **Step 1: Write multi-stage Dockerfile for the Linux agent binary**

This Dockerfile builds the agent binary. It is not meant to run in a container in production -- agents run directly on endpoint devices. The Dockerfile is used by CI to produce the static binary artifact in a reproducible environment.

```dockerfile
# deploy/docker/Dockerfile.agent-linux
# Purpose: Build the Linux agent binary in a reproducible environment.
# The output is a static binary extracted via `docker cp` or multi-stage copy.
# ---------------------------------------------------------
# Stage 1: Build static agent binary
# ---------------------------------------------------------
FROM rust:1.85-alpine AS builder

RUN apk add --no-cache musl-dev protobuf-dev

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /build

# Copy workspace manifests for dependency caching
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY .cargo .cargo
COPY crates/bb-common/Cargo.toml crates/bb-common/Cargo.toml
COPY crates/bb-proto/Cargo.toml crates/bb-proto/Cargo.toml
COPY crates/bb-api/Cargo.toml crates/bb-api/Cargo.toml
COPY crates/bb-worker/Cargo.toml crates/bb-worker/Cargo.toml
COPY crates/bb-agent-core/Cargo.toml crates/bb-agent-core/Cargo.toml
COPY crates/bb-agent-plugins/Cargo.toml crates/bb-agent-plugins/Cargo.toml
COPY crates/bb-agent-linux/Cargo.toml crates/bb-agent-linux/Cargo.toml
COPY crates/bb-cli/Cargo.toml crates/bb-cli/Cargo.toml

# Create dummy source files for dependency caching
RUN mkdir -p crates/bb-common/src && echo "" > crates/bb-common/src/lib.rs && \
    mkdir -p crates/bb-proto/src && echo "" > crates/bb-proto/src/lib.rs && \
    mkdir -p crates/bb-proto/proto && touch crates/bb-proto/proto/device.proto && \
    mkdir -p crates/bb-api/src && echo "fn main() {}" > crates/bb-api/src/main.rs && \
    mkdir -p crates/bb-worker/src && echo "fn main() {}" > crates/bb-worker/src/main.rs && \
    mkdir -p crates/bb-agent-core/src && echo "" > crates/bb-agent-core/src/lib.rs && \
    mkdir -p crates/bb-agent-plugins/src && echo "" > crates/bb-agent-plugins/src/lib.rs && \
    mkdir -p crates/bb-agent-linux/src && echo "fn main() {}" > crates/bb-agent-linux/src/main.rs && \
    mkdir -p crates/bb-cli/src && echo "fn main() {}" > crates/bb-cli/src/main.rs

RUN cargo build --release --target x86_64-unknown-linux-musl -p bb-agent-linux 2>/dev/null || true

COPY crates crates

RUN find crates -name "*.rs" -exec touch {} +

RUN cargo build --release --target x86_64-unknown-linux-musl -p bb-agent-linux

RUN strip /build/target/x86_64-unknown-linux-musl/release/betblocker-agent-linux

# ---------------------------------------------------------
# Stage 2: Extract binary into a minimal image for CI artifact extraction
# ---------------------------------------------------------
FROM scratch

COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/betblocker-agent-linux /betblocker-agent-linux
```

---

## Chunk 3: Docker Compose (Production / Self-Hosted)

### Task 5: Create production docker-compose.yml

**Files:**
- Create: `deploy/docker-compose.yml`

- [ ] **Step 1: Write full-stack docker-compose.yml for self-hosted deployment**

Per ADR-006, uses a single TimescaleDB image (which includes PostgreSQL) rather than separate PostgreSQL + TimescaleDB containers. All BetBlocker services use the same images as hosted, with `BETBLOCKER_DEPLOYMENT=self-hosted` toggling behavior.

```yaml
# deploy/docker-compose.yml
# BetBlocker self-hosted production deployment
# Usage:
#   1. Copy .env.example to .env and fill in values
#   2. Run: docker compose up -d
#   3. First run: docker compose exec api /betblocker-api setup
#      OR run: ../scripts/setup.sh
#
# For development, use docker-compose.dev.yml as an override:
#   docker compose -f docker-compose.yml -f docker-compose.dev.yml up

name: betblocker

services:
  # -------------------------------------------------------
  # API Server
  # -------------------------------------------------------
  api:
    image: ghcr.io/betblocker/betblocker-api:${BETBLOCKER_VERSION:-latest}
    restart: unless-stopped
    depends_on:
      db:
        condition: service_healthy
      cache:
        condition: service_healthy
    ports:
      - "${API_PORT:-8443}:8443"
    environment:
      BETBLOCKER_DEPLOYMENT: self-hosted
      BETBLOCKER_BILLING_ENABLED: "false"
      BETBLOCKER_MARKETING_PAGES: "false"
      BETBLOCKER_DISCOVERY_PIPELINE: "false"
      BETBLOCKER_TELEMETRY_ENABLED: "false"
      BETBLOCKER_COMMUNITY_FEED_URL: ${BETBLOCKER_COMMUNITY_FEED_URL:-https://feed.betblocker.org/v1}
      DATABASE_URL: postgres://betblocker:${DB_PASSWORD}@db:5432/betblocker
      REDIS_URL: redis://cache:6379
      BETBLOCKER_JWT_SIGNING_KEY_PATH: /keys/jwt-signing.key
      BETBLOCKER_BLOCKLIST_SIGNING_KEY_PATH: /keys/blocklist-signing.key
      BETBLOCKER_CA_KEY_PATH: /keys/root-ca.key
      BETBLOCKER_DEVICE_CA_KEY_PATH: /keys/device-ca.key
      BETBLOCKER_LOG_LEVEL: ${LOG_LEVEL:-info}
      BETBLOCKER_LOG_FORMAT: json
      BETBLOCKER_EXTERNAL_URL: ${BETBLOCKER_EXTERNAL_URL:-https://localhost:8443}
    volumes:
      - betblocker-keys:/keys:ro
      - betblocker-data:/data
    networks:
      - betblocker-internal
    healthcheck:
      test: ["CMD", "/betblocker-api", "healthcheck"]
      interval: 15s
      timeout: 5s
      retries: 3
      start_period: 10s
    read_only: true
    tmpfs:
      - /tmp:size=64M
    security_opt:
      - no-new-privileges:true

  # -------------------------------------------------------
  # Background Worker
  # -------------------------------------------------------
  worker:
    image: ghcr.io/betblocker/betblocker-worker:${BETBLOCKER_VERSION:-latest}
    restart: unless-stopped
    depends_on:
      db:
        condition: service_healthy
      cache:
        condition: service_healthy
    environment:
      BETBLOCKER_DEPLOYMENT: self-hosted
      BETBLOCKER_BILLING_ENABLED: "false"
      BETBLOCKER_DISCOVERY_PIPELINE: "false"
      DATABASE_URL: postgres://betblocker:${DB_PASSWORD}@db:5432/betblocker
      REDIS_URL: redis://cache:6379
      BETBLOCKER_BLOCKLIST_SIGNING_KEY_PATH: /keys/blocklist-signing.key
      BETBLOCKER_COMMUNITY_FEED_URL: ${BETBLOCKER_COMMUNITY_FEED_URL:-https://feed.betblocker.org/v1}
      BETBLOCKER_LOG_LEVEL: ${LOG_LEVEL:-info}
      BETBLOCKER_LOG_FORMAT: json
    volumes:
      - betblocker-keys:/keys:ro
      - betblocker-data:/data
    networks:
      - betblocker-internal
    healthcheck:
      test: ["CMD", "/betblocker-worker", "healthcheck"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s
    read_only: true
    tmpfs:
      - /tmp:size=64M
    security_opt:
      - no-new-privileges:true

  # -------------------------------------------------------
  # Web Dashboard
  # -------------------------------------------------------
  web:
    image: ghcr.io/betblocker/betblocker-web:${BETBLOCKER_VERSION:-latest}
    restart: unless-stopped
    depends_on:
      api:
        condition: service_healthy
    ports:
      - "${WEB_PORT:-80}:3000"
    environment:
      BETBLOCKER_DEPLOYMENT: self-hosted
      NEXT_PUBLIC_API_URL: ${BETBLOCKER_EXTERNAL_URL:-https://localhost:8443}
      NEXT_PUBLIC_HOSTED: "false"
      API_URL: http://api:8443
    networks:
      - betblocker-internal
    healthcheck:
      test: ["CMD", "wget", "--no-verbose", "--tries=1", "--spider", "http://localhost:3000/api/health"]
      interval: 30s
      timeout: 3s
      retries: 3
      start_period: 10s
    read_only: true
    tmpfs:
      - /tmp:size=64M
      - /app/.next/cache:size=256M
    security_opt:
      - no-new-privileges:true

  # -------------------------------------------------------
  # Database (TimescaleDB = PostgreSQL + hypertables)
  # -------------------------------------------------------
  db:
    image: timescale/timescaledb:latest-pg16
    restart: unless-stopped
    environment:
      POSTGRES_DB: betblocker
      POSTGRES_USER: betblocker
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    volumes:
      - betblocker-db:/var/lib/postgresql/data
    networks:
      - betblocker-internal
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U betblocker -d betblocker"]
      interval: 5s
      timeout: 5s
      retries: 5
      start_period: 15s
    # Do not expose port externally in production
    # Uncomment for debugging:
    # ports:
    #   - "5432:5432"
    shm_size: 256m

  # -------------------------------------------------------
  # Cache (Redis)
  # -------------------------------------------------------
  cache:
    image: redis:7-alpine
    restart: unless-stopped
    command: >
      redis-server
      --appendonly yes
      --maxmemory 256mb
      --maxmemory-policy allkeys-lru
    volumes:
      - betblocker-cache:/data
    networks:
      - betblocker-internal
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 3s
      retries: 5
      start_period: 5s

volumes:
  betblocker-db:
    driver: local
  betblocker-cache:
    driver: local
  betblocker-keys:
    driver: local
  betblocker-data:
    driver: local

networks:
  betblocker-internal:
    driver: bridge
```

- [ ] **Step 2: Create .env.example for self-hosted deployment**

```
# deploy/.env.example
# Copy this file to .env and fill in your values.

# Database password (REQUIRED - change this)
DB_PASSWORD=CHANGE_ME_TO_A_STRONG_PASSWORD

# BetBlocker version to deploy (default: latest)
# BETBLOCKER_VERSION=latest

# External URL where clients will reach the API (REQUIRED for agent enrollment)
BETBLOCKER_EXTERNAL_URL=https://betblocker.example.com:8443

# Port mappings (defaults shown)
# API_PORT=8443
# WEB_PORT=80

# Log level: trace, debug, info, warn, error
# LOG_LEVEL=info

# Community blocklist feed (default: official BetBlocker feed)
# BETBLOCKER_COMMUNITY_FEED_URL=https://feed.betblocker.org/v1

# Optional: federated report upstream (opt-in)
# BETBLOCKER_FEDERATED_REPORT_UPSTREAM=https://api.betblocker.org/v1/reports
# BETBLOCKER_FEDERATED_REPORT_API_KEY=your-api-key
```

---

## Chunk 4: Self-Hosted Setup Script

### Task 6: Create the self-hosted setup script

**Files:**
- Create: `scripts/setup.sh`

- [ ] **Step 1: Write the idempotent first-run setup script**

This script handles initial cryptographic material generation, database migration, admin account creation, and blocklist seeding. It must be safe to run multiple times (idempotent).

```bash
#!/usr/bin/env bash
# scripts/setup.sh
# BetBlocker self-hosted first-run setup.
# This script is idempotent: running it again skips already-completed steps.
#
# Usage:
#   cd deploy && ../scripts/setup.sh
#   OR
#   docker compose -f deploy/docker-compose.yml exec api /betblocker-api setup
#
# Prerequisites:
#   - docker and docker compose installed
#   - deploy/.env file exists with DB_PASSWORD and BETBLOCKER_EXTERNAL_URL set

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd "${SCRIPT_DIR}/../deploy" && pwd)"
COMPOSE_FILE="${DEPLOY_DIR}/docker-compose.yml"

# Colors for output (disabled if not a terminal)
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    BLUE='\033[0;34m'
    NC='\033[0m'
else
    RED='' GREEN='' YELLOW='' BLUE='' NC=''
fi

log_info()  { echo -e "${BLUE}[INFO]${NC}  $1"; }
log_ok()    { echo -e "${GREEN}[OK]${NC}    $1"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# -----------------------------------------------------------
# Pre-flight checks
# -----------------------------------------------------------
preflight() {
    log_info "Running pre-flight checks..."

    if ! command -v docker &>/dev/null; then
        log_error "docker is not installed. Please install Docker first."
        exit 1
    fi

    if ! docker compose version &>/dev/null; then
        log_error "docker compose (v2) is not available. Please update Docker."
        exit 1
    fi

    if [ ! -f "${DEPLOY_DIR}/.env" ]; then
        log_error "No .env file found at ${DEPLOY_DIR}/.env"
        log_info "Copy .env.example to .env and set DB_PASSWORD and BETBLOCKER_EXTERNAL_URL."
        exit 1
    fi

    # Source .env for validation
    set -a
    source "${DEPLOY_DIR}/.env"
    set +a

    if [ "${DB_PASSWORD:-}" = "CHANGE_ME_TO_A_STRONG_PASSWORD" ] || [ -z "${DB_PASSWORD:-}" ]; then
        log_error "DB_PASSWORD is not set or is still the default. Edit ${DEPLOY_DIR}/.env"
        exit 1
    fi

    if [ -z "${BETBLOCKER_EXTERNAL_URL:-}" ]; then
        log_error "BETBLOCKER_EXTERNAL_URL is not set. Edit ${DEPLOY_DIR}/.env"
        exit 1
    fi

    log_ok "Pre-flight checks passed."
}

# -----------------------------------------------------------
# Step 1: Generate cryptographic material
# -----------------------------------------------------------
generate_keys() {
    log_info "Step 1/4: Generating cryptographic material..."

    # Check if keys volume already has keys by looking for the root CA
    KEYS_EXIST=$(docker compose -f "${COMPOSE_FILE}" run --rm --entrypoint="" \
        -v betblocker-keys:/keys api \
        sh -c 'test -f /keys/root-ca.key && echo "yes" || echo "no"' 2>/dev/null || echo "no")

    if [ "${KEYS_EXIST}" = "yes" ]; then
        log_ok "Cryptographic keys already exist. Skipping generation."
        return 0
    fi

    log_info "Generating Ed25519 keypairs..."

    # Use the API container (which has the betblocker-api binary with key generation)
    docker compose -f "${COMPOSE_FILE}" run --rm \
        -v betblocker-keys:/keys \
        --entrypoint /betblocker-api \
        api generate-keys --output-dir /keys

    # The generate-keys command creates:
    #   /keys/root-ca.key          - Root CA private key (Ed25519)
    #   /keys/root-ca.pub          - Root CA public key
    #   /keys/device-ca.key        - Device CA private key (signed by Root CA)
    #   /keys/device-ca.pub        - Device CA public key
    #   /keys/device-ca.cert       - Device CA certificate
    #   /keys/blocklist-signing.key - Blocklist signing private key
    #   /keys/blocklist-signing.pub - Blocklist signing public key
    #   /keys/jwt-signing.key      - JWT signing private key (Ed25519)
    #   /keys/jwt-signing.pub      - JWT signing public key

    log_ok "Cryptographic keys generated and stored in betblocker-keys volume."
    log_warn "IMPORTANT: Back up the betblocker-keys volume. If lost, all devices must re-enroll."
}

# -----------------------------------------------------------
# Step 2: Start database and run migrations
# -----------------------------------------------------------
run_migrations() {
    log_info "Step 2/4: Starting database and running migrations..."

    # Start only the database service
    docker compose -f "${COMPOSE_FILE}" up -d db

    # Wait for database to be healthy
    log_info "Waiting for database to be ready..."
    local retries=30
    while [ $retries -gt 0 ]; do
        if docker compose -f "${COMPOSE_FILE}" exec db pg_isready -U betblocker -d betblocker &>/dev/null; then
            break
        fi
        retries=$((retries - 1))
        sleep 1
    done

    if [ $retries -eq 0 ]; then
        log_error "Database failed to start within 30 seconds."
        exit 1
    fi

    # Run migrations via the API container
    docker compose -f "${COMPOSE_FILE}" run --rm \
        --entrypoint /betblocker-api \
        api migrate

    log_ok "Database migrations complete."
}

# -----------------------------------------------------------
# Step 3: Create admin account
# -----------------------------------------------------------
create_admin() {
    log_info "Step 3/4: Creating admin account..."

    # Check if an admin account already exists
    ADMIN_EXISTS=$(docker compose -f "${COMPOSE_FILE}" run --rm \
        --entrypoint /betblocker-api \
        api admin-exists 2>/dev/null && echo "yes" || echo "no")

    if [ "${ADMIN_EXISTS}" = "yes" ]; then
        log_ok "Admin account already exists. Skipping."
        return 0
    fi

    # Prompt for admin credentials if not set in environment
    if [ -z "${BETBLOCKER_ADMIN_EMAIL:-}" ]; then
        echo -n "Admin email: "
        read -r BETBLOCKER_ADMIN_EMAIL
    fi

    if [ -z "${BETBLOCKER_ADMIN_PASSWORD:-}" ]; then
        echo -n "Admin password: "
        read -rs BETBLOCKER_ADMIN_PASSWORD
        echo
    fi

    docker compose -f "${COMPOSE_FILE}" run --rm \
        -e BETBLOCKER_ADMIN_EMAIL="${BETBLOCKER_ADMIN_EMAIL}" \
        -e BETBLOCKER_ADMIN_PASSWORD="${BETBLOCKER_ADMIN_PASSWORD}" \
        --entrypoint /betblocker-api \
        api create-admin

    log_ok "Admin account created."
}

# -----------------------------------------------------------
# Step 4: Seed blocklist
# -----------------------------------------------------------
seed_blocklist() {
    log_info "Step 4/4: Seeding blocklist..."

    # Check if blocklist already has entries
    BLOCKLIST_COUNT=$(docker compose -f "${COMPOSE_FILE}" run --rm \
        --entrypoint /betblocker-api \
        api blocklist-count 2>/dev/null || echo "0")

    if [ "${BLOCKLIST_COUNT}" != "0" ] && [ -n "${BLOCKLIST_COUNT}" ]; then
        log_ok "Blocklist already has ${BLOCKLIST_COUNT} entries. Skipping seed."
        return 0
    fi

    # Try to pull from community feed first
    if [ -n "${BETBLOCKER_COMMUNITY_FEED_URL:-}" ]; then
        log_info "Pulling initial blocklist from community feed..."
        docker compose -f "${COMPOSE_FILE}" run --rm \
            -e BETBLOCKER_COMMUNITY_FEED_URL="${BETBLOCKER_COMMUNITY_FEED_URL:-https://feed.betblocker.org/v1}" \
            --entrypoint /betblocker-api \
            api seed-blocklist --source community-feed \
            && { log_ok "Blocklist seeded from community feed."; return 0; } \
            || log_warn "Community feed unavailable. Falling back to built-in seed list."
    fi

    # Fall back to compiled-in seed list
    docker compose -f "${COMPOSE_FILE}" run --rm \
        --entrypoint /betblocker-api \
        api seed-blocklist --source builtin

    log_ok "Blocklist seeded from built-in list."
}

# -----------------------------------------------------------
# Summary
# -----------------------------------------------------------
print_summary() {
    echo ""
    echo "=============================================="
    echo "  BetBlocker Self-Hosted Setup Complete"
    echo "=============================================="
    echo ""
    echo "  API URL:        ${BETBLOCKER_EXTERNAL_URL}"
    echo "  Web Dashboard:  http://localhost:${WEB_PORT:-80}"
    echo "  Admin Email:    ${BETBLOCKER_ADMIN_EMAIL:-<set during setup>}"
    echo ""
    echo "  To start all services:"
    echo "    cd ${DEPLOY_DIR} && docker compose up -d"
    echo ""
    echo "  To export agent configuration for devices:"
    echo "    docker compose exec api /betblocker-api agent-config export \\"
    echo "      --api-url ${BETBLOCKER_EXTERNAL_URL} \\"
    echo "      --output agent-config.json"
    echo ""
    echo "  IMPORTANT: Back up the betblocker-keys Docker volume."
    echo "  If lost, all enrolled devices must re-enroll."
    echo ""
    echo "  To create a backup:"
    echo "    docker run --rm -v betblocker-keys:/keys -v \$(pwd):/backup \\"
    echo "      alpine tar czf /backup/betblocker-keys-backup.tar.gz -C /keys ."
    echo ""
    echo "=============================================="
}

# -----------------------------------------------------------
# Main
# -----------------------------------------------------------
main() {
    echo ""
    echo "=============================================="
    echo "  BetBlocker Self-Hosted Setup"
    echo "=============================================="
    echo ""

    preflight
    generate_keys
    run_migrations
    create_admin
    seed_blocklist
    print_summary
}

main "$@"
```

- [ ] **Step 2: Make the setup script executable**

```bash
chmod +x scripts/setup.sh
```

---

## Chunk 5: Helm Chart Skeleton

### Task 7: Create the Helm chart for hosted Kubernetes deployment

**Files:**
- Create: `deploy/helm/betblocker/Chart.yaml`
- Create: `deploy/helm/betblocker/values.yaml`
- Create: `deploy/helm/betblocker/templates/_helpers.tpl`
- Create: `deploy/helm/betblocker/templates/api-deployment.yaml`
- Create: `deploy/helm/betblocker/templates/api-service.yaml`
- Create: `deploy/helm/betblocker/templates/api-hpa.yaml`
- Create: `deploy/helm/betblocker/templates/worker-deployment.yaml`
- Create: `deploy/helm/betblocker/templates/web-deployment.yaml`
- Create: `deploy/helm/betblocker/templates/web-service.yaml`
- Create: `deploy/helm/betblocker/templates/web-ingress.yaml`
- Create: `deploy/helm/betblocker/templates/secrets.yaml`
- Create: `deploy/helm/betblocker/templates/configmap.yaml`

- [ ] **Step 1: Create Chart.yaml**

```yaml
# deploy/helm/betblocker/Chart.yaml
apiVersion: v2
name: betblocker
description: BetBlocker gambling blocking platform
type: application
version: 0.1.0
appVersion: "0.1.0"
keywords:
  - gambling
  - blocking
  - dns
maintainers:
  - name: BetBlocker
    url: https://betblocker.org
```

- [ ] **Step 2: Create values.yaml**

```yaml
# deploy/helm/betblocker/values.yaml

# -- Global settings
global:
  deployment: hosted
  imageRegistry: ghcr.io/betblocker
  imageTag: latest
  imagePullPolicy: IfNotPresent

# -- API server
api:
  replicaCount: 2
  image:
    repository: ghcr.io/betblocker/betblocker-api
  resources:
    requests:
      cpu: 250m
      memory: 256Mi
    limits:
      cpu: "1"
      memory: 512Mi
  service:
    type: ClusterIP
    port: 8443
  autoscaling:
    enabled: true
    minReplicas: 2
    maxReplicas: 10
    targetCPUUtilizationPercentage: 70
    targetMemoryUtilizationPercentage: 80
  env:
    BETBLOCKER_DEPLOYMENT: hosted
    BETBLOCKER_BILLING_ENABLED: "true"
    BETBLOCKER_MARKETING_PAGES: "true"
    BETBLOCKER_DISCOVERY_PIPELINE: "true"
    BETBLOCKER_LOG_LEVEL: info
    BETBLOCKER_LOG_FORMAT: json

# -- Background worker
worker:
  replicaCount: 2
  image:
    repository: ghcr.io/betblocker/betblocker-worker
  resources:
    requests:
      cpu: 250m
      memory: 256Mi
    limits:
      cpu: "1"
      memory: 512Mi
  env:
    BETBLOCKER_DEPLOYMENT: hosted
    BETBLOCKER_LOG_LEVEL: info
    BETBLOCKER_LOG_FORMAT: json

# -- Web dashboard
web:
  replicaCount: 2
  image:
    repository: ghcr.io/betblocker/betblocker-web
  resources:
    requests:
      cpu: 100m
      memory: 128Mi
    limits:
      cpu: 500m
      memory: 256Mi
  service:
    type: ClusterIP
    port: 3000
  env:
    BETBLOCKER_DEPLOYMENT: hosted
    NEXT_PUBLIC_HOSTED: "true"

# -- Ingress
ingress:
  enabled: true
  className: nginx
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/proxy-body-size: "10m"
  hosts:
    - host: betblocker.com
      paths:
        - path: /api
          pathType: Prefix
          service: api
        - path: /
          pathType: Prefix
          service: web
  tls:
    - secretName: betblocker-tls
      hosts:
        - betblocker.com

# -- External database (managed PostgreSQL / TimescaleDB)
database:
  # External database URL (e.g., RDS, Timescale Cloud)
  # Provided via secret, not values
  external: true

# -- External Redis (managed ElastiCache or similar)
redis:
  external: true

# -- Secrets (references to Kubernetes secrets)
secrets:
  # Name of the Kubernetes secret containing DATABASE_URL, REDIS_URL, etc.
  existingSecret: betblocker-secrets

# -- Pod security context
podSecurityContext:
  runAsNonRoot: true
  runAsUser: 65534
  runAsGroup: 65534
  fsGroup: 65534
  seccompProfile:
    type: RuntimeDefault

# -- Container security context
securityContext:
  allowPrivilegeEscalation: false
  readOnlyRootFilesystem: true
  capabilities:
    drop:
      - ALL
```

- [ ] **Step 3: Create template helpers**

```yaml
# deploy/helm/betblocker/templates/_helpers.tpl
{{/*
Expand the name of the chart.
*/}}
{{- define "betblocker.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "betblocker.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "betblocker.labels" -}}
helm.sh/chart: {{ include "betblocker.name" . }}-{{ .Chart.Version }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}

{{/*
Selector labels for a specific component
*/}}
{{- define "betblocker.selectorLabels" -}}
app.kubernetes.io/name: {{ include "betblocker.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Image reference helper
*/}}
{{- define "betblocker.image" -}}
{{- printf "%s:%s" .repository (default $.Chart.AppVersion $.Values.global.imageTag) }}
{{- end }}
```

- [ ] **Step 4: Create API deployment and service**

```yaml
# deploy/helm/betblocker/templates/api-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: {{ include "betblocker.fullname" . }}-api
  labels:
    {{- include "betblocker.labels" . | nindent 4 }}
    app.kubernetes.io/component: api
spec:
  {{- if not .Values.api.autoscaling.enabled }}
  replicas: {{ .Values.api.replicaCount }}
  {{- end }}
  selector:
    matchLabels:
      {{- include "betblocker.selectorLabels" . | nindent 6 }}
      app.kubernetes.io/component: api
  template:
    metadata:
      labels:
        {{- include "betblocker.selectorLabels" . | nindent 8 }}
        app.kubernetes.io/component: api
    spec:
      securityContext:
        {{- toYaml .Values.podSecurityContext | nindent 8 }}
      containers:
        - name: api
          image: {{ .Values.api.image.repository }}:{{ .Values.global.imageTag }}
          imagePullPolicy: {{ .Values.global.imagePullPolicy }}
          securityContext:
            {{- toYaml .Values.securityContext | nindent 12 }}
          ports:
            - name: http
              containerPort: 8443
              protocol: TCP
          envFrom:
            - secretRef:
                name: {{ .Values.secrets.existingSecret }}
            - configMapRef:
                name: {{ include "betblocker.fullname" . }}-config
          env:
            {{- range $key, $value := .Values.api.env }}
            - name: {{ $key }}
              value: {{ $value | quote }}
            {{- end }}
          livenessProbe:
            httpGet:
              path: /healthz
              port: http
            initialDelaySeconds: 10
            periodSeconds: 15
            timeoutSeconds: 5
            failureThreshold: 3
          readinessProbe:
            httpGet:
              path: /readyz
              port: http
            initialDelaySeconds: 5
            periodSeconds: 5
            timeoutSeconds: 3
            failureThreshold: 3
          resources:
            {{- toYaml .Values.api.resources | nindent 12 }}
          volumeMounts:
            - name: tmp
              mountPath: /tmp
      volumes:
        - name: tmp
          emptyDir:
            sizeLimit: 64Mi
```

```yaml
# deploy/helm/betblocker/templates/api-service.yaml
apiVersion: v1
kind: Service
metadata:
  name: {{ include "betblocker.fullname" . }}-api
  labels:
    {{- include "betblocker.labels" . | nindent 4 }}
    app.kubernetes.io/component: api
spec:
  type: {{ .Values.api.service.type }}
  ports:
    - port: {{ .Values.api.service.port }}
      targetPort: http
      protocol: TCP
      name: http
  selector:
    {{- include "betblocker.selectorLabels" . | nindent 4 }}
    app.kubernetes.io/component: api
```

- [ ] **Step 5: Create API horizontal pod autoscaler**

```yaml
# deploy/helm/betblocker/templates/api-hpa.yaml
{{- if .Values.api.autoscaling.enabled }}
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: {{ include "betblocker.fullname" . }}-api
  labels:
    {{- include "betblocker.labels" . | nindent 4 }}
    app.kubernetes.io/component: api
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: {{ include "betblocker.fullname" . }}-api
  minReplicas: {{ .Values.api.autoscaling.minReplicas }}
  maxReplicas: {{ .Values.api.autoscaling.maxReplicas }}
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: {{ .Values.api.autoscaling.targetCPUUtilizationPercentage }}
    - type: Resource
      resource:
        name: memory
        target:
          type: Utilization
          averageUtilization: {{ .Values.api.autoscaling.targetMemoryUtilizationPercentage }}
{{- end }}
```

- [ ] **Step 6: Create worker deployment**

```yaml
# deploy/helm/betblocker/templates/worker-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: {{ include "betblocker.fullname" . }}-worker
  labels:
    {{- include "betblocker.labels" . | nindent 4 }}
    app.kubernetes.io/component: worker
spec:
  replicas: {{ .Values.worker.replicaCount }}
  selector:
    matchLabels:
      {{- include "betblocker.selectorLabels" . | nindent 6 }}
      app.kubernetes.io/component: worker
  template:
    metadata:
      labels:
        {{- include "betblocker.selectorLabels" . | nindent 8 }}
        app.kubernetes.io/component: worker
    spec:
      securityContext:
        {{- toYaml .Values.podSecurityContext | nindent 8 }}
      containers:
        - name: worker
          image: {{ .Values.worker.image.repository }}:{{ .Values.global.imageTag }}
          imagePullPolicy: {{ .Values.global.imagePullPolicy }}
          securityContext:
            {{- toYaml .Values.securityContext | nindent 12 }}
          envFrom:
            - secretRef:
                name: {{ .Values.secrets.existingSecret }}
            - configMapRef:
                name: {{ include "betblocker.fullname" . }}-config
          env:
            {{- range $key, $value := .Values.worker.env }}
            - name: {{ $key }}
              value: {{ $value | quote }}
            {{- end }}
          livenessProbe:
            httpGet:
              path: /healthz
              port: 9090
            initialDelaySeconds: 10
            periodSeconds: 30
            timeoutSeconds: 5
            failureThreshold: 3
          resources:
            {{- toYaml .Values.worker.resources | nindent 12 }}
          volumeMounts:
            - name: tmp
              mountPath: /tmp
      volumes:
        - name: tmp
          emptyDir:
            sizeLimit: 64Mi
```

- [ ] **Step 7: Create web deployment, service, and ingress**

```yaml
# deploy/helm/betblocker/templates/web-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: {{ include "betblocker.fullname" . }}-web
  labels:
    {{- include "betblocker.labels" . | nindent 4 }}
    app.kubernetes.io/component: web
spec:
  replicas: {{ .Values.web.replicaCount }}
  selector:
    matchLabels:
      {{- include "betblocker.selectorLabels" . | nindent 6 }}
      app.kubernetes.io/component: web
  template:
    metadata:
      labels:
        {{- include "betblocker.selectorLabels" . | nindent 8 }}
        app.kubernetes.io/component: web
    spec:
      securityContext:
        {{- toYaml .Values.podSecurityContext | nindent 8 }}
      containers:
        - name: web
          image: {{ .Values.web.image.repository }}:{{ .Values.global.imageTag }}
          imagePullPolicy: {{ .Values.global.imagePullPolicy }}
          securityContext:
            {{- toYaml .Values.securityContext | nindent 12 }}
          ports:
            - name: http
              containerPort: 3000
              protocol: TCP
          envFrom:
            - configMapRef:
                name: {{ include "betblocker.fullname" . }}-config
          env:
            - name: API_URL
              value: "http://{{ include "betblocker.fullname" . }}-api:{{ .Values.api.service.port }}"
            {{- range $key, $value := .Values.web.env }}
            - name: {{ $key }}
              value: {{ $value | quote }}
            {{- end }}
          livenessProbe:
            httpGet:
              path: /api/health
              port: http
            initialDelaySeconds: 10
            periodSeconds: 30
          readinessProbe:
            httpGet:
              path: /api/health
              port: http
            initialDelaySeconds: 5
            periodSeconds: 5
          resources:
            {{- toYaml .Values.web.resources | nindent 12 }}
          volumeMounts:
            - name: tmp
              mountPath: /tmp
            - name: next-cache
              mountPath: /app/.next/cache
      volumes:
        - name: tmp
          emptyDir:
            sizeLimit: 64Mi
        - name: next-cache
          emptyDir:
            sizeLimit: 256Mi
```

```yaml
# deploy/helm/betblocker/templates/web-service.yaml
apiVersion: v1
kind: Service
metadata:
  name: {{ include "betblocker.fullname" . }}-web
  labels:
    {{- include "betblocker.labels" . | nindent 4 }}
    app.kubernetes.io/component: web
spec:
  type: {{ .Values.web.service.type }}
  ports:
    - port: {{ .Values.web.service.port }}
      targetPort: http
      protocol: TCP
      name: http
  selector:
    {{- include "betblocker.selectorLabels" . | nindent 4 }}
    app.kubernetes.io/component: web
```

```yaml
# deploy/helm/betblocker/templates/web-ingress.yaml
{{- if .Values.ingress.enabled }}
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: {{ include "betblocker.fullname" . }}
  labels:
    {{- include "betblocker.labels" . | nindent 4 }}
  {{- with .Values.ingress.annotations }}
  annotations:
    {{- toYaml . | nindent 4 }}
  {{- end }}
spec:
  {{- if .Values.ingress.className }}
  ingressClassName: {{ .Values.ingress.className }}
  {{- end }}
  {{- if .Values.ingress.tls }}
  tls:
    {{- range .Values.ingress.tls }}
    - hosts:
        {{- range .hosts }}
        - {{ . | quote }}
        {{- end }}
      secretName: {{ .secretName }}
    {{- end }}
  {{- end }}
  rules:
    {{- range .Values.ingress.hosts }}
    - host: {{ .host | quote }}
      http:
        paths:
          {{- range .paths }}
          - path: {{ .path }}
            pathType: {{ .pathType }}
            backend:
              service:
                name: {{ include "betblocker.fullname" $ }}-{{ .service }}
                port:
                  name: http
          {{- end }}
    {{- end }}
{{- end }}
```

- [ ] **Step 8: Create secrets and configmap templates**

```yaml
# deploy/helm/betblocker/templates/secrets.yaml
# This template is a placeholder. In production, use ExternalSecrets
# or a sealed-secrets approach. This is provided for development/testing.
{{- if not .Values.secrets.existingSecret }}
apiVersion: v1
kind: Secret
metadata:
  name: {{ include "betblocker.fullname" . }}-secrets
  labels:
    {{- include "betblocker.labels" . | nindent 4 }}
type: Opaque
data:
  # These values must be base64-encoded.
  # In production, use ExternalSecrets to pull from AWS Secrets Manager.
  DATABASE_URL: {{ required "database.url is required" .Values.database.url | b64enc | quote }}
  REDIS_URL: {{ required "redis.url is required" .Values.redis.url | b64enc | quote }}
{{- end }}
```

```yaml
# deploy/helm/betblocker/templates/configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: {{ include "betblocker.fullname" . }}-config
  labels:
    {{- include "betblocker.labels" . | nindent 4 }}
data:
  BETBLOCKER_DEPLOYMENT: {{ .Values.global.deployment | quote }}
```

---

## Chunk 6: CI/CD Pipeline

### Task 8: Create the merge-to-main workflow

**Files:**
- Create: `.github/workflows/merge.yml`

- [ ] **Step 1: Write the merge workflow that builds and pushes Docker images**

This workflow runs on every merge to `main`. It builds all Docker images, pushes them to GHCR with the `:main` tag, builds the Linux agent binary, and runs E2E tests against the built images.

```yaml
# .github/workflows/merge.yml
name: Build & E2E (Main)

on:
  push:
    branches: [main]

concurrency:
  group: main-build
  cancel-in-progress: false

env:
  REGISTRY: ghcr.io
  IMAGE_PREFIX: ghcr.io/${{ github.repository_owner }}/betblocker

permissions:
  contents: read
  packages: write

jobs:
  # -----------------------------------------------------------
  # Build Docker images in parallel
  # -----------------------------------------------------------
  build-api:
    runs-on: ubuntu-latest
    outputs:
      image: ${{ steps.meta.outputs.tags }}
    steps:
      - uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Log in to GHCR
        uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Extract metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ${{ env.IMAGE_PREFIX }}-api
          tags: |
            type=raw,value=main
            type=sha,prefix=

      - name: Build and push API image
        uses: docker/build-push-action@v6
        with:
          context: .
          file: deploy/docker/Dockerfile.api
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          cache-from: type=gha,scope=api
          cache-to: type=gha,mode=max,scope=api

  build-worker:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Log in to GHCR
        uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Extract metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ${{ env.IMAGE_PREFIX }}-worker
          tags: |
            type=raw,value=main
            type=sha,prefix=

      - name: Build and push worker image
        uses: docker/build-push-action@v6
        with:
          context: .
          file: deploy/docker/Dockerfile.worker
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          cache-from: type=gha,scope=worker
          cache-to: type=gha,mode=max,scope=worker

  build-web:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Log in to GHCR
        uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Extract metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ${{ env.IMAGE_PREFIX }}-web
          tags: |
            type=raw,value=main
            type=sha,prefix=

      - name: Build and push web image
        uses: docker/build-push-action@v6
        with:
          context: ./web
          file: deploy/docker/Dockerfile.web
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          cache-from: type=gha,scope=web
          cache-to: type=gha,mode=max,scope=web

  build-agent-linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Build agent binary via Docker
        uses: docker/build-push-action@v6
        with:
          context: .
          file: deploy/docker/Dockerfile.agent-linux
          push: false
          load: true
          tags: betblocker-agent-linux:build
          cache-from: type=gha,scope=agent-linux
          cache-to: type=gha,mode=max,scope=agent-linux

      - name: Extract agent binary from image
        run: |
          docker create --name agent-extract betblocker-agent-linux:build
          docker cp agent-extract:/betblocker-agent-linux ./betblocker-agent-linux
          docker rm agent-extract
          chmod +x betblocker-agent-linux

      - name: Upload agent binary as artifact
        uses: actions/upload-artifact@v4
        with:
          name: betblocker-agent-linux
          path: betblocker-agent-linux
          retention-days: 7

  # -----------------------------------------------------------
  # E2E tests against built images
  # -----------------------------------------------------------
  e2e:
    needs: [build-api, build-worker, build-agent-linux]
    runs-on: ubuntu-latest
    services:
      db:
        image: timescale/timescaledb:latest-pg16
        env:
          POSTGRES_DB: betblocker
          POSTGRES_USER: betblocker
          POSTGRES_PASSWORD: testpass
        ports: [5432:5432]
        options: >-
          --health-cmd "pg_isready -U betblocker"
          --health-interval 5s
          --health-timeout 5s
          --health-retries 5
      cache:
        image: redis:7-alpine
        ports: [6379:6379]
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 5s
          --health-timeout 5s
          --health-retries 5
    steps:
      - uses: actions/checkout@v4

      - name: Log in to GHCR
        uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Start API container
        run: |
          docker run -d --name betblocker-api \
            --network host \
            -e DATABASE_URL=postgres://betblocker:testpass@localhost:5432/betblocker \
            -e REDIS_URL=redis://localhost:6379 \
            -e BETBLOCKER_DEPLOYMENT=self-hosted \
            -e BETBLOCKER_BILLING_ENABLED=false \
            -e BETBLOCKER_LOG_LEVEL=debug \
            ${{ env.IMAGE_PREFIX }}-api:main

      - name: Wait for API to be healthy
        run: |
          for i in $(seq 1 30); do
            if curl -sf http://localhost:8443/healthz; then
              echo "API is healthy"
              break
            fi
            echo "Waiting for API... ($i/30)"
            sleep 2
          done

      - name: Download agent binary
        uses: actions/download-artifact@v4
        with:
          name: betblocker-agent-linux

      - name: Run E2E tests
        run: |
          chmod +x betblocker-agent-linux
          # E2E test runner (implemented in tests/e2e/)
          cargo test --test e2e -- --test-threads=1
        env:
          API_URL: http://localhost:8443
          DATABASE_URL: postgres://betblocker:testpass@localhost:5432/betblocker
          REDIS_URL: redis://localhost:6379

  web-e2e:
    needs: [build-api, build-web]
    runs-on: ubuntu-latest
    services:
      db:
        image: timescale/timescaledb:latest-pg16
        env:
          POSTGRES_DB: betblocker
          POSTGRES_USER: betblocker
          POSTGRES_PASSWORD: testpass
        ports: [5432:5432]
        options: >-
          --health-cmd "pg_isready -U betblocker"
          --health-interval 5s
          --health-timeout 5s
          --health-retries 5
      cache:
        image: redis:7-alpine
        ports: [6379:6379]
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 5s
          --health-timeout 5s
          --health-retries 5
    steps:
      - uses: actions/checkout@v4

      - name: Log in to GHCR
        uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Start API + Web containers
        run: |
          docker run -d --name betblocker-api \
            --network host \
            -e DATABASE_URL=postgres://betblocker:testpass@localhost:5432/betblocker \
            -e REDIS_URL=redis://localhost:6379 \
            -e BETBLOCKER_DEPLOYMENT=self-hosted \
            -e BETBLOCKER_BILLING_ENABLED=false \
            ${{ env.IMAGE_PREFIX }}-api:main

          docker run -d --name betblocker-web \
            --network host \
            -e API_URL=http://localhost:8443 \
            -e NEXT_PUBLIC_API_URL=http://localhost:8443 \
            -e NEXT_PUBLIC_HOSTED=false \
            ${{ env.IMAGE_PREFIX }}-web:main

      - name: Wait for services
        run: |
          for i in $(seq 1 30); do
            if curl -sf http://localhost:8443/healthz && curl -sf http://localhost:3000/api/health; then
              echo "All services healthy"
              break
            fi
            sleep 2
          done

      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: npm
          cache-dependency-path: web/package-lock.json

      - name: Install Playwright
        working-directory: web
        run: |
          npm ci
          npx playwright install --with-deps chromium

      - name: Run Playwright E2E tests
        working-directory: web
        run: npx playwright test
        env:
          NEXT_PUBLIC_API_URL: http://localhost:8443
```

---

### Task 9: Create the release workflow

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Write the release workflow for version tags**

Triggered by pushing a `v*` tag. Builds all artifacts, signs agent binaries with Ed25519, creates a GitHub Release, and publishes Docker images with the version tag.

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags: ["v*"]

concurrency:
  group: release-${{ github.ref_name }}
  cancel-in-progress: false

env:
  REGISTRY: ghcr.io
  IMAGE_PREFIX: ghcr.io/${{ github.repository_owner }}/betblocker

permissions:
  contents: write
  packages: write

jobs:
  # -----------------------------------------------------------
  # Extract version from tag
  # -----------------------------------------------------------
  prepare:
    runs-on: ubuntu-latest
    outputs:
      version: ${{ steps.version.outputs.version }}
    steps:
      - name: Extract version
        id: version
        run: echo "version=${GITHUB_REF_NAME#v}" >> "$GITHUB_OUTPUT"

  # -----------------------------------------------------------
  # Build + push Docker images with version tags
  # -----------------------------------------------------------
  build-api-image:
    needs: prepare
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: docker/setup-buildx-action@v3

      - uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Build and push API image
        uses: docker/build-push-action@v6
        with:
          context: .
          file: deploy/docker/Dockerfile.api
          push: true
          tags: |
            ${{ env.IMAGE_PREFIX }}-api:${{ needs.prepare.outputs.version }}
            ${{ env.IMAGE_PREFIX }}-api:latest
          cache-from: type=gha,scope=api
          cache-to: type=gha,mode=max,scope=api

  build-worker-image:
    needs: prepare
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: docker/setup-buildx-action@v3

      - uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Build and push worker image
        uses: docker/build-push-action@v6
        with:
          context: .
          file: deploy/docker/Dockerfile.worker
          push: true
          tags: |
            ${{ env.IMAGE_PREFIX }}-worker:${{ needs.prepare.outputs.version }}
            ${{ env.IMAGE_PREFIX }}-worker:latest
          cache-from: type=gha,scope=worker
          cache-to: type=gha,mode=max,scope=worker

  build-web-image:
    needs: prepare
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: docker/setup-buildx-action@v3

      - uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Build and push web image
        uses: docker/build-push-action@v6
        with:
          context: ./web
          file: deploy/docker/Dockerfile.web
          push: true
          tags: |
            ${{ env.IMAGE_PREFIX }}-web:${{ needs.prepare.outputs.version }}
            ${{ env.IMAGE_PREFIX }}-web:latest
          cache-from: type=gha,scope=web
          cache-to: type=gha,mode=max,scope=web

  # -----------------------------------------------------------
  # Build agent binaries (Linux only in Phase 1)
  # -----------------------------------------------------------
  build-agent-linux:
    needs: prepare
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: docker/setup-buildx-action@v3

      - name: Build agent binary via Docker
        uses: docker/build-push-action@v6
        with:
          context: .
          file: deploy/docker/Dockerfile.agent-linux
          push: false
          load: true
          tags: betblocker-agent-linux:build
          cache-from: type=gha,scope=agent-linux
          cache-to: type=gha,mode=max,scope=agent-linux

      - name: Extract agent binary
        run: |
          docker create --name agent-extract betblocker-agent-linux:build
          docker cp agent-extract:/betblocker-agent-linux ./betblocker-agent-linux
          docker rm agent-extract
          chmod +x betblocker-agent-linux

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: betblocker-agent-linux
          path: betblocker-agent-linux

  # -----------------------------------------------------------
  # Sign agent binaries
  # -----------------------------------------------------------
  sign-binaries:
    needs: [prepare, build-agent-linux]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Download all agent binaries
        uses: actions/download-artifact@v4
        with:
          path: artifacts

      - name: Install signing dependencies
        run: |
          sudo apt-get update && sudo apt-get install -y openssl

      - name: Sign binaries with Ed25519
        env:
          SIGNING_KEY_BASE64: ${{ secrets.ED25519_RELEASE_SIGNING_KEY }}
        run: |
          # Decode the signing key from CI secret
          echo "${SIGNING_KEY_BASE64}" | base64 -d > /tmp/release-signing.key

          # Sign each binary
          for binary_dir in artifacts/betblocker-agent-*; do
            binary_name=$(basename "${binary_dir}")
            binary_path="${binary_dir}/${binary_name}"

            if [ -f "${binary_path}" ]; then
              echo "Signing ${binary_name}..."
              bash tools/signing/sign-binary.sh \
                "${binary_path}" \
                /tmp/release-signing.key \
                "${binary_path}.sig"

              echo "Verifying signature for ${binary_name}..."
              bash tools/signing/verify-binary.sh \
                "${binary_path}" \
                "${binary_path}.sig" \
                tools/signing/release-signing.pub
            fi
          done

          # Clean up key
          rm -f /tmp/release-signing.key

      - name: Upload signed artifacts
        uses: actions/upload-artifact@v4
        with:
          name: signed-binaries
          path: artifacts/

  # -----------------------------------------------------------
  # Create GitHub Release
  # -----------------------------------------------------------
  release:
    needs: [prepare, build-api-image, build-worker-image, build-web-image, sign-binaries]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Download signed binaries
        uses: actions/download-artifact@v4
        with:
          name: signed-binaries
          path: release-artifacts

      - name: Flatten release artifacts
        run: |
          mkdir -p release-files
          find release-artifacts -type f -exec cp {} release-files/ \;

      - name: Generate changelog
        id: changelog
        run: |
          # Get previous tag
          PREV_TAG=$(git describe --tags --abbrev=0 HEAD^ 2>/dev/null || echo "")
          if [ -n "${PREV_TAG}" ]; then
            CHANGELOG=$(git log "${PREV_TAG}..HEAD" --pretty=format:"- %s (%h)" --no-merges)
          else
            CHANGELOG=$(git log --pretty=format:"- %s (%h)" --no-merges -20)
          fi
          # Write to file for the release body
          cat > /tmp/release-notes.md << 'NOTES_EOF'
          ## BetBlocker ${{ github.ref_name }}

          ### Docker Images

          ```bash
          docker pull ${{ env.IMAGE_PREFIX }}-api:${{ needs.prepare.outputs.version }}
          docker pull ${{ env.IMAGE_PREFIX }}-worker:${{ needs.prepare.outputs.version }}
          docker pull ${{ env.IMAGE_PREFIX }}-web:${{ needs.prepare.outputs.version }}
          ```

          ### Self-Hosted Upgrade

          ```bash
          cd deploy
          # Update version in .env
          echo "BETBLOCKER_VERSION=${{ needs.prepare.outputs.version }}" >> .env
          docker compose pull
          docker compose up -d
          ```

          ### Agent Binaries

          Download the agent binary for your platform below. Each binary has a corresponding `.sig` file for Ed25519 signature verification.

          Verify with:
          ```bash
          bash tools/signing/verify-binary.sh betblocker-agent-linux betblocker-agent-linux.sig
          ```

          ### Changes

          NOTES_EOF
          echo "${CHANGELOG}" >> /tmp/release-notes.md

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          tag_name: ${{ github.ref_name }}
          name: BetBlocker ${{ github.ref_name }}
          body_path: /tmp/release-notes.md
          draft: false
          prerelease: ${{ contains(github.ref_name, '-rc') || contains(github.ref_name, '-beta') }}
          files: release-files/*
```

---

## Chunk 7: Binary Signing Scripts

### Task 10: Create binary signing and verification scripts

**Files:**
- Create: `tools/signing/sign-binary.sh`
- Create: `tools/signing/verify-binary.sh`
- Create: `tools/signing/generate-release-keypair.sh`

- [ ] **Step 1: Write the signing script**

Uses `openssl` to perform Ed25519 signing. The signature is computed over the SHA-256 hash of the binary.

```bash
#!/usr/bin/env bash
# tools/signing/sign-binary.sh
# Sign a binary with an Ed25519 private key.
#
# Usage:
#   sign-binary.sh <binary-path> <private-key-path> <signature-output-path>
#
# The signature is a raw Ed25519 signature over the binary file,
# base64-encoded and written to the output path.

set -euo pipefail

if [ $# -ne 3 ]; then
    echo "Usage: $0 <binary-path> <private-key-path> <signature-output-path>"
    exit 1
fi

BINARY_PATH="$1"
KEY_PATH="$2"
SIG_PATH="$3"

if [ ! -f "${BINARY_PATH}" ]; then
    echo "Error: Binary not found: ${BINARY_PATH}"
    exit 1
fi

if [ ! -f "${KEY_PATH}" ]; then
    echo "Error: Private key not found: ${KEY_PATH}"
    exit 1
fi

# Sign the binary directly with Ed25519
# openssl pkeyutl -sign produces a raw Ed25519 signature (64 bytes)
openssl pkeyutl \
    -sign \
    -inkey "${KEY_PATH}" \
    -rawin \
    -in <(sha256sum "${BINARY_PATH}" | awk '{print $1}' | xxd -r -p) \
    -out "${SIG_PATH}.raw"

# Base64 encode the signature for portability
base64 < "${SIG_PATH}.raw" > "${SIG_PATH}"
rm -f "${SIG_PATH}.raw"

echo "Signed: ${BINARY_PATH}"
echo "Signature: ${SIG_PATH}"
echo "SHA-256: $(sha256sum "${BINARY_PATH}" | awk '{print $1}')"
```

- [ ] **Step 2: Write the verification script**

```bash
#!/usr/bin/env bash
# tools/signing/verify-binary.sh
# Verify an Ed25519 signature on a binary.
#
# Usage:
#   verify-binary.sh <binary-path> <signature-path> [public-key-path]
#
# If public-key-path is omitted, uses tools/signing/release-signing.pub

set -euo pipefail

if [ $# -lt 2 ] || [ $# -gt 3 ]; then
    echo "Usage: $0 <binary-path> <signature-path> [public-key-path]"
    exit 1
fi

BINARY_PATH="$1"
SIG_PATH="$2"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PUB_KEY_PATH="${3:-${SCRIPT_DIR}/release-signing.pub}"

if [ ! -f "${BINARY_PATH}" ]; then
    echo "Error: Binary not found: ${BINARY_PATH}"
    exit 1
fi

if [ ! -f "${SIG_PATH}" ]; then
    echo "Error: Signature not found: ${SIG_PATH}"
    exit 1
fi

if [ ! -f "${PUB_KEY_PATH}" ]; then
    echo "Error: Public key not found: ${PUB_KEY_PATH}"
    exit 1
fi

# Decode the base64 signature
base64 -d < "${SIG_PATH}" > /tmp/sig.raw

# Verify the signature
if openssl pkeyutl \
    -verify \
    -pubin \
    -inkey "${PUB_KEY_PATH}" \
    -rawin \
    -in <(sha256sum "${BINARY_PATH}" | awk '{print $1}' | xxd -r -p) \
    -sigfile /tmp/sig.raw; then
    echo "VERIFIED: ${BINARY_PATH} signature is valid."
    rm -f /tmp/sig.raw
    exit 0
else
    echo "FAILED: ${BINARY_PATH} signature verification failed!"
    rm -f /tmp/sig.raw
    exit 1
fi
```

- [ ] **Step 3: Write the keypair generation script (for initial setup)**

```bash
#!/usr/bin/env bash
# tools/signing/generate-release-keypair.sh
# Generate an Ed25519 keypair for release binary signing.
# Run this ONCE, store the private key securely, and commit the public key.
#
# Usage:
#   generate-release-keypair.sh [output-dir]
#
# Output:
#   release-signing.key  (PRIVATE - store in GitHub Actions secrets, never commit)
#   release-signing.pub  (PUBLIC  - commit to repo at tools/signing/release-signing.pub)

set -euo pipefail

OUTPUT_DIR="${1:-.}"

echo "Generating Ed25519 release signing keypair..."

# Generate private key
openssl genpkey -algorithm Ed25519 -out "${OUTPUT_DIR}/release-signing.key"

# Extract public key
openssl pkey -in "${OUTPUT_DIR}/release-signing.key" -pubout -out "${OUTPUT_DIR}/release-signing.pub"

echo ""
echo "Generated:"
echo "  Private key: ${OUTPUT_DIR}/release-signing.key"
echo "  Public key:  ${OUTPUT_DIR}/release-signing.pub"
echo ""
echo "NEXT STEPS:"
echo "  1. Add the private key to GitHub Actions secrets as ED25519_RELEASE_SIGNING_KEY:"
echo "     cat ${OUTPUT_DIR}/release-signing.key | base64 | pbcopy"
echo "     (paste into GitHub Settings > Secrets > Actions > New repository secret)"
echo ""
echo "  2. Commit the public key to the repository:"
echo "     cp ${OUTPUT_DIR}/release-signing.pub tools/signing/release-signing.pub"
echo "     git add tools/signing/release-signing.pub"
echo ""
echo "  3. DELETE the private key from your local machine after uploading to GitHub."
echo "     Store a backup in a secure offline location (e.g., encrypted USB drive)."
```

- [ ] **Step 4: Make all signing scripts executable**

```bash
chmod +x tools/signing/sign-binary.sh
chmod +x tools/signing/verify-binary.sh
chmod +x tools/signing/generate-release-keypair.sh
```

---

## Chunk 8: Enhance PR Workflow

### Task 11: Verify and enhance the existing PR workflow

**Files:**
- Update: `.github/workflows/pr.yml` (exists from SP1)

The PR workflow from SP1 already covers `cargo fmt --check`, `cargo clippy`, `cargo test`, `cargo deny check`, and web linting. This task adds a Docker build check to catch Dockerfile issues early.

- [ ] **Step 1: Add Docker build verification job to pr.yml**

Add the following job to the existing `pr.yml`:

```yaml
  # Add to existing pr.yml jobs section:
  docker-build-check:
    runs-on: ubuntu-latest
    # Only run if Dockerfiles or Cargo.toml changed
    if: |
      contains(github.event.pull_request.changed_files, 'Dockerfile') ||
      contains(github.event.pull_request.changed_files, 'Cargo.toml') ||
      contains(github.event.pull_request.changed_files, 'Cargo.lock')
    steps:
      - uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Lint Dockerfiles
        uses: hadolint/hadolint-action@v3.1.0
        with:
          dockerfile: deploy/docker/Dockerfile.api

      - name: Lint Dockerfile.worker
        uses: hadolint/hadolint-action@v3.1.0
        with:
          dockerfile: deploy/docker/Dockerfile.worker

      - name: Lint Dockerfile.web
        uses: hadolint/hadolint-action@v3.1.0
        with:
          dockerfile: deploy/docker/Dockerfile.web

      - name: Verify API image builds
        uses: docker/build-push-action@v6
        with:
          context: .
          file: deploy/docker/Dockerfile.api
          push: false
          cache-from: type=gha,scope=api
```

---

## Verification & Testing

### Task 12: Verify the complete deployment pipeline

- [ ] **Step 1: Local Docker build test**

Build all four Docker images locally and verify sizes:
```bash
# From repo root
docker build -f deploy/docker/Dockerfile.api -t betblocker-api:test .
docker build -f deploy/docker/Dockerfile.worker -t betblocker-worker:test .
docker build -f deploy/docker/Dockerfile.web -t betblocker-web:test web/
docker build -f deploy/docker/Dockerfile.agent-linux -t betblocker-agent:test .

# Check image sizes (API/worker should be ~15-20MB, web ~100-150MB)
docker images | grep betblocker
```

- [ ] **Step 2: Local docker-compose smoke test**

```bash
cd deploy
cp .env.example .env
# Edit .env with a real password
docker compose up -d
docker compose ps    # All services should be healthy
docker compose logs api --tail 20
curl -sf http://localhost:8443/healthz && echo "API OK"
curl -sf http://localhost:80/api/health && echo "Web OK"
docker compose down
```

- [ ] **Step 3: Run setup.sh end-to-end**

```bash
cd deploy
cp .env.example .env
# Set DB_PASSWORD and BETBLOCKER_EXTERNAL_URL
export BETBLOCKER_ADMIN_EMAIL=admin@test.local
export BETBLOCKER_ADMIN_PASSWORD=testpassword123
../scripts/setup.sh

# Verify idempotency
../scripts/setup.sh  # Should skip all steps with "already exists" messages
```

- [ ] **Step 4: Validate Helm chart**

```bash
# Lint the chart
helm lint deploy/helm/betblocker/

# Render templates to verify
helm template betblocker deploy/helm/betblocker/ \
  --set database.url=postgres://test:test@db:5432/test \
  --set redis.url=redis://cache:6379
```

- [ ] **Step 5: Verify security properties**

Confirm the following for each Docker image:
- Runs as non-root (UID 65534 for scratch images, UID 1001 for web)
- Read-only root filesystem (except tmpfs mounts)
- No shell or package manager in API/worker images (scratch base)
- `no-new-privileges` security option in docker-compose
- Health checks configured for all services
- Keys volume mounted read-only in API and worker containers

- [ ] **Step 6: Generate release signing keypair and verify signing flow**

```bash
# Generate test keypair
bash tools/signing/generate-release-keypair.sh /tmp/test-keys

# Build a test binary
docker build -f deploy/docker/Dockerfile.agent-linux -t agent:test .
docker create --name tmp agent:test
docker cp tmp:/betblocker-agent-linux /tmp/test-agent
docker rm tmp

# Sign and verify
bash tools/signing/sign-binary.sh /tmp/test-agent /tmp/test-keys/release-signing.key /tmp/test-agent.sig
bash tools/signing/verify-binary.sh /tmp/test-agent /tmp/test-agent.sig /tmp/test-keys/release-signing.pub

# Clean up
rm -f /tmp/test-keys/release-signing.key /tmp/test-agent /tmp/test-agent.sig
```
