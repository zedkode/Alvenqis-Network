import type { CSSProperties } from "react";
import { Blocks } from "lucide-react";
import { shortHash } from "@shared/format";

export function ChainOrb({
  height,
  tipHash,
  online
}: {
  height: number | null;
  tipHash: string | null;
  online: boolean;
}) {
  return (
    <div className={`chain-orb chain-orb-v2 ${online ? "online" : ""}`} aria-label="Chain height">
      <div className="orb-scan" />
      <div className="orb-pulse-ring" />
      <div className="orb-pulse-ring delay" />
      <div className="orb-ring ring-a">
        <i />
        <i />
        <i />
        <i />
      </div>
      <div className="orb-ring ring-b">
        <i />
        <i />
        <i />
      </div>
      <div className="orb-ring ring-c">
        <i />
        <i />
      </div>
      <div className="orb-hex" />
      <div className="orb-core">
        <Blocks size={26} />
        <strong>{height ?? "—"}</strong>
        <span>HEIGHT</span>
      </div>
      <div className="orb-tip mono">{shortHash(tipHash, 10)}</div>
      <div className="orb-particles" aria-hidden="true">
        {Array.from({ length: 8 }, (_, i) => (
          <span key={i} style={{ "--i": i } as CSSProperties} />
        ))}
      </div>
    </div>
  );
}
