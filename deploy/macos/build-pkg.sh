#!/usr/bin/env bash
# build-pkg.sh — Build and optionally notarize the BetBlocker macOS installer.
#
# Usage:
#   ./deploy/macos/build-pkg.sh [--version <version>] [--notarize] \
#       [--apple-id <email>] [--team-id <TEAMID>] \
#       [--keychain-profile <profile>]
#
# Prerequisites:
#   • Rust toolchain (cargo, rustup target add aarch64-apple-darwin)
#   • Xcode Command Line Tools (pkgbuild, productbuild, xcrun)
#   • Apple Developer certificate installed in the login Keychain
#     (for signing) or a notarization credential stored via
#     `xcrun notarytool store-credentials`.
#
# Output:
#   build/macos/BetBlocker-<version>.pkg
#
# Exit codes:
#   0  success
#   1  build or packaging failure

set -euo pipefail

# ---------------------------------------------------------------------------
# Defaults
# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

VERSION="${VERSION:-$(cargo metadata --no-deps --manifest-path "${REPO_ROOT}/Cargo.toml" \
    --format-version 1 | python3 -c "import json,sys; \
    pkgs=json.load(sys.stdin)['packages']; \
    print(next(p['version'] for p in pkgs if p['name']=='bb-agent-macos'))" 2>/dev/null || echo "0.1.0")}"

DO_NOTARIZE=false
APPLE_ID=""
TEAM_ID=""
KEYCHAIN_PROFILE=""
SIGN_IDENTITY=""          # e.g. "Developer ID Installer: Acme Corp (TEAMID)"

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------
while [[ $# -gt 0 ]]; do
    case "$1" in
        --version)         VERSION="$2";           shift 2 ;;
        --notarize)        DO_NOTARIZE=true;        shift   ;;
        --apple-id)        APPLE_ID="$2";           shift 2 ;;
        --team-id)         TEAM_ID="$2";            shift 2 ;;
        --keychain-profile) KEYCHAIN_PROFILE="$2"; shift 2 ;;
        --sign-identity)   SIGN_IDENTITY="$2";      shift 2 ;;
        *)
            echo "Unknown argument: $1" >&2
            exit 1
            ;;
    esac
done

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------
BUILD_DIR="${REPO_ROOT}/build/macos"
PKG_ROOT="${BUILD_DIR}/pkg-root"
PKG_SCRIPTS="${BUILD_DIR}/scripts"
COMPONENT_PKG="${BUILD_DIR}/BetBlocker-component.pkg"
DISTRIBUTION_PKG="${BUILD_DIR}/BetBlocker-${VERSION}.pkg"
BINARY_SRC="${REPO_ROOT}/target/release/bb-agent-macos"
BINARY_DEST="${PKG_ROOT}/usr/local/bin/bb-agent-macos"
PLIST_DEST="${PKG_ROOT}/Library/LaunchDaemons/com.betblocker.agent.plist"

log()  { echo "[build-pkg] $*"; }
warn() { echo "[build-pkg] WARNING: $*" >&2; }
die()  { echo "[build-pkg] ERROR: $*" >&2; exit 1; }

# ---------------------------------------------------------------------------
# Step 1: Build the Rust binary
# ---------------------------------------------------------------------------
log "Building bb-agent-macos (release)..."

# Detect architecture — build a universal binary if both targets are available.
TARGETS=()
if rustup target list --installed | grep -q "x86_64-apple-darwin"; then
    TARGETS+=("x86_64-apple-darwin")
fi
if rustup target list --installed | grep -q "aarch64-apple-darwin"; then
    TARGETS+=("aarch64-apple-darwin")
fi

if [[ ${#TARGETS[@]} -eq 2 ]]; then
    log "Building universal binary (x86_64 + arm64)..."
    cargo build --release -p bb-agent-macos --target x86_64-apple-darwin
    cargo build --release -p bb-agent-macos --target aarch64-apple-darwin
    mkdir -p "$(dirname "${BINARY_SRC}")"
    lipo -create \
        "${REPO_ROOT}/target/x86_64-apple-darwin/release/bb-agent-macos" \
        "${REPO_ROOT}/target/aarch64-apple-darwin/release/bb-agent-macos" \
        -output "${BINARY_SRC}"
    log "Universal binary created at ${BINARY_SRC}"
elif [[ ${#TARGETS[@]} -eq 1 ]]; then
    log "Building single-arch binary for ${TARGETS[0]}..."
    cargo build --release -p bb-agent-macos --target "${TARGETS[0]}"
    cp "${REPO_ROOT}/target/${TARGETS[0]}/release/bb-agent-macos" "${BINARY_SRC}"
else
    log "Using default host target..."
    cargo build --release -p bb-agent-macos
    log "Binary at ${BINARY_SRC}"
fi

[[ -f "${BINARY_SRC}" ]] || die "Binary not found at ${BINARY_SRC}"
log "Binary built: $(file "${BINARY_SRC}")"

# ---------------------------------------------------------------------------
# Step 2: Assemble installer package root
# ---------------------------------------------------------------------------
log "Assembling package root..."
rm -rf "${PKG_ROOT}"

# Agent binary
install -d "${PKG_ROOT}/usr/local/bin"
install -m 0755 "${BINARY_SRC}" "${BINARY_DEST}"

# LaunchDaemon plist
install -d "${PKG_ROOT}/Library/LaunchDaemons"
install -m 0644 "${SCRIPT_DIR}/com.betblocker.agent.plist" "${PLIST_DEST}"

# Application support and log directories (created at install time by
# the postinstall script; we include them here so pkg sets ownership).
install -d "${PKG_ROOT}/Library/Application Support/BetBlocker"
install -d "${PKG_ROOT}/var/log/betblocker"

# Write version file
echo "${VERSION}" > "${PKG_ROOT}/Library/Application Support/BetBlocker/version"

# ---------------------------------------------------------------------------
# Step 3: Installer scripts (postinstall / preremove)
# ---------------------------------------------------------------------------
log "Creating installer scripts..."
rm -rf "${PKG_SCRIPTS}"
mkdir -p "${PKG_SCRIPTS}"

cat > "${PKG_SCRIPTS}/postinstall" <<'POSTINSTALL_EOF'
#!/bin/bash
set -euo pipefail

log() { echo "[postinstall] $*"; }

# Set correct ownership and permissions
chown root:wheel /usr/local/bin/bb-agent-macos
chmod 0755        /usr/local/bin/bb-agent-macos

chown root:wheel /Library/LaunchDaemons/com.betblocker.agent.plist
chmod 0644        /Library/LaunchDaemons/com.betblocker.agent.plist

# Ensure directories
install -d -m 0755 -o root -g wheel "/Library/Application Support/BetBlocker"
install -d -m 0755 -o root -g wheel "/var/log/betblocker"

# Bootstrap the LaunchDaemon
if launchctl print system/com.betblocker.agent &>/dev/null; then
    log "Daemon already loaded; reloading..."
    launchctl bootout system/com.betblocker.agent 2>/dev/null || true
fi

launchctl bootstrap system /Library/LaunchDaemons/com.betblocker.agent.plist \
    && log "Daemon bootstrapped successfully" \
    || log "WARNING: launchctl bootstrap failed (may need reboot)"

exit 0
POSTINSTALL_EOF
chmod +x "${PKG_SCRIPTS}/postinstall"

cat > "${PKG_SCRIPTS}/preremove" <<'PREREMOVE_EOF'
#!/bin/bash
set -euo pipefail

log() { echo "[preremove] $*"; }

# Unload daemon
launchctl bootout system/com.betblocker.agent 2>/dev/null \
    && log "Daemon unloaded" \
    || log "Daemon was not running"

# Flush pf anchor (non-fatal)
pfctl -a com.betblocker -F all 2>/dev/null \
    && log "pf rules removed" \
    || log "pf anchor not present (OK)"

exit 0
PREREMOVE_EOF
chmod +x "${PKG_SCRIPTS}/preremove"

# ---------------------------------------------------------------------------
# Step 4: Build component package
# ---------------------------------------------------------------------------
log "Running pkgbuild to create component package..."

PKGBUILD_ARGS=(
    --root        "${PKG_ROOT}"
    --scripts     "${PKG_SCRIPTS}"
    --identifier  "com.betblocker.agent"
    --version     "${VERSION}"
    --install-location "/"
)

if [[ -n "${SIGN_IDENTITY}" ]]; then
    PKGBUILD_ARGS+=(--sign "${SIGN_IDENTITY}")
fi

PKGBUILD_ARGS+=("${COMPONENT_PKG}")

pkgbuild "${PKGBUILD_ARGS[@]}" \
    && log "Component package built: ${COMPONENT_PKG}" \
    || die "pkgbuild failed"

# ---------------------------------------------------------------------------
# Step 5: Build distribution package
# ---------------------------------------------------------------------------
log "Running productbuild to create distribution package..."

PRODUCTBUILD_ARGS=(
    --distribution  "${SCRIPT_DIR}/distribution.xml"
    --package-path  "${BUILD_DIR}"
    --resources     "${SCRIPT_DIR}/resources"
    --version       "${VERSION}"
)

if [[ -n "${SIGN_IDENTITY}" ]]; then
    PRODUCTBUILD_ARGS+=(--sign "${SIGN_IDENTITY}")
fi

PRODUCTBUILD_ARGS+=("${DISTRIBUTION_PKG}")

# Create a minimal resources directory if not present (welcome/license RTF)
mkdir -p "${SCRIPT_DIR}/resources"

productbuild "${PRODUCTBUILD_ARGS[@]}" \
    && log "Distribution package built: ${DISTRIBUTION_PKG}" \
    || die "productbuild failed"

# ---------------------------------------------------------------------------
# Step 6: Notarization (optional)
# ---------------------------------------------------------------------------
if [[ "${DO_NOTARIZE}" == "true" ]]; then
    log "Submitting package for notarization..."

    if [[ -z "${KEYCHAIN_PROFILE}" && ( -z "${APPLE_ID}" || -z "${TEAM_ID}" ) ]]; then
        die "Notarization requires either --keychain-profile or both --apple-id and --team-id"
    fi

    NOTARY_ARGS=(
        notarytool submit
        "${DISTRIBUTION_PKG}"
        --wait
    )

    if [[ -n "${KEYCHAIN_PROFILE}" ]]; then
        NOTARY_ARGS+=(--keychain-profile "${KEYCHAIN_PROFILE}")
    else
        NOTARY_ARGS+=(--apple-id "${APPLE_ID}" --team-id "${TEAM_ID}")
    fi

    log "xcrun ${NOTARY_ARGS[*]}"
    xcrun "${NOTARY_ARGS[@]}" \
        && log "Notarization succeeded" \
        || die "Notarization failed"

    log "Stapling notarization ticket..."
    xcrun stapler staple "${DISTRIBUTION_PKG}" \
        && log "Staple succeeded" \
        || warn "Staple failed (ticket may not be available yet; retry later)"
else
    log "Notarization skipped (pass --notarize to enable)"
fi

# ---------------------------------------------------------------------------
# Done
# ---------------------------------------------------------------------------
log "---------------------------------------------"
log "Package ready: ${DISTRIBUTION_PKG}"
log "Size:          $(du -sh "${DISTRIBUTION_PKG}" | cut -f1)"
log "---------------------------------------------"
