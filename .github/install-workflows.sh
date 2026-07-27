#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "Usage: $0 /path/to/alvenqis-repository" >&2
  exit 2
fi

REPO_PATH="$(cd "$1" && pwd)"
SOURCE_DIR="$(cd "$(dirname "$0")/workflows" && pwd)"
TARGET_DIR="$REPO_PATH/.github/workflows"

if [ ! -d "$REPO_PATH/.git" ]; then
  echo "Not a Git repository: $REPO_PATH" >&2
  exit 1
fi

mkdir -p "$TARGET_DIR"
BACKUP_DIR="$REPO_PATH/.github/workflows-backup-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$BACKUP_DIR"
find "$TARGET_DIR" -maxdepth 1 -type f -exec cp {} "$BACKUP_DIR/" \;

rm -f "$TARGET_DIR/candidate-release.yml" "$TARGET_DIR/rust.yml"
cp "$SOURCE_DIR"/*.yml "$TARGET_DIR/"

RELEASE_TOOLS_DIR="$REPO_PATH/scripts/release"
mkdir -p "$RELEASE_TOOLS_DIR"
cp "$(dirname "$0")/alvenqis-release.ps1" "$RELEASE_TOOLS_DIR/"
cp "$(dirname "$0")/alvenqis-release.cmd" "$RELEASE_TOOLS_DIR/"

echo "Installed rebuilt workflows into: $TARGET_DIR"
echo "Installed interactive release manager into: $RELEASE_TOOLS_DIR"
echo "Backup of previous workflows: $BACKUP_DIR"
echo "Next: review 'git diff -- .github/workflows scripts/release' and commit the changes."
echo "On Windows, run: .\scripts\release\alvenqis-release.cmd"
