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
