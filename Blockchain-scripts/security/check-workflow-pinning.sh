#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  echo "Usage: Blockchain-scripts/security/check-workflow-pinning.sh"
  echo "Fails when a third-party GitHub Action is not pinned to a full commit SHA."
  exit 0
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

issues=()
while IFS=: read -r file line_number line; do
  reference="$(sed -E 's/^[[:space:]]*-?[[:space:]]*uses:[[:space:]]*([^#[:space:]]+).*/\1/' <<<"$line")"
  if [[ "$reference" != ./* && ! "$reference" =~ @[0-9a-fA-F]{40}$ ]]; then
    issues+=("${file}:${line_number}: unpinned action ${reference}")
  fi
done < <(grep -nHE '^[[:space:]]*-?[[:space:]]*uses:[[:space:]]*[^#[:space:]]+' .github/workflows/*.yml .github/workflows/*.yaml 2>/dev/null || true)

if (( ${#issues[@]} > 0 )); then
  printf 'GitHub Actions pinning check failed:\n' >&2
  printf -- '- %s\n' "${issues[@]}" >&2
  exit 1
fi

echo "GitHub Actions pinning check passed."
