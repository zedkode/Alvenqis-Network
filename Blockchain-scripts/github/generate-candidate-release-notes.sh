#!/usr/bin/env bash
# Generate candidate release notes only from committed history reachable by a tag.
set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
release_tag="${1:-}"
output_path="${2:--}"

usage() {
  cat <<'EOF'
Usage: generate-candidate-release-notes.sh TAG [OUTPUT_PATH|-]

TAG must already exist. Notes are generated from the commit referenced by TAG
and, when available, the preceding candidate tag reachable from that commit.
The working tree is never used as release-note input.
EOF
}

if [[ -z "$release_tag" || "$release_tag" == "-h" || "$release_tag" == "--help" ]]; then
  usage
  [[ -n "$release_tag" ]] && exit 0 || exit 2
fi

cd "$repo_root"

git check-ref-format "refs/tags/$release_tag" >/dev/null 2>&1 || {
  echo "Invalid release tag: $release_tag" >&2
  exit 2
}

tag_commit="$(git rev-parse --verify "refs/tags/$release_tag^{commit}")" || {
  echo "Release tag does not exist or does not reference a commit: $release_tag" >&2
  exit 2
}
short_commit="$(git rev-parse --short=12 "$tag_commit")"

previous_tag=""
while IFS= read -r candidate; do
  [[ -n "$candidate" && "$candidate" != "$release_tag" ]] || continue
  candidate_commit="$(git rev-parse --verify "refs/tags/$candidate^{commit}" 2>/dev/null || true)"
  [[ -n "$candidate_commit" && "$candidate_commit" != "$tag_commit" ]] || continue
  if git merge-base --is-ancestor "$candidate_commit" "$tag_commit"; then
    previous_tag="$candidate"
    break
  fi
done < <(
  git tag --merged "$tag_commit" \
    --list 'desktop-v*-candidate.*' 'desktop-v*-linux.*' 'v*-candidate.*' \
      'setup-external-v*-rc.*' \
    --sort=-creatordate
)

if [[ -n "$previous_tag" ]]; then
  change_range="refs/tags/$previous_tag..$tag_commit"
  comparison_text="Changes since \`$previous_tag\`"
elif git rev-parse --verify "$tag_commit^" >/dev/null 2>&1; then
  change_range="$tag_commit^..$tag_commit"
  comparison_text="Changes in the tagged commit"
else
  change_range="$tag_commit"
  comparison_text="Changes in the initial tagged commit"
fi

render_notes() {
  cat <<EOF
## Mainnet Candidate prerelease — not public Mainnet

This prerelease is built from tag \`$release_tag\` at commit
\`$short_commit\`. Desktop, server-component, container-image, and Setup
External outputs are tested and published independently. A failed component is
reported by its own job and does not invalidate artifacts that passed their own
checks.

Verify the component-specific SHA-256 file before testing downloaded assets.
Windows assets are published only after the required code-signing certificate
is imported and the signed package build succeeds.

### $comparison_text

EOF

  git log --reverse --format='- `%h` %s' "$change_range"

  cat <<'EOF'

### Tagged source summary

EOF

  git diff-tree --root --no-commit-id --shortstat -r "$change_range" \
    | sed -e 's/^[[:space:]]*/- /'
}

if [[ "$output_path" == "-" ]]; then
  render_notes
else
  mkdir -p "$(dirname "$output_path")"
  temporary_output="${output_path}.tmp.$$"
  trap 'rm -f "$temporary_output"' EXIT
  render_notes > "$temporary_output"
  mv "$temporary_output" "$output_path"
  trap - EXIT
fi
