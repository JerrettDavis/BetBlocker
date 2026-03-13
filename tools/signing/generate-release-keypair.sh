#!/usr/bin/env bash
# tools/signing/generate-release-keypair.sh
# Generate an Ed25519 keypair for release binary signing.
# Run this ONCE, store the private key securely, and commit the public key.
#
# Usage:
#   generate-release-keypair.sh [output-dir]
#
# Output:
#   release-signing.key  (PRIVATE - store in GitHub Actions secrets, never commit)
#   release-signing.pub  (PUBLIC  - commit to repo at tools/signing/release-signing.pub)

set -euo pipefail

OUTPUT_DIR="${1:-.}"

echo "Generating Ed25519 release signing keypair..."

# Generate private key
openssl genpkey -algorithm Ed25519 -out "${OUTPUT_DIR}/release-signing.key"

# Extract public key
openssl pkey -in "${OUTPUT_DIR}/release-signing.key" -pubout -out "${OUTPUT_DIR}/release-signing.pub"

echo ""
echo "Generated:"
echo "  Private key: ${OUTPUT_DIR}/release-signing.key"
echo "  Public key:  ${OUTPUT_DIR}/release-signing.pub"
echo ""
echo "NEXT STEPS:"
echo "  1. Add the private key to GitHub Actions secrets as ED25519_RELEASE_SIGNING_KEY:"
echo "     cat ${OUTPUT_DIR}/release-signing.key | base64 | pbcopy"
echo "     (paste into GitHub Settings > Secrets > Actions > New repository secret)"
echo ""
echo "  2. Commit the public key to the repository:"
echo "     cp ${OUTPUT_DIR}/release-signing.pub tools/signing/release-signing.pub"
echo "     git add tools/signing/release-signing.pub"
echo ""
echo "  3. DELETE the private key from your local machine after uploading to GitHub."
echo "     Store a backup in a secure offline location (e.g., encrypted USB drive)."
