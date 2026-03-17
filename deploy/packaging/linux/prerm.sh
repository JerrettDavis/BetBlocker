#!/bin/sh
set -e

# Stop and disable the service before removal
if systemctl is-active --quiet betblocker-agent.service; then
    systemctl stop betblocker-agent.service
fi
systemctl disable betblocker-agent.service 2>/dev/null || true
systemctl daemon-reload
