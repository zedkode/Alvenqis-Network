#!/usr/bin/env bash
set -euo pipefail

if [[ "$(id -u)" -ne 0 ]]; then
  echo "Run this script as root inside WSL." >&2
  exit 1
fi

export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y \
  ca-certificates curl gnupg build-essential pkg-config file patchelf fakeroot rpm \
  libarchive-tools libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev libssl-dev libsecret-1-dev \
  libsoup-3.0-dev libjavascriptcoregtk-4.1-dev zenity xdg-utils

install -d -m 0755 /usr/share/keyrings
curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key \
  | gpg --dearmor --yes -o /usr/share/keyrings/nodesource.gpg
chmod 0644 /usr/share/keyrings/nodesource.gpg
cat >/etc/apt/sources.list.d/nodesource.sources <<'EOF'
Types: deb
URIs: https://deb.nodesource.com/node_22.x
Suites: nodistro
Components: main
Architectures: amd64
Signed-By: /usr/share/keyrings/nodesource.gpg
EOF

curl -fsSLo /tmp/cuda-keyring.deb \
  https://developer.download.nvidia.com/compute/cuda/repos/wsl-ubuntu/x86_64/cuda-keyring_1.1-1_all.deb
dpkg -i /tmp/cuda-keyring.deb
rm -f /tmp/cuda-keyring.deb

apt-get update
# The WSL repository contains the toolkit without a Linux display driver.
apt-get install -y nodejs cuda-toolkit

cat >/etc/profile.d/alvenqis-cuda.sh <<'EOF'
export PATH=/usr/local/cuda/bin:$PATH
export LD_LIBRARY_PATH=/usr/local/cuda/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}
EOF
chmod 0644 /etc/profile.d/alvenqis-cuda.sh
export PATH="/usr/local/cuda/bin:$PATH"
export LD_LIBRARY_PATH="/usr/local/cuda/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

node --version
npm --version
nvcc --version
pkg-config --modversion webkit2gtk-4.1
