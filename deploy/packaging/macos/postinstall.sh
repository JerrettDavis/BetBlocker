#!/bin/bash
set -e

# Create directories
mkdir -p "/Library/Application Support/BetBlocker"
mkdir -p "/Library/Logs/BetBlocker"
mkdir -p "/Library/BetBlocker"

# Copy LaunchDaemon plist
cp "/Library/BetBlocker/com.betblocker.agent.plist" "/Library/LaunchDaemons/"

# Install default config if none exists
if [ ! -f "/Library/Application Support/BetBlocker/agent.toml" ]; then
    cp "/Library/BetBlocker/agent.toml" "/Library/Application Support/BetBlocker/agent.toml"
fi

# Load the daemon
launchctl bootstrap system "/Library/LaunchDaemons/com.betblocker.agent.plist" 2>/dev/null || true

echo "BetBlocker agent installed."
