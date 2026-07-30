#!/usr/bin/env python3
"""Generate detailed Alvenqis Grafana dashboards (file provisioning).

Run from repo:
  python3 monitoring/grafana/scripts/generate-dashboards.py

Writes JSON under monitoring/grafana/dashboards/.
"""
from __future__ import annotations

import json
from pathlib import Path

OUT = Path(__file__).resolve().parents[1] / "dashboards"
PROM = {"type": "prometheus", "uid": "alvenqis-prometheus"}
LOKI = {"type": "loki", "uid": "alvenqis-loki"}

_id = 1


def nid() -> int:
    global _id
    _id += 1
    return _id


def thr(*steps):
    # steps: (color, value) value None for base
    return {
        "mode": "absolute",
        "steps": [{"color": c, "value": v} for c, v in steps],
    }


def up_map():
    return [
        {
            "type": "value",
            "options": {
                "0": {"text": "DOWN", "color": "red"},
                "1": {"text": "UP", "color": "green"},
            },
        }
    ]


def sync_map():
    return [
        {
            "type": "value",
            "options": {
                "0": {"text": "LAG", "color": "orange"},
                "1": {"text": "SYNCED", "color": "green"},
            },
        }
    ]


def target(expr: str, legend: str = "", ref: str = "A") -> dict:
    t = {
        "datasource": PROM,
        "expr": expr,
        "refId": ref,
        "legendFormat": legend or "__auto",
        "editorMode": "code",
        "range": True,
        "instant": False,
    }
    return t


def target_instant(expr: str, legend: str = "", ref: str = "A") -> dict:
    t = target(expr, legend, ref)
    t["range"] = False
    t["instant"] = True
    return t


def row(title: str, y: int, collapsed: bool = False) -> dict:
    return {
        "type": "row",
        "title": title,
        "id": nid(),
        "gridPos": {"h": 1, "w": 24, "x": 0, "y": y},
        "collapsed": collapsed,
        "panels": [],
    }


def stat(
    title: str,
    expr: str,
    x: int,
    y: int,
    w: int = 4,
    h: int = 4,
    unit: str = "none",
    mappings=None,
    thresholds=None,
    decimals: int | None = None,
    color_mode: str = "background",
    graph_mode: str = "area",
) -> dict:
    defaults = {
        "unit": unit,
        "thresholds": thresholds
        or thr(("red", None), ("green", 1) if mappings else ("green", None)),
        "mappings": mappings or [],
    }
    if decimals is not None:
        defaults["decimals"] = decimals
    return {
        "type": "stat",
        "title": title,
        "id": nid(),
        "datasource": PROM,
        "targets": [target_instant(expr)],
        "gridPos": {"h": h, "w": w, "x": x, "y": y},
        "fieldConfig": {"defaults": defaults, "overrides": []},
        "options": {
            "reduceOptions": {"calcs": ["lastNotNull"], "fields": "", "values": False},
            "colorMode": color_mode,
            "graphMode": graph_mode,
            "textMode": "auto",
            "orientation": "auto",
        },
    }


def gauge(
    title: str,
    expr: str,
    x: int,
    y: int,
    w: int = 4,
    h: int = 5,
    unit: str = "percent",
    min_v: float = 0,
    max_v: float = 100,
    thresholds=None,
) -> dict:
    return {
        "type": "gauge",
        "title": title,
        "id": nid(),
        "datasource": PROM,
        "targets": [target_instant(expr)],
        "gridPos": {"h": h, "w": w, "x": x, "y": y},
        "fieldConfig": {
            "defaults": {
                "unit": unit,
                "min": min_v,
                "max": max_v,
                "thresholds": thresholds
                or thr(("green", None), ("orange", 70), ("red", 90)),
            },
            "overrides": [],
        },
        "options": {
            "reduceOptions": {"calcs": ["lastNotNull"], "fields": "", "values": False},
            "showThresholdLabels": False,
            "showThresholdMarkers": True,
        },
    }


def timeseries(
    title: str,
    targets: list[tuple[str, str]],
    x: int,
    y: int,
    w: int = 12,
    h: int = 8,
    unit: str = "none",
    fill: float = 0.15,
    stack: bool = False,
    legend_right: bool = False,
    thresholds=None,
) -> dict:
    ts = []
    for i, (expr, legend) in enumerate(targets):
        t = target(expr, legend, ref=chr(65 + i))
        ts.append(t)
    draw = "lines"
    return {
        "type": "timeseries",
        "title": title,
        "id": nid(),
        "datasource": PROM,
        "targets": ts,
        "gridPos": {"h": h, "w": w, "x": x, "y": y},
        "fieldConfig": {
            "defaults": {
                "unit": unit,
                "custom": {
                    "drawStyle": draw,
                    "lineInterpolation": "smooth",
                    "fillOpacity": int(fill * 100),
                    "spanNulls": True,
                    "showPoints": "never",
                    "stacking": {"mode": "normal" if stack else "none", "group": "A"},
                    "axisPlacement": "auto",
                    "gradientMode": "opacity",
                },
                "thresholds": thresholds or thr(("green", None)),
            },
            "overrides": [],
        },
        "options": {
            "tooltip": {"mode": "multi", "sort": "desc"},
            "legend": {
                "displayMode": "list",
                "placement": "right" if legend_right else "bottom",
                "showLegend": True,
                "calcs": ["lastNotNull", "max", "mean"],
            },
        },
    }


def bargauge(
    title: str,
    targets: list[tuple[str, str]],
    x: int,
    y: int,
    w: int = 8,
    h: int = 8,
    unit: str = "none",
    thresholds=None,
) -> dict:
    ts = []
    for i, (expr, legend) in enumerate(targets):
        t = target_instant(expr, legend, ref=chr(65 + i))
        ts.append(t)
    return {
        "type": "bargauge",
        "title": title,
        "id": nid(),
        "datasource": PROM,
        "targets": ts,
        "gridPos": {"h": h, "w": w, "x": x, "y": y},
        "fieldConfig": {
            "defaults": {
                "unit": unit,
                "thresholds": thresholds
                or thr(("red", None), ("orange", 0.5), ("green", 1)),
                "mappings": [],
            },
            "overrides": [],
        },
        "options": {
            "reduceOptions": {"calcs": ["lastNotNull"], "fields": "", "values": False},
            "orientation": "horizontal",
            "displayMode": "gradient",
            "showUnfilled": True,
        },
    }


def loki_logs(title: str, expr: str, x: int, y: int, w: int = 24, h: int = 10) -> dict:
    return {
        "type": "logs",
        "title": title,
        "id": nid(),
        "datasource": LOKI,
        "targets": [
            {
                "datasource": LOKI,
                "expr": expr,
                "refId": "A",
                "queryType": "range",
                "editorMode": "code",
            }
        ],
        "gridPos": {"h": h, "w": w, "x": x, "y": y},
        "options": {
            "showTime": True,
            "showLabels": True,
            "showCommonLabels": False,
            "wrapLogMessage": True,
            "prettifyLogMessage": False,
            "enableLogDetails": True,
            "sortOrder": "Descending",
            "dedupStrategy": "none",
        },
    }


def dashboard(
    uid: str,
    title: str,
    description: str,
    panels: list[dict],
    tags: list[str] | None = None,
    refresh: str = "30s",
    time_from: str = "now-6h",
) -> dict:
    return {
        "uid": uid,
        "title": title,
        "description": description,
        "tags": tags or ["alvenqis", "mainnet-candidate"],
        "timezone": "browser",
        "schemaVersion": 39,
        "version": 1,
        "refresh": refresh,
        "editable": True,
        "fiscalYearStartMonth": 0,
        "graphTooltip": 1,
        "id": None,
        "links": [
            {
                "asDropdown": True,
                "icon": "dashboard",
                "includeVars": False,
                "keepTime": True,
                "tags": ["alvenqis"],
                "targetBlank": False,
                "title": "Alvenqis dashboards",
                "type": "dashboards",
            }
        ],
        "panels": panels,
        "templating": {"list": []},
        "annotations": {
            "list": [
                {
                    "builtIn": 1,
                    "datasource": {"type": "grafana", "uid": "-- Grafana --"},
                    "enable": True,
                    "hide": True,
                    "iconColor": "rgba(0, 211, 255, 1)",
                    "name": "Annotations & Alerts",
                    "type": "dashboard",
                }
            ]
        },
        "time": {"from": time_from, "to": "now"},
        "timepicker": {},
        "weekStart": "",
    }


def build_overview() -> dict:
    global _id
    _id = 100
    y = 0
    panels: list[dict] = []

    panels.append(row("Command center — service & chain pulse", y))
    y += 1
    # health strip
    health = [
        ("RPC exporter", "alvenqis_rpc_up"),
        ("Chain ready", "alvenqis_chain_ready"),
        ("Indexer sync", "alvenqis_indexer_in_sync_effective"),
        ("Control", "alvenqis_control_up"),
        ("Ops", "alvenqis_ops_up"),
        ("P2P API", "alvenqis_p2p_up"),
        ("Sync API", "alvenqis_sync_up"),
        ("Pool enabled", "alvenqis_pool_enabled"),
    ]
    for i, (title, expr) in enumerate(health):
        maps = up_map() if "sync" not in title.lower() else sync_map()
        if "Indexer" in title:
            maps = sync_map()
        panels.append(stat(title, expr, x=i * 3, y=y, w=3, h=3, mappings=maps))
    y += 3

    panels.append(
        stat(
            "Tip height",
            "alvenqis_chain_height",
            0,
            y,
            w=4,
            h=4,
            graph_mode="area",
            thresholds=thr(("blue", None)),
        )
    )
    panels.append(
        stat(
            "Block count",
            "alvenqis_chain_block_count",
            4,
            y,
            w=4,
            h=4,
            thresholds=thr(("blue", None)),
        )
    )
    panels.append(
        stat(
            "Indexer lag",
            "alvenqis_indexer_lag_blocks_effective",
            8,
            y,
            w=4,
            h=4,
            thresholds=thr(("green", None), ("orange", 1), ("red", 5)),
        )
    )
    panels.append(
        stat(
            "P2P connected",
            "alvenqis_p2p_connected_peers",
            12,
            y,
            w=4,
            h=4,
            thresholds=thr(("orange", None), ("green", 1)),
        )
    )
    panels.append(
        stat(
            "Mempool pending",
            "alvenqis_mempool_pending_count",
            16,
            y,
            w=4,
            h=4,
            thresholds=thr(("green", None), ("orange", 100), ("red", 1000)),
        )
    )
    panels.append(
        stat(
            "Emitted supply (ALVE)",
            "alvenqis_chain_emitted_supply_atomic / 1e8",
            20,
            y,
            w=4,
            h=4,
            decimals=4,
            thresholds=thr(("purple", None)),
        )
    )
    y += 4

    panels.append(row("Chain growth & indexer", y))
    y += 1
    panels.append(
        timeseries(
            "Chain height & indexer height",
            [
                ("alvenqis_chain_height", "chain tip"),
                ("alvenqis_indexer_height", "indexer tip"),
                ("alvenqis_sync_local_height", "sync local"),
                ("alvenqis_sync_network_height", "sync network"),
            ],
            0,
            y,
            w=16,
            h=9,
            unit="none",
            fill=0.1,
        )
    )
    panels.append(
        timeseries(
            "Indexer lag (blocks)",
            [
                ("alvenqis_indexer_lag_blocks_effective", "effective lag"),
                ("alvenqis_indexer_lag_blocks", "indexer lag"),
                ("alvenqis_index_lag_blocks_from_status", "status lag"),
            ],
            16,
            y,
            w=8,
            h=9,
            thresholds=thr(("green", None), ("orange", 1), ("red", 5)),
        )
    )
    y += 9

    panels.append(row("P2P / pool / mempool", y))
    y += 1
    panels.append(
        timeseries(
            "P2P peers",
            [
                ("alvenqis_p2p_connected_peers", "connected"),
                ("alvenqis_p2p_validated_peers", "validated"),
                ("alvenqis_p2p_mining_peers", "mining"),
                ("alvenqis_p2p_validating_peers", "validating"),
                ("alvenqis_p2p_banned_peers", "banned"),
            ],
            0,
            y,
            w=12,
            h=8,
            legend_right=True,
        )
    )
    panels.append(
        timeseries(
            "Pool hashrate & workers",
            [
                ("alvenqis_pool_estimated_hashrate_hs", "hashrate H/s"),
                ("alvenqis_pool_connected_workers", "workers"),
                ("alvenqis_pool_online_workers", "online workers"),
            ],
            12,
            y,
            w=12,
            h=8,
            legend_right=True,
        )
    )
    y += 8
    panels.append(
        timeseries(
            "Mempool pressure",
            [
                ("alvenqis_mempool_pending_count", "pending txs"),
                ("alvenqis_mempool_total_fees_atomic", "total fees atomic"),
                ("alvenqis_mempool_anticipated_base_fee_atomic", "base fee atomic"),
            ],
            0,
            y,
            w=12,
            h=8,
        )
    )
    panels.append(
        timeseries(
            "Pool shares / blocks / rejections",
            [
                ("increase(alvenqis_pool_accepted_shares[1h])", "shares +1h"),
                ("alvenqis_pool_blocks_found", "blocks found"),
                ("alvenqis_pool_matured_blocks", "matured"),
                ("alvenqis_pool_rejected_requests", "rejected"),
                ("alvenqis_pool_rate_limited_requests", "rate limited"),
                ("alvenqis_pool_active_bans", "active bans"),
            ],
            12,
            y,
            w=12,
            h=8,
            legend_right=True,
        )
    )
    y += 8

    panels.append(row("HTTP probes & exporter scrape health", y))
    y += 1
    panels.append(
        timeseries(
            "Blackbox probe success",
            [
                (
                    'probe_success{job=~"alvenqis-http|alvenqis-pool-http"}',
                    "{{instance}}",
                )
            ],
            0,
            y,
            w=12,
            h=8,
            legend_right=True,
        )
    )
    panels.append(
        timeseries(
            "Blackbox probe latency",
            [
                (
                    'probe_duration_seconds{job=~"alvenqis-http|alvenqis-pool-http"}',
                    "{{instance}}",
                )
            ],
            12,
            y,
            w=12,
            h=8,
            unit="s",
            legend_right=True,
        )
    )
    y += 8
    panels.append(
        timeseries(
            "Exporter scrape success by source",
            [("alvenqis_exporter_scrape_success", "{{source}}")],
            0,
            y,
            w=12,
            h=8,
            legend_right=True,
        )
    )
    panels.append(
        timeseries(
            "Exporter scrape duration by source",
            [("alvenqis_exporter_scrape_duration_seconds", "{{source}}")],
            12,
            y,
            w=12,
            h=8,
            unit="s",
            legend_right=True,
        )
    )
    y += 8

    panels.append(row("Host pulse (see Host dashboard for detail)", y))
    y += 1
    panels.append(
        gauge(
            "CPU %",
            '100 - (avg(rate(node_cpu_seconds_total{mode="idle"}[5m])) * 100)',
            0,
            y,
            w=6,
            h=6,
        )
    )
    panels.append(
        gauge(
            "Memory used %",
            "100 * (1 - (node_memory_MemAvailable_bytes / node_memory_MemTotal_bytes))",
            6,
            y,
            w=6,
            h=6,
        )
    )
    panels.append(
        gauge(
            "Root disk used %",
            '100 * (1 - (node_filesystem_avail_bytes{mountpoint="/",fstype!~"tmpfs|overlay"} / node_filesystem_size_bytes{mountpoint="/",fstype!~"tmpfs|overlay"}))',
            12,
            y,
            w=6,
            h=6,
        )
    )
    panels.append(
        stat(
            "Backup last success age",
            "time() - alvenqis_backup_last_success_unixtime",
            18,
            y,
            w=6,
            h=6,
            unit="s",
            thresholds=thr(("green", None), ("orange", 90000), ("red", 200000)),
        )
    )

    return dashboard(
        "alvenqis-docker-overview",
        "Alvenqis Network Overview",
        "Mainnet Candidate command center: chain, indexer, P2P, pool, probes, host pulse. Not Mainnet Live.",
        panels,
        tags=["alvenqis", "overview", "mainnet-candidate"],
        time_from="now-12h",
    )


def build_chain() -> dict:
    global _id
    _id = 200
    y = 0
    panels: list[dict] = []

    panels.append(row("Canonical chain", y))
    y += 1
    for i, (t, e) in enumerate(
        [
            ("Height", "alvenqis_chain_height"),
            ("Blocks", "alvenqis_chain_block_count"),
            ("Ready", "alvenqis_chain_ready"),
            ("RPC up", "alvenqis_rpc_up"),
            ("Supply ALVE", "alvenqis_chain_emitted_supply_atomic / 1e8"),
            ("Tx indexed", "alvenqis_indexer_transaction_count"),
        ]
    ):
        maps = up_map() if t in ("Ready", "RPC up") else None
        panels.append(
            stat(
                t,
                e,
                i * 4,
                y,
                w=4,
                h=4,
                mappings=maps,
                decimals=2 if "ALVE" in t else None,
            )
        )
    y += 4

    panels.append(
        timeseries(
            "Tip height over time",
            [
                ("alvenqis_chain_height", "height"),
                ("deriv(alvenqis_chain_height[10m]) * 60", "blocks/min (deriv)"),
            ],
            0,
            y,
            w=16,
            h=9,
        )
    )
    panels.append(
        timeseries(
            "Block production rate",
            [
                ("rate(alvenqis_chain_height[5m]) * 60", "blocks/min (5m)"),
                ("rate(alvenqis_chain_height[15m]) * 60", "blocks/min (15m)"),
                ("rate(alvenqis_chain_height[1h]) * 60", "blocks/min (1h)"),
            ],
            16,
            y,
            w=8,
            h=9,
            unit="cpm",
        )
    )
    y += 9

    panels.append(row("Indexer depth", y))
    y += 1
    panels.append(
        timeseries(
            "Indexer vs chain",
            [
                ("alvenqis_chain_height", "chain"),
                ("alvenqis_indexer_height", "indexer"),
                ("alvenqis_indexer_chain_height", "indexer observed chain"),
                ("alvenqis_index_height_from_status", "status index height"),
            ],
            0,
            y,
            w=12,
            h=9,
        )
    )
    panels.append(
        timeseries(
            "Indexer lag & sync flag",
            [
                ("alvenqis_indexer_lag_blocks_effective", "lag effective"),
                ("alvenqis_indexer_in_sync_effective", "in_sync effective"),
                ("alvenqis_indexer_initialized", "initialized"),
            ],
            12,
            y,
            w=12,
            h=9,
        )
    )
    y += 9
    panels.append(
        timeseries(
            "Indexer catalog growth",
            [
                ("alvenqis_indexer_block_count", "blocks"),
                ("alvenqis_indexer_transaction_count", "transactions"),
                ("alvenqis_indexer_address_count", "addresses"),
            ],
            0,
            y,
            w=16,
            h=8,
            legend_right=True,
        )
    )
    panels.append(
        bargauge(
            "Indexer / RPC scrape OK",
            [
                ('alvenqis_exporter_scrape_success{source="rpc_status"}', "rpc"),
                ('alvenqis_exporter_scrape_success{source="indexer_status"}', "indexer"),
                ('alvenqis_exporter_scrape_success{source="mempool_status"}', "mempool"),
            ],
            16,
            y,
            w=8,
            h=8,
        )
    )
    y += 8

    panels.append(row("Mempool economics", y))
    y += 1
    panels.append(
        timeseries(
            "Pending transactions",
            [("alvenqis_mempool_pending_count", "pending")],
            0,
            y,
            w=12,
            h=8,
        )
    )
    panels.append(
        timeseries(
            "Fees (atomic units)",
            [
                ("alvenqis_mempool_total_fees_atomic", "total fees"),
                ("alvenqis_mempool_anticipated_base_fee_atomic", "base fee"),
            ],
            12,
            y,
            w=12,
            h=8,
        )
    )
    y += 8

    panels.append(row("Supply", y))
    y += 1
    panels.append(
        timeseries(
            "Emitted supply (ALVE)",
            [("alvenqis_chain_emitted_supply_atomic / 1e8", "ALVE emitted")],
            0,
            y,
            w=16,
            h=8,
            unit="none",
        )
    )
    panels.append(
        timeseries(
            "Emission rate (ALVE/hour)",
            [
                (
                    "rate(alvenqis_chain_emitted_supply_atomic[30m]) * 3600 / 1e8",
                    "ALVE/h",
                )
            ],
            16,
            y,
            w=8,
            h=8,
        )
    )

    return dashboard(
        "alvenqis-chain",
        "Alvenqis Chain & Indexer",
        "Deep dive: tip growth, block rate, indexer lag, mempool fees, emission. Mainnet Candidate only.",
        panels,
        tags=["alvenqis", "chain", "indexer"],
        time_from="now-24h",
    )


def build_network() -> dict:
    global _id
    _id = 300
    y = 0
    panels: list[dict] = []

    panels.append(row("P2P surface", y))
    y += 1
    for i, (t, e) in enumerate(
        [
            ("Connected", "alvenqis_p2p_connected_peers"),
            ("Validated", "alvenqis_p2p_validated_peers"),
            ("Mining peers", "alvenqis_p2p_mining_peers"),
            ("Validating", "alvenqis_p2p_validating_peers"),
            ("Banned", "alvenqis_p2p_banned_peers"),
            ("Seeds cfg", "alvenqis_p2p_configured_seeds"),
            ("Syncing", "alvenqis_p2p_syncing"),
            ("P2P API", "alvenqis_p2p_up"),
        ]
    ):
        maps = up_map() if t in ("Syncing", "P2P API") else None
        panels.append(stat(t, e, (i % 8) * 3, y, w=3, h=3, mappings=maps))
    y += 3

    panels.append(
        timeseries(
            "Peer topology over time",
            [
                ("alvenqis_p2p_connected_peers", "connected"),
                ("alvenqis_p2p_validated_peers", "validated"),
                ("alvenqis_p2p_mining_peers", "mining"),
                ("alvenqis_p2p_validating_peers", "validating"),
                ("alvenqis_p2p_banned_peers", "banned"),
                ("alvenqis_sync_connected_peers", "sync connected"),
                ("alvenqis_sync_validated_peers", "sync validated"),
            ],
            0,
            y,
            w=16,
            h=10,
            legend_right=True,
        )
    )
    panels.append(
        timeseries(
            "Observed network hashrate (H/s)",
            [
                (
                    "alvenqis_p2p_observed_network_hashrate_hs",
                    "observed H/s",
                )
            ],
            16,
            y,
            w=8,
            h=10,
            unit="ops",
        )
    )
    y += 10

    panels.append(row("Sync status", y))
    y += 1
    panels.append(
        timeseries(
            "Local vs network height",
            [
                ("alvenqis_sync_local_height", "local"),
                ("alvenqis_sync_network_height", "network"),
                ("alvenqis_chain_height", "canonical tip"),
            ],
            0,
            y,
            w=12,
            h=9,
        )
    )
    panels.append(
        timeseries(
            "Sync remaining / progress",
            [
                ("alvenqis_sync_remaining_blocks", "remaining"),
                ("alvenqis_sync_progress_percent", "progress %"),
            ],
            12,
            y,
            w=12,
            h=9,
        )
    )
    y += 9

    panels.append(row("Edge probes", y))
    y += 1
    panels.append(
        timeseries(
            "Probe success matrix",
            [('probe_success{job=~"alvenqis-http|alvenqis-pool-http"}', "{{instance}}")],
            0,
            y,
            w=12,
            h=9,
            legend_right=True,
        )
    )
    panels.append(
        timeseries(
            "Probe latency",
            [
                (
                    'probe_duration_seconds{job=~"alvenqis-http|alvenqis-pool-http"}',
                    "{{instance}}",
                )
            ],
            12,
            y,
            w=12,
            h=9,
            unit="s",
            legend_right=True,
        )
    )

    return dashboard(
        "alvenqis-network",
        "Alvenqis P2P & Sync",
        "Peer counts, bans, seeds, sync progress, blackbox edge probes. Mainnet Candidate.",
        panels,
        tags=["alvenqis", "p2p", "sync"],
        time_from="now-12h",
    )


def build_pool() -> dict:
    global _id
    _id = 400
    y = 0
    panels: list[dict] = []

    panels.append(row("Pool status", y))
    y += 1
    for i, (t, e, m) in enumerate(
        [
            ("Enabled", "alvenqis_pool_enabled", up_map()),
            ("Pool up", "alvenqis_pool_up", up_map()),
            ("Workers", "alvenqis_pool_connected_workers", None),
            ("Online", "alvenqis_pool_online_workers", None),
            ("Hashrate H/s", "alvenqis_pool_estimated_hashrate_hs", None),
            ("Shares", "alvenqis_pool_accepted_shares", None),
            ("Blocks", "alvenqis_pool_blocks_found", None),
            ("Matured", "alvenqis_pool_matured_blocks", None),
        ]
    ):
        panels.append(stat(t, e, (i % 8) * 3, y, w=3, h=4, mappings=m))
    y += 4

    panels.append(
        timeseries(
            "Estimated hashrate",
            [("alvenqis_pool_estimated_hashrate_hs", "H/s")],
            0,
            y,
            w=12,
            h=9,
            unit="ops",
        )
    )
    panels.append(
        timeseries(
            "Workers online vs connected",
            [
                ("alvenqis_pool_connected_workers", "connected"),
                ("alvenqis_pool_online_workers", "online"),
            ],
            12,
            y,
            w=12,
            h=9,
        )
    )
    y += 9

    panels.append(
        timeseries(
            "Shares (lifetime + 1h increase)",
            [
                ("alvenqis_pool_accepted_shares", "lifetime"),
                ("increase(alvenqis_pool_accepted_shares[1h])", "+1h"),
                ("rate(alvenqis_pool_accepted_shares[15m]) * 60", "shares/min"),
            ],
            0,
            y,
            w=12,
            h=9,
        )
    )
    panels.append(
        timeseries(
            "Blocks found / matured / immature gap",
            [
                ("alvenqis_pool_blocks_found", "found"),
                ("alvenqis_pool_matured_blocks", "matured"),
                (
                    "alvenqis_pool_blocks_found - alvenqis_pool_matured_blocks",
                    "immature gap",
                ),
            ],
            12,
            y,
            w=12,
            h=9,
        )
    )
    y += 9

    panels.append(row("Abuse & admission controls", y))
    y += 1
    panels.append(
        timeseries(
            "Rejected / rate-limited / bans",
            [
                ("alvenqis_pool_rejected_requests", "rejected"),
                ("alvenqis_pool_rate_limited_requests", "rate limited"),
                ("alvenqis_pool_active_bans", "active bans"),
            ],
            0,
            y,
            w=16,
            h=9,
            legend_right=True,
        )
    )
    panels.append(
        bargauge(
            "Pool scrape & enable flags",
            [
                ("alvenqis_pool_enabled", "enabled"),
                ("alvenqis_pool_up", "up"),
                ('alvenqis_exporter_scrape_success{source="pool_status"}', "scrape"),
            ],
            16,
            y,
            w=8,
            h=9,
        )
    )
    y += 9

    panels.append(row("Pool HTTP probes", y))
    y += 1
    panels.append(
        timeseries(
            "Pool blackbox",
            [('probe_success{job="alvenqis-pool-http"}', "{{instance}}")],
            0,
            y,
            w=12,
            h=8,
        )
    )
    panels.append(
        timeseries(
            "Pool probe latency",
            [('probe_duration_seconds{job="alvenqis-pool-http"}', "{{instance}}")],
            12,
            y,
            w=12,
            h=8,
            unit="s",
        )
    )

    return dashboard(
        "alvenqis-pool",
        "Alvenqis Mining Pool",
        "Pool prototype metrics: workers, hashrate, shares, maturity, abuse controls. Not a production pool claim.",
        panels,
        tags=["alvenqis", "pool", "mining"],
        time_from="now-12h",
    )


def build_host() -> dict:
    global _id
    _id = 500
    y = 0
    panels: list[dict] = []

    panels.append(row("Host vitals", y))
    y += 1
    panels.append(
        gauge(
            "CPU %",
            '100 - (avg(rate(node_cpu_seconds_total{mode="idle"}[5m])) * 100)',
            0,
            y,
            w=6,
            h=6,
        )
    )
    panels.append(
        gauge(
            "Memory used %",
            "100 * (1 - (node_memory_MemAvailable_bytes / node_memory_MemTotal_bytes))",
            6,
            y,
            w=6,
            h=6,
        )
    )
    panels.append(
        gauge(
            "Root disk used %",
            '100 * (1 - (node_filesystem_avail_bytes{mountpoint="/",fstype!~"tmpfs|overlay"} / node_filesystem_size_bytes{mountpoint="/",fstype!~"tmpfs|overlay"}))',
            12,
            y,
            w=6,
            h=6,
        )
    )
    panels.append(
        gauge(
            "Load / CPU count",
            'node_load1 / count(node_cpu_seconds_total{mode="idle"})',
            18,
            y,
            w=6,
            h=6,
            unit="none",
            min_v=0,
            max_v=4,
            thresholds=thr(("green", None), ("orange", 1), ("red", 2)),
        )
    )
    y += 6

    panels.append(
        timeseries(
            "CPU usage %",
            [
                (
                    '100 - (avg by (mode) (rate(node_cpu_seconds_total{mode="idle"}[5m])) * 100)',
                    "used",
                ),
                (
                    'avg by (mode) (rate(node_cpu_seconds_total{mode!="idle"}[5m])) * 100',
                    "{{mode}}",
                ),
            ],
            0,
            y,
            w=12,
            h=9,
            unit="percent",
            legend_right=True,
        )
    )
    panels.append(
        timeseries(
            "Memory breakdown",
            [
                ("node_memory_MemTotal_bytes", "total"),
                ("node_memory_MemAvailable_bytes", "available"),
                ("node_memory_MemTotal_bytes - node_memory_MemAvailable_bytes", "used"),
                ("node_memory_Cached_bytes", "cached"),
                ("node_memory_Buffers_bytes", "buffers"),
            ],
            12,
            y,
            w=12,
            h=9,
            unit="bytes",
            legend_right=True,
        )
    )
    y += 9

    panels.append(
        timeseries(
            "Load average",
            [
                ("node_load1", "load1"),
                ("node_load5", "load5"),
                ("node_load15", "load15"),
                ('count(node_cpu_seconds_total{mode="idle"})', "cpus"),
            ],
            0,
            y,
            w=12,
            h=8,
        )
    )
    panels.append(
        timeseries(
            "Disk free (root)",
            [
                (
                    'node_filesystem_avail_bytes{mountpoint="/",fstype!~"tmpfs|overlay"}',
                    "avail",
                ),
                (
                    'node_filesystem_size_bytes{mountpoint="/",fstype!~"tmpfs|overlay"}',
                    "size",
                ),
            ],
            12,
            y,
            w=12,
            h=8,
            unit="bytes",
        )
    )
    y += 8

    panels.append(row("Network & disk IO", y))
    y += 1
    panels.append(
        timeseries(
            "Network traffic",
            [
                (
                    'rate(node_network_receive_bytes_total{device!~"lo|veth.*|docker.*|br-.*"}[5m])',
                    "rx {{device}}",
                ),
                (
                    'rate(node_network_transmit_bytes_total{device!~"lo|veth.*|docker.*|br-.*"}[5m])',
                    "tx {{device}}",
                ),
            ],
            0,
            y,
            w=12,
            h=9,
            unit="Bps",
            legend_right=True,
        )
    )
    panels.append(
        timeseries(
            "Disk IO",
            [
                (
                    "rate(node_disk_read_bytes_total[5m])",
                    "read {{device}}",
                ),
                (
                    "rate(node_disk_written_bytes_total[5m])",
                    "write {{device}}",
                ),
            ],
            12,
            y,
            w=12,
            h=9,
            unit="Bps",
            legend_right=True,
        )
    )
    y += 9

    panels.append(row("Backup metric (textfile)", y))
    y += 1
    panels.append(
        stat(
            "Last backup success (unix)",
            "alvenqis_backup_last_success_unixtime",
            0,
            y,
            w=8,
            h=4,
            unit="dateTimeFromNow",
        )
    )
    panels.append(
        timeseries(
            "Backup last success age (seconds)",
            [
                (
                    "time() - alvenqis_backup_last_success_unixtime",
                    "age seconds",
                )
            ],
            8,
            y,
            w=16,
            h=8,
            unit="s",
            thresholds=thr(("green", None), ("orange", 90000), ("red", 200000)),
        )
    )

    return dashboard(
        "alvenqis-host",
        "Alvenqis Host Metrics",
        "Node exporter: CPU, memory, disk, network, load, backup textfile. Host is validator/control-plane — not a CUDA miner claim.",
        panels,
        tags=["alvenqis", "host", "node-exporter"],
        time_from="now-6h",
    )


def build_ops() -> dict:
    global _id
    _id = 600
    y = 0
    panels: list[dict] = []

    panels.append(row("Control-plane health matrix", y))
    y += 1
    panels.append(
        bargauge(
            "Component up flags",
            [
                ("alvenqis_rpc_up", "rpc"),
                ("alvenqis_control_up", "control"),
                ("alvenqis_ops_up", "ops"),
                ("alvenqis_p2p_up", "p2p"),
                ("alvenqis_sync_up", "sync"),
                ("alvenqis_indexer_up", "indexer"),
                ("alvenqis_mempool_up", "mempool"),
                ("alvenqis_pool_up", "pool"),
            ],
            0,
            y,
            w=12,
            h=10,
        )
    )
    panels.append(
        timeseries(
            "Exporter scrape success",
            [("alvenqis_exporter_scrape_success", "{{source}}")],
            12,
            y,
            w=12,
            h=10,
            legend_right=True,
        )
    )
    y += 10

    panels.append(
        timeseries(
            "Exporter scrape latency (s)",
            [("alvenqis_exporter_scrape_duration_seconds", "{{source}}")],
            0,
            y,
            w=24,
            h=9,
            unit="s",
            legend_right=True,
        )
    )
    y += 9

    panels.append(row("Prometheus self", y))
    y += 1
    panels.append(
        timeseries(
            "Prometheus TSDB head series",
            [("prometheus_tsdb_head_series", "series")],
            0,
            y,
            w=12,
            h=8,
        )
    )
    panels.append(
        timeseries(
            "Prometheus scrape duration",
            [
                (
                    "max by (job) (scrape_duration_seconds)",
                    "{{job}}",
                )
            ],
            12,
            y,
            w=12,
            h=8,
            unit="s",
            legend_right=True,
        )
    )
    y += 8

    panels.append(row("Container logs (Loki)", y))
    y += 1
    # Alloy typically labels container names; keep flexible LogQL
    panels.append(
        loki_logs(
            "Alvenqis container logs (errors)",
            '{container=~"alvenqis-.*"} |~ "(?i)error|fail|panic|fatal"',
            0,
            y,
            w=24,
            h=12,
        )
    )
    y += 12
    panels.append(
        loki_logs(
            "RPC / node / pool stream",
            '{container=~"alvenqis-(rpc|node|pool|indexer).*"}',
            0,
            y,
            w=24,
            h=12,
        )
    )

    return dashboard(
        "alvenqis-ops",
        "Alvenqis Ops & Logs",
        "Exporter scrapes, Prometheus self metrics, Loki error streams for the control plane.",
        panels,
        tags=["alvenqis", "ops", "loki"],
        time_from="now-3h",
        refresh="15s",
    )


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    boards = {
        "alvenqis-overview.json": build_overview(),
        "alvenqis-chain.json": build_chain(),
        "alvenqis-network.json": build_network(),
        "alvenqis-pool.json": build_pool(),
        "alvenqis-host.json": build_host(),
        "alvenqis-ops.json": build_ops(),
    }
    for name, board in boards.items():
        path = OUT / name
        path.write_text(json.dumps(board, indent=2) + "\n", encoding="utf-8")
        print(f"wrote {path} panels={len(board['panels'])} uid={board['uid']}")


if __name__ == "__main__":
    main()
