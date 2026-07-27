import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppSettings,
  DiagnosticsInfo,
  MinerStartOptions,
  NetworkSnapshot,
  OperatorCommand,
  PathInfo,
  PoolCatalog,
  PoolSnapshot,
  PreparedTransaction,
  RpcSettings,
  RuntimeHealth,
  SubmissionResult,
  UpdateState,
  AlvenqisBridge,
  WalletCreateResult,
  WalletMetadata
} from "@shared/types";

function platform(): "windows" | "linux" | "other" {
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes("windows")) return "windows";
  if (ua.includes("linux")) return "linux";
  return "other";
}

export function createAlvenqisBridge(): AlvenqisBridge {
  return {
    network: {
      snapshot: () => invoke<NetworkSnapshot>("network_snapshot"),
      addSeed: (seed) => invoke<string>("network_add_seed", { seed })
    },
    wallet: {
      metadata: () => invoke<WalletMetadata | null>("wallet_metadata"),
      list: () => invoke<WalletMetadata[]>("wallet_list"),
      select: (walletId) => invoke<WalletMetadata>("wallet_select", { walletId }),
      create: (displayName) => invoke<WalletCreateResult>("wallet_create", { displayName }),
      import: (displayName, recoveryPhrase) =>
        invoke<WalletMetadata>("wallet_import", {
          displayName,
          recoveryPhrase: recoveryPhrase ?? null
        }),
      remove: () => invoke<void>("wallet_remove")
    },
    transactions: {
      prepare: (recipient, amount, tip) =>
        invoke<PreparedTransaction>("tx_prepare", { recipient, amount, tip }),
      signAndSubmit: (prepared, confirmed) =>
        invoke<SubmissionResult>("tx_sign_submit", { prepared, confirmed })
    },
    operator: {
      run: (command: OperatorCommand, minerOptions?: MinerStartOptions) =>
        invoke<string>("operator_run", { command, minerOptions: minerOptions ?? null })
    },
    logs: {
      recent: (service, lines) => invoke<string>("logs_recent", { service, lines: lines ?? null }),
      export: (service) => invoke<string | null>("logs_export", { service })
    },
    miner: {
      devices: () => invoke("miner_devices"),
      runCommand: (line) => invoke<string>("miner_run_command", { line })
    },
    explorer: {
      open: (path) => invoke<void>("explorer_open", { path }),
      lookup: (query) => invoke("explorer_lookup", { query })
    },
    pool: {
      snapshot: (poolUrl, minerAddress) =>
        invoke<PoolSnapshot>("pool_snapshot", {
          poolUrl: poolUrl ?? null,
          minerAddress: minerAddress ?? null
        }),
      catalog: () => invoke<PoolCatalog>("pool_catalog")
    },
    settings: {
      rpc: () => invoke<RpcSettings>("settings_rpc"),
      setRpcUrl: (value) => invoke<string>("settings_set_rpc_url", { value }),
      get: () => invoke<AppSettings>("settings_get"),
      update: (patch) => invoke<AppSettings>("settings_update", { patch }),
      reset: () => invoke<AppSettings>("settings_reset"),
      defaults: () => invoke<AppSettings>("settings_defaults"),
      paths: () => invoke<PathInfo>("settings_paths"),
      diagnostics: () => invoke<DiagnosticsInfo>("settings_diagnostics"),
      openPath: (kind) => invoke<void>("settings_open_path", { kind }),
      health: () => invoke<RuntimeHealth>("runtime_health")
    },
    updates: {
      state: () => invoke<UpdateState>("updates_state"),
      check: () => invoke<UpdateState>("updates_check"),
      download: () => invoke<void>("updates_download"),
      install: (restart) => invoke<void>("updates_install", { restart }),
      onState: (listener) => {
        let unlisten: UnlistenFn | undefined;
        void listen<UpdateState>("updates:state", (event) => listener(event.payload)).then((fn) => {
          unlisten = fn;
        });
        return () => {
          unlisten?.();
        };
      }
    },
    app: {
      platform: platform(),
      workspace: () => invoke<string>("app_workspace"),
      minimize: () => invoke<void>("app_minimize"),
      maximize: () => invoke<void>("app_maximize"),
      close: () => invoke<void>("app_close"),
      version: () => invoke<string>("app_version")
    }
  };
}

export function installAlvenqisBridge(): AlvenqisBridge {
  const bridge = createAlvenqisBridge();
  window.alvenqis = bridge;
  return bridge;
}
