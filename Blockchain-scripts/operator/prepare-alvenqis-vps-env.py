#!/usr/bin/env python3
"""Prepare Alvenqis VPS .env from legacy Vireon/Veiron env (no secret printing)."""
from __future__ import annotations

import argparse
from pathlib import Path

DEFAULT_OLD = Path("/home/vireon/network/vireon-release/vps-control-plane/.env")
DEFAULT_REPO = Path("/opt/alvenqis/repo")
DEFAULT_NEW = (
    DEFAULT_REPO
    / "Blockchain-prototype"
    / "alvenqis-release"
    / "vps-control-plane"
    / ".env"
)
# Genesis recipient for Mainnet Candidate (provisional pool treasury until operator rotates)
GENESIS_ALVE = "alve1qr4y5mrru2w9yz4774g8kyewchue23mk46ltu7ujgg0w56g5gmfzcnfqv0q"


def load_env(path: Path) -> dict[str, str]:
    vals: dict[str, str] = {}
    if not path.is_file():
        return vals
    for raw in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = raw.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        key = key.strip()
        value = value.strip().strip("'").strip('"')
        vals[key] = value
    return vals


def pick(vals: dict[str, str], *keys: str, default: str = "") -> str:
    for key in keys:
        if key in vals and vals[key] != "":
            return vals[key]
    return default


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--old", type=Path, default=DEFAULT_OLD)
    parser.add_argument("--new", type=Path, default=DEFAULT_NEW)
    parser.add_argument("--repo-root", type=Path, default=DEFAULT_REPO)
    parser.add_argument("--version", default="2.1.0-candidate.1")
    parser.add_argument("--stack-version", default="2.1.0-candidate.1")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    old = args.old.resolve()
    new = args.new.resolve()
    repo_root = args.repo_root.resolve()
    prototype_root = repo_root / "Blockchain-prototype"
    if not (prototype_root / "alvenqis-release" / "vps-control-plane" / "docker" / "Dockerfile").is_file():
        raise SystemExit(
            f"canonical Docker build context is missing below checkout: {prototype_root}"
        )
    workspace = new.parent
    vals = load_env(old)
    env = {
        "COMPOSE_PROJECT_NAME": "alvenqis-control-plane",
        "STACK_VERSION": args.stack_version,
        "TZ": pick(vals, "TZ", default="Europe/Bucharest"),
        "DEPLOYMENT_SOURCE": "build",
        "ALVENQIS_HOST_WORKSPACE": str(workspace),
        "ALVENQIS_HOST_REPO": str(prototype_root),
        "ALVENQIS_VERSION": args.version,
        "ALVENQIS_RUNTIME_IMAGE": "alvenqis-runtime",
        "ALVENQIS_OPS_IMAGE": "alvenqis-ops",
        "ALVENQIS_BACKUP_IMAGE": "alvenqis-backup-scheduler",
        "BASE_DOMAIN": pick(vals, "BASE_DOMAIN", default="dohotstudio.com"),
        "WEBSITE_HOST": pick(vals, "WEBSITE_HOST", default="dohotstudio.com"),
        "WWW_HOST": pick(vals, "WWW_HOST", default="www.dohotstudio.com"),
        "WEBSITE_ORIGIN": pick(vals, "WEBSITE_ORIGIN", default="http://alvenqis-website-runtime:18081"),
        "NODE_NAME": "bootstrap-eu-1",
        "ADMIN_EMAIL": pick(vals, "ADMIN_EMAIL", default="contact@dohotstudio.com"),
        "ADMIN_USER": "alvenqis-admin",
        "CONTROL_ROLE": "controller",
        "CONTROLLER_URL": pick(vals, "CONTROLLER_URL"),
        "ENROLLMENT_TOKEN": pick(vals, "ENROLLMENT_TOKEN"),
        "RELEASE_BUNDLE_URL": "",
        "CONTROL_HOST": pick(vals, "CONTROL_HOST", default="control.dohotstudio.com"),
        "RPC_HOST": pick(vals, "RPC_HOST", default="rpcnode.dohotstudio.com"),
        "FLEET_HOST": pick(vals, "FLEET_HOST", default="fleet.dohotstudio.com"),
        "GRAFANA_HOST": pick(vals, "GRAFANA_HOST", default="grafana.dohotstudio.com"),
        "PROMETHEUS_HOST": pick(vals, "PROMETHEUS_HOST", default="prometheus.dohotstudio.com"),
        "POOL_HOST": pick(vals, "POOL_HOST", default="pool.dohotstudio.com"),
        "STRATUM_HOST": pick(vals, "STRATUM_HOST", default="stratum.dohotstudio.com"),
        "P2P_HOST": pick(vals, "P2P_HOST", default="node.dohotstudio.com"),
        "HTTP_PORT": pick(vals, "HTTP_PORT", default="80"),
        "HTTPS_PORT": pick(vals, "HTTPS_PORT", default="443"),
        "P2P_PORT": pick(vals, "P2P_PORT", default="20787"),
        "STRATUM_PORT": pick(vals, "STRATUM_PORT", default="3333"),
        "STRATUM_INTERNAL_PORT": "3333",
        "SEED_NODES_TOML": pick(vals, "SEED_NODES_TOML"),
        "OPS_BOOTSTRAP_PORT": pick(vals, "OPS_BOOTSTRAP_PORT", default="8080"),
        "CLOUDFLARE_MODE": pick(vals, "CLOUDFLARE_MODE", default="tunnel"),
        "CLOUDFLARE_ACCOUNT_ID": pick(vals, "CLOUDFLARE_ACCOUNT_ID"),
        "CLOUDFLARE_ZONE_ID": pick(vals, "CLOUDFLARE_ZONE_ID"),
        "CLOUDFLARE_TUNNEL_NAME": "alvenqis-control-plane",
        "PUBLIC_IPV4": pick(vals, "PUBLIC_IPV4"),
        "CLOUDFLARE_PROXY_HTTP": pick(vals, "CLOUDFLARE_PROXY_HTTP", default="true"),
        "ENABLE_POOL": "true",
        "POOL_NAME": "Alvenqis Reference Pool",
        "POOL_ADDRESS": pick(vals, "POOL_ADDRESS", default=GENESIS_ALVE),
        "INDEXER_INTERVAL_SECONDS": pick(vals, "INDEXER_INTERVAL_SECONDS", default="15"),
        "PROMETHEUS_RETENTION": pick(vals, "PROMETHEUS_RETENTION", default="30d"),
        "LOKI_RETENTION_HOURS": pick(vals, "LOKI_RETENTION_HOURS", default="720"),
        "GRAFANA_ADMIN_USER": pick(vals, "GRAFANA_ADMIN_USER", default="admin"),
        "BACKUP_INTERVAL_SECONDS": pick(vals, "BACKUP_INTERVAL_SECONDS", default="86400"),
        "BACKUP_RETENTION_DAYS": pick(vals, "BACKUP_RETENTION_DAYS", default="30"),
        "CHAIN_SNAPSHOT_STOP_SERVICES": pick(vals, "CHAIN_SNAPSHOT_STOP_SERVICES", default="true"),
        "BACKUP_REMOTE_ENABLED": pick(vals, "BACKUP_REMOTE_ENABLED", default="false"),
        "R2_ENDPOINT": pick(vals, "R2_ENDPOINT"),
        "R2_BUCKET": pick(vals, "R2_BUCKET"),
        "R2_ACCESS_KEY_ID": pick(vals, "R2_ACCESS_KEY_ID"),
        "R2_REGION": pick(vals, "R2_REGION", default="auto"),
        "NODE_MEMORY_LIMIT": pick(vals, "NODE_MEMORY_LIMIT", default="3G"),
        "RPC_MEMORY_LIMIT": pick(vals, "RPC_MEMORY_LIMIT", default="3G"),
        "CONTROL_MEMORY_LIMIT": pick(vals, "CONTROL_MEMORY_LIMIT", default="1G"),
        "INDEXER_MEMORY_LIMIT": pick(vals, "INDEXER_MEMORY_LIMIT", default="1G"),
        "ALERT_DISCORD_ENABLED": pick(vals, "ALERT_DISCORD_ENABLED", default="false"),
        "ALERT_TELEGRAM_ENABLED": pick(vals, "ALERT_TELEGRAM_ENABLED", default="false"),
        "ALERT_EMAIL_ENABLED": pick(vals, "ALERT_EMAIL_ENABLED", default="false"),
    }
    new.parent.mkdir(parents=True, exist_ok=True)
    new.write_text("\n".join(f"{k}={v}" for k, v in env.items()) + "\n", encoding="utf-8")
    new.chmod(0o600)
    print(f"wrote {new} keys={len(env)}")


if __name__ == "__main__":
    main()
