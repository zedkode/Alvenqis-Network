#!/usr/bin/env bash
set -Eeuo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
require_docker=false
[[ "${1:-}" == --require-docker ]] && require_docker=true
[[ -f .env ]] || cp .env.example .env
source scripts/lib.sh
load_dotenv .env
export ALVENQIS_STATE_ROOT="${ALVENQIS_VALIDATION_STATE_ROOT:-$root/state}"
resolve_state_root "$root"
mkdir -p "$STATE_ROOT/secrets" "$STATE_ROOT/config/generated"
for secret in admin_password grafana_password setup_token broker_token cloudflare_api_token cloudflare_tunnel_token pool_admin_token backup_passphrase r2_secret_access_key discord_webhook telegram_bot_token smtp_password; do
  [[ -f "$STATE_ROOT/secrets/$secret" ]] || printf 'validation-placeholder\n' > "$STATE_ROOT/secrets/$secret"
done
[[ -f "$STATE_ROOT/secrets/alvenqis_storage_key" ]] || printf '%064d\n' 0 > "$STATE_ROOT/secrets/alvenqis_storage_key"
[[ -f "$STATE_ROOT/config/generated/alertmanager.yml" ]] || cp monitoring/alertmanager/alertmanager.yml "$STATE_ROOT/config/generated/alertmanager.yml"
python3 - <<'PY2'
import ast
import json
from pathlib import Path
import yaml
root=Path.cwd()
compose_paths=sorted((root/'compose').glob('*.yaml'))
yaml_paths=[
 *compose_paths,
 root/'monitoring/prometheus/prometheus.yml', root/'monitoring/prometheus/alerts.yml',
 root/'monitoring/alertmanager/alertmanager.yml', root/'monitoring/blackbox/blackbox.yml',
 root/'monitoring/loki/loki.yml', root/'monitoring/grafana/provisioning/datasources/datasources.yml',
 root/'monitoring/grafana/provisioning/dashboards/dashboard-provider.yml',
]
for path in yaml_paths:
 yaml.safe_load(path.read_text())
for path in (
 root/'docker/ops/app.py',
 root/'docker/ops/broker.py',
 root/'docker/metrics-exporter/exporter.py',
):
 ast.parse(path.read_text(), filename=str(path))
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
 *[str(path.relative_to(root)) for path in compose_paths],
 'docker/entrypoint.sh','docker/templates/rpc.toml.template',
 'docker/ops/app.py','docker/ops/broker.py','docker/ops/templates/index.html',
 'monitoring/prometheus/prometheus.yml','monitoring/prometheus/alerts.yml',
])
for legacy in ('/data/chain','/data/mempool','/data/indexer','/data/node'):
 assert legacy not in operational, f'legacy storage path remains: {legacy}'
for forbidden in ('init: true','DATABASE_URL','postgres-exporter','alvenqis-postgres','watchtower','update-stack.sh','/api/update','/api/rollback',"a=='update'","a=='rollback'","compose('pull'",'DEPLOYMENT_SOURCE'):
 assert forbidden not in operational, f'forbidden mechanism remains: {forbidden}'
assert 'value="latest"' not in operational
assert '${ALVENQIS_VERSION:-latest}' not in operational
assert '--no-autoupdate' in (root/'compose/cloudflare.yaml').read_text()
assert not (root/'scripts/update-stack.sh').exists()
for legacy_path in (
 'install.sh','install-interactive.sh','auto-install.sh','auto-update.sh','health-check.sh',
 'uninstall.sh','nginx','systemd',
):
 assert not (root/legacy_path).exists(), f'legacy host deployment path remains: {legacy_path}'
runtime_paths=[
 root/'compose/node.yaml',
 root/'compose/rpc.yaml',
 root/'compose/indexer-explorer.yaml',
 root/'compose/pool.yaml',
 root/'compose/project-edge.yaml',
 root/'compose/project-observability.yaml',
 root/'compose/cloudflare.yaml',
]
service_sources={}
compose_services={}
for path in runtime_paths:
 data=yaml.safe_load(path.read_text()) or {}
 for name, service in data.get('services', {}).items():
  assert name not in service_sources, f'duplicate service definition: {name} in {service_sources.get(name)} and {path}'
  service_sources[name]=path
  compose_services[name]=service
main='\n'.join(path.read_text() for path in [root/'compose/base.yaml', *runtime_paths])
installer=(root/'compose/installer.yaml').read_text()
assert not (root/'compose.yaml').exists()
assert not (root/'compose.direct.yaml').exists()
assert not (root/'installer.compose.yaml').exists()
roles=json.loads((root/'compose/roles.json').read_text())
assert set(roles['roles']) == {'node','validator','rpc','indexer','indexer-explorer','explorer','pool','stratum','full-stack'}
for role in roles['roles'].values():
 assert role['files'][0] == 'base.yaml'
 assert len(role['files']) == len(set(role['files']))
assert ',mode=' not in main
assert ',mode=' not in installer
assert main.count('/var/run/docker.sock:/var/run/docker.sock') == 1
assert installer.count('/var/run/docker.sock:/var/run/docker.sock') == 1
assert 'alvenqis-mining-rpc:' not in main
assert 'ALVENQIS_COMPONENT: mining-rpc' not in main
assert 'RPC_ACCESS_MODE: ${RPC_ACCESS_MODE:-private-mining}' in main
assert 'RPC_EXPOSE_MINING: ${RPC_EXPOSE_MINING:-true}' in main
assert 'ENABLE_MINING_RPC' not in (root/'.env.example').read_text()
assert 'working_dir: /app' in main
assert 'ALVENQIS_OPERATOR_ROLE=node' in (root/'.env.example').read_text()
assert 'ALVENQIS_STATE_ROOT=/var/lib/alvenqis' in (root/'.env.example').read_text()
assert 'ALVENQIS_STORAGE_KEY_FILE=/run/secrets/alvenqis_storage_key' in (root/'.env.example').read_text()
assert 'ALVENQIS_REQUIRE_STORAGE_ENCRYPTION=true' in (root/'.env.example').read_text()
assert 'ALVENQIS_ALLOW_PLAINTEXT_STORAGE_MIGRATION=false' in (root/'.env.example').read_text()
assert '${ALVENQIS_STATE_ROOT:-./state}/data:/data/.alvenqis-mainnet' in main
assert '${ALVENQIS_STATE_ROOT:-./state}/backups/rocksdb-repository:/backups/rocksdb-repository' in main
assert '${ALVENQIS_STATE_ROOT:-./state}/grafana:/var/lib/grafana' in main
assert '${ALVENQIS_STATE_ROOT:-./state}/secrets/admin_password' in main
assert '${ALVENQIS_STATE_ROOT:-./state}/secrets/alvenqis_storage_key' in main
assert 'secrets: [alvenqis_storage_key]' in main
assert 'resolve_state_root "$root"' in (root/'scripts/prepare-state.sh').read_text()
assert '"$STATE_ROOT/data"' in (root/'scripts/prepare-state.sh').read_text()
assert 'create_owned 473 473 "$STATE_ROOT/alloy"' in (root/'scripts/prepare-state.sh').read_text()
assert 'user: "473:0"' in main
assert 'chmod 0444' in (root/'scripts/prepare-state.sh').read_text()
assert 'chmod 0750 "$STATE_ROOT"' in (root/'scripts/prepare-state.sh').read_text()
assert 'create_owned 10001 10001 "$STATE_ROOT/backups/rocksdb-repository"' in (root/'scripts/prepare-state.sh').read_text()
assert (root/'scripts/migrate-release-state.sh').is_file()
assert "run(['bash',str(WORKSPACE/'scripts/backup-now.sh')])" in (root/'docker/ops/broker.py').read_text()
backup_script=(root/'scripts/backup-now.sh').read_text()
restore_script=(root/'scripts/restore-from-backup.sh').read_text()
assert 'BACKUP_COMPLETE' in backup_script
assert 'backup-rocksdb' in backup_script
assert "rocksdb_network_id=$rocks_network_id" in backup_script
assert "rocksdb_block_count=$rocks_block_count" in backup_script
assert "--exclude '/chain/state.rocksdb/'" in backup_script
assert '.backup-restore.lock' in backup_script
assert 'restore-latest-rocksdb' in restore_script
assert 'verify-rocksdb' in restore_script
assert 'Staged RocksDB restore and full SQLite replay verification: ok' in restore_script
assert 'rolling back the pre-restore snapshot' in restore_script
assert '.backup-restore.lock' in restore_script
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
assert 'verify-rocksdb' in health
assert 'RocksDB readiness failed' in health
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
assert '${STRATUM_BIND_ADDRESS:-0.0.0.0}:${STRATUM_PORT:-3333}:${STRATUM_INTERNAL_PORT:-3333}/tcp' in main
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
assert 'libclang-dev' in runtime_dockerfile
assert '--features alvenqis-node/storage-rocksdb' in runtime_dockerfile
entrypoint=(root/'docker/entrypoint.sh').read_text()
assert 'rebuild-rocksdb' in entrypoint
assert 'state.rocksdb/CURRENT' in entrypoint
assert 'ALVENQIS_REQUIRE_STORAGE_ENCRYPTION' in entrypoint
preflight=(root/'scripts/runtime-preflight.sh').read_text()
assert 'ALVENQIS_REQUIRE_STORAGE_ENCRYPTION=true is mandatory.' in preflight
assert 'assert_owner_mode "$STATE_ROOT" "0:0" "750"' in preflight
assert 'alvenqis_storage_key' in preflight
assert 'VPS_MIN_FREE_DISK_BYTES:-34359738368' in preflight
assert 'VPS_MIN_FREE_DISK_BYTES=17179869184' in (root/'.env.example').read_text()
assert not any(token in '\n'.join((root/path).read_text() for path in ('scripts/prepare-state.sh','scripts/runtime-preflight.sh','scripts/install-docker-stack.sh')) for token in ('chmod 777','chmod 0777'))
pool_app=(root/'../../alvenqis-mining-pool/src/app.rs').resolve().read_text()
assert '.route(\"/api/v1/work\"' not in pool_app
assert '.route(\"/api/v1/shares\"' not in pool_app
assert '/data/.alvenqis-mainnet/chain' in operational
assert '/data/.alvenqis-mainnet/mempool' in operational
assert 'alvenqis-metrics-exporter' in main
assert 'veiron' not in main.lower() and 'vireon' not in main.lower()
for name,service in compose_services.items():
 assert service.get('restart') == 'unless-stopped', f'{name} must restart unless-stopped'
 assert 'healthcheck' in service, f'{name} lacks healthcheck'
 assert service.get('mem_limit'), f'{name} lacks mem_limit'
 assert service.get('cpus'), f'{name} lacks cpus'
 assert service.get('pids_limit'), f'{name} lacks pids_limit'
 assert service.get('logging', {}).get('driver') == 'json-file', f'{name} lacks bounded json logging'
 assert not service.get('privileged', False) or name == 'cadvisor', f'{name} must not be privileged'
assert 'grafana' not in compose_services['gateway']['depends_on']
assert compose_services['cadvisor']['privileged'] is True
assert '/var/run/docker.sock:/var/run/docker.sock' not in str(compose_services['cadvisor'])
assert compose_services['alvenqis-indexer']['depends_on'] == {'alvenqis-node': {'condition': 'service_healthy'}}
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
find scripts docker -type f -name '*.sh' -print0 | xargs -0 -n1 bash -n
if grep -R -n --exclude=validate-stack.sh -E 'source[[:space:]]+\.env|docker[[:space:]]+rm|docker[[:space:]]+container[[:space:]]+rm|docker compose down -v' scripts docker compose; then
  echo "Unsafe dotenv execution, container deletion, or volume deletion found." >&2
  exit 1
fi
echo "Static YAML, JSON, Python, Bash, storage, security and no-auto-update validation passed."
if command -v docker >/dev/null 2>&1 && docker compose version >/dev/null 2>&1; then
  original_role="${ALVENQIS_OPERATOR_ROLE:-}"
  original_pool="${ENABLE_POOL:-}"
  for role in node rpc indexer-explorer pool full-stack; do
    export ALVENQIS_OPERATOR_ROLE="$role"
    [[ "$role" == pool ]] && export ENABLE_POOL=true || export ENABLE_POOL=false
    compose_args "$root/.env.example"
    "${ALVENQIS_COMPOSE_ARGS[@]}" "${ALVENQIS_PROFILE_ARGS[@]}" config --quiet
  done
  export ALVENQIS_OPERATOR_ROLE="$original_role" ENABLE_POOL="$original_pool"
  ALVENQIS_HOST_WORKSPACE="$root" ALVENQIS_HOST_REPO="$(cd ../.. && pwd)" \
    docker compose --project-directory "$root" --env-file .env.example -f compose/installer.yaml config --quiet
  echo "Docker Compose role rendering passed."
elif [[ "$require_docker" == true ]]; then
  echo "Docker Compose v2 is required for full validation." >&2; exit 127
else
  echo "WARNING: Docker unavailable; Compose rendering and image builds were not executed." >&2
fi
