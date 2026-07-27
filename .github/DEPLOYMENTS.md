# Alvenqis Deployments

This is the repository-level source of truth for public deployment targets. A
service is not considered current until its health check identifies the same
commit that was released from `main`.

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

Mining work and share submission are not exposed over HTTP. The former
`/mining/template` and `/mining/submit` public routes return `410 Gone`.

## Desktop release channels

| Application | Candidate version | GitHub tag |
|---|---:|---|
| Alvenqis Control Center V1 | `1.1.0-candidate.1` | `desktop-v1.1.0-candidate.1` |
| Alvenqis Control Center V2 | `2.1.0-candidate.1` | `desktop-v2.1.0-candidate.1` |

Windows releases contain NSIS, MSI and portable packages plus a `SHA256SUMS`
manifest. V1 and V2 use separate tags so their release assets cannot be mixed.
Linux packaging is currently deferred by operator decision.

## Workflows

- `candidate-windows-release.yml` validates the tag against the selected V1/V2
  package version, builds native sidecars and publishes Windows assets.
- `release-gate.yml` validates the Rust workspace and public product surfaces.
- `vps-control-plane-release.yml` builds the immutable VPS deployment bundle.
- `docker-control-plane-images.yml` publishes canonical Alvenqis container
  images; obsolete Veiron/Vireon image packages must not be referenced.

## Deployment order

1. Merge the verified rebrand commit into `main`.
2. Wait for required checks on that exact commit.
3. Build and publish the two Windows desktop releases.
4. Deploy the VPS control plane from that same `main` commit.
5. Create/update the website through
   `scripts/configure-1panel-website-runtime.sh`; the runtime builds with
   production RPC/pool variables.
6. Re-apply Cloudflare Tunnel ingress and DNS.
7. Verify website, RPC, pool statistics, Stratum TLS, P2P and release checksums.

Operational details:

- [Website/1Panel deployment](../Blockchain-prototype/alvenqis-website/DEPLOYMENT.md)
- [VPS control plane](../Blockchain-prototype/alvenqis-release/vps-control-plane/README.md)
- [Private mining and Stratum](../Blockchain-docs/human/operator/PRIVATE_MINING_OPS.md)
