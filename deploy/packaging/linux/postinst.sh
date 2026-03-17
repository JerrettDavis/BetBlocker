#!/bin/sh
set -e

# Create betblocker system user if it doesn't exist
if ! getent passwd betblocker > /dev/null 2>&1; then
    useradd --system --no-create-home --shell /usr/sbin/nologin betblocker
fi

# Create data and log directories
mkdir -p /var/lib/betblocker /var/log/betblocker
chown betblocker:betblocker /var/lib/betblocker /var/log/betblocker

# Reload systemd and enable the service
systemctl daemon-reload
systemctl enable betblocker-agent.service

echo "BetBlocker agent installed. Run 'sudo systemctl start betblocker-agent' to start."
