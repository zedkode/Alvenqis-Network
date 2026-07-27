#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  echo "Usage: Blockchain-scripts/security/check-repo-hygiene.sh"
  echo "Fails when tracked files match ignore rules, or when runtime/build artifacts can enter the repository."
  exit 0
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

issues=()

# 1) Structural: any tracked path that is now ignored must fail the gate.
mapfile -t cached_ignored < <(git ls-files -ci --exclude-standard || true)
if ((${#cached_ignored[@]} > 0)); then
  sample=("${cached_ignored[@]:0:40}")
  more=""
  if ((${#cached_ignored[@]} > 40)); then
    more=$'\n'"- ... and $((${#cached_ignored[@]} - 40)) more"
  fi
  issues+=("Tracked files that match .gitignore (git ls-files -ci --exclude-standard) — untrack with git rm --cached:")
  for f in "${sample[@]}"; do
    [[ -n "$f" ]] && issues+=("$f")
  done
  [[ -n "$more" ]] && issues+=("${more#- }")
fi

is_state_gitkeep() {
  case "$1" in
    Blockchain-prototype/alvenqis-release/vps-control-plane/state/config/generated/.gitkeep|\
    Blockchain-prototype/alvenqis-release/vps-control-plane/state/secrets/.gitkeep|\
    alvenqis-release/vps-control-plane/state/config/generated/.gitkeep|\
    alvenqis-release/vps-control-plane/state/secrets/.gitkeep)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

# 2) Content scan: tracked + unignored untracked paths
while IFS= read -r file; do
  [[ -z "$file" ]] && continue
  file="${file//\\//}"

  if [[ "$file" == .review/pipeline/runs/* || "$file" == .review/pipeline/worktrees/* ]] \
    && [[ "$file" != .review/pipeline/runs/.gitkeep ]]; then
    issues+=("Forbidden local pipeline artifact: $file")
    continue
  fi

  if is_state_gitkeep "$file"; then
    continue
  fi

  if [[ "$file" == Blockchain-prototype/alvenqis-release/vps-control-plane/state/* \
     || "$file" == alvenqis-release/vps-control-plane/state/* ]]; then
    issues+=("Forbidden control-plane runtime state: $file")
    continue
  fi

  if [[ "$file" == Blockchain-docs/internal/* \
     || "$file" == Blockchain-docs/ai/rebrand-pack/* \
     || "$file" == Blockchain-docs/human/source-info/* \
     || "$file" == Blockchain-docs/human/internal/* \
     || "$file" == .review/* \
     || "$file" == .agents/* \
     || "$file" == .codex/* \
     || "$file" == .cursor/* \
     || "$file" == .grok/* \
     || "$file" == .claude/* ]]; then
    issues+=("Forbidden private/local path tracked or unignored: $file")
    continue
  fi

  if grep -Eq \
    '(^|/)\.(alvenqis|vireon|veiron)-(dev|testnet|mainnet|local)(/|$)|(^|/)(target|target-msvc|target-msvc-[^/]+|target-miner-test|target-rebrand|target-rebrand-msvc|node_modules|logs|devnet-data|node-data|\.artifacts|coverage)(/|$)|(^|/)chain\.jsonl$|\.(log|pid|tmp|bak|orig|rej|db|sqlite|exe|dll|msi|AppImage|deb|rpm|apk|aab)$' \
    <<<"$file"; then
    issues+=("Forbidden tracked or unignored artifact: $file")
  fi
done < <((git ls-files; git ls-files --others --exclude-standard) | sort -u)

if ((${#issues[@]} > 0)); then
  printf 'Repository hygiene check failed:\n' >&2
  printf -- '- %s\n' "${issues[@]}" >&2
  exit 1
fi

echo "Repository hygiene check passed."
