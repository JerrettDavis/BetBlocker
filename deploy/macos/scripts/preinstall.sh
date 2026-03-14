#!/bin/bash
# Pre-installation script — stop existing daemon if running
set -e

if launchctl print system/com.betblocker.agent &>/dev/null; then
    launchctl bootout system/com.betblocker.agent 2>/dev/null || true
    sleep 1
fi

exit 0
