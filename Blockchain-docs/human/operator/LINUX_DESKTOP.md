# Linux Desktop - Alvenqis Control Center

Status: **2.0.1 Mainnet Candidate / Prototype** (not public Mainnet)

Linux and Windows use the same React/Tauri codebase. The product surface covers
wallet, optional NVIDIA CUDA FiroPoW mining, pool view, explorer, local node/RPC
sidecars, settings, recovery, and user-approved updates.

## Supported packages

| Distribution | Artifact |
|---|---|
| Ubuntu 22.04+/24.04 and Debian 12+ | `.deb` and `.AppImage` |
| Arch Linux and pacman derivatives (e.g. CachyOS, EndeavourOS) | `.AppImage` or `packaging/arch/PKGBUILD` |
| Fedora/RHEL-like | `.rpm` and `.AppImage` (not the focus of the current packaging QA pass) |

## Mining requirement (NVIDIA CUDA only)

The bundled miner is **NVIDIA CUDA-only**. Building a mining-capable release
requires the CUDA Toolkit and `nvcc`; running the miner requires a compatible
NVIDIA driver and enough VRAM for the FiroPoW DAG.

**AMD and Intel GPUs cannot mine with this build.** There is no OpenCL or CPU
mining fallback. Arch/CachyOS users with AMD cards can still install the Control
Center for **wallet and chain monitoring**; mining simply will not start.

## Wallet secrets (Secret Service)

Wallet private material is stored via **Secret Service** (libsecret → GNOME
Keyring or KWallet), not in plaintext files.

| Environment | Expected behavior |
|---|---|
| Desktop with `gnome-keyring` or `kwallet` running and unlocked | Create/unlock wallet works |
| Minimal server / container without a keyring daemon | **Clear error** naming Secret Service and suggesting `gnome-keyring` or `kwallet` — not a silent hang |

Install examples:

```bash
# Ubuntu / Debian (GNOME)
sudo apt install gnome-keyring libsecret-1-0

# Arch / CachyOS
sudo pacman -S gnome-keyring   # or kwallet on Plasma
```

## User data paths (Linux)

| Role | Path |
|---|---|
| Control Center settings / app data | `~/.local/share/Alvenqis/ControlCenter` (`$XDG_DATA_HOME` if set) |
| Wallet metadata + migration target | `~/.local/share/Alvenqis/Desktop` |
| Legacy migration sources (read, not deleted) | `…/Vireon/…` and `…/Veiron/…` under the same XDG root |

If migration sources incorrectly point at the Alvenqis path (rebrand bug class),
the app no-ops instead of self-copying (avoids “Access is denied” / metadata
failures on wallet open).

## Build host prerequisites

Install Rust, Node.js 20+, the NVIDIA CUDA Toolkit (only if you need a mining
binary), and platform Tauri dependencies. Example for Ubuntu/Debian:

```bash
sudo apt update
sudo apt install -y \
  build-essential curl wget file pkg-config patchelf fakeroot \
  libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev libssl-dev libsecret-1-dev libsoup-3.0-dev \
  libjavascriptcoregtk-4.1-dev libarchive-tools zenity
# Optional for mining-capable builds:
# nvcc --version   # from NVIDIA CUDA Toolkit
```

Arch / CachyOS build deps are listed in
`Blockchain-prototype/alvenqis-desktop-v2/packaging/arch/PKGBUILD`
(`makedepends` / `depends` / `optdepends`).

## Build

From the **monorepo root** (directory that contains `Blockchain-prototype/` and
`Blockchain-scripts/`):

```bash
bash Blockchain-scripts/release/build-linux-desktop.sh
```

Selected bundles:

```bash
bash Blockchain-scripts/release/build-linux-desktop.sh --bundles deb,appimage
```

The script requires a **Linux** host (`uname -s` == Linux). On Windows use WSL2
Ubuntu, a VM, or GitHub Actions `ubuntu-latest`.

Output:

```text
release-artifacts/linux-v2/
  *.deb
  *.AppImage
  *.rpm          # when requested and tooling is present
  INSTALL.txt
  SHA256SUMS
```

## Install

```bash
# Ubuntu / Debian
sudo apt install ./release-artifacts/linux-v2/*.deb

# AppImage (Ubuntu, Debian, Arch, CachyOS, …)
chmod +x release-artifacts/linux-v2/*.AppImage
./release-artifacts/linux-v2/*.AppImage

# Arch / CachyOS from monorepo PKGBUILD
cd Blockchain-prototype/alvenqis-desktop-v2/packaging/arch
makepkg -si
```

Verify `SHA256SUMS` before installation. Network maturity remains governed by
`Blockchain-docs/human/release/NETWORK_MATURITY.md`.

## QA notes (packaging pass)

- Preferred smoke path after install: launch → splash → **Choose your wallet**
  without metadata “Access is denied” / self-copy errors.
- Wallet create/import requires a working Secret Service (see above). Without a
  keyring daemon the keystore helper returns an explicit error naming
  Secret Service and suggesting `gnome-keyring` or `kwallet` (not a silent hang).
- Mining UI may appear; only NVIDIA CUDA can actually hash FiroPoW in this build.
- `makepkg -si` on Arch/CachyOS rebuilds via `build-linux-desktop.sh` (needs
  `base-devel`, Rust, Node, and CUDA toolkit if you want a mining-capable miner
  binary). For a quicker Arch install path, use the prebuilt `.AppImage`.
