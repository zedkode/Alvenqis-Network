#!/usr/bin/env python3
import json
import urllib.request

status = json.load(urllib.request.urlopen("http://127.0.0.1:30787/api/v1/pool/status"))
chain = json.load(urllib.request.urlopen("http://127.0.0.1:10787/status"))
tip = chain.get("height")
conf = status.get("block_maturity_confirmations", 12)
print(f"chain_tip={tip}")
print(
    f"pool_blocks_found={status.get('blocks_found')} matured={status.get('matured_blocks')} conf_needed={conf}"
)
for b in status.get("recent_blocks", []):
    need = b["height"] + conf
    ok = tip is not None and tip >= need
    print(
        f"h={b['height']} status={b['status']} need_tip>={need} tip_ok={ok} hash={b['hash'][:24]}..."
    )
