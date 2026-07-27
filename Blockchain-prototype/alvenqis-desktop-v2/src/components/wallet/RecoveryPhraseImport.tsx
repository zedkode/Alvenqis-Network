import { KeyRound, ShieldCheck, X } from "lucide-react";
import { useState } from "react";

interface RecoveryPhraseImportProps {
  open: boolean;
  walletName: string;
  busy: boolean;
  onClose(): void;
  /** In-app paste import — 24 words stay in this dialog until submit. */
  onImport(walletName: string, recoveryPhrase: string): Promise<void>;
}

/**
 * Import 24-word wallet with paste support inside the app (no OS prompt).
 */
export function RecoveryPhraseImport({
  open,
  walletName,
  busy,
  onClose,
  onImport
}: RecoveryPhraseImportProps) {
  const [phrase, setPhrase] = useState("");
  const [error, setError] = useState<string | null>(null);

  if (!open) return null;

  const close = () => {
    if (busy) return;
    setPhrase("");
    setError(null);
    onClose();
  };

  const normalized = phrase
    .trim()
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean);
  const wordCount = normalized.length;

  const submit = async () => {
    if (!walletName.trim()) return;
    if (wordCount !== 24) {
      setError(`Expected 24 words, found ${wordCount}.`);
      return;
    }
    setError(null);
    try {
      await onImport(walletName.trim(), normalized.join(" "));
      setPhrase("");
      onClose();
    } catch (err) {
      setError(String(err));
    }
  };

  return (
    <div
      className="secret-modal-backdrop"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) close();
      }}
    >
      <section className="secret-modal" role="dialog" aria-modal="true" aria-labelledby="recovery-import-title">
        <header>
          <span className="secret-modal-icon">
            <KeyRound size={22} />
          </span>
          <div>
            <small>Secure wallet recovery</small>
            <h2 id="recovery-import-title">Import 24-word wallet</h2>
          </div>
          <button className="icon-button" aria-label="Close import dialog" disabled={busy} onClick={close}>
            <X size={18} />
          </button>
        </header>
        <p className="muted">
          Paste or type your recovery phrase below. Words stay inside this window — no Windows
          system prompt.
        </p>
        <div className="secret-warning">
          <ShieldCheck size={18} />
          <span>
            Importing as <b>{walletName.trim() || "Unnamed wallet"}</b>. Only continue on a trusted
            device.
          </span>
        </div>
        <label className="field" style={{ marginTop: 14 }}>
          <span>Recovery phrase ({wordCount}/24 words)</span>
          <textarea
            className="recovery-phrase-input"
            rows={4}
            spellCheck={false}
            autoComplete="off"
            autoCorrect="off"
            value={phrase}
            disabled={busy}
            placeholder="Paste all 24 words here…"
            onChange={(e) => setPhrase(e.target.value)}
            onPaste={(e) => {
              // Allow default paste into the field
              void e;
            }}
          />
        </label>
        {error && <div className="notice error" style={{ marginTop: 12 }}>{error}</div>}
        <footer>
          <button className="button" disabled={busy} onClick={close}>
            Cancel
          </button>
          <button
            className="button primary"
            disabled={busy || !walletName.trim() || wordCount !== 24}
            onClick={() => void submit()}
          >
            {busy ? "Importing…" : "Import wallet"}
          </button>
        </footer>
      </section>
    </div>
  );
}
