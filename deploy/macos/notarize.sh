#!/bin/bash
# Notarize a BetBlocker macOS package
set -e

PKG_PATH="${1:?Usage: notarize.sh <pkg-path>}"
APPLE_ID="${APPLE_ID:?Set APPLE_ID environment variable}"
TEAM_ID="${TEAM_ID:?Set TEAM_ID environment variable}"
APP_PASSWORD="${APP_PASSWORD:?Set APP_PASSWORD environment variable}"

echo "Submitting $PKG_PATH for notarization..."
xcrun notarytool submit "$PKG_PATH" \
    --apple-id "$APPLE_ID" \
    --team-id "$TEAM_ID" \
    --password "$APP_PASSWORD" \
    --wait

echo "Stapling notarization ticket..."
xcrun stapler staple "$PKG_PATH"

echo "Verifying..."
spctl --assess --type install "$PKG_PATH"

echo "Notarization complete"
