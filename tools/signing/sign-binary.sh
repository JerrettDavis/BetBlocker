#!/usr/bin/env bash
# tools/signing/sign-binary.sh
# Sign a binary with an Ed25519 private key.
#
# Usage:
#   sign-binary.sh <binary-path> <private-key-path> <signature-output-path>
#
# The signature is a raw Ed25519 signature over the SHA-256 hash of the binary,
# base64-encoded and written to the output path.

set -euo pipefail

if [ $# -ne 3 ]; then
    echo "Usage: $0 <binary-path> <private-key-path> <signature-output-path>"
    exit 1
fi

BINARY_PATH="$1"
KEY_PATH="$2"
SIG_PATH="$3"

if [ ! -f "${BINARY_PATH}" ]; then
    echo "Error: Binary not found: ${BINARY_PATH}"
    exit 1
fi

if [ ! -f "${KEY_PATH}" ]; then
    echo "Error: Private key not found: ${KEY_PATH}"
    exit 1
fi

# Sign the binary directly with Ed25519
# openssl pkeyutl -sign produces a raw Ed25519 signature (64 bytes)
openssl pkeyutl \
    -sign \
    -inkey "${KEY_PATH}" \
    -rawin \
    -in <(sha256sum "${BINARY_PATH}" | awk '{print $1}' | xxd -r -p) \
    -out "${SIG_PATH}.raw"

# Base64 encode the signature for portability
base64 < "${SIG_PATH}.raw" > "${SIG_PATH}"
rm -f "${SIG_PATH}.raw"

echo "Signed: ${BINARY_PATH}"
echo "Signature: ${SIG_PATH}"
echo "SHA-256: $(sha256sum "${BINARY_PATH}" | awk '{print $1}')"
