#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  echo "Usage: Blockchain-scripts/security/check-config-safety.sh"
  echo "Fails when repository config files expose unsafe RPC settings, devnet data paths in mainnet-candidate configs, reset flags, secrets, or local wallet material."
  exit 0
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"
# Support both flat layout and Blockchain-prototype/ layout.
proto=""
if [[ -d Blockchain-prototype ]]; then
  proto="Blockchain-prototype/"
fi

allowed_placeholder_regex='CHANGE_ME|example|localhost|127\.0\.0\.1'
secret_patterns=(
  "PRIVATE_KEY="
  "WALLET_SEED="
  "MNEMONIC="
  "API_TOKEN="
  "GITHUB_TOKEN="
  "SECRET="
  "PASSWORD="
  "RPC_PASSWORD="
  "ADMIN_TOKEN="
)

issues=()
config_files=()

while IFS= read -r file; do
  [[ -n "$file" ]] && config_files+=("$file")
done < <(find \
  ${proto}configs \
  ${proto}alvenqis-rpc-gateway/config \
  ${proto}alvenqis-devnet/config \
  -type f -name "*.toml" 2>/dev/null | sort -u)

for file in "${config_files[@]}"; do
  content="$(cat "$file")"

  if grep -Eq '^[[:space:]]*bind_host[[:space:]]*=[[:space:]]*"0\.0\.0\.0"' <<<"$content" && ! grep -Eq '^[[:space:]]*public_rpc_allowed[[:space:]]*=[[:space:]]*true' <<<"$content"; then
    issues+=("Unsafe RPC bind without public opt-in: $file")
  fi
  if [[ "$file" == *mainnet-candidate*.toml ]] && grep -Eq '\.alvenqis-dev' <<<"$content"; then
    issues+=("Mainnet-candidate config uses devnet data path: $file")
  fi
  if [[ "$file" == *mainnet-candidate*.toml ]] && grep -Eiq '^[[:space:]]*(allow_reset|reset)[[:space:]]*=[[:space:]]*true[[:space:]]*$' <<<"$content"; then
    issues+=("Mainnet-candidate config enables reset-like behavior: $file")
  fi

  for pattern in "${secret_patterns[@]}"; do
    while IFS= read -r line; do
      [[ -z "$line" ]] && continue
      if grep -Eiq "$allowed_placeholder_regex" <<<"$line"; then
        continue
      fi
      issues+=("Secret pattern '$pattern' found in config ${file}:${line%%:*}")
    done < <(grep -nF "$pattern" "$file" || true)
  done
done

while IFS= read -r file; do
  [[ -z "$file" ]] && continue
  if [[ "$file" == alvenqis-wallet/* ]]; then
    continue
  fi
  issues+=("Wallet material inside repository tree: $file")
done < <(find . -path ./alvenqis-wallet -prune -o -type f \( -path "*/wallets/*" -o -name "*.wallet" -o -name "*.seed" -o -name "*.key" -o -name "*.pem" \) -print)

# VPS compose: every service should declare hardening intent (user / read_only / cap review).
compose_file="${proto}alvenqis-release/vps-control-plane/compose.yaml"
if [[ -f "$compose_file" ]]; then
  # Extract service keys under top-level `services:` (simple YAML scan).
  mapfile -t compose_services < <(awk '
    /^services:/ { in_services=1; next }
    in_services && /^[^ #]/t{ if ($0 ~ /^  [a-zA-Z0-9_.-]+:/) { gsub(/:/,"",$1); print $1 } else if ($0 !~ /^ /) exit }
  ' "$compose_file")
  for svc in "${compose_services[@]+"${compose_services[@]}"}"; do
    [[ -z "$svc" ]] && continue
    # Pull the service block until next peer service or end.
    block="$(awk -v svc="$svc" '
      $0 ~ "^  "svc":" { grab=1; print; next }
      grab && /^  [a-zA-Z0-9_.-]+:/ { exit }
      grab { print }
    ' "$compose_file")"
    has_user=0
    has_ro=0
    has_cap=0
    has_review=0
    grep -Eq '^[[:space:]]+user:[[:space:]]' <<<"$block" && has_user=1
    grep -Eq '^[[:space:]]+read_only:[[:space:]]*true' <<<"$block" && has_ro=1
    grep -Eq '^[[:space:]]+cap_add:|^[[:space:]]+cap_drop:' <<<"$block" && has_cap=1
    grep -Eiq 'hardening-review|security-review|requires root|privileged intent' <<<"$block" && has_review=1
    if [[ $has_user -eq 0 && $has_ro -eq 0 && $has_cap -eq 0 && $has_review -eq 0 ]]; then
      issues+=("compose service '$svc' lacks user:/read_only:/cap_* or an explicit hardening-review comment ($compose_file)")
    fi
  done
fi

if (( ${#issues[@]} > 0 )); then
  printf 'Config safety check failed:\n' >&2
  printf -- '- %s\n' "${issues[@]}" >&2
  exit 1
fi

echo "Config safety check passed."
