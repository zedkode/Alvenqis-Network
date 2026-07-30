#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  echo "Usage: Blockchain-scripts/release/release-gate.sh"
  echo "Runs G1: local Mainnet Candidate software/hygiene release gate."
  echo "Passing does NOT approve public Mainnet launch."
  echo "See Blockchain-docs/human/release/NETWORK_MATURITY.md."
  exit 0
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
prototype_root="$repo_root/Blockchain-prototype"
docs_root="$repo_root/Blockchain-docs/human"
scripts_root="$repo_root/Blockchain-scripts"
cd "$repo_root"
temp_cargo_target_dir="$(mktemp -d "${TMPDIR:-/tmp}/alvenqis-release-gate-target.XXXXXX")"

cleanup() {
  rm -rf "$temp_cargo_target_dir"
  rm -rf \
    "$prototype_root/alvenqis-explorer/node_modules" \
    "$prototype_root/alvenqis-website/node_modules" \
    "$prototype_root/alvenqis-website/server/node_modules"
}

trap cleanup EXIT

assert_path_exists() {
  local path="$1"
  local description="$2"
  if [[ ! -e "$path" ]]; then
    echo "${description} is missing at ${path}" >&2
    exit 1
  fi
}

echo "Running Alvenqis G1 security and release gate (Mainnet Candidate rehearsal only)..."
echo "This is NOT a public Mainnet launch approval. See Blockchain-docs/human/release/NETWORK_MATURITY.md"

rm -rf \
  "$repo_root/target" \
  "$repo_root/target-msvc" \
  "$prototype_root/target" \
  "$prototype_root/alvenqis-explorer/node_modules" \
  "$prototype_root/alvenqis-website/node_modules" \
  "$prototype_root/alvenqis-website/server/node_modules"

bash "$scripts_root/security/check-secrets.sh"
bash "$scripts_root/security/check-repo-hygiene.sh"
bash "$scripts_root/security/check-config-safety.sh"
bash "$scripts_root/security/check-workflow-pinning.sh"
node "$scripts_root/docs/check-english-content.mjs"
node "$scripts_root/docs/audit-docs.mjs"
bash "$prototype_root/alvenqis-release/alvenqis-setup-external/scripts/validate-stack.sh"

assert_path_exists "$prototype_root/configs/mainnet-candidate.toml" "Mainnet-candidate config"
assert_path_exists "$docs_root/release/MAINNET_CANDIDATE_CHECKLIST.md" "Mainnet-candidate checklist"
assert_path_exists "$docs_root/release/RELEASE_GATE.md" "Release gate documentation"
assert_path_exists "$docs_root/release/NETWORK_MATURITY.md" "Network maturity documentation"
assert_path_exists "$docs_root/security/SECURITY_GATE.md" "Security gate documentation"
assert_path_exists "$docs_root/security/SECRET_HANDLING.md" "Secret handling documentation"
assert_path_exists "$docs_root/release/GENESIS.md" "Genesis documentation"
assert_path_exists "$docs_root/source-info/README.md" "Canonical source-information index"

(
  cd "$prototype_root"
  cargo fmt --all --check
  CARGO_TARGET_DIR="$temp_cargo_target_dir" cargo test --workspace
  CARGO_TARGET_DIR="$temp_cargo_target_dir" cargo clippy --workspace --all-targets -- -D warnings
  CARGO_TARGET_DIR="$temp_cargo_target_dir" cargo build --workspace --release
)

if [[ -f "$prototype_root/alvenqis-explorer/package.json" ]]; then
  pushd "$prototype_root/alvenqis-explorer" >/dev/null
  npm ci
  npm run build
  popd >/dev/null
fi

if [[ -f "$prototype_root/alvenqis-website/package.json" ]]; then
  pushd "$prototype_root/alvenqis-website" >/dev/null
  npm ci
  npm run build
  popd >/dev/null
fi

echo ""
echo "G1 release gate PASSED (Mainnet Candidate software/hygiene only)."
echo "NOT a public Mainnet approval. Next: G2 checklist + NETWORK_MATURITY.md G4 for launch."
