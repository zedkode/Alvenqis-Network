import { ShieldCheck } from "lucide-react";
import type { PreparedTransaction } from "@shared/types";
import { formatAtomic } from "@shared/format";
import { AddressChip } from "../ui/AddressChip";
import { KeyValue } from "../ui/Panel";
import { Dialog } from "./Dialog";

export function SignTransactionDialog({
  open,
  prepared,
  confirmed,
  busy,
  onConfirmedChange,
  onSign,
  onClose
}: {
  open: boolean;
  prepared: PreparedTransaction | null;
  confirmed: boolean;
  busy: boolean;
  onConfirmedChange(next: boolean): void;
  onSign(): void | Promise<void>;
  onClose(): void;
}) {
  if (!prepared) return null;

  return (
    <Dialog
      open={open}
      title="Sign transaction"
      subtitle="Review exact amounts before signing with the local keystore"
      critical
      onClose={onClose}
      footer={
        <div className="button-row" style={{ justifyContent: "flex-end", width: "100%" }}>
          <button type="button" className="button" disabled={busy} onClick={onClose}>
            Cancel
          </button>
          <button
            type="button"
            className="button primary"
            disabled={!confirmed || busy}
            onClick={() => void onSign()}
          >
            <ShieldCheck size={15} /> Sign and submit
          </button>
        </div>
      }
    >
      <div className="send-total-card" style={{ marginBottom: 14 }}>
        <small>Total debit</small>
        <strong>{formatAtomic(prepared.total_atomic)} ALVE</strong>
      </div>
      <KeyValue label="Recipient">
        <AddressChip value={prepared.recipient} />
      </KeyValue>
      <KeyValue label="Amount" mono>
        {formatAtomic(prepared.amount_atomic)} ALVE
      </KeyValue>
      <KeyValue label="Base fee (burned)" mono>
        {formatAtomic(prepared.base_fee_atomic)} ALVE
      </KeyValue>
      <KeyValue label="Priority tip" mono>
        {formatAtomic(prepared.tip_atomic)} ALVE
      </KeyValue>
      <KeyValue label="Available" mono>
        {formatAtomic(prepared.available_atomic)} ALVE
      </KeyValue>
      <KeyValue label="Nonce">{prepared.nonce}</KeyValue>
      <label className="send-confirm" style={{ marginTop: 14 }}>
        <input
          type="checkbox"
          checked={confirmed}
          onChange={(e) => onConfirmedChange(e.target.checked)}
        />
        <span>I verified the recipient, amount and fees. This action signs with the local vault.</span>
      </label>
    </Dialog>
  );
}
