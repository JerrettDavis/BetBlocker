#!/usr/bin/env bash
# BetBlocker Agent -- Linux Installation Script
#
# Usage:
#   sudo ./install.sh [--binary /path/to/bb-agent-linux]
#
# Expected output on a clean Ubuntu 22.04+ or Debian 12+ system:
#   1. Copies the agent binary to /usr/lib/betblocker/
#   2. Creates data and log directories
#   3. Installs the systemd service unit
#   4. Enables and starts the service
#   5. Verifies the service is healthy within 10 seconds
#
# Prerequisites:
#   - Root access (run with sudo)
#   - systemd
#   - nftables (for DNS redirection)

set -euo pipefail

INSTALL_DIR="/usr/lib/betblocker"
DATA_DIR="/var/lib/betblocker"
LOG_DIR="/var/log/betblocker"
SERVICE_NAME="betblocker-agent"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"

# Default binary path: same directory as this script
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BINARY_PATH="${SCRIPT_DIR}/bb-agent-linux"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info()  { echo -e "${GREEN}[INFO]${NC}  $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*"; exit 1; }

# --- Argument parsing ---
while [[ $# -gt 0 ]]; do
    case "$1" in
        --binary)
            BINARY_PATH="$2"
            shift 2
            ;;
        --help|-h)
            echo "Usage: sudo $0 [--binary /path/to/bb-agent-linux]"
            exit 0
            ;;
        *)
            error "Unknown argument: $1"
            ;;
    esac
done

# --- Pre-flight checks ---
if [[ $EUID -ne 0 ]]; then
    error "This script must be run as root (use sudo)"
fi

if ! command -v systemctl &>/dev/null; then
    error "systemd is required but not found"
fi

if ! command -v nft &>/dev/null; then
    warn "nftables (nft) not found. DNS redirection will not work."
    warn "Install with: apt install nftables"
fi

if [[ ! -f "${BINARY_PATH}" ]]; then
    error "Agent binary not found at: ${BINARY_PATH}"
fi

# --- Stop existing service if running ---
if systemctl is-active --quiet "${SERVICE_NAME}" 2>/dev/null; then
    info "Stopping existing ${SERVICE_NAME} service..."
    systemctl stop "${SERVICE_NAME}" || true
fi

# --- Install binary ---
info "Installing agent binary to ${INSTALL_DIR}/"
mkdir -p "${INSTALL_DIR}"
cp "${BINARY_PATH}" "${INSTALL_DIR}/bb-agent-linux"
chown root:root "${INSTALL_DIR}/bb-agent-linux"
chmod 755 "${INSTALL_DIR}/bb-agent-linux"

# Set immutable attribute to prevent casual deletion
if command -v chattr &>/dev/null; then
    chattr +i "${INSTALL_DIR}/bb-agent-linux" 2>/dev/null || \
        warn "Could not set immutable attribute (filesystem may not support it)"
fi

# --- Create data directories ---
info "Creating data directories..."
mkdir -p "${DATA_DIR}" "${DATA_DIR}/certs" "${LOG_DIR}"
chown root:root "${DATA_DIR}" "${DATA_DIR}/certs" "${LOG_DIR}"
chmod 700 "${DATA_DIR}" "${DATA_DIR}/certs"
chmod 750 "${LOG_DIR}"

# --- Install systemd unit file ---
info "Installing systemd service unit..."
cp "${SCRIPT_DIR}/betblocker-agent.service" "${SERVICE_FILE}"
chown root:root "${SERVICE_FILE}"
chmod 644 "${SERVICE_FILE}"

# --- Reload systemd and enable service ---
info "Enabling ${SERVICE_NAME} service..."
systemctl daemon-reload
systemctl enable "${SERVICE_NAME}"

# --- Start service ---
info "Starting ${SERVICE_NAME} service..."
systemctl start "${SERVICE_NAME}"

# --- Verify health ---
info "Verifying service health (10s timeout)..."
HEALTHY=false
for i in $(seq 1 10); do
    if systemctl is-active --quiet "${SERVICE_NAME}" 2>/dev/null; then
        HEALTHY=true
        break
    fi
    sleep 1
done

if $HEALTHY; then
    info "BetBlocker Agent installed and running successfully!"
    info ""
    info "  Binary:  ${INSTALL_DIR}/bb-agent-linux"
    info "  Config:  ${DATA_DIR}/agent.toml"
    info "  Logs:    journalctl -u ${SERVICE_NAME}"
    info "  Status:  systemctl status ${SERVICE_NAME}"
    info ""
    info "To enroll a device, run:"
    info "  ${INSTALL_DIR}/bb-agent-linux --enroll <TOKEN>"
else
    error "Service failed to start within 10 seconds. Check logs:"
    error "  journalctl -u ${SERVICE_NAME} -n 50 --no-pager"
fi
