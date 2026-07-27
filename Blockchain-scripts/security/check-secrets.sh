#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  echo "Usage: Blockchain-scripts/security/check-secrets.sh"
  echo "Fails when forbidden secret files or non-placeholder secret patterns are found in the repository."
  exit 0
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

allowed_placeholder_regex='CHANGE_ME|example|replace_|generated_|youshallnotpass|localhost|127\.0\.0\.1|^\$|^\$\(|^<.+>$'
secret_assignment_regex='^[[:space:]]*(export[[:space:]]+)?[A-Z0-9_]*(PRIVATE_KEY|WALLET_SEED|MNEMONIC|API_TOKEN|GITHUB_TOKEN|SECRET|PASSWORD|RPC_PASSWORD|ADMIN_TOKEN)[[:space:]]*=[[:space:]]*[^[:space:]#]*'
self_rule_files=(
  "Blockchain-scripts/git/check-forbidden-files.ps1"
  "Blockchain-scripts/git/check-forbidden-files.sh"
  "Blockchain-scripts/security/check-secrets.ps1"
  "Blockchain-scripts/security/check-secrets.sh"
  "Blockchain-scripts/security/check-config-safety.ps1"
  "Blockchain-scripts/security/check-config-safety.sh"
)

issues=()

while IFS= read -r file; do
  [[ -z "$file" ]] && continue
  issues+=("Forbidden environment file: $file")
done < <((git ls-files; git ls-files --others --exclude-standard) | sort -u | grep -E '(^|/)\.env($|\.)' | grep -vE '(^|/)\.env\.example$' || true)

mapfile -t candidate_files < <((git ls-files; git ls-files --others --exclude-standard) | sort -u)
for file in "${candidate_files[@]}"; do
  [[ -z "$file" || "$(basename "$file")" == ".env.example" || ! -f "$file" ]] && continue
  if [[ "$file" =~ \.(key|pem|seed|wallet|mnemonic)$ ]]; then
    issues+=("Forbidden tracked or unignored secret or wallet file: $file")
  fi
  skip_file=0
  for self_rule_file in "${self_rule_files[@]}"; do
    if [[ "$file" == "$self_rule_file" ]]; then
      skip_file=1
      break
    fi
  done
  (( skip_file == 1 )) && continue
  while IFS=: read -r line_number assignment; do
    [[ -z "$line_number" ]] && continue
    value="${assignment#*=}"
    value="${value%%#*}"
    value="$(sed -E "s/^[[:space:]'\"]+|[[:space:]'\"]+$//g" <<<"$value")"
    [[ -z "$value" ]] && continue
    if grep -Eiq "$allowed_placeholder_regex" <<<"$value"; then
      continue
    fi
    issues+=("Non-placeholder secret assignment found in ${file}:${line_number}")
  done < <(grep -nE "$secret_assignment_regex" "$file" || true)

  while IFS=: read -r line_number _; do
    [[ -z "$line_number" ]] && continue
    issues+=("Credential-like value found in ${file}:${line_number}")
  done < <(grep -nEi '\bgh[pousr]_[A-Za-z0-9]{20,}\b|\bAKIA[0-9A-Z]{16}\b|-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY-----' "$file" || true)
done

if (( ${#issues[@]} > 0 )); then
  printf 'Secret scan failed:\n' >&2
  printf -- '- %s\n' "${issues[@]}" >&2
  exit 1
fi

echo "Secret scan passed."
