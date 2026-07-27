#!/usr/bin/env bash
# Manual rotation for alvenqis-mining-pool admin_token_file.
# The pool fails closed when the token file mtime exceeds admin_token_max_age_seconds
# (default 90 days). Operators must rotate deliberately — there is no auto-renewal.
#
# Usage:
#   bash Blockchain-scripts/operator/rotate-pool-admin-token.sh /path/to/admin.token

set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <admin_token_file>" >&2
  exit 1
fi

TOKEN_FILE="$1"
mkdir -p "$(dirname "$TOKEN_FILE")"
# 32 random bytes as hex
if command -v openssl >/dev/null 2>&1; then
  TOKEN="$(openssl rand -hex 32)"
else
  TOKEN="$(head -c 32 /dev/urandom | xxd -p -c 32)"
fi
printf '%s' "$TOKEN" >"$TOKEN_FILE"
chmod 600 "$TOKEN_FILE" || true
echo "Wrote new admin token to $TOKEN_FILE"
echo "Restart alvenqis-mining-pool so require_admin loads the new token."
echo "Store the token only in your secrets manager; do not commit it."
