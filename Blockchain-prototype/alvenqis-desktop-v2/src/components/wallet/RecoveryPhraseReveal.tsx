import { Check, Copy, Eye, EyeOff, ShieldAlert } from "lucide-react";
import { useMemo, useState } from "react";

interface RecoveryPhraseRevealProps {
  open: boolean;
  words: string[];
  walletName: string;
  address: string;
  onConfirmed(): void;
}

/**
 * One-time recovery phrase backup UI: hidden by default (stars), reveal with eye,
 * copy to clipboard in-app — no OS prompts.
 */
export function RecoveryPhraseReveal({
  open,
  words,
  walletName,
  address,
  onConfirmed
}: RecoveryPhraseRevealProps) {
  const [revealed, setRevealed] = useState(false);
  const [copied, setCopied] = useState(false);
  const [acknowledged, setAcknowledged] = useState(false);

  const phrase = useMemo(() => words.join(" "), [words]);

  if (!open || words.length === 0) return null;

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(phrase);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 2000);
    } catch {
      // Fallback for restricted WebViews
      const ta = document.createElement("textarea");
      ta.value = phrase;
      ta.style.position = "fixed";
      ta.style.opacity = "0";
      document.body.appendChild(ta);
      ta.select();
      document.execCommand("copy");
      document.body.removeChild(ta);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <div className="secret-modal-backdrop recovery-reveal-backdrop" role="presentation">
      <section
        className="secret-modal recovery-reveal-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="recovery-reveal-title"
      >
        <header>
          <span className="secret-modal-icon">
            <ShieldAlert size={22} />
          </span>
          <div>
            <small>Write this down offline</small>
            <h2 id="recovery-reveal-title">Recovery phrase</h2>
          </div>
        </header>
        <p className="muted">
          Wallet <strong>{walletName}</strong> · {shortAddress(address)}. These 24 words are shown
          once. Store them offline — anyone with this phrase can spend your ALVE.
        </p>

        <div className={`recovery-words-panel ${revealed ? "is-revealed" : "is-hidden"}`}>
          <div className="recovery-words-grid" aria-live="polite">
            {words.map((word, index) => (
              <div key={`w-${index}`} className="recovery-word-chip">
                <span className="recovery-word-index">{index + 1}</span>
                <span className={`recovery-word-text ${revealed ? "reveal-in" : ""}`}>
                  {revealed ? word : "••••••"}
                </span>
              </div>
            ))}
          </div>
          {!revealed && <div className="recovery-words-veil" aria-hidden />}
        </div>

        <div className="recovery-toolbar">
          <button
            type="button"
            className="button"
            onClick={() => setRevealed((v) => !v)}
            aria-pressed={revealed}
          >
            {revealed ? <EyeOff size={16} /> : <Eye size={16} />}
            {revealed ? "Hide words" : "Reveal words"}
          </button>
          <button type="button" className="button" onClick={() => void copy()} disabled={!revealed}>
            {copied ? <Check size={16} /> : <Copy size={16} />}
            {copied ? "Copied" : "Copy phrase"}
          </button>
        </div>

        <label className="recovery-ack">
          <input
            type="checkbox"
            checked={acknowledged}
            onChange={(e) => setAcknowledged(e.target.checked)}
          />
          <span>I have saved these words offline. I understand they will not be shown again.</span>
        </label>

        <footer>
          <button
            type="button"
            className="button primary"
            disabled={!acknowledged}
            onClick={onConfirmed}
          >
            Continue
          </button>
        </footer>
      </section>
    </div>
  );
}

function shortAddress(address: string): string {
  return address.length > 22 ? `${address.slice(0, 12)}…${address.slice(-8)}` : address;
}
