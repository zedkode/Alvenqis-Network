#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"
expected_repository="zedkode/Alvenqis-Network"
workflow="vps-control-plane-release.yml"
sync_only=false
if [[ "${1:-}" == "--sync-only" ]]; then sync_only=true; shift; fi
message="${1:-release(vps): control-plane update $(date -u +'%Y-%m-%d %H:%M:%S UTC')}"

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  echo "Usage: ./scripts/github/sync-and-release-vps.sh [--sync-only] [commit-message]"
  exit 0
fi
command -v gh >/dev/null || { echo "GitHub CLI is required" >&2; exit 1; }
[[ "$(git branch --show-current)" == main ]] || { echo "VPS releases are allowed only from main" >&2; exit 1; }
git remote get-url origin | grep -Eqi 'github\.com[/:]zedkode/alvenqis-network(\.git)?$' || {
  echo "origin must point to $expected_repository" >&2
  exit 1
}
gh auth status --hostname github.com >/dev/null

bash Blockchain-scripts/security/check-secrets.sh
bash Blockchain-scripts/git/check-forbidden-files.sh
bash Blockchain-scripts/security/check-repo-hygiene.sh
bash Blockchain-scripts/security/check-config-safety.sh
git add --all
if ! git diff --cached --quiet; then git commit -m "$message"; fi
git fetch origin --prune
git pull --rebase origin main

bash Blockchain-scripts/security/check-secrets.sh
bash Blockchain-scripts/git/check-forbidden-files.sh
bash Blockchain-scripts/security/check-repo-hygiene.sh
bash Blockchain-scripts/security/check-config-safety.sh
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
[[ -z "$(git status --porcelain)" ]] || { echo "Checks changed the working tree" >&2; exit 1; }
git push origin main

if [[ "$sync_only" == true ]]; then
  echo "Main synchronized. VPS release was intentionally skipped."
  exit 0
fi

version="$(tr -d '[:space:]' < alvenqis-release/vps-control-plane/VERSION)"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "Invalid VERSION: $version" >&2; exit 1; }
prefix="vps-control-v${version}-rc."
existing_head_tag="$(git tag --points-at HEAD --list "${prefix}*" | head -n1)"
if [[ -n "$existing_head_tag" ]]; then
  echo "Current commit is already released as $existing_head_tag. No duplicate tag created."
  exit 0
fi
next="$(git ls-remote --tags origin "${prefix}*" | sed -n "s|.*refs/tags/${prefix}\([0-9][0-9]*\)$|\1|p" | sort -n | tail -n1)"
next="$(( ${next:-0} + 1 ))"
tag="${prefix}${next}"
git tag -a "$tag" -m "Alvenqis VPS Control Plane $tag"
if ! git push origin "$tag"; then git tag -d "$tag"; exit 1; fi

run_id=""
for _ in $(seq 1 30); do
  sleep 2
  run_id="$(gh run list --workflow "$workflow" --limit 20 --json databaseId,headBranch --jq ".[] | select(.headBranch == \"$tag\") | .databaseId" | head -n1)"
  [[ -n "$run_id" ]] && break
done
[[ -n "$run_id" ]] || { echo "Workflow run was not discovered" >&2; exit 1; }
gh run watch "$run_id" --exit-status
gh release view "$tag" --json url --jq .url
