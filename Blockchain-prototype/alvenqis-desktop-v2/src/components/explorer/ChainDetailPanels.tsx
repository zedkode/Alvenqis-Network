import { ExternalLink } from "lucide-react";
import {
  confirmations,
  formatAtomic,
  formatTimestamp,
  shortHash
} from "@shared/format";
import type { DesktopBlock, DesktopTransaction } from "@shared/types";
import { AddressChip } from "../ui/AddressChip";
import { CopyField } from "../ui/CopyField";
import { KeyValue } from "../ui/Panel";

function asAtomic(value: unknown): string {
  if (value == null) return "0";
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "bigint") return String(value);
  return "0";
}

function asNum(value: unknown): number {
  if (typeof value === "number" && Number.isFinite(value)) return value;
  if (typeof value === "string" && value.trim()) {
    const n = Number(value);
    return Number.isFinite(n) ? n : 0;
  }
  return 0;
}

function asStr(value: unknown): string {
  return typeof value === "string" ? value : value == null ? "" : String(value);
}

function elapsed(seconds: number): string {
  if (!seconds) return "—";
  const value = Math.max(0, Math.floor(Date.now() / 1000) - seconds);
  if (value < 60) return `${value}s ago`;
  if (value < 3600) return `${Math.floor(value / 60)}m ago`;
  if (value < 86_400) return `${Math.floor(value / 3600)}h ago`;
  return `${Math.floor(value / 86_400)}d ago`;
}

export type BlockLike = DesktopBlock & Record<string, unknown>;
export type TxLike = DesktopTransaction & Record<string, unknown>;

export function normalizeBlock(raw: Record<string, unknown> | DesktopBlock): BlockLike {
  const r = raw as Record<string, unknown>;
  const hashes = Array.isArray(r.transaction_hashes)
    ? (r.transaction_hashes as unknown[]).map((h) => asStr(h)).filter(Boolean)
    : [];
  return {
    height: asNum(r.height),
    hash: asStr(r.hash),
    previous_hash: asStr(r.previous_hash),
    merkle_root: asStr(r.merkle_root),
    timestamp: asNum(r.timestamp),
    nonce: asNum(r.nonce),
    difficulty_leading_zero_bits: asNum(r.difficulty_leading_zero_bits),
    transaction_count: asNum(r.transaction_count) || hashes.length,
    miner_address: asStr(r.miner_address),
    coinbase_payout_atomic: asAtomic(r.coinbase_payout_atomic),
    miner_reward_atomic: asAtomic(r.miner_reward_atomic),
    fees_atomic: asAtomic(r.fees_atomic),
    burned_fees_atomic: asAtomic(r.burned_fees_atomic),
    priority_fees_atomic: asAtomic(r.priority_fees_atomic),
    base_fee_atomic: asAtomic(r.base_fee_atomic),
    transaction_hashes: hashes
  };
}

export function normalizeTx(raw: Record<string, unknown> | DesktopTransaction): TxLike {
  const r = raw as Record<string, unknown>;
  const from = r.from == null || r.from === "" ? null : asStr(r.from);
  return {
    lifecycle_status: asStr(r.lifecycle_status) || "confirmed",
    hash: asStr(r.hash),
    block_height: asNum(r.block_height),
    transaction_index: asNum(r.transaction_index),
    nonce: asNum(r.nonce),
    from,
    to: asStr(r.to),
    amount_atomic: asAtomic(r.amount_atomic),
    effective_fee_atomic: asAtomic(r.effective_fee_atomic ?? r.fee_atomic),
    burned_fee_atomic: asAtomic(r.burned_fee_atomic),
    effective_priority_fee_atomic: asAtomic(
      r.effective_priority_fee_atomic ?? r.priority_fee_atomic
    ),
    authorization_state: asStr(r.authorization_state) || (from ? "signed" : "coinbase"),
    block_hash: asStr(r.block_hash),
    version: asNum(r.version),
    max_fee_atomic: asAtomic(r.max_fee_atomic),
    base_fee_atomic: asAtomic(r.base_fee_atomic),
    memo_hash: asStr(r.memo_hash) || null,
    sender_public_key_hex: asStr(r.sender_public_key_hex) || null,
    signature_hex: asStr(r.signature_hex) || null,
    block_transaction_count: asNum(r.block_transaction_count)
  } as TxLike;
}

export function BlockDetailBody({
  block,
  tipHeight,
  intervalLabel,
  onOpenTx,
  onOpenExternal
}: {
  block: BlockLike | DesktopBlock;
  tipHeight: number | null;
  intervalLabel?: string;
  onOpenTx?: (hash: string) => void;
  onOpenExternal?: () => void;
}) {
  const b = normalizeBlock(block);
  const rewardFees =
    Number(b.miner_reward_atomic || 0) + Number(b.fees_atomic || 0);
  const totalOut =
    rewardFees > 0 ? rewardFees : Number(b.coinbase_payout_atomic || 0);

  return (
    <div className="detail-grid detail-rich">
      <KeyValue label="Status">
        <span className="positive">Canonical</span>
      </KeyValue>
      <KeyValue label="Confirmations">{confirmations(tipHeight, b.height)}</KeyValue>
      <KeyValue label="Height" mono>
        {b.height}
      </KeyValue>
      <KeyValue label="Age">{elapsed(b.timestamp)}</KeyValue>
      <KeyValue label="Timestamp">{formatTimestamp(b.timestamp)}</KeyValue>
      {intervalLabel ? <KeyValue label="Interval to prev">{intervalLabel}</KeyValue> : null}

      <div className="detail-span-full">
        <KeyValue label="Block hash">
          <CopyField value={b.hash} label="block hash" />
        </KeyValue>
      </div>
      <div className="detail-span-full">
        <KeyValue label="Previous hash">
          <CopyField value={b.previous_hash} label="previous hash" />
        </KeyValue>
      </div>
      <div className="detail-span-full">
        <KeyValue label="Merkle root">
          <CopyField value={b.merkle_root} label="merkle root" />
        </KeyValue>
      </div>

      <KeyValue label="Nonce" mono>
        {b.nonce}
      </KeyValue>
      <KeyValue label="PoW difficulty">{b.difficulty_leading_zero_bits} leading zero bits</KeyValue>
      <KeyValue label="Transactions">{b.transaction_count}</KeyValue>
      <KeyValue label="Base fee" mono>
        {formatAtomic(b.base_fee_atomic)} ALVE
      </KeyValue>

      <div className="detail-span-full">
        <KeyValue label="Miner payout address">
          {b.miner_address ? <AddressChip value={b.miner_address} full /> : "—"}
        </KeyValue>
      </div>

      <KeyValue label="Coinbase payout" mono>
        {formatAtomic(b.coinbase_payout_atomic)} ALVE
      </KeyValue>
      <KeyValue label="Miner reward" mono>
        {formatAtomic(b.miner_reward_atomic)} ALVE
      </KeyValue>
      <KeyValue label="Total fees" mono>
        {formatAtomic(b.fees_atomic)} ALVE
      </KeyValue>
      <KeyValue label="Priority fees" mono>
        {formatAtomic(b.priority_fees_atomic)} ALVE
      </KeyValue>
      <KeyValue label="Burned fees" mono>
        {formatAtomic(b.burned_fees_atomic)} ALVE
      </KeyValue>
      <KeyValue label="Coinbase + fees (check)" mono>
        {formatAtomic(String(totalOut))} ALVE
      </KeyValue>

      <div className="detail-span-full">
        <KeyValue label="Transaction hashes in block">
          {b.transaction_hashes.length ? (
            <div className="hash-list">
              {b.transaction_hashes.map((hash, index) => (
                <div key={`${hash}-${index}`} className="hash-list-row">
                  <span className="muted mono">{index}</span>
                  {onOpenTx ? (
                    <button
                      type="button"
                      className="linkish mono"
                      onClick={() => onOpenTx(hash)}
                      title="Open transaction"
                    >
                      {hash}
                    </button>
                  ) : (
                    <CopyField value={hash} label="tx hash" compact />
                  )}
                  <CopyField value={hash} label="tx hash" compact />
                </div>
              ))}
            </div>
          ) : (
            <span className="muted">No transaction hashes in this window.</span>
          )}
        </KeyValue>
      </div>

      {onOpenExternal ? (
        <div className="detail-span-full detail-actions-row">
          <button className="button" type="button" onClick={onOpenExternal}>
            Open public explorer <ExternalLink size={14} />
          </button>
        </div>
      ) : null}
    </div>
  );
}

export function TransactionDetailBody({
  tx,
  tipHeight,
  onOpenBlock,
  onOpenExternal
}: {
  tx: TxLike | DesktopTransaction;
  tipHeight: number | null;
  onOpenBlock?: (height: number) => void;
  onOpenExternal?: () => void;
}) {
  const t = normalizeTx(tx);
  const blockHash = asStr((t as Record<string, unknown>).block_hash);
  const version = asNum((t as Record<string, unknown>).version);
  const maxFee = asAtomic((t as Record<string, unknown>).max_fee_atomic);
  const baseFee = asAtomic((t as Record<string, unknown>).base_fee_atomic);
  const memo = asStr((t as Record<string, unknown>).memo_hash);
  const pubkey = asStr((t as Record<string, unknown>).sender_public_key_hex);
  const sig = asStr((t as Record<string, unknown>).signature_hex);
  const isCoinbase = !t.from;

  return (
    <div className="detail-grid detail-rich">
      <KeyValue label="Lifecycle">
        <span className="positive">{t.lifecycle_status || "confirmed"}</span>
      </KeyValue>
      <KeyValue label="Confirmations">
        {t.block_height ? confirmations(tipHeight, t.block_height) : "—"}
      </KeyValue>
      <KeyValue label="Type">{isCoinbase ? "Coinbase issuance" : "Signed transfer"}</KeyValue>
      <KeyValue label="Authorization">{t.authorization_state || "—"}</KeyValue>

      <div className="detail-span-full">
        <KeyValue label="Transaction hash">
          <CopyField value={t.hash} label="tx hash" />
        </KeyValue>
      </div>

      <KeyValue label="Block height">
        {onOpenBlock && t.block_height ? (
          <button type="button" className="linkish mono" onClick={() => onOpenBlock(t.block_height)}>
            {t.block_height}
          </button>
        ) : (
          t.block_height || "—"
        )}
      </KeyValue>
      <KeyValue label="Index in block">{t.transaction_index}</KeyValue>
      {blockHash ? (
        <div className="detail-span-full">
          <KeyValue label="Block hash">
            <CopyField value={blockHash} label="block hash" />
          </KeyValue>
        </div>
      ) : null}
      {version ? (
        <KeyValue label="Tx version" mono>
          {version}
        </KeyValue>
      ) : null}
      <KeyValue label="Account nonce" mono>
        {t.nonce}
      </KeyValue>

      <div className="detail-span-full">
        <KeyValue label="From">
          {t.from ? <AddressChip value={t.from} full /> : "Coinbase (protocol reward)"}
        </KeyValue>
      </div>
      <div className="detail-span-full">
        <KeyValue label="To">
          {t.to ? <AddressChip value={t.to} full /> : "—"}
        </KeyValue>
      </div>

      <KeyValue label="Amount" mono>
        {formatAtomic(t.amount_atomic)} ALVE
      </KeyValue>
      <KeyValue label="Effective fee" mono>
        {formatAtomic(t.effective_fee_atomic)} ALVE
      </KeyValue>
      <KeyValue label="Burned fee" mono>
        {formatAtomic(t.burned_fee_atomic)} ALVE
      </KeyValue>
      <KeyValue label="Priority fee" mono>
        {formatAtomic(t.effective_priority_fee_atomic)} ALVE
      </KeyValue>
      {maxFee !== "0" ? (
        <KeyValue label="Max fee (cap)" mono>
          {formatAtomic(maxFee)} ALVE
        </KeyValue>
      ) : null}
      {baseFee !== "0" ? (
        <KeyValue label="Base fee at mine" mono>
          {formatAtomic(baseFee)} ALVE
        </KeyValue>
      ) : null}

      {memo ? (
        <div className="detail-span-full">
          <KeyValue label="Memo hash">
            <CopyField value={memo} label="memo hash" />
          </KeyValue>
        </div>
      ) : null}
      {pubkey ? (
        <div className="detail-span-full">
          <KeyValue label="Sender public key">
            <CopyField value={pubkey} label="public key" />
          </KeyValue>
        </div>
      ) : null}
      {sig ? (
        <div className="detail-span-full">
          <KeyValue label="Signature (hex)">
            <CopyField value={sig} label="signature" />
          </KeyValue>
        </div>
      ) : null}

      <div className="detail-span-full">
        <KeyValue label="Interpretation">
          {isCoinbase
            ? "Protocol coinbase: block reward paid to the miner address under emission rules. Not a user-signed transfer."
            : "User-signed transfer: validated by consensus, included in a block, and indexed. Public key and signature are on-chain material (not wallet secrets)."}
        </KeyValue>
      </div>

      {onOpenExternal ? (
        <div className="detail-span-full detail-actions-row">
          <button className="button" type="button" onClick={onOpenExternal}>
            Open public explorer <ExternalLink size={14} />
          </button>
        </div>
      ) : null}
    </div>
  );
}

export function AddressDetailBody({
  data,
  onOpenTx,
  onOpenBlock
}: {
  data: Record<string, unknown>;
  onOpenTx?: (hash: string) => void;
  onOpenBlock?: (height: number) => void;
}) {
  const address = asStr(data.address);
  const balance = asAtomic(data.balance_atomic);
  const received = asAtomic(data.total_received_atomic);
  const sent = asAtomic(data.total_sent_atomic);
  const mined = asAtomic(data.mined_reward_atomic);
  const exists = Boolean(data.exists_in_ledger ?? data.exists);
  const nextNonce = data.next_nonce != null ? asNum(data.next_nonce) : null;
  const tipHeight = data.tip_height != null ? asNum(data.tip_height) : null;
  const tipHash = asStr(data.tip_hash);
  const txHashes = Array.isArray(data.transaction_hashes)
    ? (data.transaction_hashes as unknown[]).map(asStr).filter(Boolean)
    : [];
  const sentTx = Array.isArray(data.sent_tx_hashes)
    ? (data.sent_tx_hashes as unknown[]).map(asStr).filter(Boolean)
    : [];
  const recvTx = Array.isArray(data.received_tx_hashes)
    ? (data.received_tx_hashes as unknown[]).map(asStr).filter(Boolean)
    : [];
  const minedHeights = Array.isArray(data.mined_block_heights)
    ? (data.mined_block_heights as unknown[]).map(asNum).filter((n) => n > 0)
    : [];
  const poolWorkers = Array.isArray(data.pool_workers)
    ? (data.pool_workers as Record<string, unknown>[])
    : [];

  return (
    <div className="detail-grid detail-rich">
      <KeyValue label="Ledger">
        <span className={exists ? "positive" : "muted"}>
          {exists ? "Known in state" : "No confirmed balance yet"}
        </span>
      </KeyValue>
      <KeyValue label="Balance" mono>
        {formatAtomic(balance)} ALVE
      </KeyValue>
      <KeyValue label="Total received" mono>
        {formatAtomic(received)} ALVE
      </KeyValue>
      <KeyValue label="Total sent" mono>
        {formatAtomic(sent)} ALVE
      </KeyValue>
      <KeyValue label="Mined rewards" mono>
        {formatAtomic(mined)} ALVE
      </KeyValue>
      {nextNonce != null ? (
        <KeyValue label="Next nonce" mono>
          {nextNonce}
        </KeyValue>
      ) : null}
      {tipHeight != null ? <KeyValue label="Tip height">{tipHeight}</KeyValue> : null}
      {tipHash ? (
        <div className="detail-span-full">
          <KeyValue label="Tip hash">
            <CopyField value={tipHash} label="tip hash" />
          </KeyValue>
        </div>
      ) : null}
      <div className="detail-span-full">
        <KeyValue label="Address">
          <AddressChip value={address} full />
        </KeyValue>
      </div>

      {poolWorkers.length ? (
        <div className="detail-span-full">
          <KeyValue label="Pool workers (public pool status)">
            <div className="hash-list">
              {poolWorkers.map((w, i) => (
                <div key={i} className="hash-list-row">
                  <span className="mono">
                    {asStr(w.worker_name)} · {asNum(w.estimated_hashrate_hs)} H/s ·{" "}
                    {w.online ? "online" : "offline"} · shares {asNum(w.accepted_shares)}
                  </span>
                </div>
              ))}
            </div>
          </KeyValue>
        </div>
      ) : null}

      {minedHeights.length ? (
        <div className="detail-span-full">
          <KeyValue label="Mined block heights">
            <div className="chip-row">
              {minedHeights.slice(0, 40).map((h) =>
                onOpenBlock ? (
                  <button key={h} type="button" className="chip-btn" onClick={() => onOpenBlock(h)}>
                    #{h}
                  </button>
                ) : (
                  <span key={h} className="chip-btn">
                    #{h}
                  </span>
                )
              )}
            </div>
          </KeyValue>
        </div>
      ) : null}

      <div className="detail-span-full">
        <KeyValue label={`Related txs (${txHashes.length})`}>
          {txHashes.length ? (
            <div className="hash-list">
              {txHashes.slice(0, 50).map((hash) => (
                <div key={hash} className="hash-list-row">
                  {onOpenTx ? (
                    <button type="button" className="linkish mono" onClick={() => onOpenTx(hash)}>
                      {shortHash(hash, 12)}
                    </button>
                  ) : (
                    <span className="mono">{shortHash(hash, 12)}</span>
                  )}
                  <CopyField value={hash} label="tx hash" compact />
                </div>
              ))}
            </div>
          ) : (
            <span className="muted">No indexed transactions for this address yet.</span>
          )}
        </KeyValue>
      </div>
      {sentTx.length || recvTx.length ? (
        <>
          <KeyValue label="Sent count">{sentTx.length}</KeyValue>
          <KeyValue label="Received count">{recvTx.length}</KeyValue>
        </>
      ) : null}
    </div>
  );
}

export function PeerDetailBody({ data }: { data: Record<string, unknown> }) {
  const peerId = asStr(data.peer_id ?? data.local_peer_id);
  const address = asStr(data.address);
  const multiaddrs = Array.isArray(data.listen_addresses)
    ? (data.listen_addresses as unknown[]).map(asStr).filter(Boolean)
    : address
      ? [address]
      : [];

  return (
    <div className="detail-grid detail-rich">
      <KeyValue label="Handshake">
        <span className={data.handshake_validated ? "positive" : "muted"}>
          {data.handshake_validated ? "Validated" : data.is_local ? "Local node" : "Unvalidated / presence"}
        </span>
      </KeyValue>
      <KeyValue label="Roles">
        {[
          data.validating ? "validating" : null,
          data.mining ? "mining" : null,
          data.is_local ? "local" : null
        ]
          .filter(Boolean)
          .join(", ") || "peer"}
      </KeyValue>
      <KeyValue label="Best height">{data.best_height != null ? asNum(data.best_height) : "—"}</KeyValue>
      <KeyValue label="Hashrate" mono>
        {asNum(data.hashrate_hs)} H/s
      </KeyValue>
      <div className="detail-span-full">
        <KeyValue label="Peer ID">
          <CopyField value={peerId} label="peer id" />
        </KeyValue>
      </div>
      {multiaddrs.map((addr) => (
        <div className="detail-span-full" key={addr}>
          <KeyValue label="Address / multiaddr">
            <CopyField value={addr} label="address" />
          </KeyValue>
        </div>
      ))}
      {asStr(data.last_error) ? (
        <div className="detail-span-full">
          <KeyValue label="Last error">{asStr(data.last_error)}</KeyValue>
        </div>
      ) : null}
      <div className="detail-span-full">
        <KeyValue label="Note">
          Peer identifiers and multiaddrs are public network presence data from the gateway P2P view —
          not wallet secrets.
        </KeyValue>
      </div>
    </div>
  );
}

export function PoolWorkerDetailBody({ data }: { data: Record<string, unknown> }) {
  return (
    <div className="detail-grid detail-rich">
      <KeyValue label="Worker">{asStr(data.worker_name) || "—"}</KeyValue>
      <KeyValue label="Online">
        <span className={data.online ? "positive" : "muted"}>
          {data.online ? "Yes" : "No / stale"}
        </span>
      </KeyValue>
      <KeyValue label="Hashrate" mono>
        {asNum(data.estimated_hashrate_hs)} H/s
      </KeyValue>
      <KeyValue label="Accepted shares">{asNum(data.accepted_shares)}</KeyValue>
      <KeyValue label="Blocks found">{asNum(data.blocks_found)}</KeyValue>
      <KeyValue label="Assigned difficulty">
        {asNum(data.assigned_difficulty_leading_zero_bits)} bits
      </KeyValue>
      <KeyValue label="Last share">
        {asNum(data.last_share_unix_seconds)
          ? formatTimestamp(asNum(data.last_share_unix_seconds))
          : "—"}
      </KeyValue>
      <div className="detail-span-full">
        <KeyValue label="Miner address">
          {asStr(data.miner_address) ? (
            <AddressChip value={asStr(data.miner_address)} full />
          ) : (
            "—"
          )}
        </KeyValue>
      </div>
      {asStr(data.pool_name) ? <KeyValue label="Pool">{asStr(data.pool_name)}</KeyValue> : null}
      {asStr(data.pool_address) ? (
        <div className="detail-span-full">
          <KeyValue label="Pool coinbase address">
            <AddressChip value={asStr(data.pool_address)} full />
          </KeyValue>
        </div>
      ) : null}
      <div className="detail-span-full">
        <KeyValue label="Note">
          Pool worker rows come from the public pool status API (payout address + worker name +
          share stats). No credentials or stratum secrets are shown.
        </KeyValue>
      </div>
    </div>
  );
}
