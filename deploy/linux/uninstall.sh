#!/usr/bin/env bash
# BetBlocker Agent -- Linux Uninstallation Script
#
# Usage:
#   sudo ./uninstall.sh --confirm
#
# This script:
#   1. Stops and disables the systemd service
#   2. Removes nftables rules
#   3. Removes the binary, data directory, and log directory
#   4. Removes the systemd unit file

set -euo pipefail

INSTALL_DIR="/usr/lib/betblocker"
DATA_DIR="/var/lib/betblocker"
LOG_DIR="/var/log/betblocker"
SERVICE_NAME="betblocker-agent"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[INFO]${NC}  $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*"; exit 1; }

# --- Require --confirm ---
if [[ "${1:-}" != "--confirm" ]]; then
    echo "BetBlocker Agent Uninstaller"
    echo ""
    echo "This will completely remove the BetBlocker Agent, including:"
    echo "  - Agent binary (${INSTALL_DIR}/)"
    echo "  - Configuration and data (${DATA_DIR}/)"
    echo "  - Log files (${LOG_DIR}/)"
    echo "  - systemd service unit"
    echo "  - nftables DNS redirect rules"
    echo ""
    echo "To proceed, run:"
    echo "  sudo $0 --confirm"
    exit 1
fi

# --- Pre-flight checks ---
if [[ $EUID -ne 0 ]]; then
    error "This script must be run as root (use sudo)"
fi

# --- Stop and disable service ---
if systemctl is-active --quiet "${SERVICE_NAME}" 2>/dev/null; then
    info "Stopping ${SERVICE_NAME} service..."
    systemctl stop "${SERVICE_NAME}" || true
fi

if systemctl is-enabled --quiet "${SERVICE_NAME}" 2>/dev/null; then
    info "Disabling ${SERVICE_NAME} service..."
    systemctl disable "${SERVICE_NAME}" || true
fi

# --- Remove nftables rules ---
if command -v nft &>/dev/null; then
    info "Removing nftables rules..."
    nft delete table inet betblocker 2>/dev/null || true
fi

# --- Remove systemd unit file ---
if [[ -f "${SERVICE_FILE}" ]]; then
    info "Removing systemd unit file..."
    rm -f "${SERVICE_FILE}"
    systemctl daemon-reload
fi

# --- Remove binary ---
if [[ -d "${INSTALL_DIR}" ]]; then
    info "Removing agent binary..."
    # Clear immutable attribute first
    if command -v chattr &>/dev/null; then
        chattr -i "${INSTALL_DIR}/bb-agent-linux" 2>/dev/null || true
    fi
    rm -rf "${INSTALL_DIR}"
fi

# --- Remove data and log directories ---
if [[ -d "${DATA_DIR}" ]]; then
    info "Removing data directory (${DATA_DIR})..."
    rm -rf "${DATA_DIR}"
fi

if [[ -d "${LOG_DIR}" ]]; then
    info "Removing log directory (${LOG_DIR})..."
    rm -rf "${LOG_DIR}"
fi

info "BetBlocker Agent has been completely uninstalled."
