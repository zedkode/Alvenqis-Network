import { useCallback, useEffect, useMemo, useState, type ComponentType } from "react";
import {
  Bell, Boxes, Cpu, FolderOpen, HardDrive, Info, KeyRound, Monitor, Network,
  Plug, RefreshCw, RotateCcw, Shield, ShieldAlert, SlidersHorizontal, Trash2, Wallet
} from "lucide-react";
import { AlvenqisLogo } from "../components/brand/AlvenqisLogo";
import type {
  AppSettings, DiagnosticsInfo, LanguageId, PathInfo, ThemeId, DensityId, AccentId
} from "@shared/types";
import { NETWORK_ID } from "@shared/constants";
import { AddressChip } from "../components/ui/AddressChip";
import { EmptyState } from "../components/ui/EmptyState";
import { KeyValue, Panel } from "../components/ui/Panel";
import { ConfirmDialog } from "../components/dialogs/ConfirmDialog";
import { RecoveryPhraseImport } from "../components/wallet/RecoveryPhraseImport";
import { RecoveryPhraseReveal } from "../components/wallet/RecoveryPhraseReveal";
import { useApp } from "../model";
import { useAppSettings } from "../hooks/useAppSettings";

type SectionId =
  | "general"
  | "appearance"
  | "network"
  | "mining"
  | "wallet"
  | "notifications"
  | "data"
  | "services"
  | "privacy"
  | "advanced"
  | "about"
  | "danger";

const sections: Array<{ id: SectionId; label: string; icon: typeof Monitor; hint: string }> = [
  { id: "general", label: "General", icon: SlidersHorizontal, hint: "Language, cadence, startup" },
  { id: "appearance", label: "Appearance", icon: Monitor, hint: "Theme, density, accent, motion (V2)" },
  { id: "network", label: "Network", icon: Network, hint: "RPC endpoint and identity" },
  { id: "mining", label: "Mining defaults", icon: Cpu, hint: "CUDA, solo, pool and Stratum" },
  { id: "wallet", label: "Wallet & security", icon: KeyRound, hint: "Identity, vault, recovery phrase" },
  { id: "notifications", label: "Notifications", icon: Bell, hint: "Blocks, sound, updates, message center" },
  { id: "data", label: "Data & paths", icon: HardDrive, hint: "Workspace and chain root" },
  { id: "services", label: "Services", icon: Boxes, hint: "Miner process diagnostics" },
  { id: "privacy", label: "Privacy", icon: Shield, hint: "Balances and address masking" },
  { id: "advanced", label: "Advanced", icon: FolderOpen, hint: "Operator confirmations" },
  { id: "about", label: "About", icon: Info, hint: "Build and platform" },
  { id: "danger", label: "Danger zone", icon: Trash2, hint: "Disconnect wallet" }
];

function Toggle({
  checked, onChange, label, description, disabled
}: {
  checked: boolean;
  onChange(next: boolean): void;
  label: string;
  description?: string;
  disabled?: boolean;
}) {
  return (
    <label className={`settings-toggle ${disabled ? "is-disabled" : ""}`}>
      <span>
        <b>{label}</b>
        {description && <small>{description}</small>}
      </span>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        disabled={disabled}
        className={`switch ${checked ? "on" : ""}`}
        onClick={() => onChange(!checked)}
      >
        <i />
      </button>
    </label>
  );
}

function Segmented<T extends string>({
  value, options, onChange
}: {
  value: T;
  options: Array<{ id: T; label: string; detail?: string; icon?: ComponentType<{ size?: number }> }>;
  onChange(next: T): void;
}) {
  return (
    <div className="settings-segmented" role="radiogroup">
      {options.map((option) => {
        const Icon = option.icon;
        const active = value === option.id;
        return (
          <button
            key={option.id}
            type="button"
            role="radio"
            aria-checked={active}
            className={active ? "active" : ""}
            onClick={() => onChange(option.id)}
          >
            <strong>
              {Icon && <Icon size={13} />}
              {option.label}
            </strong>
            {option.detail && <small>{option.detail}</small>}
          </button>
        );
      })}
    </div>
  );
}

function formatBytes(value: number): string {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${(value / (1024 * 1024)).toFixed(2)} MB`;
}

export function Settings() {
  const { wallet, reloadWallet, setNotice, snapshot, operator, refresh } = useApp();
  const { settings, loading, update, reset } = useAppSettings();
  const [section, setSection] = useState<SectionId>("general");
  const [paths, setPaths] = useState<PathInfo | null>(null);
  const [diagnostics, setDiagnostics] = useState<DiagnosticsInfo | null>(null);
  const [rpcDraft, setRpcDraft] = useState(settings.rpc_url);
  const [miningRpcDraft, setMiningRpcDraft] = useState(settings.mining_rpc_url);
  const [poolDraft, setPoolDraft] = useState(settings.default_pool_url);
  const [workerDraft, setWorkerDraft] = useState(settings.default_worker_name);
  const [backendDraft, setBackendDraft] = useState(settings.default_miner_backend || "cuda");
  const [gpuIntensityDraft, setGpuIntensityDraft] = useState(
    String(settings.default_gpu_intensity || 75)
  );
  const [gpuBatchSizeDraft, setGpuBatchSizeDraft] = useState(
    String(settings.default_gpu_batch_size ?? 0)
  );
  const [templateRefreshDraft, setTemplateRefreshDraft] = useState(
    String(settings.default_template_refresh_seconds ?? 3)
  );
  const [statusIntervalDraft, setStatusIntervalDraft] = useState(
    String(settings.default_status_interval_seconds ?? 2)
  );
  const [stratumHostDraft, setStratumHostDraft] = useState(settings.stratum_host ?? "");
  const [stratumPortDraft, setStratumPortDraft] = useState(
    String(settings.stratum_port ?? 3333)
  );
  const [stratumUseTlsDraft, setStratumUseTlsDraft] = useState(settings.stratum_use_tls ?? true);
  const [stratumSkipTlsVerifyDraft, setStratumSkipTlsVerifyDraft] = useState(
    settings.stratum_skip_tls_verify ?? false
  );
  const [stratumPasswordDraft, setStratumPasswordDraft] = useState(settings.stratum_password ?? "");
  const [stratumTimeoutDraft, setStratumTimeoutDraft] = useState(
    String(settings.stratum_timeout_seconds ?? 20)
  );
  const [disconnectOpen, setDisconnectOpen] = useState(false);
  const [disconnectBusy, setDisconnectBusy] = useState(false);
  const [recoveryImportOpen, setRecoveryImportOpen] = useState(false);
  const [recoveryBusy, setRecoveryBusy] = useState(false);
  const [recoveryWalletName, setRecoveryWalletName] = useState("Recovered wallet");
  const [resetConfirmed, setResetConfirmed] = useState(false);
  const [saving, setSaving] = useState(false);
  const [version, setVersion] = useState("…");

  useEffect(() => {
    setRpcDraft(settings.rpc_url);
    setMiningRpcDraft(settings.mining_rpc_url);
    setPoolDraft(settings.default_pool_url);
    setWorkerDraft(settings.default_worker_name);
    setBackendDraft(settings.default_miner_backend || "cuda");
    setGpuIntensityDraft(String(settings.default_gpu_intensity || 75));
    setGpuBatchSizeDraft(String(settings.default_gpu_batch_size ?? 0));
    setTemplateRefreshDraft(String(settings.default_template_refresh_seconds ?? 3));
    setStatusIntervalDraft(String(settings.default_status_interval_seconds ?? 2));
    setStratumHostDraft(settings.stratum_host ?? "");
    setStratumPortDraft(String(settings.stratum_port ?? 3333));
    setStratumUseTlsDraft(settings.stratum_use_tls ?? true);
    setStratumSkipTlsVerifyDraft(settings.stratum_skip_tls_verify ?? false);
    setStratumPasswordDraft(settings.stratum_password ?? "");
    setStratumTimeoutDraft(String(settings.stratum_timeout_seconds ?? 20));
  }, [settings]);

  const loadMeta = useCallback(async () => {
    const [pathInfo, diag, appVersion] = await Promise.all([
      window.alvenqis.settings.paths(),
      window.alvenqis.settings.diagnostics(),
      window.alvenqis.app.version()
    ]);
    setPaths(pathInfo);
    setDiagnostics(diag);
    setVersion(appVersion);
  }, []);

  useEffect(() => {
    void loadMeta().catch((error) => setNotice({ error: true, text: String(error) }));
  }, [loadMeta, setNotice]);

  const patch = async (next: Partial<AppSettings>, okMessage?: string) => {
    setSaving(true);
    try {
      await update(next);
      if (okMessage) setNotice({ error: false, text: okMessage });
    } catch (error) {
      setNotice({ error: true, text: String(error instanceof Error ? error.message : error) });
    } finally {
      setSaving(false);
    }
  };

  const saveRpc = async () => {
    setSaving(true);
    try {
      const applied = await window.alvenqis.settings.setRpcUrl(rpcDraft);
      await update({ rpc_url: applied });
      setRpcDraft(applied);
      setNotice({ error: false, text: `RPC endpoint set to ${applied}. New requests use it immediately.` });
      await refresh();
    } catch (error) {
      setNotice({ error: true, text: String(error instanceof Error ? error.message : error) });
    } finally {
      setSaving(false);
    }
  };

  const saveMiningDefaults = async () => {
    const intensity = Math.max(1, Math.min(100, Number(gpuIntensityDraft) || 75));
    const gpuBatchSize = Math.max(0, Math.min(131072, Number(gpuBatchSizeDraft) || 0));
    const templateRefresh = Math.max(1, Math.min(60, Number(templateRefreshDraft) || 3));
    const statusInterval = Math.max(1, Math.min(60, Number(statusIntervalDraft) || 2));
    const backend = "cuda";
    const pool = poolDraft.trim().replace(/\/$/, "");
    const urls = Array.from(
      new Set([pool, ...(settings.pool_urls ?? [])].filter(Boolean))
    );
    const stratumPort = Math.max(1, Math.min(65535, Number(stratumPortDraft) || 3333));
    const stratumTimeout = Math.max(5, Math.min(120, Number(stratumTimeoutDraft) || 20));
    await patch({
      default_miner_backend: backend,
      default_gpu_intensity: intensity,
      default_gpu_batch_size: gpuBatchSize,
      default_template_refresh_seconds: templateRefresh,
      default_status_interval_seconds: statusInterval,
      mining_rpc_url: miningRpcDraft.trim().replace(/\/$/, ""),
      default_pool_url: pool,
      pool_urls: urls,
      default_worker_name: workerDraft.trim() || "desktop-01",
      stratum_host: stratumHostDraft.trim(),
      stratum_port: stratumPort,
      stratum_use_tls: stratumUseTlsDraft,
      stratum_skip_tls_verify: stratumSkipTlsVerifyDraft,
      stratum_password: stratumPasswordDraft,
      stratum_timeout_seconds: stratumTimeout
    }, "Mining defaults saved for the next miner start.");
  };

  const removeWallet = async () => {
    setDisconnectBusy(true);
    try {
      await window.alvenqis.wallet.remove();
      await reloadWallet();
      setDisconnectOpen(false);
      setNotice({
        error: false,
        text: "Wallet metadata and operating-system credential entry removed from this user."
      });
    } catch (error) {
      setNotice({ error: true, text: String(error) });
    } finally {
      setDisconnectBusy(false);
    }
  };

  const [revealWords, setRevealWords] = useState<string[] | null>(null);
  const [revealMeta, setRevealMeta] = useState<{ name: string; address: string } | null>(null);

  const importRecoveryPhrase = async (walletName: string, recoveryPhrase: string) => {
    setRecoveryBusy(true);
    try {
      await window.alvenqis.wallet.import(walletName, recoveryPhrase);
      await reloadWallet();
      setRecoveryImportOpen(false);
      setNotice({
        error: false,
        text: "Wallet recovered. Keys are in the OS credential vault."
      });
    } catch (error) {
      setNotice({ error: true, text: String(error) });
      throw error;
    } finally {
      setRecoveryBusy(false);
    }
  };

  /** Create wallet and show the 24-word phrase once in-app (hide/reveal). */
  const createWithRecoveryPhrase = async () => {
    const name = recoveryWalletName.trim();
    if (!name) {
      setNotice({ error: true, text: "Enter a wallet name before creating a recovery wallet." });
      return;
    }
    setRecoveryBusy(true);
    try {
      const created = await window.alvenqis.wallet.create(name);
      await reloadWallet();
      const words = (created.recovery_phrase ?? "").trim().split(/\s+/).filter(Boolean);
      if (words.length) {
        setRevealWords(words);
        setRevealMeta({ name: created.metadata.display_name, address: created.metadata.address });
      }
      setNotice({
        error: false,
        text: "Wallet created. Back up the 24-word recovery phrase shown in the app — it will not be shown again."
      });
    } catch (error) {
      setNotice({ error: true, text: String(error) });
    } finally {
      setRecoveryBusy(false);
    }
  };

  const resetAll = async () => {
    setSaving(true);
    try {
      await reset();
      setResetConfirmed(false);
      setNotice({ error: false, text: "All Control Center preferences restored to defaults." });
      await loadMeta();
    } catch (error) {
      setNotice({ error: true, text: String(error) });
    } finally {
      setSaving(false);
    }
  };

  const openPath = async (kind: "workspace" | "local_root" | "logs" | "user_data" | "settings_file") => {
    try {
      await window.alvenqis.settings.openPath(kind);
    } catch (error) {
      setNotice({ error: true, text: String(error) });
    }
  };

  const runService = async (command: "start" | "stop" | "restart" | "status") => {
    try {
      const output = await operator(command);
      setNotice({ error: false, text: output.slice(0, 280) || `Operator ${command} completed.` });
      await refresh();
      await loadMeta();
    } catch (error) {
      setNotice({ error: true, text: String(error) });
    }
  };

  const activeSection = useMemo(
    () => sections.find((item) => item.id === section) ?? sections[0],
    [section]
  );

  return (
    <div className="page grid settings-root">
      <section className="page-hero settings-hero">
        <div className="page-hero-copy">
          <span className="hero-kicker">
            <i /> PREFERENCES · V2 · THIS OS USER
          </span>
          <h2>
            Control <b>settings</b>
          </h2>
          <p>
            RPC, mining (pool &amp; Stratum defaults), appearance, privacy, notifications and
            diagnostics. Stored locally for this operating-system user only.
          </p>
        </div>
        <div className="page-hero-side">
          <div className="page-hero-metric">
            <small>App version</small>
            <strong>{version}</strong>
          </div>
          <div className="page-hero-metric">
            <small>RPC</small>
            <strong className="mono" style={{ fontSize: 11 }}>
              {settings.rpc_url.replace(/^https?:\/\//, "").slice(0, 28)}
            </strong>
          </div>
          <div className="page-hero-metric">
            <small>Section</small>
            <strong>{activeSection.label}</strong>
          </div>
        </div>
      </section>

    <div className="settings-page">
      <aside className="settings-nav" aria-label="Settings sections">
        <div className="settings-nav-head">
          <span className="eyebrow">Sections</span>
          <strong>Settings</strong>
          <p className="muted">12 control surfaces · local vault preferences</p>
        </div>
        <nav>
          {sections.map(({ id, label, icon: Icon, hint }) => (
            <button
              key={id}
              type="button"
              className={`settings-nav-item ${section === id ? "active" : ""} ${id === "danger" ? "danger" : ""}`}
              onClick={() => setSection(id)}
            >
              <Icon size={16} />
              <span>
                <b>{label}</b>
                <small>{hint}</small>
              </span>
            </button>
          ))}
        </nav>
      </aside>

      <div className="settings-content">
        <header className="settings-content-head">
          <div>
            <div className="eyebrow">{activeSection.hint}</div>
            <h2>{activeSection.label}</h2>
          </div>
          <div className="button-row">
            <button className="button" type="button" disabled={loading || saving} onClick={() => void loadMeta()}>
              <RefreshCw size={14} /> Refresh diagnostics
            </button>
          </div>
        </header>

        {section === "general" && (
          <div className="grid settings-stack">
            <Panel title="Language" detail="UI copy language">
              <Segmented<LanguageId>
                value={settings.language}
                onChange={(language) => void patch({ language })}
                options={[
                  { id: "en", label: "English", detail: "Default product language" },
                  { id: "ro", label: "Română", detail: "Romanian labels (progressive)" }
                ]}
              />
            </Panel>
            <Panel title="Refresh cadence" detail="Live data polling (VPS-safe defaults)">
              <div className="field">
                <label htmlFor="refresh-ms">Network snapshot interval (ms)</label>
                <input
                  id="refresh-ms"
                  type="number"
                  min={3000}
                  max={60000}
                  step={1000}
                  value={settings.refresh_interval_ms}
                  onChange={(event) => void patch({ refresh_interval_ms: Number(event.target.value) || 12000 })}
                />
              </div>
              <div className="field">
                <label htmlFor="log-ms">Service log tail interval (ms)</label>
                <input
                  id="log-ms"
                  type="number"
                  min={2000}
                  max={30000}
                  step={500}
                  value={settings.live_log_interval_ms}
                  onChange={(event) => void patch({ live_log_interval_ms: Number(event.target.value) || 5000 })}
                />
              </div>
              <p className="muted">
                Remote VPS RPC is clamped to at least 10s and backs off on 429/503/504. Prefer 12–15s
                for public gateways; lower values only on local loopback.
              </p>
            </Panel>
            <Panel title="Startup behavior" detail="VPS gateway session">
              <Toggle
                checked={settings.auto_start_services}
                onChange={(auto_start_services) => void patch({ auto_start_services })}
                label="Legacy local stack prompts"
                description="Ignored in VPS mode. Control Center uses the remote RPC gateway; node/RPC are not started on this PC."
              />
              <Toggle
                checked={settings.start_minimized}
                onChange={(start_minimized) => void patch({ start_minimized })}
                label="Remember start minimized preference"
                description="Reserved for tray support; stored now for future builds."
              />
              <div className="field">
                <label htmlFor="default-page">Default landing page</label>
                <select
                  id="default-page"
                  value={settings.default_page}
                  onChange={(event) => void patch({ default_page: event.target.value })}
                >
                  {["overview", "wallet", "mining", "node", "settings"].map((page) => (
                    <option key={page} value={page}>{page}</option>
                  ))}
                </select>
              </div>
            </Panel>
          </div>
        )}

        {section === "appearance" && (
          <div className="grid settings-stack">
            <Panel title="Theme" detail="Dark / light shell + variants">
              <Segmented<ThemeId>
                value={
                  settings.theme === "alvenqis-dark"
                    ? "dark"
                    : settings.theme === "alvenqis-midnight"
                      ? "midnight"
                      : settings.theme
                }
                onChange={(theme) => void patch({ theme }, "Theme applied.")}
                options={[
                  { id: "dark", label: "Dark", detail: "Signature cyan control surface" },
                  { id: "light", label: "Light", detail: "Bright panels, AA accents" },
                  { id: "midnight", label: "Midnight", detail: "Deeper base, softer panels" },
                  { id: "high-contrast", label: "High contrast", detail: "Stronger borders and text" }
                ]}
              />
            </Panel>
            <Panel title="Density & accent" detail="Layout comfort">
              <Segmented<DensityId>
                value={settings.density}
                onChange={(density) => void patch({ density })}
                options={[
                  { id: "comfortable", label: "Comfortable", detail: "Default spacing" },
                  { id: "compact", label: "Compact", detail: "More data density" }
                ]}
              />
              <div style={{ height: 12 }} />
              <Segmented<AccentId>
                value={settings.accent}
                onChange={(accent) => void patch({ accent })}
                options={[
                  { id: "cyan", label: "Cyan", detail: "Primary Alvenqis accent" },
                  { id: "gold", label: "Gold", detail: "Reward-forward accent" },
                  { id: "emerald", label: "Emerald", detail: "Status-forward accent" }
                ]}
              />
            </Panel>
            <Panel title="Motion" detail="Accessibility">
              <Toggle
                checked={settings.reduce_motion}
                onChange={(reduce_motion) => void patch({ reduce_motion })}
                label="Reduce motion"
                description="Disable decorative animations and reactor sweeps."
              />
              <Toggle
                checked={settings.show_technical_labels}
                onChange={(show_technical_labels) => void patch({ show_technical_labels })}
                label="Show technical labels"
                description="Display network IDs, derivation paths and peer multiaddrs."
              />
            </Panel>
          </div>
        )}

        {section === "network" && (
          <div className="grid settings-stack">
            <Panel title="VPS RPC gateway" detail="Source of truth for chain, mining templates and balances">
              <KeyValue label="Active network">Mainnet Candidate</KeyValue>
              <KeyValue label="Network ID" mono>{NETWORK_ID}</KeyValue>
              <KeyValue label="Mode">Remote VPS · no local chain stack</KeyValue>
              <KeyValue label="Live RPC status">
                <span className={snapshot.rpc_running ? "positive" : "negative"}>
                  {snapshot.rpc_running ? "ONLINE" : "OFFLINE"}
                </span>
              </KeyValue>
              <KeyValue label="Gateway height">{snapshot.height ?? "—"}</KeyValue>
              <div className="field">
                <label htmlFor="rpc-endpoint">RPC base URL (https://rpcnode.example.com)</label>
                <input
                  id="rpc-endpoint"
                  value={rpcDraft}
                  spellCheck={false}
                  placeholder={settings.rpc_url}
                  onChange={(event) => setRpcDraft(event.target.value)}
                />
              </div>
              <div className="button-row">
                <button
                  className="button primary"
                  disabled={saving || !rpcDraft.trim() || rpcDraft === settings.rpc_url}
                  onClick={() => void saveRpc()}
                >
                  Apply RPC endpoint
                </button>
                <button
                  className="button"
                  disabled={saving || settings.rpc_url === "https://rpcnode.dohotstudio.com"}
                  onClick={() => {
                    setRpcDraft("https://rpcnode.dohotstudio.com");
                    void window.alvenqis.settings.setRpcUrl("https://rpcnode.dohotstudio.com").then(async (applied) => {
                      await update({ rpc_url: applied });
                      setNotice({ error: false, text: `RPC reset to ${applied}` });
                      await refresh();
                    });
                  }}
                >
                  Reset to public candidate RPC
                </button>
                <button
                  className="button"
                  disabled={saving}
                  onClick={() => {
                    setRpcDraft("http://127.0.0.1:10787");
                    void window.alvenqis.settings.setRpcUrl("http://127.0.0.1:10787").then(async (applied) => {
                      await update({ rpc_url: applied });
                      setNotice({ error: false, text: `RPC set to local ${applied}` });
                      await refresh();
                    });
                  }}
                >
                  Use local 127.0.0.1:10787
                </button>
              </div>
              <p className="muted">
                Port 20787 is P2P, not HTTP RPC. Do not paste multiaddrs into the RPC field.
              </p>
            </Panel>
            <Panel title="Explorer" detail="External browser surface">
              <Toggle
                checked={settings.open_external_explorer}
                onChange={(open_external_explorer) => void patch({ open_external_explorer })}
                label="Open explorer links externally"
                description="Launch the selected RPC explorer URL in the system browser."
              />
            </Panel>
          </div>
        )}

        {section === "mining" && (
          <div className="grid settings-stack">
            <Panel title="Default miner profile" detail="Applied on Miner start">
              <Segmented<"solo" | "stratum">
                value={settings.default_miner_mode}
                onChange={(default_miner_mode) => void patch({ default_miner_mode })}
                options={[
                  { id: "solo", label: "Solo", detail: "Mine to the active wallet", icon: Wallet },
                  { id: "stratum", label: "Pool / Stratum", detail: "Persistent TLS socket", icon: Plug }
                ]}
              />
              <div className="field" style={{ marginTop: 12 }}>
                <label htmlFor="mining-rpc-url">Mining RPC URL (solo work over HTTP)</label>
                <input
                  id="mining-rpc-url"
                  value={miningRpcDraft}
                  spellCheck={false}
                  placeholder="http://rpcnode.dohotstudio.com"
                  onChange={(event) => setMiningRpcDraft(event.target.value)}
                />
                <div className="field-actions">
                  <button
                    type="button"
                    className="button ghost"
                    disabled={saving}
                    onClick={() => setMiningRpcDraft("http://127.0.0.1:10787")}
                  >
                    Use local 127.0.0.1:10787
                  </button>
                </div>
                <p className="field-hint">
                  Solo mode fetches mining templates from this endpoint — it is a separate setting
                  from the general RPC endpoint above. Point it at your own node's address (e.g.
                  the local one) to mine solo entirely against your own machine instead of the
                  shared gateway.
                </p>
              </div>
              <div className="field" style={{ marginTop: 12 }}>
                <label htmlFor="pool-url">Pool dashboard URL (status and payout views only)</label>
                <input
                  id="pool-url"
                  value={poolDraft}
                  spellCheck={false}
                  placeholder="https://pool.dohotstudio.com"
                  onChange={(event) => setPoolDraft(event.target.value)}
                />
              </div>
              {(settings.pool_urls ?? []).length > 0 ? (
                <p className="field-hint" style={{ marginTop: 6 }}>
                  Saved pools ({settings.pool_urls.length}): manage multi-pool list on the{" "}
                  <strong>Pool</strong> page. Mining work itself always uses Stratum TLS.
                </p>
              ) : null}
              <div className="field" style={{ marginTop: 12 }}>
                <label htmlFor="backend">Default compute backend (GPU-only)</label>
                <select
                  id="backend"
                  value="cuda"
                  onChange={(event) => setBackendDraft(event.target.value as typeof backendDraft)}
                >
                  <option value="cuda">CUDA (NVIDIA GPU only)</option>
                </select>
                <p className="field-hint">
                  The miner requires a supported NVIDIA CUDA GPU; no CPU fallback exists.
                </p>
              </div>
              <div className="field">
                <label htmlFor="gpu-intensity">Default GPU intensity (1–100)</label>
                <input
                  id="gpu-intensity"
                  type="number"
                  min={1}
                  max={100}
                  value={gpuIntensityDraft}
                  onChange={(event) => setGpuIntensityDraft(event.target.value)}
                />
              </div>
              <div className="field">
                <label htmlFor="worker">Default worker name</label>
                <input id="worker" value={workerDraft} maxLength={48} onChange={(event) => setWorkerDraft(event.target.value)} />
              </div>
              <div className="grid cols-2">
                <div className="field">
                  <label htmlFor="gpu-batch-size">GPU batch size (0 = automatic)</label>
                  <input
                    id="gpu-batch-size"
                    type="number"
                    min={0}
                    max={131072}
                    step={256}
                    value={gpuBatchSizeDraft}
                    onChange={(event) => setGpuBatchSizeDraft(event.target.value)}
                  />
                </div>
                <div className="field">
                  <label htmlFor="template-refresh">Template refresh (seconds)</label>
                  <input
                    id="template-refresh"
                    type="number"
                    min={1}
                    max={60}
                    value={templateRefreshDraft}
                    onChange={(event) => setTemplateRefreshDraft(event.target.value)}
                  />
                </div>
                <div className="field">
                  <label htmlFor="status-interval">Telemetry interval (seconds)</label>
                  <input
                    id="status-interval"
                    type="number"
                    min={1}
                    max={60}
                    value={statusIntervalDraft}
                    onChange={(event) => setStatusIntervalDraft(event.target.value)}
                  />
                </div>
                <div className="field">
                  <label htmlFor="stratum-timeout">Stratum timeout (seconds)</label>
                  <input
                    id="stratum-timeout"
                    type="number"
                    min={5}
                    max={120}
                    value={stratumTimeoutDraft}
                    onChange={(event) => setStratumTimeoutDraft(event.target.value)}
                  />
                </div>
              </div>
              <div className="field-stack" style={{ marginTop: 4 }}>
                <p className="field-stack-title">Default Stratum endpoint (alvenqis-stratum-v1)</p>
                <p className="field-hint" style={{ marginTop: -4 }}>
                  Pool mining uses this endpoint exclusively. HTTP work/share routes are retired.
                  The Miner page can override these values for one session.
                </p>
                <div className="grid cols-2">
                  <div className="field">
                    <label htmlFor="stratum-host">Host</label>
                    <input
                      id="stratum-host"
                      value={stratumHostDraft}
                      spellCheck={false}
                      placeholder="stratum.example.com"
                      onChange={(event) => setStratumHostDraft(event.target.value)}
                    />
                  </div>
                  <div className="field">
                    <label htmlFor="stratum-port">Port</label>
                    <input
                      id="stratum-port"
                      type="number"
                      min={1}
                      max={65535}
                      value={stratumPortDraft}
                      onChange={(event) => setStratumPortDraft(event.target.value)}
                    />
                  </div>
                </div>
                <Toggle
                  checked={stratumUseTlsDraft}
                  onChange={(value) => {
                    const local = /^(localhost|127\.0\.0\.1|::1|\[::1\])$/i.test(stratumHostDraft.trim());
                    setStratumUseTlsDraft(local ? value : true);
                  }}
                  label="TLS transport"
                  description="Required for every remote Stratum endpoint"
                />
                <Toggle
                  checked={stratumSkipTlsVerifyDraft}
                  disabled={!/^(localhost|127\.0\.0\.1|::1|\[::1\])$/i.test(stratumHostDraft.trim())}
                  onChange={setStratumSkipTlsVerifyDraft}
                  label="Skip TLS verify"
                  description="Lab only — never on public internet"
                />
                <div className="field">
                  <label htmlFor="stratum-password">Password (optional)</label>
                  <input
                    id="stratum-password"
                    type="password"
                    autoComplete="off"
                    value={stratumPasswordDraft}
                    placeholder="pool password / empty"
                    onChange={(event) => setStratumPasswordDraft(event.target.value)}
                  />
                </div>
              </div>
              <button className="button primary" disabled={saving} onClick={() => void saveMiningDefaults()}>
                Save mining defaults
              </button>
            </Panel>
            <Panel title="Live miner telemetry" detail="Current session">
              <KeyValue label="Miner">{snapshot.miner_running ? "RUNNING" : "STOPPED"}</KeyValue>
              <KeyValue label="Accepted blocks">{snapshot.miner_accepted_blocks ?? "—"}</KeyValue>
              <KeyValue label="Status">{snapshot.miner_status ?? "No telemetry"}</KeyValue>
            </Panel>
          </div>
        )}

        {section === "wallet" && (
          <div className="grid settings-stack">
            <Panel title="Active wallet" detail="OS user-bound identity">
              {wallet ? (
                <>
                  <KeyValue label="Name">{wallet.display_name}</KeyValue>
                  <KeyValue label="Address"><AddressChip value={wallet.address} /></KeyValue>
                  <KeyValue label="Network" mono>{wallet.network_id}</KeyValue>
                  <KeyValue label="Derivation" mono>{wallet.derivation_path}</KeyValue>
                  <KeyValue label="Key origin">{wallet.key_origin}</KeyValue>
                  <KeyValue label="Private key">OS credential vault · verified at signing</KeyValue>
                  <KeyValue label="Recovery phrase">Never stored · cannot be re-shown</KeyValue>
                </>
              ) : (
                <EmptyState>No wallet metadata is configured for this user.</EmptyState>
              )}
            </Panel>

            <Panel title="Recovery phrase" detail="Critical backup · native keystore only">
              <div className="secret-warning" style={{ marginBottom: 14 }}>
                <ShieldAlert size={18} />
                <span>
                  The 24-word recovery phrase is the only way to restore this wallet on another device
                  or after disconnect. Alvenqis never keeps a copy. Support will never ask for these words.
                </span>
              </div>
              <KeyValue label="Storage policy">Not persisted by Control Center or the WebView</KeyValue>
              <KeyValue label="Create flow">Shown once in a Rust-owned native dialog</KeyValue>
              <KeyValue label="Import flow">Entered only in a native OS dialog (never React)</KeyValue>
              <KeyValue label="Re-display">Impossible by design — write it down when you create</KeyValue>

              <div className="field" style={{ marginTop: 14 }}>
                <label htmlFor="recovery-wallet-name">Wallet name for create / import</label>
                <input
                  id="recovery-wallet-name"
                  value={recoveryWalletName}
                  maxLength={48}
                  spellCheck={false}
                  disabled={recoveryBusy}
                  placeholder="e.g. Main wallet"
                  onChange={(event) => setRecoveryWalletName(event.target.value)}
                />
              </div>

              <div className="button-row" style={{ marginTop: 12 }}>
                <button
                  className="button primary"
                  type="button"
                  disabled={recoveryBusy || !recoveryWalletName.trim()}
                  onClick={() => setRecoveryImportOpen(true)}
                >
                  <KeyRound size={15} /> Recovery phrase…
                </button>
                <button
                  className="button"
                  type="button"
                  disabled={recoveryBusy || !recoveryWalletName.trim()}
                  onClick={() => void createWithRecoveryPhrase()}
                >
                  Create wallet (show phrase once)
                </button>
              </div>
              <p className="muted" style={{ marginTop: 12, marginBottom: 0 }}>
                <strong>Recovery phrase…</strong> opens the secure import path: confirm in-app, then enter
                the 24 BIP-39 words in the operating-system dialog owned by the keystore helper.
              </p>
            </Panel>

            <Panel title="Security boundary" detail="Tauri shell">
              <p className="muted">
                The React UI cannot read the recovery phrase or private keys. Wallet create, import and
                sign operations run through the native Rust keystore helper and the operating-system
                credential vault.
              </p>
              <KeyValue label="Shell">Tauri 2 + system WebView</KeyValue>
              <KeyValue label="Keystore helper" mono>{paths?.keystore_helper ?? "…"}</KeyValue>
            </Panel>
          </div>
        )}

        {section === "notifications" && (
          <div className="grid settings-stack">
            <Panel title="Mining alerts" detail="Local blocks only">
              <Toggle
                checked={settings.notify_block_mined}
                onChange={(notify_block_mined) => void patch({ notify_block_mined })}
                label="Notify when a block is mined to the active wallet"
                description="Uses the operating-system notification center after the first baseline snapshot."
              />
              <Toggle
                checked={settings.notify_sound}
                onChange={(notify_sound) => void patch({ notify_sound })}
                label="Play system sound with mined-block alerts"
              />
            </Panel>
            <Panel title="Application updates" detail="Disabled">
              <p className="muted" style={{ margin: 0 }}>
                Automatic updates are permanently disabled in this build. Install new versions
                manually from release packages when needed.
              </p>
            </Panel>
          </div>
        )}

        {section === "data" && (
          <div className="grid settings-stack">
            <Panel title="Runtime paths" detail="This machine">
              <KeyValue label="Workspace" mono>{paths?.workspace ?? "…"}</KeyValue>
              <KeyValue label="Local chain root" mono>{paths?.local_root ?? "…"}</KeyValue>
              <KeyValue label="User data" mono>{paths?.user_data ?? "…"}</KeyValue>
              <KeyValue label="Settings file" mono>{paths?.settings_file ?? "…"}</KeyValue>
              <KeyValue label="Logs directory" mono>{paths?.logs_dir ?? "…"}</KeyValue>
              <div className="button-row" style={{ marginTop: 12 }}>
                <button className="button" onClick={() => void openPath("workspace")}>Open workspace</button>
                <button className="button" onClick={() => void openPath("local_root")}>Open chain data</button>
                <button className="button" onClick={() => void openPath("logs")}>Open logs</button>
                <button className="button" onClick={() => void openPath("user_data")}>Open user data</button>
                <button className="button" onClick={() => void openPath("settings_file")}>Reveal settings.json</button>
              </div>
            </Panel>
            <Panel title="Log retention preference" detail="Policy for operators">
              <div className="field">
                <label htmlFor="keep-logs">Keep exported/local log history target (days)</label>
                <input
                  id="keep-logs"
                  type="number"
                  min={1}
                  max={365}
                  value={settings.keep_logs_days}
                  onChange={(event) => void patch({ keep_logs_days: Number(event.target.value) || 14 })}
                />
              </div>
              <p className="muted">
                Preference is stored for future cleanup tooling. Manual exports never include wallet secrets.
              </p>
              <div className="button-row">
                {["node", "rpc", "miner", "explorer"].map((service) => (
                  <button
                    key={service}
                    className="button"
                    onClick={() => void window.alvenqis.logs.export(service).then((path) => {
                      setNotice({
                        error: false,
                        text: path ? `Exported ${service} log to ${path}` : "Log export cancelled."
                      });
                    })}
                  >
                    Export {service}
                  </button>
                ))}
              </div>
            </Panel>
          </div>
        )}

        {section === "services" && (
          <div className="grid settings-stack">
            <Panel title="Managed process state" detail="PID + log diagnostics">
              <div className="settings-diag-grid">
                <article><small>NODE</small><b className={diagnostics?.node_pid_present ? "positive" : "muted"}>{diagnostics?.node_pid_present ? "PID present" : "No PID"}</b><span>{formatBytes(diagnostics?.node_log_bytes ?? 0)}</span></article>
                <article><small>RPC</small><b className={diagnostics?.rpc_pid_present ? "positive" : "muted"}>{diagnostics?.rpc_pid_present ? "PID present" : "No PID"}</b><span>{formatBytes(diagnostics?.rpc_log_bytes ?? 0)}</span></article>
                <article><small>MINER</small><b className={diagnostics?.miner_pid_present ? "positive" : "muted"}>{diagnostics?.miner_pid_present ? "PID present" : "No PID"}</b><span>{formatBytes(diagnostics?.miner_log_bytes ?? 0)}</span></article>
                <article><small>EXPLORER</small><b className={diagnostics?.explorer_pid_present ? "positive" : "muted"}>{diagnostics?.explorer_pid_present ? "PID present" : "No PID"}</b><span>{formatBytes(diagnostics?.explorer_log_bytes ?? 0)}</span></article>
              </div>
              <KeyValue label="metrics.json">{diagnostics?.metrics_present ? "Present" : "Missing"}</KeyValue>
              <KeyValue label="node.toml">{diagnostics?.node_config_present ? "Present" : "Missing"}</KeyValue>
            </Panel>
            <Panel title="Operator actions" detail="Uses alvenqis.ps1 / alvenqis.sh">
              <Toggle
                checked={settings.confirm_before_operator}
                onChange={(confirm_before_operator) => void patch({ confirm_before_operator })}
                label="Confirm destructive operator actions in UI"
                description="Stop/restart prompts can be enforced by pages that honor this flag."
              />
              <div className="button-row" style={{ marginTop: 12 }}>
                <button className="button primary" onClick={() => void runService("start")}>Start stack</button>
                <button className="button" onClick={() => void runService("restart")}>Restart</button>
                <button className="button" onClick={() => void runService("status")}>Status</button>
                <button className="button danger" onClick={() => void runService("stop")}>Stop stack</button>
              </div>
            </Panel>
          </div>
        )}

        {section === "privacy" && (
          <div className="grid settings-stack">
            <Panel title="On-screen privacy" detail="Does not change chain data">
              <Toggle
                checked={settings.hide_balances}
                onChange={(hide_balances) => void patch({ hide_balances })}
                label="Hide balances"
                description="Replace ALVE amounts with masked values in the UI."
              />
              <Toggle
                checked={settings.mask_addresses}
                onChange={(mask_addresses) => void patch({ mask_addresses })}
                label="Mask addresses by default"
                description="Show shortened addresses until you expand a chip."
              />
              <Toggle
                checked={settings.show_advanced_metrics}
                onChange={(show_advanced_metrics) => void patch({ show_advanced_metrics })}
                label="Show advanced telemetry panels"
                description="Hash streams, peer multiaddrs and fee disposition details."
              />
            </Panel>
          </div>
        )}

        {section === "advanced" && (
          <div className="grid settings-stack">
            <Panel title="Developer surface" detail="Prototype controls">
              <KeyValue label="Shell engine">Tauri 2</KeyValue>
              <KeyValue label="Packaged">{paths?.packaged ? "Yes" : "Development"}</KeyValue>
              <KeyValue label="Platform">{paths?.platform ?? window.alvenqis.app.platform}</KeyValue>
              <p className="muted">
                Advanced flags that would expose unsafe remote debugging remain intentionally unavailable
                in production candidate builds.
              </p>
            </Panel>
            <Panel title="Reset preferences" detail="Does not touch wallets or chain data">
              <label className="settings-check">
                <input
                  type="checkbox"
                  checked={resetConfirmed}
                  onChange={(event) => setResetConfirmed(event.target.checked)}
                />
                Reset theme, RPC, mining defaults, notifications and privacy toggles to factory defaults.
              </label>
              <button className="button" disabled={!resetConfirmed || saving} onClick={() => void resetAll()}>
                <RotateCcw size={14} /> Reset all settings
              </button>
            </Panel>
          </div>
        )}

        {section === "about" && (
          <div className="grid settings-stack">
            <Panel title="Alvenqis Control Center" detail="Tauri edition">
              <div className="settings-about-hero">
                <AlvenqisLogo size="lg" alt="Alvenqis Network logo" />
                <div>
                  <div className="eyebrow">Identity</div>
                  <h3 style={{ margin: "4px 0 8px" }}>Alvenqis Network</h3>
                  <p className="muted" style={{ margin: 0, lineHeight: 1.5 }}>
                    Control Center shell for Mainnet Candidate operations — wallet, node, miner and chain visibility.
                  </p>
                </div>
              </div>
              <KeyValue label="Version" mono>{version}</KeyValue>
              <KeyValue label="Network status label">{snapshot.status_label}</KeyValue>
              <KeyValue label="Public mainnet">Not launched</KeyValue>
              <KeyValue label="Honesty rule">No market prices, fiat values or fabricated telemetry</KeyValue>
              <p className="muted" style={{ marginTop: 12 }}>
                This is Mainnet Candidate / Prototype software. Packaging or connecting a wallet does not
                authorize a public mainnet claim.
              </p>
            </Panel>
            <Panel title="Build surface" detail="Local metadata">
              <KeyValue label="Workspace" mono>{paths?.workspace ?? "…"}</KeyValue>
              <KeyValue label="User data" mono>{paths?.user_data ?? "…"}</KeyValue>
              <KeyValue label="Keystore helper" mono>{paths?.keystore_helper ?? "…"}</KeyValue>
            </Panel>
          </div>
        )}

        {section === "danger" && (
          <div className="grid settings-stack">
            <Panel title="Disconnect wallet" detail="Critical modal confirmation" className="danger-panel">
              <p className="muted">
                Removes the active wallet metadata and the operating-system credential entry for this user.
                Chain history, mined blocks and other wallets are not deleted.
              </p>
              <button
                className="button danger"
                type="button"
                disabled={!wallet}
                onClick={() => setDisconnectOpen(true)}
              >
                Disconnect wallet…
              </button>
            </Panel>
          </div>
        )}
      </div>
    </div>
    <ConfirmDialog
      open={disconnectOpen}
      title="Disconnect active wallet?"
      description="This removes the active wallet profile and OS credential entry for this user."
      consequences={[
        "Local signing key material for this profile is removed",
        "Chain history and other wallets are not deleted",
        "Re-access requires the 24-word recovery phrase"
      ]}
      danger
      busy={disconnectBusy}
      confirmLabel="Disconnect wallet"
      onConfirm={removeWallet}
      onClose={() => {
        if (!disconnectBusy) setDisconnectOpen(false);
      }}
    />
    <RecoveryPhraseImport
      open={recoveryImportOpen}
      walletName={recoveryWalletName}
      busy={recoveryBusy}
      onClose={() => {
        if (!recoveryBusy) setRecoveryImportOpen(false);
      }}
      onImport={importRecoveryPhrase}
    />
    <RecoveryPhraseReveal
      open={Boolean(revealWords?.length)}
      words={revealWords ?? []}
      walletName={revealMeta?.name ?? recoveryWalletName}
      address={revealMeta?.address ?? ""}
      onConfirmed={() => {
        setRevealWords(null);
        setRevealMeta(null);
      }}
    />
    </div>
  );
}
