#!/usr/bin/env bash
set -euo pipefail

"$(dirname "$0")/check-forbidden-files.sh"
"$(dirname "$0")/../release/check-release.sh"

git add .
if [[ -z "$(git status --short)" ]]; then
  echo "No changes to commit."
  exit 0
fi

timestamp="$(date '+%Y-%m-%d %H:%M:%S')"
git commit -m "main: mainnet-candidate update ${timestamp}"
git push origin main
