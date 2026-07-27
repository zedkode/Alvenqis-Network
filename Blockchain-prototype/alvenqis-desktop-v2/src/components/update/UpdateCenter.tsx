import { AlertTriangle, CheckCircle2, Download, LoaderCircle, RefreshCw, X } from "lucide-react";
import { useEffect, useState } from "react";
import type { UpdateState } from "@shared/types";
import { AlvenqisLogo } from "../brand/AlvenqisLogo";
import { formatUpdateBytes, isBlockingUpdate, shouldShowUpdateNotice, updateHeading, updateNoticeKey } from "./updatePresentation";

const initialState: UpdateState = {
  phase: "idle", current_version: "", available_version: null, release_name: null,
  release_date: null, message: "Verified update service is ready.", manual: false, progress: null
};

export function UpdateCenter() {
  const [state, setState] = useState(initialState);
  const [dismissedNotice, setDismissedNotice] = useState<string | null>(null);
  /** Operators must never be locked out of the app by a stuck/false auto-update. */
  const [allowThrough, setAllowThrough] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    void window.alvenqis.updates.state().then((next) => { if (active) setState(next); });
    const unsubscribe = window.alvenqis.updates.onState((next) => {
      setState(next);
      if (next.phase !== "error") setActionError(null);
      // When update finishes or idles, clear the "use app now" override for the next cycle.
      if (next.phase === "idle" || next.phase === "error") {
        setAllowThrough(false);
      }
    });
    return () => { active = false; unsubscribe(); };
  }, []);

  const download = async () => {
    setActionError(null);
    try { await window.alvenqis.updates.download(); } catch (error) { setActionError(String(error)); }
  };
  const install = async (restart: boolean) => {
    setActionError(null);
    try { await window.alvenqis.updates.install(restart); } catch (error) { setActionError(String(error)); }
  };

  const blocking = isBlockingUpdate(state.phase) && !allowThrough;
  const noticeKey = updateNoticeKey(state);
  const showMessage = shouldShowUpdateNotice(state, dismissedNotice);

  const components = (state.components ?? []).join(", ");

  return <>
    {showMessage && !blocking && <aside className={`update-toast ${state.phase === "error" ? "update-error" : ""}`} role="status" aria-live="polite">
      <span className="update-toast-icon">{state.phase === "available" || state.phase === "checking" ? <Download size={19} /> : state.phase === "error" ? <AlertTriangle size={19} /> : <CheckCircle2 size={19} />}</span>
      <div>
        <small>ALVENQIS UPDATE · GITHUB · SHA-256 VERIFIED</small>
        <strong>
          {state.phase === "available"
            ? `Update ${state.available_version} available`
            : state.phase === "error"
              ? "Update failed"
              : state.phase === "checking"
                ? "Checking GitHub…"
                : "Application is current"}
        </strong>
        <p>{state.message}{components ? ` · ${components}` : ""}</p>
      </div>
      <div className="update-toast-actions">
        {state.phase === "available" ? <button className="button primary" onClick={() => void download()}><Download size={16} />Review & install</button> : null}
        <button className="icon-button" aria-label="Dismiss update notification" title="Dismiss" onClick={() => setDismissedNotice(noticeKey)}><X size={16} /></button>
      </div>
    </aside>}

    {blocking && <div className="update-lock" role="dialog" aria-modal="true" aria-label="Alvenqis approved update">
      <section className="update-modal">
        <div className={`update-reactor update-${state.phase}`}><span className="update-reactor-logo"><AlvenqisLogo size="md" alt="" /></span><i /><i /></div>
        <div className="update-copy"><small>APPROVED UPDATE · GITHUB · SHA-256 VERIFIED</small><h2>{updateHeading(state)}</h2><p>{state.message}</p></div>
        <div className="update-version-row"><span><small>Installed</small><b>v{state.current_version}</b></span><i /><span><small>Update</small><b>v{state.available_version ?? "--"}</b></span></div>
        {components ? <p className="muted" style={{ marginTop: 8 }}>Components: {components}</p> : null}
        {state.phase === "downloading" && <DownloadProgress state={state} />}
        {state.phase === "installing" && <div className="update-installing"><LoaderCircle size={18} /><span>Applying verified binaries. Do not power off this computer.</span></div>}
        {state.phase === "downloaded" && <>
          <div className="update-ready"><CheckCircle2 size={18} /><span>Verified package is ready to finish installation.</span></div>
          <div className="update-actions">
            <button className="button primary" onClick={() => void install(true)}><RefreshCw size={16} />Restart now</button>
            <button className="button" onClick={() => void download()}><Download size={16} />Retry apply</button>
          </div>
        </>}
        {actionError && <div className="notice error update-action-error">{actionError}</div>}
        <div className="update-actions" style={{ marginTop: 16 }}>
          <button
            className="button primary"
            type="button"
            onClick={() => setAllowThrough(true)}
            title="Close the update overlay and use Control Center"
          >
            Use app now
          </button>
        </div>
        <p className="update-lock-note">
          Updates install only after explicit approval and a matching <strong>SHA256SUMS</strong> entry.
          You can always continue into the app; background checks never execute downloads.
        </p>
      </section>
    </div>}
  </>;
}

function DownloadProgress({ state }: { state: UpdateState }) {
  const progress = state.progress; const percent = progress?.percent ?? 0;
  return <div className="update-download"><div className="update-progress-label"><span>Package transfer</span><strong>{percent.toFixed(1)}%</strong></div><div className="update-progress-track"><i style={{ width: `${percent}%` }} /></div><div className="update-transfer-stats"><span><small>Downloaded</small><b>{formatUpdateBytes(progress?.transferred ?? 0)}</b></span><span><small>Total size</small><b>{formatUpdateBytes(progress?.total ?? 0)}</b></span><span><small>Transfer rate</small><b>{formatUpdateBytes(progress?.bytes_per_second ?? 0)}/s</b></span></div></div>;
}
