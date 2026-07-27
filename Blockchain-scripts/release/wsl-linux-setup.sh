#!/usr/bin/env bash
# One-shot WSL/Ubuntu setup for building Alvenqis Control Center Linux packages.
set -euo pipefail

export PATH="${HOME}/.cargo/bin:${PATH}"

if ! command -v rustc >/dev/null || ! command -v cargo >/dev/null; then
  echo "Rust not found. Install via: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" >&2
  exit 1
fi

if ! command -v node >/dev/null || ! command -v npm >/dev/null; then
  echo "==> Installing Node.js 22"
  curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
  sudo apt-get install -y nodejs
fi

echo "==> Installing Tauri / packaging packages"
sudo apt-get update -qq
sudo DEBIAN_FRONTEND=noninteractive apt-get install -y \
  build-essential curl wget file pkg-config \
  libwebkit2gtk-4.1-dev \
  libappindicator3-dev \
  librsvg2-dev \
  patchelf \
  libgtk-3-dev \
  libsoup-3.0-dev \
  libjavascriptcoregtk-4.1-dev \
  libsecret-1-dev \
  libdbus-1-dev \
  libssl-dev \
  zenity \
  fakeroot \
  rpm \
  libarchive-tools

echo "node: $(node -v)"
echo "npm:  $(npm -v)"
echo "rustc: $(rustc -V)"
pkg-config --modversion webkit2gtk-4.1
echo "WSL Linux desktop setup OK"
