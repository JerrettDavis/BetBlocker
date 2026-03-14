#!/bin/bash
# Uninstall BetBlocker macOS agent
set -e

echo "Stopping BetBlocker agent..."
launchctl bootout system/com.betblocker.agent 2>/dev/null || true

echo "Removing files..."
rm -f /usr/local/bin/betblocker-agent
rm -f /Library/LaunchDaemons/com.betblocker.agent.plist

# Remove pfctl rules
pfctl -a com.betblocker -F all 2>/dev/null || true

echo "BetBlocker agent uninstalled"
echo "Note: Data directory at /Library/Application Support/BetBlocker/ preserved"
echo "To remove all data: sudo rm -rf '/Library/Application Support/BetBlocker/'"
