import { useState } from "react";
import { ArrowRightLeft } from "lucide-react";
import type { PreparedTransaction } from "@shared/types";
import { formatAtomic } from "@shared/format";
import { AddressChip } from "../components/ui/AddressChip";
import { EmptyState } from "../components/ui/EmptyState";
import { PageHero } from "../components/ui/PageHero";
import { KeyValue, Panel } from "../components/ui/Panel";
import { StatCard } from "../components/ui/StatCard";
import { SignTransactionDialog } from "../components/dialogs/SignTransactionDialog";
import { useNotificationsOptional } from "../shared/notifications";
import { useApp } from "../model";

export function Send() {
  const { wallet, snapshot, setNotice, refresh } = useApp();
  const notifications = useNotificationsOptional();
  const [recipient, setRecipient] = useState("");
  const [amount, setAmount] = useState("");
  const [tip, setTip] = useState("0.00000001");
  const [prepared, setPrepared] = useState<PreparedTransaction | null>(null);
  const [signOpen, setSignOpen] = useState(false);
  const [confirmed, setConfirmed] = useState(false);
  const [busy, setBusy] = useState(false);

  const prepare = async () => {
    setBusy(true);
    try {
      const next = await window.alvenqis.transactions.prepare(recipient, amount, tip);
      setPrepared(next);
      setConfirmed(false);
      setSignOpen(true);
      setNotice(null);
      notifications?.notify({
        kind: "info",
        title: "Transaction prepared",
        body: `Exact total ${formatAtomic(next.total_atomic)} ALVE · review before signing.`,
        severity: "toast",
        source: "send:prepare"
      });
    } catch (error) {
      setNotice({ error: true, text: String(error) });
      notifications?.notify({
        kind: "error",
        title: "Prepare failed",
        body: String(error).replace(/^Error:\s*/i, ""),
        sticky: true,
        source: "send:prepare"
      });
    } finally {
      setBusy(false);
    }
  };

  const submit = async () => {
    if (!prepared) return;
    setBusy(true);
    try {
      const result = await window.alvenqis.transactions.signAndSubmit(prepared, confirmed);
      setPrepared(null);
      setConfirmed(false);
      setSignOpen(false);
      setNotice({
        error: false,
        text: `Transaction ${result.tx_hash} is ${result.lifecycle_status}; mempool size ${result.mempool_size}.`
      });
      notifications?.notify({
        kind: "success",
        title: "Transaction submitted",
        body: `${result.tx_hash.slice(0, 16)}… · ${result.lifecycle_status}`,
        severity: "both",
        source: "send:submit"
      });
      await refresh();
    } catch (error) {
      const text = String(error);
      if (text.includes("chain state changed")) {
        setPrepared(null);
        setSignOpen(false);
      }
      setNotice({ error: true, text });
      notifications?.notify({
        kind: "error",
        title: "Sign / submit failed",
        body: text.replace(/^Error:\s*/i, ""),
        sticky: true,
        source: "send:submit"
      });
    } finally {
      setBusy(false);
    }
  };

  if (!wallet) {
    return (
      <div className="page">
        <EmptyState>Create or import a wallet before preparing a transaction.</EmptyState>
      </div>
    );
  }

  return (
    <div className="page grid send-page">
      <PageHero
        kicker="SIGN LOCALLY · BROADCAST TO VPS"
        title="Send &"
        titleAccent="receive"
        description="Prepare a transfer with deterministic fees, then confirm signing in a dedicated modal — harder to mis-click."
        side={
          <>
            <div className="page-hero-metric">
              <small>Available</small>
              <strong>
                {snapshot.balance_atomic !== null ? formatAtomic(snapshot.balance_atomic) : "—"} ALVE
              </strong>
            </div>
            <div className="page-hero-metric">
              <small>Mempool</small>
              <strong>{snapshot.mempool_count} pending</strong>
            </div>
            <div className="page-hero-metric">
              <small>From</small>
              <strong className="mono" style={{ fontSize: 12 }}>
                {wallet.address.slice(0, 14)}…
              </strong>
            </div>
          </>
        }
      />

      <div className="grid cols-3 telemetry-strip">
        <StatCard
          label="Available"
          value={snapshot.balance_atomic !== null ? formatAtomic(snapshot.balance_atomic) : "—"}
          detail="Active wallet"
          tone="gold"
        />
        <StatCard label="Base fee path" value="Deterministic" detail="No estimated gas fiction" />
        <StatCard
          label="Status"
          value={prepared && signOpen ? "Review in dialog" : "Compose"}
          detail={prepared ? "Confirm & sign" : "Enter recipient + amount"}
          tone={prepared ? "positive" : undefined}
        />
      </div>

      <div className="grid cols-2">
        <Panel title="Compose transfer" detail="Mainnet Candidate">
          <KeyValue label="From">
            <AddressChip value={wallet.address} />
          </KeyValue>
          <div className="field">
            <label>Recipient address</label>
            <input
              value={recipient}
              onChange={(e) => {
                setRecipient(e.target.value);
                setPrepared(null);
                setSignOpen(false);
              }}
              placeholder="alve1…"
              spellCheck={false}
              autoComplete="off"
            />
          </div>
          <div className="field">
            <label>Amount (ALVE)</label>
            <input
              value={amount}
              onChange={(e) => {
                setAmount(e.target.value);
                setPrepared(null);
                setSignOpen(false);
              }}
              inputMode="decimal"
              placeholder="0.0"
            />
          </div>
          <div className="field">
            <label>Priority tip (ALVE)</label>
            <input
              value={tip}
              onChange={(e) => {
                setTip(e.target.value);
                setPrepared(null);
                setSignOpen(false);
              }}
              inputMode="decimal"
            />
          </div>
          <button
            className="button primary"
            type="button"
            disabled={busy || !recipient || !amount}
            onClick={() => void prepare()}
          >
            <ArrowRightLeft size={15} /> Prepare & review
          </button>
        </Panel>

        <Panel title="Signing boundary" detail="Modal confirmation">
          <EmptyState>
            After prepare, a critical sign dialog shows recipient, fees and total. Signing uses the OS
            credential vault and never invents fees.
          </EmptyState>
          {prepared ? (
            <button
              className="button"
              type="button"
              style={{ marginTop: 12 }}
              onClick={() => setSignOpen(true)}
            >
              Re-open sign dialog
            </button>
          ) : null}
        </Panel>
      </div>

      <SignTransactionDialog
        open={signOpen && prepared !== null}
        prepared={prepared}
        confirmed={confirmed}
        busy={busy}
        onConfirmedChange={setConfirmed}
        onSign={submit}
        onClose={() => {
          if (!busy) setSignOpen(false);
        }}
      />
    </div>
  );
}
