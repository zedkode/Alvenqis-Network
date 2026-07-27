#!/usr/bin/env bash
# Register alvenqis-browser-host for Chromium-based browsers on Linux.
# Usage:
#   ./scripts/browser/register-native-host.sh --extension-id <id>
#   ./scripts/browser/register-native-host.sh --extension-id <id> --build --browser chrome

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

EXTENSION_ID=""
BROWSER="chrome"
HOST_BINARY=""
BUILD=0
# Host defaults to require OS confirm (CR-C02). Only --no-require-os-confirm opts out.
REQUIRE_OS_CONFIRM=0
NO_OS_CONFIRM=0
LOCAL_RPC=0
INSTALL_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/alvenqis/browser-host"
HOST_NAME="com.alvenqis.browser_host"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --extension-id) EXTENSION_ID="${2:-}"; shift 2 ;;
    --browser) BROWSER="${2:-}"; shift 2 ;;
    --host-binary) HOST_BINARY="${2:-}"; shift 2 ;;
    --install-dir) INSTALL_DIR="${2:-}"; shift 2 ;;
    --build) BUILD=1; shift ;;
    --require-os-confirm) REQUIRE_OS_CONFIRM=1; shift ;; # no-op alias; host default is already on
    --no-require-os-confirm) NO_OS_CONFIRM=1; shift ;;
    --local) LOCAL_RPC=1; shift ;;
    -h|--help)
      sed -n '1,12p' "$0"
      exit 0
      ;;
    *) echo "Unknown arg: $1" >&2; exit 1 ;;
  esac
done

if [[ -z "$EXTENSION_ID" ]]; then
  echo "--extension-id is required" >&2
  exit 1
fi

if [[ $BUILD -eq 1 || -z "$HOST_BINARY" ]]; then
  cargo build --manifest-path (Join-Path $RepoRoot "Blockchain-prototype\Cargo.toml") -p alvenqis-browser-host --release
  HOST_BINARY="$REPO_ROOT/Blockchain-prototype/target/release/alvenqis-browser-host"
fi

HOST_BINARY="$(readlink -f "$HOST_BINARY")"
if [[ ! -x "$HOST_BINARY" && ! -f "$HOST_BINARY" ]]; then
  echo "Host binary not found: $HOST_BINARY" >&2
  exit 1
fi

mkdir -p "$INSTALL_DIR"
INSTALLED_BINARY="$INSTALL_DIR/alvenqis-browser-host"
cp -f "$HOST_BINARY" "$INSTALLED_BINARY"
chmod +x "$INSTALLED_BINARY"

LAUNCHER="$INSTALL_DIR/alvenqis-browser-host-launcher.sh"
{
  echo "#!/usr/bin/env bash"
  echo "set -euo pipefail"
  EXTRA=()
  if [[ $NO_OS_CONFIRM -eq 1 ]]; then
    echo "WARNING: registering with --no-require-os-confirm (dev/test only; unsafe for funds)" >&2
    EXTRA+=(--no-require-os-confirm)
  elif [[ $REQUIRE_OS_CONFIRM -eq 1 ]]; then
    EXTRA+=(--require-os-confirm) # explicit, already host default
  fi
  if [[ $LOCAL_RPC -eq 1 ]]; then EXTRA+=(--local); fi
  printf 'exec %q ' "$INSTALLED_BINARY"
  for a in "${EXTRA[@]+"${EXTRA[@]}"}"; do printf '%q ' "$a"; done
  echo '"$@"'
} > "$LAUNCHER"
chmod +x "$LAUNCHER"

MANIFEST="$INSTALL_DIR/$HOST_NAME.json"
ORIGIN="chrome-extension://${EXTENSION_ID}/"
cat > "$MANIFEST" <<EOF
{
  "name": "$HOST_NAME",
  "description": "Alvenqis Network browser native messaging host (Mainnet Candidate)",
  "path": "$LAUNCHER",
  "type": "stdio",
  "allowed_origins": [
    "$ORIGIN"
  ]
}
EOF

register_dir() {
  local dir="$1"
  mkdir -p "$dir"
  ln -sfn "$MANIFEST" "$dir/$HOST_NAME.json"
  echo "Registered in $dir"
}

case "$BROWSER" in
  chrome)
    register_dir "$HOME/.config/google-chrome/NativeMessagingHosts"
    register_dir "$HOME/.config/chromium/NativeMessagingHosts"
    ;;
  edge)
    register_dir "$HOME/.config/microsoft-edge/NativeMessagingHosts"
    ;;
  brave)
    register_dir "$HOME/.config/BraveSoftware/Brave-Browser/NativeMessagingHosts"
    ;;
  all)
    register_dir "$HOME/.config/google-chrome/NativeMessagingHosts"
    register_dir "$HOME/.config/chromium/NativeMessagingHosts"
    register_dir "$HOME/.config/microsoft-edge/NativeMessagingHosts"
    register_dir "$HOME/.config/BraveSoftware/Brave-Browser/NativeMessagingHosts"
    ;;
  *)
    echo "Unknown browser: $BROWSER" >&2
    exit 1
    ;;
esac

echo
echo "Install complete"
echo "  binary  : $INSTALLED_BINARY"
echo "  launcher: $LAUNCHER"
echo "  manifest: $MANIFEST"
echo "  origin  : $ORIGIN"
"$INSTALLED_BINARY" --print-info
