# Fleet RBAC and agent mTLS status — 2026-08-01

Scope: TM-1212, project-operated fleet tooling only. These controls do not
authorize P2P participation, validator status, block acceptance, mining, or
consensus changes.

## Current safeguards implemented locally

| Area | Current safeguard | Evidence |
|---|---|---|
| Human authorization | The Rust service separates viewer read routes from operator mutation routes. Missing authentication returns 401, a missing/invalid role returns 403, and a viewer receives 403 on operator routes. | `admin-server/src/app.rs`; tests `protected_routes_require_proxy_authentication`, `authenticated_request_without_role_is_rejected`, and `viewer_can_read_but_cannot_mutate` |
| Separate human credentials | The gateway builds separate viewer and operator Basic-auth entries from distinct generated secrets. It maps the authenticated username to a role and overwrites the identity and role headers sent upstream. | `docker/gateway/gateway-entrypoint.sh`, `docker/gateway/nginx.conf.template`, `compose/base.yaml` |
| UI least privilege | The UI reads `/api/session`; viewer sessions do not receive mutation controls. Server-side authorization remains authoritative. | `admin-server/static/app.js` |
| Agent key custody | Each agent generates its private key and CSR locally. Enrollment sends only the CSR with the single-use invitation token. The fleet CA never receives the private key. | `admin-server/src/pki.rs`, test `agent_key_stays_with_requester_and_controller_signs_only_the_csr` |
| Report authentication | The dedicated `FLEET_MTLS_HOST` listener on port 10443 requires a CA-verified client certificate. The application also requires the gateway's verified-certificate headers and the existing per-node bearer credential. The separate `FLEET_HOST` remains the tunneled HTTP enrollment hostname. | `docker/gateway/nginx.conf.template`, `docker/entrypoint.sh`, `admin-server/src/app.rs` |
| PKI key isolation | The controller keeps the fleet CA private key below `control/pki/ca`. The gateway mounts only `control/pki/edge`, which contains the fleet CA certificate and fleet server certificate/key. Gateway startup refuses an exposed `fleet-ca.key.pem`. | `compose/project-edge.yaml`, `docker/gateway/gateway-entrypoint.sh`, `admin-server/src/pki.rs` |
| Certificate binding | The current certificate fingerprint is bound to the node record. A mismatched fingerprint is rejected even with the correct node bearer credential. | `admin-server/src/store.rs`, test `invitation_is_single_use_and_report_credentials_are_required` |
| Expiry and rotation | Client certificates expire after 90 days. Agents rotate automatically in the final seven days or explicitly with `alvenqis-vps-admin --rotate-agent-certificate`; successful rotation replaces the local key/certificate and invalidates the prior fingerprint. | `admin-server/src/pki.rs`, `admin-server/src/app.rs`, `admin-server/src/main.rs` |
| Revocation and bans | Operators can revoke a node certificate through an operator-only, idempotent route. Revocation, replacement, removal, and fleet bans stop fleet reporting; they do not alter public P2P participation. | `admin-server/src/store.rs`, test `certificate_rotation_invalidates_old_identity_and_revocation_blocks_reporting` |
| Backup coverage | Fleet CA material and node certificate metadata live under controller state, which the encrypted control-state backup and restore paths already include. | `scripts/backup-now.sh`, `scripts/restore-from-backup.sh` |

The SHA-1 value exposed by Nginx is used only as an identifier for an already
CA-verified certificate, not as a signature or trust decision. TLS chain and
validity verification remain the authentication decision.

## Local verification performed

From the repository root on 2026-08-01:

```text
cargo clippy --all-targets --manifest-path Blockchain-prototype/alvenqis-release/alvenqis-setup-external/admin-server/Cargo.toml -- -D warnings
Result: success

cargo test --manifest-path Blockchain-prototype/alvenqis-release/alvenqis-setup-external/admin-server/Cargo.toml
Result: 15 passed; 0 failed

bash Blockchain-prototype/alvenqis-release/alvenqis-setup-external/scripts/validate-stack.sh
Result: static YAML, JSON, Python, Bash, storage, security, fleet PKI mount,
DNS/Tunnel separation, and no-auto-update validation passed; RPC role/profile
selection validation passed

Focused execution of the actual docker/ops/app.py validate_payload AST (the
host Python environment does not have Flask installed)
Result: controller explicit bind accepted; duplicate fleet host, invalid IPv4,
invalid port, and non-loopback agent bind rejected

Blockchain-scripts/operator/prepare-alvenqis-vps-env.py with a temporary output
Result: generated distinct fleet/fleet-mTLS hosts, loopback bind, and port 10443
```

## Verification still required

- Docker was unavailable on this workstation, so `validate-stack.sh
  --require-docker`, Compose rendering, image builds, and an Nginx configuration
  test were not executed.
- The positive/negative TLS handshake probes have not been run against a live
  composed stack. The default host bind is intentionally
  `127.0.0.1:10443`. The generated `.env` and setup UI accept an explicit IPv4
  bind for a controller, but selecting a public interface and firewall policy
  remains an operator decision.
- `FLEET_MTLS_HOST` defaults to `fleet-mtls.<base-domain>` and Cloudflare
  automation manages it as a DNS-only direct `A` record. It is deliberately
  absent from the HTTP Tunnel ingress used by `FLEET_HOST`.
- No fleet mTLS deployment, firewall change, public-port change, or live TLS
  handshake was performed as part of this local transport-boundary change.

Until those integration checks are recorded, TM-1212 remains **In Progress**.
