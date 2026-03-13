#!/usr/bin/env bash
# tools/signing/verify-binary.sh
# Verify an Ed25519 signature on a binary.
#
# Usage:
#   verify-binary.sh <binary-path> <signature-path> [public-key-path]
#
# If public-key-path is omitted, uses tools/signing/release-signing.pub

set -euo pipefail

if [ $# -lt 2 ] || [ $# -gt 3 ]; then
    echo "Usage: $0 <binary-path> <signature-path> [public-key-path]"
    exit 1
fi

BINARY_PATH="$1"
SIG_PATH="$2"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PUB_KEY_PATH="${3:-${SCRIPT_DIR}/release-signing.pub}"

if [ ! -f "${BINARY_PATH}" ]; then
    echo "Error: Binary not found: ${BINARY_PATH}"
    exit 1
fi

if [ ! -f "${SIG_PATH}" ]; then
    echo "Error: Signature not found: ${SIG_PATH}"
    exit 1
fi

if [ ! -f "${PUB_KEY_PATH}" ]; then
    echo "Error: Public key not found: ${PUB_KEY_PATH}"
    exit 1
fi

# Decode the base64 signature
base64 -d < "${SIG_PATH}" > /tmp/sig.raw

# Verify the signature
if openssl pkeyutl \
    -verify \
    -pubin \
    -inkey "${PUB_KEY_PATH}" \
    -rawin \
    -in <(sha256sum "${BINARY_PATH}" | awk '{print $1}' | xxd -r -p) \
    -sigfile /tmp/sig.raw; then
    echo "VERIFIED: ${BINARY_PATH} signature is valid."
    rm -f /tmp/sig.raw
    exit 0
else
    echo "FAILED: ${BINARY_PATH} signature verification failed!"
    rm -f /tmp/sig.raw
    exit 1
fi
