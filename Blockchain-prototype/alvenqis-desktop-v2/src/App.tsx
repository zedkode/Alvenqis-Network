import { useCallback, useEffect, useRef, useState, type ComponentType } from "react";
import type { MinerStartOptions, NetworkSnapshot, OperatorCommand, WalletMetadata } from "@shared/types";
import {
  LOCAL_REFRESH_MIN_MS,
  REFRESH_INTERVAL_MS,
  REMOTE_REFRESH_MIN_MS,
  RPC_URL
} from "@shared/constants";
import { Sidebar } from "./components/layout/Sidebar";
import { TitleBar } from "./components/layout/TitleBar";
import { TopBar } from "./components/layout/TopBar";
import { StartupGate } from "./components/startup/StartupGate";
import { useAppSettings } from "./hooks/useAppSettings";
import { useAlvenqisEvents } from "./hooks/useAlvenqisEvents";
import { ToastStack } from "./components/notifications/Toast";
import { CommandPalette } from "./components/CommandPalette";
import { useTheme } from "./shared/theme";
import { AppContext, type PageId } from "./model";
import { Assets } from "./pages/Assets";
import { ActivityLog } from "./pages/ActivityLog";
import { Blocks } from "./pages/Blocks";
import { Explorer } from "./pages/Explorer";
import { Mining } from "./pages/Mining";
import { Mempool } from "./pages/Mempool";
import { Node } from "./pages/Node";
import { Overview } from "./pages/Overview";
import { Analytics } from "./pages/Analytics";
import { Messages } from "./pages/Messages";
import { Pool } from "./pages/Pool";
import { Rewards } from "./pages/Rewards";
import { Send } from "./pages/Send";
import { Settings } from "./pages/Settings";
import { Transactions } from "./pages/Transactions";
import { Wallet } from "./pages/Wallet";
import { useNotificationsOptional } from "./shared/notifications";

function isLocalRpcUrl(url: string): boolean {
  try {
    const host = new URL(url).hostname.toLowerCase();
    return host === "127.0.0.1" || host === "localhost" || host === "::1" || host === "[::1]";
  } catch {
    return false;
  }
}

/** Adaptive telemetry interval for VPS versus local operation. */
function snapshotPollMs(
  settingsMs: number,
  rpcUrl: string,
  failStreak: number,
  degraded: boolean,
  online: boolean
): number {
  const remote = !isLocalRpcUrl(rpcUrl || RPC_URL);
  const floor = remote ? REMOTE_REFRESH_MIN_MS : LOCAL_REFRESH_MIN_MS;
  let ms = Math.max(floor, settingsMs || REFRESH_INTERVAL_MS);
  if (degraded || !online || failStreak > 0) {
    // Exponential backoff while the gateway is throttling / stalling (cap 60s).
    const boost = Math.min(4, Math.max(1, failStreak + (degraded ? 1 : 0)));
    ms = Math.min(60_000, ms * (1 + boost));
    if (remote) ms = Math.max(ms, 15_000);
  }
  return ms;
}

const empty: NetworkSnapshot = {
  online: false,
  degraded: false,
  status_label: "Mainnet Candidate",
  height: null,
  block_count: 0,
  mempool_count: 0,
  mempool_transactions: [],
  mempool_anticipated_base_fee_atomic: "0",
  mempool_total_fees_atomic: "0",
  mempool_total_burned_fees_atomic: "0",
  mempool_total_priority_fees_atomic: "0",
  balance_atomic: null,
  emitted_supply_atomic: null,
  max_supply_atomic: null,
  tip_hash: null,
  indexed_height: null,
  indexed_blocks: 0,
  indexed_transactions: 0,
  indexed_addresses: 0,
  latest_block_timestamp: null,
  latest_block_transactions: 0,
  latest_block_reward_atomic: null,
  latest_block_fees_atomic: null,
  node_running: false,
  rpc_running: false,
  indexer_ready: false,
  miner_running: false,
  miner_hashrate_hs: null,
  miner_height: null,
  miner_accepted_blocks: null,
  miner_accepted_shares: null,
  miner_status: null,
  miner_last_error: null,
  miner_template_id: null,
  miner_difficulty_leading_zero_bits: null,
  miner_share_difficulty_leading_zero_bits: null,
  miner_eta_block_seconds: null,
  miner_eta_share_seconds: null,
  miner_hashes_attempted: null,
  miner_updated_at_unix_seconds: null,
  miner_backend_mode: null,
  miner_active_backend: null,
  miner_gpu_devices: null,
  local_peer_id: null,
  p2p_listen_addresses: [],
  configured_seed_count: 0,
  connected_peer_count: 0,
  validated_peer_count: 0,
  mining_peer_count: 0,
  observed_network_hashrate_hs: 0,
  miners: [],
  validating_peer_count: 0,
  banned_peer_count: 0,
  reputation_enabled: true,
  p2p_syncing: false,
  p2p_error: null,
  recent_blocks: [],
  recent_transactions: [],
  peers: [],
  fleet_nodes: [],
  fleet_registered_nodes: 0,
  fleet_online_nodes: 0,
  pool_online: false,
  pool_name: null,
  pool_workers: 0,
  pool_hashrate_hs: 0,
  pool_blocks_found: 0,
  pool_vardiff_target_seconds: null,
  pool_rejected_requests: 0,
  pool_rate_limited_requests: 0,
  pool_active_bans: 0,
  sync_status: "offline",
  sync_target_height: null,
  sync_remaining_blocks: null,
  sync_progress_percent: null,
  sync_target_peer_count: 0,
  detail: "Waiting for VPS RPC gateway"
};

const pages: Record<PageId, ComponentType> = {
  overview: Overview,
  analytics: Analytics,
  wallet: Wallet,
  send: Send,
  mining: Mining,
  pool: Pool,
  explorer: Explorer,
  blocks: Blocks,
  transactions: Transactions,
  mempool: Mempool,
  node: Node,
  activity: ActivityLog,
  messages: Messages,
  rewards: Rewards,
  assets: Assets,
  settings: Settings
};

const pageIds = new Set<string>(Object.keys(pages));

export default function App() {
  const { settings, update: updateSettings } = useAppSettings();
  const { toggleTheme, theme } = useTheme();
  const notifications = useNotificationsOptional();
  const unreadMessages = notifications?.items.filter((i) => !i.read).length ?? 0;
  const [page, setPage] = useState<PageId>("overview");
  const [snapshot, setSnapshot] = useState(empty);
  const [wallet, setWallet] = useState<WalletMetadata | null>(null);
  const [wallets, setWallets] = useState<WalletMetadata[]>([]);
  const [busy, setBusy] = useState(false);
  const [startupComplete, setStartupComplete] = useState(false);
  const [startupError, setStartupError] = useState<string | null>(null);
  const [notice, setNotice] = useState<{ error: boolean; text: string } | null>(null);
  const [defaultPageApplied, setDefaultPageApplied] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);

  useEffect(() => {
    if (defaultPageApplied) return;
    if (pageIds.has(settings.default_page)) {
      setPage(settings.default_page as PageId);
      setDefaultPageApplied(true);
    }
  }, [settings.default_page, defaultPageApplied]);

  const reloadWallet = useCallback(async () => {
    const [active, available] = await Promise.all([
      window.alvenqis.wallet.metadata(),
      window.alvenqis.wallet.list()
    ]);
    setWallet(active);
    setWallets(available);
  }, []);

  const selectWallet = useCallback(async (walletId: string) => {
    setBusy(true);
    setStartupError(null);
    try {
      setWallet(await window.alvenqis.wallet.select(walletId));
      await reloadWallet();
    } catch (error) {
      setStartupError(String(error));
    } finally {
      setBusy(false);
    }
  }, [reloadWallet]);

  const pollFailStreak = useRef(0);
  const [pollTick, setPollTick] = useState(0);

  // Background snapshot polls must never flip global busy — that froze the UI
  // at zeros while the miner FiroPoW DAG burned the CPU.
  const refresh = useCallback(async (opts?: { busy?: boolean }) => {
    const showBusy = opts?.busy === true;
    if (showBusy) setBusy(true);
    try {
      const next = await window.alvenqis.network.snapshot();
      setSnapshot(next);
      const prevStreak = pollFailStreak.current;
      if (next.online && !next.degraded) {
        pollFailStreak.current = 0;
      } else if (next.degraded || !next.online) {
        pollFailStreak.current = Math.min(8, pollFailStreak.current + 1);
        // Degraded / brief throttle is not a hard error toast every few seconds.
        if (!next.online && pollFailStreak.current >= 3) {
          setNotice({
            error: true,
            text: next.detail || "RPC gateway unreachable"
          });
        } else if (next.degraded && pollFailStreak.current === 1) {
          setNotice({
            error: false,
            text: next.detail || "RPC degraded — using last-known data; slowing poll"
          });
        }
      }
      if (prevStreak !== pollFailStreak.current) {
        setPollTick((t) => t + 1);
      }
    } catch (error) {
      pollFailStreak.current = Math.min(8, pollFailStreak.current + 1);
      setPollTick((t) => t + 1);
      setNotice({ error: true, text: String(error) });
    } finally {
      if (showBusy) setBusy(false);
    }
  }, []);

  const operator = useCallback(async (command: OperatorCommand, minerOptions?: MinerStartOptions) => {
    const destructive = command === "stop" || command === "restart" || command === "miner-stop";
    if (settings.confirm_before_operator && destructive) {
      const ok = window.confirm(`Run operator command "${command}"? Managed Alvenqis services may restart or stop.`);
      if (!ok) return "Operator command cancelled.";
    }
    setBusy(true);
    try {
      return await window.alvenqis.operator.run(command, minerOptions);
    } finally {
      setBusy(false);
    }
  }, [settings.confirm_before_operator]);

  useEffect(() => {
    void reloadWallet()
      .then(() => refresh({ busy: true }))
      .catch((error) => setStartupError(String(error)));
  }, [refresh, reloadWallet]);

  // Adaptive poll: slower on VPS and while degraded.
  useEffect(() => {
    const interval = snapshotPollMs(
      settings.refresh_interval_ms,
      settings.rpc_url,
      pollFailStreak.current,
      Boolean(snapshot.degraded),
      snapshot.online
    );
    let inFlight = false;
    const timer = window.setInterval(() => {
      if (inFlight) return;
      inFlight = true;
      void refresh()
        .catch(() => undefined)
        .finally(() => {
          inFlight = false;
        });
    }, interval);
    return () => window.clearInterval(timer);
  }, [
    refresh,
    settings.refresh_interval_ms,
    settings.rpc_url,
    snapshot.degraded,
    snapshot.online,
    pollTick
  ]);

  useAlvenqisEvents({
    minerWasRunning: snapshot.miner_running,
    online: snapshot.online || snapshot.rpc_running
  });

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const meta = event.ctrlKey || event.metaKey;
      if (meta && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setPaletteOpen(true);
        return;
      }
      if (meta && event.key.toLowerCase() === "r" && !event.shiftKey) {
        // Allow native refresh only when not focusing an input
        const tag = (event.target as HTMLElement | null)?.tagName;
        if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
        event.preventDefault();
        void refresh();
      }
      if (meta && event.key >= "1" && event.key <= "9") {
        const order: PageId[] = [
          "overview",
          "analytics",
          "wallet",
          "send",
          "mining",
          "explorer",
          "messages",
          "node",
          "settings"
        ];
        const next = order[Number(event.key) - 1];
        if (next) {
          event.preventDefault();
          setPage(next);
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [refresh]);

  const Page = pages[page];
  const noticeClass = notice?.error ? "notice error" : "notice";
  const rpcLabel = snapshot.rpc_running ? "ONLINE" : "OFFLINE";
  const detailClass = snapshot.online ? "positive" : "negative";

  const createWallet = async (displayName: string) => {
    setBusy(true);
    setStartupError(null);
    try {
      const created = await window.alvenqis.wallet.create(displayName);
      await reloadWallet();
      const phrase = (created.recovery_phrase ?? "").trim();
      if (!phrase) return null;
      return {
        phrase,
        address: created.metadata.address,
        name: created.metadata.display_name
      };
    } catch (error) {
      setStartupError(String(error));
      return null;
    } finally {
      setBusy(false);
    }
  };

  const importWallet = async (displayName: string, recoveryPhrase: string) => {
    setBusy(true);
    setStartupError(null);
    try {
      await window.alvenqis.wallet.import(displayName, recoveryPhrase);
      await reloadWallet();
    } catch (error) {
      setStartupError(String(error));
      throw error;
    } finally {
      setBusy(false);
    }
  };

  const startServices = async () => {
    try {
      await operator("start");
      await refresh();
    } catch (error) {
      setStartupError(String(error));
    }
  };

  const addSeed = async (seed: string) => {
    try {
      await window.alvenqis.network.addSeed(seed);
      await operator("restart");
      await refresh();
    } catch (error) {
      setStartupError(String(error));
    }
  };

  return (
    <AppContext.Provider
      value={{
        page,
        setPage,
        snapshot,
        wallet,
        wallets,
        busy,
        notice,
        refresh,
        operator,
        setNotice,
        reloadWallet,
        selectWallet
      }}
    >
      {!startupComplete ? (
        <>
          <TitleBar />
          <StartupGate
            snapshot={snapshot}
            wallets={wallets}
            activeWallet={wallet}
            busy={busy}
            error={startupError}
            onSelect={selectWallet}
            onCreate={createWallet}
            onImport={importWallet}
            onStartServices={startServices}
            onAddSeed={addSeed}
            onRefresh={refresh}
            onContinue={() => setStartupComplete(true)}
          />
        </>
      ) : (
        <div className="app-shell">
          <TitleBar />
          <div className="workspace">
            <Sidebar
              page={page}
              setPage={setPage}
              nodeRunning={snapshot.node_running}
              height={snapshot.height}
              online={snapshot.online || snapshot.rpc_running}
              unreadMessages={unreadMessages}
              peers={snapshot.connected_peer_count}
              mempool={snapshot.mempool_count}
            />
            <main className="main">
              <TopBar
                page={page}
                snapshot={snapshot}
                wallet={wallet}
                busy={busy}
                refresh={() => {
                  void refresh();
                }}
                refreshMs={settings.refresh_interval_ms}
                onOpenPalette={() => setPaletteOpen(true)}
              />
              {notice ? <div className={noticeClass}>{notice.text}</div> : null}
              <Page />
            </main>
          </div>
          <footer className="footer">
            <span>
              RPC <b>{rpcLabel}</b>
            </span>
            <span>
              PEERS <b>{snapshot.connected_peer_count}</b>
            </span>
            <span>
              MEMPOOL <b>{snapshot.mempool_count}</b>
            </span>
            <span>
              HEIGHT <b>{snapshot.height ?? "—"}</b>
            </span>
            <span>
              INDEX <b>{snapshot.indexed_height ?? "—"}</b>
            </span>
            <span>
              MINER{" "}
              <b className={snapshot.miner_running ? "positive" : undefined}>
                {snapshot.miner_running ? "ON" : "OFF"}
              </b>
            </span>
            <span>
              MSG <b>{unreadMessages}</b>
            </span>
            <span className="muted" title="Ctrl+K">
              ⌘K
            </span>
            <span className={detailClass}>{snapshot.detail}</span>
            <span className="footer-brand">ALVENQIS V2</span>
          </footer>
          <ToastStack />
          <CommandPalette
            open={paletteOpen}
            language={settings.language}
            onClose={() => setPaletteOpen(false)}
            onNavigate={setPage}
            onRefresh={() => {
              void refresh();
            }}
            onToggleTheme={() => {
              toggleTheme();
              void updateSettings({ theme: theme === "dark" ? "light" : "dark" });
            }}
            onOpenWalletSwitcher={() => setPage("wallet")}
          />
        </div>
      )}
    </AppContext.Provider>
  );
}
