#!/usr/bin/env python3
"""Alvenqis metrics exporter.

Polls live JSON status endpoints (RPC, indexer, P2P, mempool, optional pool)
and exposes Prometheus text format on /metrics. No synthetic panel data —
values come from the running control-plane services.
"""
from __future__ import annotations

import json
import os
import time
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any

RPC_BASE = os.environ.get("ALVENQIS_RPC_URL", "http://alvenqis-rpc:10787").rstrip("/")
POOL_BASE = os.environ.get("ALVENQIS_POOL_URL", "http://alvenqis-pool:30787").rstrip("/")
CONTROL_HEALTH = os.environ.get(
    "ALVENQIS_CONTROL_URL", "http://alvenqis-control:10788/health"
)
OPS_HEALTH = os.environ.get("ALVENQIS_OPS_URL", "http://alvenqis-ops:8080/health")
ENABLE_POOL = os.environ.get("ENABLE_POOL", "false").strip().lower() in (
    "1",
    "true",
    "yes",
    "on",
)
LISTEN_HOST = os.environ.get("LISTEN_HOST", "0.0.0.0")
LISTEN_PORT = int(os.environ.get("LISTEN_PORT", "9101"))
TIMEOUT = float(os.environ.get("SCRAPE_TIMEOUT_SECONDS", "5"))
VERSION = "1.0.0"


def _fnum(value: Any, default: float = 0.0) -> float:
    if value is None:
        return default
    if isinstance(value, bool):
        return 1.0 if value else 0.0
    try:
        return float(value)
    except (TypeError, ValueError):
        return default


def _fetch_json(url: str) -> tuple[dict[str, Any] | None, float, str | None]:
    started = time.perf_counter()
    try:
        req = urllib.request.Request(
            url,
            headers={
                "Accept": "application/json",
                "User-Agent": f"alvenqis-metrics-exporter/{VERSION}",
            },
            method="GET",
        )
        with urllib.request.urlopen(req, timeout=TIMEOUT) as resp:
            body = resp.read().decode("utf-8", errors="replace")
            data = json.loads(body) if body.strip() else {}
            if not isinstance(data, dict):
                return None, time.perf_counter() - started, "non-object-json"
            return data, time.perf_counter() - started, None
    except urllib.error.HTTPError as exc:
        return None, time.perf_counter() - started, f"http-{exc.code}"
    except Exception as exc:  # noqa: BLE001 — exporter must never crash a scrape
        return None, time.perf_counter() - started, type(exc).__name__


def _line(name: str, value: float, labels: dict[str, str] | None = None) -> str:
    if labels:
        parts = ",".join(f'{k}="{v}"' for k, v in sorted(labels.items()))
        return f"{name}{{{parts}}} {value}"
    return f"{name} {value}"


def _help_type(name: str, help_text: str, metric_type: str = "gauge") -> list[str]:
    return [f"# HELP {name} {help_text}", f"# TYPE {name} {metric_type}"]


def collect() -> str:
    lines: list[str] = []
    scrape_ok: dict[str, float] = {}
    scrape_ms: dict[str, float] = {}

    def record(source: str, ok: bool, duration: float) -> None:
        scrape_ok[source] = 1.0 if ok else 0.0
        scrape_ms[source] = duration

    # --- RPC /status (chain tip) ---
    status, dur, err = _fetch_json(f"{RPC_BASE}/status")
    record("rpc_status", status is not None, dur)
    chain_ready = 0.0
    height = 0.0
    block_count = 0.0
    supply = 0.0
    index_height = 0.0
    index_in_sync = 0.0
    index_lag = 0.0
    if status is not None:
        chain_ready = 1.0 if status.get("initialized") else 0.0
        height = _fnum(status.get("height"))
        block_count = _fnum(status.get("block_count"))
        supply = _fnum(status.get("emitted_supply_atomic"))
        index_height = _fnum(status.get("index_height"))
        index_in_sync = 1.0 if status.get("index_in_sync") else 0.0
        index_lag = _fnum(status.get("index_lag_blocks"))

    lines += _help_type("alvenqis_rpc_up", "1 if RPC /status is reachable")
    lines.append(_line("alvenqis_rpc_up", scrape_ok.get("rpc_status", 0.0)))
    lines += _help_type("alvenqis_chain_ready", "1 when chain is initialized")
    lines.append(_line("alvenqis_chain_ready", chain_ready))
    lines += _help_type("alvenqis_chain_height", "Canonical chain tip height")
    lines.append(_line("alvenqis_chain_height", height))
    lines += _help_type("alvenqis_chain_block_count", "Number of blocks on the local chain")
    lines.append(_line("alvenqis_chain_block_count", block_count))
    lines += _help_type(
        "alvenqis_chain_emitted_supply_atomic",
        "Emitted supply in atomic units",
    )
    lines.append(_line("alvenqis_chain_emitted_supply_atomic", supply))
    lines += _help_type(
        "alvenqis_index_height_from_status",
        "Indexed height reported by /status",
    )
    lines.append(_line("alvenqis_index_height_from_status", index_height))
    lines += _help_type(
        "alvenqis_index_in_sync_from_status",
        "1 when index tip matches chain tip (/status)",
    )
    lines.append(_line("alvenqis_index_in_sync_from_status", index_in_sync))
    lines += _help_type(
        "alvenqis_index_lag_blocks_from_status",
        "Indexer lag in blocks from /status",
    )
    lines.append(_line("alvenqis_index_lag_blocks_from_status", index_lag))

    # --- indexer/status ---
    idx, dur, _ = _fetch_json(f"{RPC_BASE}/indexer/status")
    record("indexer_status", idx is not None, dur)
    idx_initialized = 0.0
    idx_height = 0.0
    idx_blocks = 0.0
    idx_txs = 0.0
    idx_addrs = 0.0
    idx_chain_height = 0.0
    idx_in_sync = 0.0
    idx_lag = 0.0
    if idx is not None:
        idx_initialized = 1.0 if idx.get("initialized") else 0.0
        idx_height = _fnum(idx.get("indexed_height"))
        idx_blocks = _fnum(idx.get("indexed_block_count"))
        idx_txs = _fnum(idx.get("transaction_count"))
        idx_addrs = _fnum(idx.get("address_count"))
        idx_chain_height = _fnum(idx.get("chain_height"))
        idx_in_sync = 1.0 if idx.get("in_sync") else 0.0
        idx_lag = _fnum(idx.get("lag_blocks"))

    lines += _help_type("alvenqis_indexer_up", "1 if /indexer/status is reachable")
    lines.append(_line("alvenqis_indexer_up", scrape_ok.get("indexer_status", 0.0)))
    lines += _help_type("alvenqis_indexer_initialized", "1 when the index is initialized")
    lines.append(_line("alvenqis_indexer_initialized", idx_initialized))
    lines += _help_type("alvenqis_indexer_height", "Highest indexed block height")
    lines.append(_line("alvenqis_indexer_height", idx_height))
    lines += _help_type(
        "alvenqis_indexer_block_count", "Number of indexed blocks"
    )
    lines.append(_line("alvenqis_indexer_block_count", idx_blocks))
    lines += _help_type(
        "alvenqis_indexer_transaction_count", "Indexed transaction count"
    )
    lines.append(_line("alvenqis_indexer_transaction_count", idx_txs))
    lines += _help_type("alvenqis_indexer_address_count", "Indexed address count")
    lines.append(_line("alvenqis_indexer_address_count", idx_addrs))
    lines += _help_type(
        "alvenqis_indexer_chain_height",
        "Chain height observed by indexer status",
    )
    lines.append(_line("alvenqis_indexer_chain_height", idx_chain_height))
    lines += _help_type(
        "alvenqis_indexer_in_sync", "1 when indexer tip matches chain tip"
    )
    lines.append(_line("alvenqis_indexer_in_sync", idx_in_sync))
    lines += _help_type(
        "alvenqis_indexer_lag_blocks", "Blocks the indexer is behind the chain tip"
    )
    lines.append(_line("alvenqis_indexer_lag_blocks", idx_lag))

    # Prefer dedicated indexer lag when available.
    lag_effective = idx_lag if scrape_ok.get("indexer_status") else index_lag
    in_sync_effective = (
        idx_in_sync if scrape_ok.get("indexer_status") else index_in_sync
    )
    lines += _help_type(
        "alvenqis_indexer_lag_blocks_effective",
        "Effective indexer lag (indexer/status preferred over /status)",
    )
    lines.append(_line("alvenqis_indexer_lag_blocks_effective", lag_effective))
    lines += _help_type(
        "alvenqis_indexer_in_sync_effective",
        "Effective indexer in-sync flag",
    )
    lines.append(_line("alvenqis_indexer_in_sync_effective", in_sync_effective))

    # --- sync/status ---
    sync, dur, _ = _fetch_json(f"{RPC_BASE}/sync/status")
    record("sync_status", sync is not None, dur)
    local_height = 0.0
    network_height = 0.0
    remaining = 0.0
    progress = 0.0
    sync_peers = 0.0
    sync_validated = 0.0
    if sync is not None:
        local_height = _fnum(sync.get("local_height"))
        network_height = _fnum(sync.get("network_height"))
        remaining = _fnum(sync.get("remaining_blocks"))
        progress = _fnum(sync.get("progress_percent"))
        sync_peers = _fnum(sync.get("connected_peer_count"))
        sync_validated = _fnum(sync.get("validated_peer_count"))

    lines += _help_type("alvenqis_sync_up", "1 if /sync/status is reachable")
    lines.append(_line("alvenqis_sync_up", scrape_ok.get("sync_status", 0.0)))
    lines += _help_type("alvenqis_sync_local_height", "Local height from sync status")
    lines.append(_line("alvenqis_sync_local_height", local_height))
    lines += _help_type(
        "alvenqis_sync_network_height", "Observed network height from sync status"
    )
    lines.append(_line("alvenqis_sync_network_height", network_height))
    lines += _help_type(
        "alvenqis_sync_remaining_blocks", "Blocks remaining to catch up"
    )
    lines.append(_line("alvenqis_sync_remaining_blocks", remaining))
    lines += _help_type(
        "alvenqis_sync_progress_percent", "Sync progress percent (0-100)"
    )
    lines.append(_line("alvenqis_sync_progress_percent", progress))
    lines += _help_type(
        "alvenqis_sync_connected_peers",
        "Connected peers reported by sync status",
    )
    lines.append(_line("alvenqis_sync_connected_peers", sync_peers))
    lines += _help_type(
        "alvenqis_sync_validated_peers",
        "Validated peers reported by sync status",
    )
    lines.append(_line("alvenqis_sync_validated_peers", sync_validated))

    # --- p2p/status ---
    p2p, dur, _ = _fetch_json(f"{RPC_BASE}/p2p/status")
    record("p2p_status", p2p is not None, dur)
    connected = 0.0
    validated = 0.0
    mining_peers = 0.0
    validating = 0.0
    banned = 0.0
    syncing = 0.0
    net_hashrate = 0.0
    seeds = 0.0
    if p2p is not None:
        connected = _fnum(p2p.get("connected_peer_count"))
        validated = _fnum(p2p.get("validated_peer_count"))
        mining_peers = _fnum(p2p.get("mining_peer_count"))
        validating = _fnum(p2p.get("validating_peer_count"))
        banned = _fnum(p2p.get("banned_peer_count"))
        syncing = 1.0 if p2p.get("syncing") else 0.0
        net_hashrate = _fnum(p2p.get("observed_network_hashrate_hs"))
        seeds = _fnum(p2p.get("configured_seed_count"))

    lines += _help_type("alvenqis_p2p_up", "1 if /p2p/status is reachable")
    lines.append(_line("alvenqis_p2p_up", scrape_ok.get("p2p_status", 0.0)))
    lines += _help_type("alvenqis_p2p_connected_peers", "Connected P2P peers")
    lines.append(_line("alvenqis_p2p_connected_peers", connected))
    lines += _help_type("alvenqis_p2p_validated_peers", "Validated P2P peers")
    lines.append(_line("alvenqis_p2p_validated_peers", validated))
    lines += _help_type("alvenqis_p2p_mining_peers", "Mining peers advertised on P2P")
    lines.append(_line("alvenqis_p2p_mining_peers", mining_peers))
    lines += _help_type(
        "alvenqis_p2p_validating_peers", "Peers currently validating"
    )
    lines.append(_line("alvenqis_p2p_validating_peers", validating))
    lines += _help_type("alvenqis_p2p_banned_peers", "Banned peer count")
    lines.append(_line("alvenqis_p2p_banned_peers", banned))
    lines += _help_type("alvenqis_p2p_syncing", "1 when the node is syncing")
    lines.append(_line("alvenqis_p2p_syncing", syncing))
    lines += _help_type(
        "alvenqis_p2p_observed_network_hashrate_hs",
        "Observed network hashrate (H/s) from peer advertisements",
    )
    lines.append(_line("alvenqis_p2p_observed_network_hashrate_hs", net_hashrate))
    lines += _help_type(
        "alvenqis_p2p_configured_seeds", "Configured seed peer count"
    )
    lines.append(_line("alvenqis_p2p_configured_seeds", seeds))

    # --- mempool/status ---
    mempool, dur, _ = _fetch_json(f"{RPC_BASE}/mempool/status")
    record("mempool_status", mempool is not None, dur)
    pending = 0.0
    base_fee = 0.0
    total_fees = 0.0
    if mempool is not None:
        pending = _fnum(mempool.get("pending_count"))
        base_fee = _fnum(mempool.get("anticipated_base_fee_atomic"))
        total_fees = _fnum(mempool.get("total_fees_atomic"))

    lines += _help_type("alvenqis_mempool_up", "1 if /mempool/status is reachable")
    lines.append(_line("alvenqis_mempool_up", scrape_ok.get("mempool_status", 0.0)))
    lines += _help_type("alvenqis_mempool_pending_count", "Pending mempool transactions")
    lines.append(_line("alvenqis_mempool_pending_count", pending))
    lines += _help_type(
        "alvenqis_mempool_anticipated_base_fee_atomic",
        "Anticipated base fee (atomic)",
    )
    lines.append(_line("alvenqis_mempool_anticipated_base_fee_atomic", base_fee))
    lines += _help_type(
        "alvenqis_mempool_total_fees_atomic", "Total fees in mempool (atomic)"
    )
    lines.append(_line("alvenqis_mempool_total_fees_atomic", total_fees))

    # --- control / ops health ---
    control, dur, _ = _fetch_json(CONTROL_HEALTH)
    record("control_health", control is not None and bool(control.get("ok", True)), dur)
    lines += _help_type("alvenqis_control_up", "1 if control /health is OK")
    lines.append(_line("alvenqis_control_up", scrape_ok.get("control_health", 0.0)))

    ops, dur, _ = _fetch_json(OPS_HEALTH)
    record("ops_health", ops is not None and bool(ops.get("ok", True)), dur)
    lines += _help_type("alvenqis_ops_up", "1 if ops /health is OK")
    lines.append(_line("alvenqis_ops_up", scrape_ok.get("ops_health", 0.0)))

    # --- optional pool ---
    lines += _help_type(
        "alvenqis_pool_enabled",
        "1 when ENABLE_POOL is true (pool metrics expected)",
    )
    lines.append(_line("alvenqis_pool_enabled", 1.0 if ENABLE_POOL else 0.0))

    pool_up = 0.0
    workers = 0.0
    hashrate = 0.0
    shares = 0.0
    blocks_found = 0.0
    matured = 0.0
    rejected = 0.0
    rate_limited = 0.0
    bans = 0.0
    online_workers = 0.0
    if ENABLE_POOL:
        pool, dur, _ = _fetch_json(f"{POOL_BASE}/api/v1/pool/status")
        record("pool_status", pool is not None, dur)
        if pool is not None:
            pool_up = 1.0
            workers = _fnum(pool.get("connected_workers"))
            hashrate = _fnum(pool.get("estimated_hashrate_hs"))
            shares = _fnum(pool.get("accepted_shares"))
            blocks_found = _fnum(pool.get("blocks_found"))
            matured = _fnum(pool.get("matured_blocks"))
            rejected = _fnum(pool.get("rejected_requests"))
            rate_limited = _fnum(pool.get("rate_limited_requests"))
            bans = _fnum(pool.get("active_bans"))
            workers_list = pool.get("workers") or []
            if isinstance(workers_list, list):
                online_workers = float(
                    sum(1 for w in workers_list if isinstance(w, dict) and w.get("online"))
                )
        else:
            pool_up = 0.0
    else:
        record("pool_status", True, 0.0)  # not expected

    lines += _help_type("alvenqis_pool_up", "1 if pool status is reachable (when enabled)")
    lines.append(_line("alvenqis_pool_up", pool_up if ENABLE_POOL else 0.0))
    lines += _help_type("alvenqis_pool_connected_workers", "Connected pool workers")
    lines.append(_line("alvenqis_pool_connected_workers", workers))
    lines += _help_type(
        "alvenqis_pool_online_workers", "Workers marked online in pool status"
    )
    lines.append(_line("alvenqis_pool_online_workers", online_workers))
    lines += _help_type(
        "alvenqis_pool_estimated_hashrate_hs", "Pool estimated hashrate (H/s)"
    )
    lines.append(_line("alvenqis_pool_estimated_hashrate_hs", hashrate))
    lines += _help_type("alvenqis_pool_accepted_shares", "Lifetime accepted shares")
    lines.append(_line("alvenqis_pool_accepted_shares", shares))
    lines += _help_type("alvenqis_pool_blocks_found", "Pool blocks found")
    lines.append(_line("alvenqis_pool_blocks_found", blocks_found))
    lines += _help_type("alvenqis_pool_matured_blocks", "Matured pool blocks")
    lines.append(_line("alvenqis_pool_matured_blocks", matured))
    lines += _help_type("alvenqis_pool_rejected_requests", "Rejected pool requests")
    lines.append(_line("alvenqis_pool_rejected_requests", rejected))
    lines += _help_type(
        "alvenqis_pool_rate_limited_requests", "Rate-limited pool requests"
    )
    lines.append(_line("alvenqis_pool_rate_limited_requests", rate_limited))
    lines += _help_type("alvenqis_pool_active_bans", "Active pool bans")
    lines.append(_line("alvenqis_pool_active_bans", bans))

    # --- scrape metadata ---
    lines += _help_type(
        "alvenqis_exporter_scrape_success",
        "1 if a source scrape succeeded",
        "gauge",
    )
    for source, ok in sorted(scrape_ok.items()):
        lines.append(_line("alvenqis_exporter_scrape_success", ok, {"source": source}))

    lines += _help_type(
        "alvenqis_exporter_scrape_duration_seconds",
        "Duration of each source scrape",
        "gauge",
    )
    for source, duration in sorted(scrape_ms.items()):
        lines.append(
            _line(
                "alvenqis_exporter_scrape_duration_seconds",
                duration,
                {"source": source},
            )
        )

    lines += _help_type(
        "alvenqis_exporter_build_info",
        "Exporter build info",
        "gauge",
    )
    lines.append(
        _line(
            "alvenqis_exporter_build_info",
            1.0,
            {"version": VERSION, "brand": "alvenqis"},
        )
    )

    return "\n".join(lines) + "\n"


class Handler(BaseHTTPRequestHandler):
    server_version = f"AlvenqisMetricsExporter/{VERSION}"

    def log_message(self, fmt: str, *args: Any) -> None:
        # Keep container logs quiet; Prometheus scrapes frequently.
        if self.path in ("/health", "/metrics"):
            return
        super().log_message(fmt, *args)

    def do_GET(self) -> None:  # noqa: N802
        if self.path in ("/health", "/"):
            body = json.dumps(
                {
                    "ok": True,
                    "service": "alvenqis-metrics-exporter",
                    "version": VERSION,
                    "brand": "alvenqis",
                    "pool_enabled": ENABLE_POOL,
                }
            ).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return

        if self.path.startswith("/metrics"):
            try:
                body = collect().encode("utf-8")
                self.send_response(200)
                self.send_header("Content-Type", "text/plain; version=0.0.4; charset=utf-8")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
            except Exception as exc:  # noqa: BLE001
                err = f"exporter_error {type(exc).__name__}\n".encode("utf-8")
                self.send_response(500)
                self.send_header("Content-Type", "text/plain; charset=utf-8")
                self.send_header("Content-Length", str(len(err)))
                self.end_headers()
                self.wfile.write(err)
            return

        self.send_response(404)
        self.end_headers()


def main() -> None:
    server = ThreadingHTTPServer((LISTEN_HOST, LISTEN_PORT), Handler)
    print(
        f"alvenqis-metrics-exporter {VERSION} listening on {LISTEN_HOST}:{LISTEN_PORT} "
        f"rpc={RPC_BASE} pool_enabled={ENABLE_POOL}",
        flush=True,
    )
    server.serve_forever()


if __name__ == "__main__":
    main()
