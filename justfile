# BetBlocker development commands

# Start dev infrastructure (PostgreSQL + Redis)
infra:
    docker compose -f deploy/docker-compose.dev.yml up -d

# Stop dev infrastructure
infra-down:
    docker compose -f deploy/docker-compose.dev.yml down

# Run database migrations
migrate:
    sqlx migrate run --source migrations

# Run API server
api:
    cargo run -p bb-api

# Run worker
worker:
    cargo run -p bb-worker

# Run web dev server
web:
    cd web && npm run dev

# Build agent for current OS
agent:
    cargo build -p bb-agent-linux

# Run all tests
test:
    cargo test --workspace

# Run clippy
lint:
    cargo clippy --workspace -- -D warnings

# Format check
fmt-check:
    cargo fmt --all -- --check

# Format
fmt:
    cargo fmt --all

# Full CI check (format + lint + test)
ci: fmt-check lint test
