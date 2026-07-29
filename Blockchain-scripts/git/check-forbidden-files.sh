#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

if (git ls-files; git ls-files --others --exclude-standard) | sort -u | grep -E '(^|/)\.env($|\.)' | grep -vE '(^|/)\.env\.example$' | grep -q .; then
  echo "Forbidden tracked or unignored .env files found." >&2
  exit 1
fi

if find . -type f \( -name "*.key" -o -name "*.pem" -o -name "*.seed" -o -name "*.wallet" \) | grep -q .; then
  echo "Forbidden key or wallet files found." >&2
  exit 1
fi

tracked="$(git ls-files)"
for pattern in ".alvenqis-dev/" ".alvenqis-testnet/" ".alvenqis-mainnet/" "target/" "node_modules/"; do
  if grep -Fq "$pattern" <<<"$tracked"; then
    echo "Tracked files contain forbidden pattern: $pattern" >&2
    exit 1
  fi
done

bash "$repo_root/Blockchain-scripts/security/check-secrets.sh"

echo "Forbidden file check passed."
