#!/usr/bin/env bash
set -Eeuo pipefail

[[ "$(id -u)" -eq 0 ]] || {
  echo "Run as root: sudo ./scripts/bootstrap-host.sh" >&2
  exit 77
}

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export DEBIAN_FRONTEND=noninteractive

apt-get update
apt-get install -y ca-certificates curl gnupg jq openssl python3 tar

if ! command -v docker >/dev/null 2>&1 || ! docker compose version >/dev/null 2>&1; then
  install -m 0755 -d /etc/apt/keyrings
  curl -fsSL https://download.docker.com/linux/ubuntu/gpg \
    -o /etc/apt/keyrings/docker.asc
  chmod a+r /etc/apt/keyrings/docker.asc
  . /etc/os-release
  codename="${UBUNTU_CODENAME:-${VERSION_CODENAME:-}}"
  [[ -n "$codename" ]] || {
    echo "Cannot determine the Ubuntu/Debian codename." >&2
    exit 78
  }
  printf '%s\n' \
    "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu $codename stable" \
    > /etc/apt/sources.list.d/docker.list
  apt-get update
  apt-get install -y docker-ce docker-ce-cli containerd.io \
    docker-buildx-plugin docker-compose-plugin
fi

systemctl enable --now docker
"$root/scripts/install-docker-stack.sh"

login_file="/root/alvenqis-login.txt"
install -m 0600 /dev/null "$login_file"
cat > "$login_file" <<EOF
Alvenqis bootstrap access
Generated UTC: $(date -u +%Y-%m-%dT%H:%M:%SZ)

SSH tunnel:
ssh -N -L 18080:127.0.0.1:${OPS_BOOTSTRAP_PORT:-8080} root@SERVER_IP

Setup URL:
http://127.0.0.1:18080/?token=$(cat "$root/state/secrets/setup_token")

After deployment, final master and Grafana credentials are written to:
$root/state/control/LOGIN.txt
EOF
chmod 0600 "$login_file"

echo "Bootstrap complete. Private access file: $login_file"
