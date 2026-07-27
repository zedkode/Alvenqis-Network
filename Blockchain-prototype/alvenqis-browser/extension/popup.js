const LIVE_MS = 12_000;

function callHost(method, params = {}) {
  return new Promise((resolve) => {
    chrome.runtime.sendMessage({ type: "host", method, params }, (response) => {
      const error = chrome.runtime.lastError;
      if (error) {
        resolve({ ok: false, error: error.message || String(error) });
        return;
      }
      resolve(response || { ok: false, error: "no response" });
    });
  });
}

function show(id, value) {
  const el = document.getElementById(id);
  if (!el) return;
  el.textContent =
    typeof value === "string" ? value : JSON.stringify(value, null, 2);
}

function passphrase() {
  return document.getElementById("passphrase").value || "";
}

function clearPassphrase() {
  const el = document.getElementById("passphrase");
  if (el) el.value = "";
}

function clearDangerFields() {
  const oldPass = document.getElementById("oldPass");
  const newPass = document.getElementById("newPass");
  if (oldPass) oldPass.value = "";
  if (newPass) newPass.value = "";
}

async function run(method, outId, params = {}) {
  show(outId, "…");
  const response = await callHost(method, params);
  show(outId, response);
  return response;
}

function formatAtomic(atomic) {
  if (atomic == null) return "—";
  const n = Number(atomic);
  if (!Number.isFinite(n)) return String(atomic);
  return (n / 100_000_000).toFixed(8).replace(/\.?0+$/, "") + " ALVE";
}

async function refreshLiveStrip() {
  const strip = document.getElementById("liveStrip");
  if (!strip) return;
  const [status, sync, mempool, indexer] = await Promise.all([
    callHost("rpc_status"),
    callHost("sync_status"),
    callHost("mempool_status"),
    callHost("indexer_status"),
  ]);
  if (!status?.ok && !sync?.ok) {
    strip.classList.add("error");
    strip.textContent =
      status?.error || sync?.error || "Host / RPC offline";
    return;
  }
  strip.classList.remove("error");
  const s = status?.result || {};
  const y = sync?.result || {};
  const m = mempool?.result || {};
  const i = indexer?.result || {};
  const height = s.height ?? y.local_height ?? "—";
  const tip = (s.tip_hash || "").slice(0, 12);
  const syncState = y.sync_state || "—";
  const pending = m.pending_count ?? "—";
  const idxH = i.indexed_height ?? "—";
  const lag = i.lag_blocks ?? "—";
  strip.textContent = `h=${height}  idx=${idxH}  lag=${lag}  sync=${syncState}  mempool=${pending}  tip=${tip || "—"}…`;
}

document.getElementById("btnPing").addEventListener("click", () => {
  run("ping", "hostOut");
});
document.getElementById("btnNetwork").addEventListener("click", () => {
  run("network_info", "hostOut");
});
document.getElementById("btnRpcStatus").addEventListener("click", () => {
  run("rpc_status", "hostOut");
});
document.getElementById("btnSync").addEventListener("click", () => {
  run("sync_status", "hostOut");
});
document.getElementById("btnMempool").addEventListener("click", () => {
  run("mempool_status", "hostOut");
});
document.getElementById("btnSupply").addEventListener("click", () => {
  run("supply", "hostOut");
});

document.getElementById("btnLatest").addEventListener("click", async () => {
  const response = await run("block_latest", "exploreOut");
  if (response?.ok && response.result) {
    const b = response.result;
    show(
      "exploreOut",
      {
        height: b.height,
        hash: b.hash,
        txs: b.transaction_count,
        coinbase: formatAtomic(
          (b.transactions || []).find((t) => !t.from)?.amount_atomic
        ),
        full: b,
      }
    );
  }
});
document.getElementById("btnRecent").addEventListener("click", () => {
  run("recent_blocks", "exploreOut", { count: 5 });
});
document.getElementById("btnIndexer").addEventListener("click", () => {
  run("indexer_status", "exploreOut");
});
document.getElementById("btnIndexerBlock").addEventListener("click", () => {
  run("indexer_block_latest", "exploreOut");
});
document.getElementById("btnBlockHeight").addEventListener("click", () => {
  const height = Number(document.getElementById("blockHeight").value);
  if (!Number.isFinite(height) || height < 0) {
    show("exploreOut", { ok: false, error: "enter a valid height" });
    return;
  }
  run("block_by_height", "exploreOut", { height });
});
document.getElementById("btnTxHash").addEventListener("click", () => {
  const hash = document.getElementById("lookupHash").value.trim();
  if (!hash) {
    show("exploreOut", { ok: false, error: "enter a hash" });
    return;
  }
  run("transaction_by_hash", "exploreOut", { hash });
});
document.getElementById("btnBlockHash").addEventListener("click", () => {
  const hash = document.getElementById("lookupHash").value.trim();
  if (!hash) {
    show("exploreOut", { ok: false, error: "enter a hash" });
    return;
  }
  run("block_by_hash", "exploreOut", { hash });
});

document.getElementById("btnKeystore").addEventListener("click", () => {
  run("keystore_status", "sessionOut");
});
document.getElementById("btnExportPublic").addEventListener("click", () => {
  run("export_public", "sessionOut");
});

document.getElementById("btnCreateWallet").addEventListener("click", async () => {
  const pass = passphrase();
  if (pass.length < 8) {
    show("sessionOut", { ok: false, error: "passphrase must be at least 8 characters" });
    return;
  }
  const ok = window.confirm(
    "Create encrypted host keystore?\n\nThe recovery phrase is shown only by the native host (OS dialog or host CLI), never in this extension.\nYou must acknowledge the backup there before the wallet is created.\n\nPrefer: alvenqis-browser-host --init-wallet for a clean offline backup flow."
  );
  if (!ok) return;
  await run("create_wallet", "sessionOut", { passphrase: pass });
  clearPassphrase();
  await run("keystore_status", "sessionOut");
});

document.getElementById("btnUnlock").addEventListener("click", async () => {
  const pass = passphrase();
  if (!pass) {
    show("sessionOut", { ok: false, error: "enter passphrase" });
    return;
  }
  await run("unlock", "sessionOut", { passphrase: pass });
  clearPassphrase();
  await run("session_status", "sessionOut");
});

document.getElementById("btnLock").addEventListener("click", async () => {
  await run("lock", "sessionOut");
  await run("keystore_status", "sessionOut");
});

document.getElementById("btnAccount").addEventListener("click", () => {
  run("account", "sessionOut");
});

document.getElementById("btnChangePass").addEventListener("click", async () => {
  const old_passphrase = document.getElementById("oldPass").value || "";
  const new_passphrase = document.getElementById("newPass").value || "";
  if (old_passphrase.length < 1 || new_passphrase.length < 8) {
    show("dangerOut", {
      ok: false,
      error: "old passphrase and new passphrase (min 8) required",
    });
    return;
  }
  await run("change_passphrase", "dangerOut", { old_passphrase, new_passphrase });
  clearDangerFields();
  await run("keystore_status", "sessionOut");
});

document.getElementById("btnDeleteWallet").addEventListener("click", async () => {
  const pass =
    document.getElementById("oldPass").value ||
    passphrase() ||
    "";
  if (!pass) {
    show("dangerOut", { ok: false, error: "enter current passphrase to delete" });
    return;
  }
  const ok = window.confirm(
    "Delete the encrypted host keystore file?\n\nThis cannot be undone without a CLI mnemonic backup."
  );
  if (!ok) return;
  await run("delete_wallet", "dangerOut", { passphrase: pass });
  clearDangerFields();
  clearPassphrase();
  await run("keystore_status", "sessionOut");
});

document.getElementById("btnSend").addEventListener("click", async () => {
  const to = document.getElementById("sendTo").value.trim();
  const amount_alve = document.getElementById("sendAmount").value.trim();
  const fee = document.getElementById("sendFee").value.trim();
  if (!to || !amount_alve) {
    show("sendOut", { ok: false, error: "to and amount are required" });
    return;
  }
  const ok = window.confirm(
    `Send ${amount_alve} ALVE to\n${to}\n\nMainnet Candidate only. Continue?`
  );
  if (!ok) {
    show("sendOut", { ok: false, error: "cancelled in extension UI" });
    return;
  }
  const params = { to, amount_alve };
  if (fee) params.priority_fee_alve = fee;
  await run("send", "sendOut", params);
});

// Initial status + live strip
run("ping", "hostOut");
run("keystore_status", "sessionOut");
refreshLiveStrip();
setInterval(refreshLiveStrip, LIVE_MS);
