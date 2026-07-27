#!/usr/bin/env bash
# Shared repo path helpers. Source from Blockchain-scripts/<area>/*.sh:
#   source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/../lib/repo-paths.sh"
alvenqis_repo_root() {
  local dir
  dir="$(cd "$(dirname "${BASH_SOURCE[1]:-${BASH_SOURCE[0]}}")" && pwd)"
  while [[ -n "$dir" ]]; do
    if [[ -f "$dir/Blockchain-prototype/Cargo.toml" || -f "$dir/init.md" ]]; then
      echo "$dir"
      return 0
    fi
    local parent
    parent="$(dirname "$dir")"
    [[ "$parent" == "$dir" ]] && break
    dir="$parent"
  done
  echo "Could not locate Alvenqis repo root" >&2
  return 1
}
alvenqis_prototype_root() { echo "$(alvenqis_repo_root)/Blockchain-prototype"; }
alvenqis_scripts_root() { echo "$(alvenqis_repo_root)/Blockchain-scripts"; }
alvenqis_docs_root() { echo "$(alvenqis_repo_root)/Blockchain-docs"; }
