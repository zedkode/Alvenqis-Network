#!/usr/bin/env bash
set -Eeuo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
require_docker=false
[[ "${1:-}" == --require-docker ]] && require_docker=true
[[ -f .env ]] || cp .env.example .env
mkdir -p state/secrets state/config/generated
for secret in admin_password grafana_password setup_token broker_token cloudflare_api_token cloudflare_tunnel_token pool_admin_token backup_passphrase r2_secret_access_key discord_webhook telegram_bot_token smtp_password; do
  [[ -f "state/secrets/$secret" ]] || printf 'validation-placeholder\n' > "state/secrets/$secret"
done
[[ -f state/config/generated/alertmanager.yml ]] || cp monitoring/alertmanager/alertmanager.yml state/config/generated/alertmanager.yml
python3 - <<'PY2'
import json
from pathlib import Path
import yaml
root=Path.cwd()
yaml_paths=[
 root/'compose.yaml', root/'compose.direct.yaml', root/'installer.compose.yaml',
 root/'monitoring/prometheus/prometheus.yml', root/'monitoring/prometheus/alerts.yml',
 root/'monitoring/alertmanager/alertmanager.yml', root/'monitoring/blackbox/blackbox.yml',
 root/'monitoring/loki/loki.yml', root/'monitoring/grafana/provisioning/datasources/datasources.yml',
 root/'monitoring/grafana/provisioning/dashboards/dashboard-provider.yml',
]
for path in yaml_paths:
 yaml.safe_load(path.read_text())
for dash in (root/'monitoring/grafana/dashboards').glob('*.json'):
 json.loads(dash.read_text())
assert (root/'docker/metrics-exporter/exporter.py').is_file()
assert (root/'docker/metrics-exporter/Dockerfile').is_file()
prom_yml=(root/'monitoring/prometheus/prometheus.yml').read_text()
assert 'job_name: alvenqis-metrics' in prom_yml
assert 'alvenqis-metrics-exporter:9101' in prom_yml
assert 'project: alvenqis-network' in prom_yml
assert 'veiron' not in prom_yml.lower() and 'vireon' not in prom_yml.lower()
operational='\n'.join((root/p).read_text() for p in [
 'compose.yaml','installer.compose.yaml','docker/entrypoint.sh','docker/templates/rpc.toml.template',
 'docker/ops/app.py','docker/ops/broker.py','docker/ops/templates/index.html',
 'monitoring/prometheus/prometheus.yml','monitoring/prometheus/alerts.yml',
])
for legacy in ('/data/chain','/data/mempool','/data/indexer','/data/node'):
 assert legacy not in operational, f'legacy storage path remains: {legacy}'
for forbidden in ('init: true','DATABASE_URL','postgres-exporter','alvenqis-postgres','watchtower','update-stack.sh','/api/update','/api/rollback',"a=='update'","a=='rollback'","compose('pull'",'DEPLOYMENT_SOURCE'):
 assert forbidden not in operational, f'forbidden mechanism remains: {forbidden}'
assert 'value="latest"' not in operational
assert '${ALVENQIS_VERSION:-latest}' not in operational
assert '--no-autoupdate' in (root/'compose.yaml').read_text()
assert not (root/'scripts/update-stack.sh').exists()
for legacy_path in (
 'install.sh','install-interactive.sh','auto-install.sh','auto-update.sh','health-check.sh',
 'uninstall.sh','nginx','systemd',
):
 assert not (root/legacy_path).exists(), f'legacy host deployment path remains: {legacy_path}'
main=(root/'compose.yaml').read_text(); installer=(root/'installer.compose.yaml').read_text()
assert ',mode=' not in main
assert ',mode=' not in installer
assert main.count('/var/run/docker.sock:/var/run/docker.sock') == 1
assert installer.count('/var/run/docker.sock:/var/run/docker.sock') == 1
assert 'alvenqis-mining-rpc:' not in main
assert 'ALVENQIS_COMPONENT: mining-rpc' not in main
assert 'profiles: [pool]' in main
assert 'RPC_ACCESS_MODE: private-mining' in main
assert 'RPC_EXPOSE_MINING: "true"' in main
assert 'ENABLE_MINING_RPC' not in (root/'.env.example').read_text()
assert 'working_dir: /app' in main
assert 'state/data \\' in (root/'scripts/prepare-state.sh').read_text()
assert 'create_owned 473 473 state/alloy' in (root/'scripts/prepare-state.sh').read_text()
assert 'user: "473:0"' in main
assert 'chmod 0444' in (root/'scripts/prepare-state.sh').read_text()
assert 'http://alvenqis-rpc:10787' in (root/'docker/templates/pool.toml.template').read_text()
assert 'exec /usr/local/bin/alvenqis-indexer-loop' in (root/'docker/entrypoint.sh').read_text()
assert 'exec /usr/local/bin/alvenqis-pool-supervisor' in (root/'docker/entrypoint.sh').read_text()
assert (root/'docker/indexer-loop.sh').is_file()
assert (root/'docker/pool-supervisor.sh').is_file()
assert (root/'scripts/runtime-preflight.sh').is_file()
assert 'runtime-preflight.sh' in (root/'docker/ops/broker.py').read_text()
health=(root/'scripts/health-check-docker.sh').read_text()
assert 'Public solo mining template is unavailable or invalid.' in health
assert 'alvenqis-mining-rpc' not in health
assert 'P2P_MIN_VALIDATED_PEERS' in health
assert 'Private mining RPC did not return a valid live template.' in health
proxy=(root/'docker/gateway/nginx.conf.template').read_text()
assert 'location /pool/' in proxy
assert 'alvenqis-pool:30787' in proxy
assert 'location /mining/' in proxy
assert 'location = /mining/template' in proxy
assert 'location = /mining/submit' in proxy
assert 'limit_req zone=mining_template' in proxy
assert 'limit_req zone=mining_submit' in proxy
assert 'X-Alvenqis-Admin-Authenticated' in proxy
assert not (root/'docker/caddy/Caddyfile.template').exists()
assert not (root/'docker/caddy/caddy-entrypoint.sh').exists()
assert 'stratum-certbot:' in main
assert 'certbot/dns-cloudflare:v5.7.0' in main
assert '${STRATUM_PORT:-3333}:${STRATUM_INTERNAL_PORT:-3333}/tcp' in main
assert (root/'docker/stratum/certbot-loop.sh').is_file()
cloudflare_bootstrap=(root/'scripts/cloudflare-bootstrap.sh').read_text()
assert 'upsert_dns A "$STRATUM_HOST" "$public_ip" false' in cloudflare_bootstrap
assert 'hostname:$website, service:"http://gateway:8080"' in cloudflare_bootstrap
assert 'hostname:$explorer, service:"http://gateway:8080"' in cloudflare_bootstrap
assert 'onepanel-runtime' not in main
prepare_vps_env=(root.parents[2]/'Blockchain-scripts/operator/prepare-alvenqis-vps-env.py').read_text()
assert 'prototype_root = repo_root / "Blockchain-prototype"' in prepare_vps_env
assert '"ALVENQIS_HOST_REPO": str(prototype_root)' in prepare_vps_env
runtime_dockerfile=(root/'docker/Dockerfile').read_text()
assert '-p alvenqis-miner' not in runtime_dockerfile
assert '/out/alvenqis-miner' not in runtime_dockerfile
pool_app=(root/'../../alvenqis-mining-pool/src/app.rs').resolve().read_text()
assert '.route(\"/api/v1/work\"' not in pool_app
assert '.route(\"/api/v1/shares\"' not in pool_app
assert '/data/.alvenqis-mainnet/chain' in operational
assert '/data/.alvenqis-mainnet/mempool' in operational
assert 'alvenqis-metrics-exporter' in main
assert 'veiron' not in main.lower() and 'vireon' not in main.lower()
compose_data=yaml.safe_load(main)
for name,service in compose_data['services'].items():
 assert service.get('restart') == 'unless-stopped', f'{name} must restart unless-stopped'
 assert 'healthcheck' in service, f'{name} lacks healthcheck'
 assert service.get('mem_limit'), f'{name} lacks mem_limit'
 assert service.get('cpus'), f'{name} lacks cpus'
 assert service.get('pids_limit'), f'{name} lacks pids_limit'
 assert service.get('logging', {}).get('driver') == 'json-file', f'{name} lacks bounded json logging'
 assert not service.get('privileged', False) or name == 'cadvisor', f'{name} must not be privileged'
assert compose_data['services']['gateway']['depends_on']['grafana']['condition'] == 'service_started'
assert compose_data['services']['cadvisor']['privileged'] is True
assert '/var/run/docker.sock:/var/run/docker.sock' not in str(compose_data['services']['cadvisor'])
assert compose_data['services']['alvenqis-indexer']['depends_on'] == {'alvenqis-node': {'condition': 'service_healthy'}}
prometheus=(root/'monitoring/prometheus/prometheus.yml').read_text()
for expensive_probe in ('http://alvenqis-rpc:10787/status','http://alvenqis-rpc:10787/indexer/status','http://alvenqis-rpc:10787/p2p/status'):
 assert expensive_probe not in prometheus, f'duplicate heavy blackbox probe remains: {expensive_probe}'
assert 'collect_cached()' in (root/'docker/metrics-exporter/exporter.py').read_text()
assert 'uid": "alvenqis-docker-overview"' in (root/'monitoring/grafana/dashboards/alvenqis-overview.json').read_text()
assert 'uid": "alvenqis-host"' in (root/'monitoring/grafana/dashboards/alvenqis-host.json').read_text()
assert 'uid": "alvenqis-chain"' in (root/'monitoring/grafana/dashboards/alvenqis-chain.json').read_text()
assert 'uid": "alvenqis-network"' in (root/'monitoring/grafana/dashboards/alvenqis-network.json').read_text()
assert 'uid": "alvenqis-pool"' in (root/'monitoring/grafana/dashboards/alvenqis-pool.json').read_text()
assert 'uid": "alvenqis-ops"' in (root/'monitoring/grafana/dashboards/alvenqis-ops.json').read_text()
assert 'uid": "alvenqis-containers"' in (root/'monitoring/grafana/dashboards/alvenqis-containers.json').read_text()
assert 'alvenqis_chain_height' in (root/'monitoring/grafana/dashboards/alvenqis-overview.json').read_text()
assert 'alvenqis_indexer_lag_blocks_effective' in (root/'monitoring/grafana/dashboards/alvenqis-chain.json').read_text()
PY2
python3 -m py_compile docker/ops/app.py docker/ops/broker.py docker/metrics-exporter/exporter.py
find scripts docker -type f -name '*.sh' -print0 | xargs -0 -n1 bash -n
if grep -R -n --exclude=validate-stack.sh -E 'source[[:space:]]+\.env|docker[[:space:]]+rm|docker[[:space:]]+container[[:space:]]+rm|docker compose down -v' scripts docker compose.yaml installer.compose.yaml; then
  echo "Unsafe dotenv execution, container deletion, or volume deletion found." >&2
  exit 1
fi
echo "Static YAML, JSON, Python, Bash, storage, security and no-auto-update validation passed."
if command -v docker >/dev/null 2>&1 && docker compose version >/dev/null 2>&1; then
  docker compose --env-file .env -f compose.yaml config >/dev/null
  ALVENQIS_HOST_WORKSPACE="$root" ALVENQIS_HOST_REPO="$(cd ../.. && pwd)" docker compose -f installer.compose.yaml config >/dev/null
  echo "Docker Compose rendering passed."
elif [[ "$require_docker" == true ]]; then
  echo "Docker Compose v2 is required for full validation." >&2; exit 127
else
  echo "WARNING: Docker unavailable; Compose rendering and image builds were not executed." >&2
fi
