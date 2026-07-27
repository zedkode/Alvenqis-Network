import { describe, expect, it } from "vitest";
import type { NetworkSnapshot } from "@shared/types";
import { startupAccessMode } from "./startupPolicy";

function snapshot(overrides: Partial<NetworkSnapshot>): NetworkSnapshot {
  return {
    online: false, status_label: "Mainnet Candidate", height: null, block_count: 0,
    mempool_count: 0, mempool_transactions: [], mempool_anticipated_base_fee_atomic: "0",
    mempool_total_fees_atomic: "0", mempool_total_burned_fees_atomic: "0",
    mempool_total_priority_fees_atomic: "0", balance_atomic: null, emitted_supply_atomic: null,
    max_supply_atomic: null, tip_hash: null, indexed_height: null, indexed_blocks: 0,
    indexed_transactions: 0, indexed_addresses: 0, latest_block_timestamp: null,
    latest_block_transactions: 0, latest_block_reward_atomic: null, latest_block_fees_atomic: null,
    node_running: false, rpc_running: false, indexer_ready: false, miner_running: false,
    miner_hashrate_hs: null, miner_height: null, miner_accepted_blocks: null, miner_accepted_shares: null,
    miner_share_difficulty_leading_zero_bits: null, miner_eta_block_seconds: null, miner_eta_share_seconds: null,
    miner_status: null, miner_template_id: null, miner_difficulty_leading_zero_bits: null,
    miner_hashes_attempted: null, miner_updated_at_unix_seconds: null,
    miner_backend_mode: null, miner_active_backend: null, local_peer_id: null,
    p2p_listen_addresses: [], configured_seed_count: 0, connected_peer_count: 0,
    validated_peer_count: 0, mining_peer_count: 0, observed_network_hashrate_hs: 0, miners: [],
    validating_peer_count: 0, p2p_syncing: false,
    p2p_error: null, sync_status: "offline", sync_target_height: null,
    sync_remaining_blocks: null, sync_progress_percent: null, sync_target_peer_count: 0,
    recent_blocks: [], recent_transactions: [], peers: [], detail: "offline", ...overrides
  };
}

describe("startup access policy", () => {
  it("allows a fully synchronized peer-backed node", () => {
    expect(startupAccessMode(snapshot({ online: true, node_running: true, rpc_running: true, height: 231, block_count: 232, sync_status: "synced" }))).toBe("network-synced");
  });

  it("allows an explicit isolated session for a valid local chain", () => {
    expect(
      startupAccessMode(
        snapshot({
          online: true,
          node_running: true,
          rpc_running: true,
          height: 231,
          block_count: 232,
          sync_status: "discovering",
          detail: "local stack ready"
        })
      )
    ).toBe("local-isolated");
  });

  it("treats VPS gateway chain visibility as gateway-ready while peers discover", () => {
    expect(
      startupAccessMode(
        snapshot({
          online: true,
          node_running: true,
          rpc_running: true,
          height: 650,
          block_count: 651,
          sync_status: "discovering",
          detail: "VPS gateway RPC verified at https://rpcnode.dohotstudio.com"
        })
      )
    ).toBe("gateway-ready");
  });

  it("keeps the panel blocked when local services are incomplete", () => {
    expect(startupAccessMode(snapshot({ online: true, node_running: true, height: 231, block_count: 232 }))).toBe("blocked");
  });

  it("rejects a stale synchronized status when local services are offline", () => {
    expect(startupAccessMode(snapshot({ sync_status: "synced" }))).toBe("blocked");
  });
});
