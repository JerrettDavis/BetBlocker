#!/bin/bash
# Post-installation script for BetBlocker macOS agent
set -e

INSTALL_DIR="/Library/Application Support/BetBlocker"
BINARY="/usr/local/bin/betblocker-agent"
PLIST="/Library/LaunchDaemons/com.betblocker.agent.plist"

# Create directories
mkdir -p "$INSTALL_DIR"/{certs,logs,plugins}
chmod 755 "$INSTALL_DIR"
chmod 700 "$INSTALL_DIR/certs"

# Set binary permissions
chmod 755 "$BINARY"
chown root:wheel "$BINARY"

# Load launch daemon
launchctl bootstrap system "$PLIST" 2>/dev/null || true
launchctl enable system/com.betblocker.agent 2>/dev/null || true

echo "BetBlocker agent installed successfully"
exit 0
