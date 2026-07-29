# Alvenqis Deployments

This file records project-operated deployment targets and verification
requirements. It is not evidence that an endpoint is currently healthy. A
service is current only when a dated probe identifies the same immutable
release commit that was approved from `main`.

## Public endpoints

| Surface | Public endpoint | Transport | Source/runtime |
|---|---|---|---|
| Website | `https://dohotstudio.com` | Cloudflare Tunnel HTTPS | 1Panel Node.js 24, port `18081` |
| Website alias | `https://www.dohotstudio.com` | Cloudflare Tunnel HTTPS | Same website runtime |
| RPC | `https://rpcnode.dohotstudio.com` | Cloudflare Tunnel HTTPS | VPS control plane |
| Explorer | `https://dohotstudio.com/explorer` | Website + public RPC | Website runtime |
| Pool statistics | `https://pool.dohotstudio.com` | Cloudflare Tunnel HTTPS | Read-only pool HTTP API |
| Pool mining | `stratum+tls://stratum.dohotstudio.com:3333` | Direct TLS TCP, DNS-only | Pool Stratum listener |
| P2P node | `node.dohotstudio.com` | Direct TCP, DNS-only | Public node listener |

Accepted policy requires mining work and share submission to remain off the
public HTTP edge and `/mining/*` to return `410 Gone`. The current gateway, RPC
profile, and smoke configuration do not yet prove this contract consistently;
see `Blockchain-docs/human/security/KNOWN_LIMITATIONS.md`.

## Desktop release channels

| Application | Source version | Candidate tag pattern |
|---|---:|---|
| Alvenqis Control Center V2 | `2.0.1` | `desktop-v2.0.1-candidate.N` |

Windows releases contain a signed NSIS installer, a portable package, and a
`SHA256SUMS` manifest. Linux candidate releases build AppImage, Debian, and RPM
packages plus a platform checksum manifest. Existing asset names are immutable;
replacement requires a new candidate tag.

## Workflows

- `candidate-windows-release.yml` validates the tag against the Control Center
  V2 package version, builds native sidecars, requires signing secrets, and
  publishes Windows assets.
- `candidate-linux-release.yml` builds and publishes Control Center V2 Linux
  packages from the same desktop candidate tag.
- `candidate-vps-release.yml` validates and publishes the VPS control-plane
  bundle from core or desktop candidate tags.
- `candidate-quality.yml` runs repository checks for core and desktop candidate
  tags.
- `release-gate.yml` validates documentation, security, Rust, web, and VPS
  control-plane surfaces as separate required jobs.
- `vps-control-plane-release.yml` builds the immutable VPS deployment bundle.
- `docker-control-plane-images.yml` publishes canonical Alvenqis container
  images; obsolete Veiron/Vireon image packages must not be referenced.

## Deployment order

1. Merge the verified rebrand commit into `main`.
2. Wait for required checks on that exact commit.
3. Build and publish the Windows and Linux desktop candidate releases.
4. Deploy the VPS control plane from that same `main` commit.
5. Build and deploy the website according to
   `Blockchain-prototype/alvenqis-website/DEPLOYMENT.md`; do not replace or
   delete an existing 1Panel installation as part of blockchain automation.
6. Re-apply Cloudflare Tunnel ingress and DNS.
7. Verify website, RPC, pool statistics, Stratum TLS, P2P and release checksums.

Operational details:

- [Website/1Panel deployment](../Blockchain-prototype/alvenqis-website/DEPLOYMENT.md)
- [VPS control plane](../Blockchain-prototype/alvenqis-release/vps-control-plane/README.md)
- [Private mining and Stratum](../Blockchain-docs/human/operator/PRIVATE_MINING_OPS.md)
