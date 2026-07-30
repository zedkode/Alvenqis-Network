from __future__ import annotations

import json
import os
import re
import secrets
import threading
import time
from pathlib import Path
from typing import Any

import requests
from flask import Flask, Response, jsonify, make_response, redirect, render_template, request, url_for

app = Flask(__name__)
app.secret_key = os.environ.get("FLASK_SECRET_KEY", secrets.token_hex(32))

WORKSPACE = Path(os.environ.get("ALVENQIS_WORKSPACE", "/workspace")).resolve()
BROKER_URL = os.environ.get("BROKER_URL", "http://docker-broker:8090").rstrip("/")
BROKER_TOKEN_FILE = Path(os.environ.get("BROKER_TOKEN_FILE", "/run/secrets/broker_token"))
STATE_DIR = WORKSPACE / "state"
SECRETS_DIR = STATE_DIR / "secrets"
GENERATED_DIR = STATE_DIR / "config" / "generated"
LOG_DIR = STATE_DIR / "ops"
SETUP_TOKEN_FILE = Path(os.environ.get("SETUP_TOKEN_FILE", SECRETS_DIR / "setup_token"))
DOMAIN_RE = re.compile(r"^(?=.{1,253}$)(?:[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?\.)+[A-Za-z]{2,63}$")
NAME_RE = re.compile(r"^[A-Za-z0-9._-]{1,64}$")
MULTIADDR_RE = re.compile(r"^/(?:dns4|dns6|ip4|ip6)/[^/]+/tcp/[0-9]{1,5}$")

job_lock = threading.Lock()
job_state: dict[str, Any] = {
    "running": False,
    "kind": None,
    "started_at": None,
    "finished_at": None,
    "success": None,
    "output": "",
}

def ensure_layout() -> None:
    # Bind mounts use the fixed, non-root UIDs of their corresponding images.
    # Keep host state private instead of making every directory world-writable.
    directories = {
        GENERATED_DIR: (10001, 10001),
        LOG_DIR: (0, 0),
        STATE_DIR / "data": (10001, 10001),
        STATE_DIR / "data" / "chain": (10001, 10001),
        STATE_DIR / "data" / "mempool": (10001, 10001),
        STATE_DIR / "data" / "indexer": (10001, 10001),
        STATE_DIR / "data" / "node": (10001, 10001),
        STATE_DIR / "control": (10001, 10001),
        STATE_DIR / "pool": (10001, 10001),
        STATE_DIR / "prometheus": (65534, 65534),
        STATE_DIR / "grafana": (472, 472),
        STATE_DIR / "loki": (10001, 10001),
        STATE_DIR / "alloy": (473, 473),
        STATE_DIR / "alertmanager": (65534, 65534),
        STATE_DIR / "backups": (0, 0),
        STATE_DIR / "metrics": (0, 0),
    }
    SECRETS_DIR.mkdir(parents=True, exist_ok=True)
    os.chmod(SECRETS_DIR, 0o700)
    for directory, (uid, gid) in directories.items():
        directory.mkdir(parents=True, exist_ok=True)
        os.chown(directory, uid, gid)
        os.chmod(directory, 0o750)
    os.chmod(STATE_DIR / "metrics", 0o755)

def read_or_create_secret(path: Path, length: int = 32) -> str:
    if path.exists() and path.stat().st_size:
        return path.read_text(encoding="utf-8").strip()
    value = secrets.token_urlsafe(length)
    path.write_text(value + "\n", encoding="utf-8")
    os.chmod(path, 0o600)
    return value

def read_or_create_hex_secret(path: Path, byte_length: int = 32) -> str:
    if path.exists() and path.stat().st_size:
        value = path.read_text(encoding="utf-8").strip()
        if len(value) != byte_length * 2 or re.fullmatch(r"[0-9a-fA-F]+", value) is None:
            raise ValueError(f"invalid hexadecimal secret: {path}")
        return value
    value = secrets.token_hex(byte_length)
    path.write_text(value + "\n", encoding="utf-8")
    os.chmod(path, 0o600)
    return value

def setup_token() -> str:
    ensure_layout()
    return read_or_create_secret(SETUP_TOKEN_FILE, 24)

def authorized() -> bool:
    expected = setup_token()
    supplied = (
        request.args.get("token")
        or request.headers.get("X-Alvenqis-Setup-Token")
        or request.cookies.get("ALVENQIS_setup_token")
    )
    return bool(supplied and secrets.compare_digest(supplied, expected))

def require_auth():
    if not authorized():
        return jsonify({"error": "invalid or missing setup token"}), 401
    return None

def write_secret(name: str, value: str, keep_existing: bool = True) -> None:
    path = SECRETS_DIR / name
    if keep_existing and not value and path.exists():
        return
    path.write_text(value.strip() + ("\n" if value.strip() else ""), encoding="utf-8")
    os.chmod(path, 0o600)

def write_access_summary(
    data: dict[str, Any],
    hosts: dict[str, str],
    admin_password: str,
    grafana_password: str,
) -> None:
    control_dir = STATE_DIR / "control"
    control_dir.mkdir(parents=True, exist_ok=True)
    path = control_dir / "LOGIN.txt"
    content = "\n".join(
        [
            "Alvenqis Control Plane access",
            f"Generated UTC: {time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}",
            "",
            f"Master panel: https://{hosts['CONTROL_HOST']}",
            f"Master user: {data.get('admin_user', 'alvenqis-admin')}",
            f"Master password: {admin_password}",
            "",
            f"Grafana: https://{hosts['GRAFANA_HOST']}",
            f"Grafana user: {data.get('grafana_admin_user', 'admin')}",
            f"Grafana password: {grafana_password}",
            "",
            "Keep this file private. Permissions must remain 0600.",
        ]
    )
    path.write_text(content + "\n", encoding="utf-8")
    os.chmod(path, 0o600)

def env_quote(value: Any) -> str:
    text = str(value)
    if "\n" in text or "\r" in text:
        raise ValueError("environment values may not contain newlines")
    return json.dumps(text, ensure_ascii=False)

def bool_text(value: Any) -> str:
    return "true" if value in (True, "true", "1", 1, "yes", "on") else "false"

def broker_token() -> str:
    return BROKER_TOKEN_FILE.read_text(encoding="utf-8").strip()

def broker_call(action: str, **payload: Any) -> dict[str, Any]:
    response=requests.post(f"{BROKER_URL}/v1/action",headers={"X-Alvenqis-Broker-Token":broker_token()},json={"action":action,**payload},timeout=7300)
    body=response.json() if response.content else {}
    if not response.ok: raise RuntimeError(body.get("error",f"broker HTTP {response.status_code}"))
    return body

def validate_payload(data: dict[str, Any]) -> dict[str, Any]:
    base_domain = str(data.get("base_domain", "")).strip().lower()
    node_name = str(data.get("node_name", "")).strip()
    admin_email = str(data.get("admin_email", "")).strip()
    if not DOMAIN_RE.match(base_domain):
        raise ValueError("base_domain is invalid")
    if not NAME_RE.match(node_name):
        raise ValueError("node_name must use letters, digits, dot, underscore or hyphen")
    if "@" not in admin_email:
        raise ValueError("admin_email is invalid")
    for field in ("admin_password", "grafana_password"):
        if not str(data.get(field, "")).strip():
            raise ValueError(f"{field} is required")

    cloudflare_mode = str(data.get("cloudflare_mode", "disabled"))
    if cloudflare_mode not in {"disabled", "dns", "tunnel"}:
        raise ValueError("cloudflare_mode must be disabled, dns or tunnel")

    control_role = str(data.get("control_role", "controller"))
    if control_role not in {"standalone", "controller", "agent"}:
        raise ValueError("control_role must be standalone, controller or agent")
    if control_role == "agent" and not str(data.get("controller_url", "")).startswith("https://"):
        raise ValueError("agent role requires an HTTPS controller_url")

    ALVENQIS_version = str(data.get("ALVENQIS_version", "2.1.0-no-autoupdate")).strip()
    if not NAME_RE.match(ALVENQIS_version) or ALVENQIS_version.lower() == "latest":
        raise ValueError("ALVENQIS_version must be an explicit immutable-style tag; latest is forbidden")

    enable_pool = bool(data.get("enable_pool"))
    if enable_pool and not str(data.get("pool_address", "")).strip():
        raise ValueError("pool_address is required when the pool is enabled")
    if enable_pool and not str(data.get("cloudflare_api_token", "")).strip():
        raise ValueError("cloudflare_api_token is required for Stratum DNS-01 TLS")

    if cloudflare_mode != "disabled":
        for field in ("cloudflare_account_id", "cloudflare_zone_id", "cloudflare_api_token"):
            if not str(data.get(field, "")).strip():
                raise ValueError(f"{field} is required for Cloudflare automation")

    if bool(data.get("alert_email_enabled")):
        for field in ("alert_smtp_host", "alert_smtp_from", "alert_smtp_to"):
            if not str(data.get(field, "")).strip():
                raise ValueError(f"{field} is required when email alerts are enabled")

    if str(data.get("telegram_bot_token", "")).strip() and not str(data.get("telegram_chat_id", "")).strip():
        raise ValueError("telegram_chat_id is required when Telegram alerts are configured")

    raw_seeds = str(data.get("seed_nodes", "")).replace("\n", ",")
    seed_nodes = [item.strip() for item in raw_seeds.split(",") if item.strip()]
    for seed in seed_nodes:
        if not MULTIADDR_RE.match(seed):
            raise ValueError(f"invalid P2P seed multiaddress: {seed}")
        try:
            port = int(seed.rsplit("/", 1)[1])
        except ValueError as exc:
            raise ValueError(f"invalid P2P seed port: {seed}") from exc
        if port < 1 or port > 65535:
            raise ValueError(f"P2P seed port is out of range: {seed}")
    if len(seed_nodes) != len(set(seed_nodes)):
        raise ValueError("duplicate P2P seeds are not allowed")
    if control_role == "agent" and not seed_nodes:
        raise ValueError("agent role requires at least one P2P seed")

    return {
        **data,
        "base_domain": base_domain,
        "node_name": node_name,
        "admin_email": admin_email,
        "cloudflare_mode": cloudflare_mode,
        "control_role": control_role,
        "ALVENQIS_version": ALVENQIS_version,
        "enable_pool": enable_pool,
        "seed_nodes": seed_nodes,
    }

def write_alertmanager_config(data: dict[str, Any]) -> None:
    email_enabled = bool(data.get("alert_email_enabled"))
    smtp_host = str(data.get("alert_smtp_host", "")).strip()
    smtp_port = int(data.get("alert_smtp_port", 587) or 587)
    smtp_smarthost = f"{smtp_host}:{smtp_port}" if smtp_host else ""
    smtp_from = str(data.get("alert_smtp_from", "")).strip()
    smtp_to = str(data.get("alert_smtp_to", "")).strip()
    smtp_username = str(data.get("alert_smtp_username", "")).strip()
    smtp_password = str(data.get("alert_smtp_password", "")).strip()
    if smtp_password:
        write_secret("smtp_password", smtp_password, keep_existing=False)

    receivers = [
        {
            "name": "alvenqis-webhook",
            "webhook_configs": [{
                "url": "http://alvenqis-ops:8080/api/alerts/discord",
                "send_resolved": True,
            }],
        }
    ]
    receiver_name = "alvenqis-webhook"

    config_lines = [
        "global:",
        "  resolve_timeout: 5m",
    ]
    if email_enabled and smtp_host and smtp_from and smtp_to:
        config_lines += [
            f"  smtp_smarthost: {json.dumps(smtp_smarthost)}",
            f"  smtp_from: {json.dumps(smtp_from)}",
            f"  smtp_auth_username: {json.dumps(smtp_username)}",
            "  smtp_auth_password_file: /run/secrets/smtp_password",
            f"  smtp_require_tls: {str(bool(data.get('alert_smtp_starttls', True))).lower()}",
        ]
        receiver_name = "alvenqis-combined"
        receivers.append({
            "name": receiver_name,
            "webhook_configs": [{
                "url": "http://alvenqis-ops:8080/api/alerts/discord",
                "send_resolved": True,
            }],
            "email_configs": [{
                "to": smtp_to,
                "send_resolved": True,
            }],
        })

    config_lines += [
        "route:",
        f"  receiver: {receiver_name}",
        "  group_by: [alertname, service]",
        "  group_wait: 30s",
        "  group_interval: 5m",
        "  repeat_interval: 4h",
        "receivers:",
    ]
    for receiver in receivers:
        config_lines.append(f"  - name: {receiver['name']}")
        for webhook in receiver.get("webhook_configs", []):
            config_lines += [
                "    webhook_configs:",
                f"      - url: {json.dumps(webhook['url'])}",
                f"        send_resolved: {str(webhook['send_resolved']).lower()}",
            ]
        for email in receiver.get("email_configs", []):
            config_lines += [
                "    email_configs:",
                f"      - to: {json.dumps(email['to'])}",
                f"        send_resolved: {str(email['send_resolved']).lower()}",
            ]
    config_lines += [
        "inhibit_rules:",
        "  - source_matchers: [severity=\"critical\"]",
        "    target_matchers: [severity=\"warning\"]",
        "    equal: [alertname, service]",
    ]
    (GENERATED_DIR / "alertmanager.yml").write_text("\n".join(config_lines) + "\n", encoding="utf-8")

def configure(data: dict[str, Any]) -> None:
    ensure_layout()
    base = data["base_domain"]

    hosts = {
        "CONTROL_HOST": data.get("control_host") or f"control.{base}",
        "RPC_HOST": data.get("rpc_host") or f"rpc.{base}",
        "FLEET_HOST": data.get("fleet_host") or f"fleet.{base}",
        "GRAFANA_HOST": data.get("grafana_host") or f"grafana.{base}",
        "PROMETHEUS_HOST": data.get("prometheus_host") or f"prometheus.{base}",
        "EXPLORER_HOST": data.get("explorer_host") or f"explorer.{base}",
        "POOL_HOST": data.get("pool_host") or f"pool.{base}",
        "STRATUM_HOST": data.get("stratum_host") or f"stratum.{base}",
        "P2P_HOST": data.get("p2p_host") or f"node.{base}",
    }

    admin_password = str(data.get("admin_password", "")).strip()
    grafana_password = str(data.get("grafana_password", "")).strip()

    write_secret("admin_password", admin_password)
    write_secret("grafana_password", grafana_password)
    pool_admin_token = str(data.get("pool_admin_token", "")).strip()
    if pool_admin_token:
        write_secret("pool_admin_token", pool_admin_token)
    else:
        read_or_create_secret(SECRETS_DIR / "pool_admin_token", 48)
    read_or_create_secret(SECRETS_DIR / "broker_token", 48)
    backup_passphrase = str(data.get("backup_passphrase", "")).strip()
    if backup_passphrase:
        write_secret("backup_passphrase", backup_passphrase)
    else:
        read_or_create_secret(SECRETS_DIR / "backup_passphrase", 48)
    write_secret("cloudflare_api_token", str(data.get("cloudflare_api_token", "")), keep_existing=False)
    write_secret("r2_secret_access_key", str(data.get("r2_secret_access_key", "")), keep_existing=False)
    write_secret("discord_webhook", str(data.get("discord_webhook", "")), keep_existing=False)
    write_secret("telegram_bot_token", str(data.get("telegram_bot_token", "")), keep_existing=False)
    write_secret("smtp_password", str(data.get("alert_smtp_password", "")), keep_existing=False)
    read_or_create_secret(SECRETS_DIR / "cloudflare_tunnel_token", 48)
    read_or_create_hex_secret(SECRETS_DIR / "alvenqis_storage_key", 32)

    env_values = {
        "COMPOSE_PROJECT_NAME": "alvenqis-setup-external",
        "STACK_VERSION": "2.1.0-no-autoupdate",
        "ALVENQIS_OPERATOR_ROLE": os.environ.get("ALVENQIS_OPERATOR_ROLE", "node"),
        "ALVENQIS_DEPLOYMENT_ROLE": os.environ.get("ALVENQIS_OPERATOR_ROLE", "node"),
        "ALVENQIS_HOST_WORKSPACE": os.environ.get("ALVENQIS_HOST_WORKSPACE", str(WORKSPACE)),
        "ALVENQIS_HOST_REPO": os.environ.get("ALVENQIS_HOST_REPO", str(WORKSPACE)),
        "ALVENQIS_STATE_ROOT": os.environ.get("ALVENQIS_STATE_ROOT", str(STATE_DIR)),
        "ALVENQIS_STORAGE_KEY_FILE": "/run/secrets/alvenqis_storage_key",
        "ALVENQIS_REQUIRE_STORAGE_ENCRYPTION": "true",
        "ALVENQIS_ALLOW_PLAINTEXT_STORAGE_MIGRATION": "false",
        "TZ": data.get("timezone", "Europe/Bucharest"),
        "ALVENQIS_VERSION": data.get("ALVENQIS_version", "2.1.0-no-autoupdate"),
        "ALVENQIS_RUNTIME_IMAGE": data.get("alvenqis_runtime_image", "ghcr.io/zedkode/alvenqis-runtime"),
        "ALVENQIS_OPS_IMAGE": data.get("alvenqis_ops_image", "ghcr.io/zedkode/alvenqis-ops"),
        "ALVENQIS_BACKUP_IMAGE": data.get("alvenqis_backup_image", "ghcr.io/zedkode/alvenqis-backup-scheduler"),
        "ALVENQIS_GATEWAY_IMAGE": data.get("alvenqis_gateway_image", "ghcr.io/zedkode/alvenqis-gateway"),
        "ALVENQIS_WEBSITE_IMAGE": data.get("alvenqis_website_image", "ghcr.io/zedkode/alvenqis-website"),
        "ALVENQIS_EXPLORER_IMAGE": data.get("alvenqis_explorer_image", "ghcr.io/zedkode/alvenqis-explorer"),
        "BASE_DOMAIN": base,
        "PUBLIC_RPC_HOST": hosts["RPC_HOST"],
        "RPC_PUBLIC_EDGE": "true",
        "P2P_ADVERTISE_HOST": hosts["P2P_HOST"],
        "MINING_RPC_BIND": "docker-internal",
        "WEBSITE_HOST": data.get("website_host") or base,
        "WWW_HOST": data.get("www_host") or f"www.{base}",
        "NODE_NAME": data["node_name"],
        "ADMIN_EMAIL": data["admin_email"],
        "ADMIN_USER": data.get("admin_user", "alvenqis-admin"),
        "CONTROL_ROLE": data["control_role"],
        "CONTROLLER_URL": data.get("controller_url", ""),
        "ENROLLMENT_TOKEN": data.get("enrollment_token", ""),
        "RELEASE_BUNDLE_URL": data.get("release_bundle_url", ""),
        **hosts,
        "HTTP_PORT": data.get("http_port", 80),
        "P2P_PORT": data.get("p2p_port", 20787),
        "STRATUM_PORT": data.get("stratum_port", 3333),
        "STRATUM_INTERNAL_PORT": 3333,
        "SEED_NODES_TOML": ", ".join(json.dumps(seed) for seed in data.get("seed_nodes", [])),
        "P2P_MIN_VALIDATED_PEERS": data.get("p2p_min_validated_peers", 0),
        "OPS_BOOTSTRAP_PORT": data.get("ops_bootstrap_port", 8080),
        "CLOUDFLARE_MODE": data["cloudflare_mode"],
        "CLOUDFLARE_ACCOUNT_ID": data.get("cloudflare_account_id", ""),
        "CLOUDFLARE_ZONE_ID": data.get("cloudflare_zone_id", ""),
        "CLOUDFLARE_TUNNEL_NAME": data.get("cloudflare_tunnel_name", "alvenqis-setup-external"),
        "PUBLIC_IPV4": data.get("public_ipv4", ""),
        "CLOUDFLARE_PROXY_HTTP": bool_text(data.get("cloudflare_proxy_http", True)),
        "ENABLE_POOL": bool_text(data["enable_pool"]),
        "POOL_NAME": data.get("pool_name", "Alvenqis Reference Pool"),
        "POOL_ADDRESS": data.get("pool_address", ""),
        "INDEXER_INTERVAL_SECONDS": data.get("indexer_interval_seconds", 5),
        "INDEXER_FAILURE_BACKOFF_MAX_SECONDS": data.get("indexer_failure_backoff_max_seconds", 60),
        "PROMETHEUS_RETENTION": data.get("prometheus_retention", "15d"),
        "PROMETHEUS_RETENTION_SIZE": data.get("prometheus_retention_size", "8GB"),
        "LOKI_RETENTION_HOURS": data.get("loki_retention_hours", 168),
        "GRAFANA_ADMIN_USER": data.get("grafana_admin_user", "admin"),
        "ALERT_DISCORD_ENABLED": bool_text(bool(data.get("discord_webhook"))),
        "ALERT_TELEGRAM_ENABLED": bool_text(bool(data.get("telegram_bot_token"))),
        "TELEGRAM_CHAT_ID": data.get("telegram_chat_id", ""),
        "ALERT_EMAIL_ENABLED": bool_text(data.get("alert_email_enabled")),
        "ALERT_SMTP_HOST": data.get("alert_smtp_host", ""),
        "ALERT_SMTP_PORT": data.get("alert_smtp_port", 587),
        "ALERT_SMTP_FROM": data.get("alert_smtp_from", ""),
        "ALERT_SMTP_TO": data.get("alert_smtp_to", ""),
        "ALERT_SMTP_USERNAME": data.get("alert_smtp_username", ""),
        "ALERT_SMTP_STARTTLS": bool_text(data.get("alert_smtp_starttls", True)),
        "BACKUP_INTERVAL_SECONDS": data.get("backup_interval_seconds", 86400),
        "BACKUP_RETENTION_DAYS": data.get("backup_retention_days", 30),
        "CHAIN_SNAPSHOT_ENABLED": bool_text(data.get("chain_snapshot_enabled", True)),
        "BACKUP_REMOTE_ENABLED": bool_text(data.get("backup_remote_enabled", False)),
        "R2_ENDPOINT": data.get("r2_endpoint", ""),
        "R2_BUCKET": data.get("r2_bucket", ""),
        "R2_ACCESS_KEY_ID": data.get("r2_access_key_id", ""),
        "R2_REGION": data.get("r2_region", "auto"),
        "NODE_MEMORY_LIMIT": data.get("node_memory_limit", "2304M"),
        "RPC_MEMORY_LIMIT": data.get("rpc_memory_limit", "1536M"),
        "CONTROL_MEMORY_LIMIT": data.get("control_memory_limit", "384M"),
        "INDEXER_MEMORY_LIMIT": data.get("indexer_memory_limit", "768M"),
    }

    lines = ["# Generated by Alvenqis Docker Setup. Do not commit this file."]
    for key, value in env_values.items():
        lines.append(f"{key}={env_quote(value)}")
    (WORKSPACE / ".env").write_text("\n".join(lines) + "\n", encoding="utf-8")
    os.chmod(WORKSPACE / ".env", 0o600)
    write_access_summary(data, hosts, admin_password, grafana_password)

    write_alertmanager_config(data)

def deploy(data: dict[str, Any]) -> str:
    configure(data)
    return str(broker_call("deploy").get("output", ""))

def start_job(kind: str, fn, *args) -> bool:
    with job_lock:
        if job_state["running"]:
            return False
        job_state.update({
            "running": True,
            "kind": kind,
            "started_at": time.time(),
            "finished_at": None,
            "success": None,
            "output": "",
        })

    def runner():
        try:
            output = fn(*args)
            success = True
        except Exception as exc:  # noqa: BLE001
            output = f"{type(exc).__name__}: {exc}"
            success = False
        with job_lock:
            job_state.update({
                "running": False,
                "finished_at": time.time(),
                "success": success,
                "output": output[-100000:],
            })
            (LOG_DIR / f"{kind}-{int(time.time())}.log").write_text(output, encoding="utf-8")
    threading.Thread(target=runner, daemon=True).start()
    return True

@app.before_request
def initialize() -> None:
    ensure_layout()

@app.get("/health")
def health() -> Response:
    return jsonify({"ok": True, "workspace": str(WORKSPACE)})

@app.get("/")
def index() -> Response:
    token = request.args.get("token")
    if token and secrets.compare_digest(token, setup_token()):
        response = make_response(render_template("index.html"))
        secure_cookie = request.headers.get("X-Forwarded-Proto", "").lower() == "https"
        response.set_cookie(
            "ALVENQIS_setup_token",
            token,
            httponly=True,
            samesite="Strict",
            secure=secure_cookie,
        )
        return response
    if not authorized():
        return render_template("login.html"), 401
    return render_template("index.html")

@app.post("/api/deploy")
def api_deploy() -> Response:
    denied = require_auth()
    if denied:
        return denied
    try:
        data = validate_payload(request.get_json(force=True))
    except ValueError as exc:
        return jsonify({"error": str(exc)}), 400
    if not start_job("deploy", deploy, data):
        return jsonify({"error": "another operation is already running"}), 409
    return jsonify({"accepted": True})

@app.get("/api/job")
def api_job() -> Response:
    denied = require_auth()
    if denied:
        return denied
    with job_lock:
        return jsonify(dict(job_state))

@app.get("/api/stack")
def api_stack() -> Response:
    denied = require_auth()
    if denied:
        return denied
    if not (WORKSPACE / ".env").exists():
        return jsonify({"configured": False, "services": []})
    result=broker_call("status")
    return jsonify({"configured":True,"services":result.get("services",[]),"raw":result.get("raw","")})

@app.post("/api/backup")
def api_backup() -> Response:
    denied = require_auth()
    if denied:
        return denied
    if not start_job("backup", lambda: str(broker_call("backup").get("output", ""))):
        return jsonify({"error": "another operation is already running"}), 409
    return jsonify({"accepted": True})


@app.post("/api/alerts/discord")
def alert_fanout() -> Response:
    payload = request.get_json(silent=True) or {}
    alerts = payload.get("alerts", [])
    discord_lines = []
    plain_lines = []
    for alert in alerts[:20]:
        status = str(alert.get("status", "unknown")).upper()
        labels = alert.get("labels", {})
        annotations = alert.get("annotations", {})
        name = labels.get("alertname", "Alvenqis alert")
        service = labels.get("service", labels.get("job", "unknown"))
        summary = annotations.get("summary", annotations.get("description", ""))
        discord_lines.append(f"**[{status}] {name}** - `{service}`\n{summary}")
        plain_lines.append(f"[{status}] {name} - {service}\n{summary}")
    if not discord_lines:
        discord_lines = ["Alvenqis Alertmanager notification received."]
        plain_lines = ["Alvenqis Alertmanager notification received."]

    delivered = []
    errors = []
    webhook_file = Path(os.environ.get("DISCORD_WEBHOOK_FILE", SECRETS_DIR / "discord_webhook"))
    webhook = webhook_file.read_text(encoding="utf-8").strip() if webhook_file.exists() else ""
    if webhook:
        try:
            response = requests.post(webhook, json={"content": "\n\n".join(discord_lines)[:1900]}, timeout=10)
            response.raise_for_status()
            delivered.append("discord")
        except requests.RequestException as exc:
            errors.append(f"discord: {exc}")

    telegram_file = Path(os.environ.get("TELEGRAM_BOT_TOKEN_FILE", SECRETS_DIR / "telegram_bot_token"))
    telegram_token = telegram_file.read_text(encoding="utf-8").strip() if telegram_file.exists() else ""
    telegram_chat_id = os.environ.get("TELEGRAM_CHAT_ID", "").strip()
    if telegram_token and telegram_chat_id:
        try:
            response = requests.post(
                f"https://api.telegram.org/bot{telegram_token}/sendMessage",
                json={"chat_id": telegram_chat_id, "text": "\n\n".join(plain_lines)[:3900]},
                timeout=10,
            )
            response.raise_for_status()
            delivered.append("telegram")
        except requests.RequestException as exc:
            errors.append(f"telegram: {exc}")

    status = 200 if delivered else 202
    return jsonify({"accepted": bool(delivered), "delivered": delivered, "errors": errors}), status


if __name__ == "__main__":
    app.run(host="0.0.0.0", port=8080)
