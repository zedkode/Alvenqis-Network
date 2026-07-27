import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties
} from "react";
import { formatCompactCount, formatHashrate } from "@shared/format";

export interface MiningCoreProps {
  running: boolean;
  hashrate: number | null;
  height: number | null;
  hashesAttempted: string | null;
  acceptedBlocks?: number | null;
  acceptedShares?: number | null;
  /** Network difficulty (leading zero bits) for honest ETA. */
  networkDifficultyBits?: number | null;
  /** Pool share difficulty when mining pool. */
  shareDifficultyBits?: number | null;
  backend?: string | null;
  recentHashes?: string[];
  consoleText?: string;
}

const GLYPHS =
  "アイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワヲン0123456789ABCDEF#@$%&*";

function energyLevel(running: boolean, hashrate: number | null): number {
  if (!running || hashrate === null || hashrate <= 0) return 0;
  // 1e5 H/s → ~0.55, 1e8 → ~0.9, 1e9 → 1
  return Math.min(1, Math.max(0.08, Math.log10(hashrate + 1) / 9));
}

function etaSeconds(hashrate: number | null, bits: number | null | undefined): number | null {
  if (!hashrate || hashrate <= 0 || bits == null || bits < 0 || bits > 62) return null;
  return Math.pow(2, bits) / hashrate;
}

function formatEta(sec: number | null): string {
  if (sec === null || !Number.isFinite(sec)) return "—";
  if (sec < 1) return `${(sec * 1000).toFixed(0)} ms`;
  if (sec < 90) return `${sec.toFixed(1)} s`;
  if (sec < 3600) return `${(sec / 60).toFixed(1)} min`;
  return `${(sec / 3600).toFixed(2)} h`;
}

function cleanHex(raw: string | undefined | null, n = 12): string | null {
  if (!raw) return null;
  const h = raw.replace(/[^a-f0-9]/gi, "").slice(0, n);
  return h.length >= 8 ? h : null;
}

type Drop = {
  x: number;
  y: number;
  speed: number;
  len: number;
  chars: string[];
};

type Particle = {
  x: number;
  y: number;
  vx: number;
  vy: number;
  life: number;
  color: string;
};

/**
 * Matrix rain mining visual — density/speed from real hashrate.
 * Floating block preview while mining; explosion on accepted full block.
 */
export function MiningCore({
  running,
  hashrate,
  height,
  hashesAttempted,
  acceptedBlocks = null,
  acceptedShares = null,
  networkDifficultyBits = null,
  shareDifficultyBits = null,
  backend,
  recentHashes,
  consoleText
}: MiningCoreProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const wrapRef = useRef<HTMLDivElement>(null);
  const dropsRef = useRef<Drop[]>([]);
  const particlesRef = useRef<Particle[]>([]);
  const rafRef = useRef(0);
  const [maxSeen, setMaxSeen] = useState(0);
  const [boom, setBoom] = useState(false);
  const [boomLabel, setBoomLabel] = useState<string | null>(null);
  const [blockChip, setBlockChip] = useState<string | null>(null);
  const lastAccepted = useRef<number | null>(null);
  const acceptedReady = useRef(false);
  const lastShare = useRef<number | null>(null);
  const shareReady = useRef(false);
  const [shareFlash, setShareFlash] = useState(false);

  const energy = energyLevel(running, hashrate);
  const etaBlock = etaSeconds(hashrate, networkDifficultyBits);
  const etaShare = etaSeconds(hashrate, shareDifficultyBits);

  useEffect(() => {
    if (!running || hashrate === null || hashrate <= 0) return;
    setMaxSeen((prev) => (hashrate > prev ? hashrate : prev));
  }, [running, hashrate]);

  // Floating block hash chip (template / tip fragments)
  useEffect(() => {
    if (!running) {
      setBlockChip(null);
      return;
    }
    const next = cleanHex(recentHashes?.[0], 16);
    if (next) setBlockChip(next);
  }, [running, recentHashes]);

  // Share flash
  useEffect(() => {
    if (acceptedShares == null) return;
    if (!shareReady.current) {
      shareReady.current = true;
      lastShare.current = acceptedShares;
      return;
    }
    if (lastShare.current !== null && acceptedShares > lastShare.current) {
      setShareFlash(true);
      const t = window.setTimeout(() => setShareFlash(false), 700);
      lastShare.current = acceptedShares;
      return () => window.clearTimeout(t);
    }
    lastShare.current = acceptedShares;
  }, [acceptedShares]);

  // Block mined explosion
  useEffect(() => {
    if (acceptedBlocks == null) return;
    if (!acceptedReady.current) {
      acceptedReady.current = true;
      lastAccepted.current = acceptedBlocks;
      return;
    }
    if (lastAccepted.current !== null && acceptedBlocks > lastAccepted.current) {
      lastAccepted.current = acceptedBlocks;
      const hash = cleanHex(recentHashes?.[0], 16) ?? "BLOCK";
      setBoomLabel(hash);
      setBoom(true);
      // Spawn particles
      const parts: Particle[] = [];
      for (let i = 0; i < 80; i++) {
        const a = Math.random() * Math.PI * 2;
        const sp = 2 + Math.random() * 7;
        parts.push({
          x: 0.5,
          y: 0.45,
          vx: Math.cos(a) * sp,
          vy: Math.sin(a) * sp,
          life: 1,
          color: Math.random() > 0.5 ? "#20d5ff" : "#e1b05b"
        });
      }
      particlesRef.current = parts;
      const t = window.setTimeout(() => {
        setBoom(false);
        setBoomLabel(null);
      }, 2200);
      return () => window.clearTimeout(t);
    }
    lastAccepted.current = acceptedBlocks;
  }, [acceptedBlocks, recentHashes]);

  // Console block phrases also trigger boom
  useEffect(() => {
    if (!consoleText) return;
    if (/\b(block\s*(found|accepted|mined)|accepted block)\b/i.test(consoleText.slice(-500))) {
      // light share flash only if not already booming
      if (!boom) setShareFlash(true);
    }
  }, [consoleText, boom]);

  // Matrix canvas loop
  useEffect(() => {
    const canvas = canvasRef.current;
    const wrap = wrapRef.current;
    if (!canvas || !wrap) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let w = 0;
    let h = 0;
    const resize = () => {
      const rect = wrap.getBoundingClientRect();
      w = Math.max(320, Math.floor(rect.width));
      h = Math.max(240, Math.floor(rect.height - 56));
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      canvas.width = Math.floor(w * dpr);
      canvas.height = Math.floor(h * dpr);
      canvas.style.width = `${w}px`;
      canvas.style.height = `${h}px`;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      // rebuild columns
      const colW = 14;
      const cols = Math.ceil(w / colW);
      const baseSpeed = running ? 0.35 + energy * 2.2 : 0.12;
      dropsRef.current = Array.from({ length: cols }, (_, i) => ({
        x: i * colW,
        y: Math.random() * h,
        speed: baseSpeed * (0.5 + Math.random()),
        len: 8 + Math.floor(Math.random() * (10 + energy * 20)),
        chars: Array.from({ length: 24 }, () => GLYPHS[(Math.random() * GLYPHS.length) | 0])
      }));
    };
    resize();
    const ro = new ResizeObserver(resize);
    ro.observe(wrap);

    let last = performance.now();
    // Idle matrix rain at ~12 fps; full 60 fps only while mining — saves CPU so
    // Control Center telemetry can keep updating during heavy FiroPoW warm-up.
    const idleFrameMs = 1000 / 12;
    let lastPaint = 0;
    const tick = (now: number) => {
      rafRef.current = requestAnimationFrame(tick);
      if (!running && now - lastPaint < idleFrameMs) {
        return;
      }
      lastPaint = now;
      const dt = Math.min(0.05, (now - last) / 1000);
      last = now;
      const e = energyLevel(running, hashrate);
      const density = running ? 0.08 + e * 0.45 : 0.04;
      const trail = running ? 0.12 + e * 0.08 : 0.2;

      ctx.fillStyle = `rgba(1, 8, 13, ${trail})`;
      ctx.fillRect(0, 0, w, h);

      const fontSize = 13;
      ctx.font = `${fontSize}px "JetBrains Mono", ui-monospace, monospace`;
      ctx.textBaseline = "top";

      for (const d of dropsRef.current) {
        if (Math.random() > density + 0.55) continue;
        d.y += d.speed * (40 + e * 120) * dt;
        if (d.y - d.len * fontSize > h) {
          d.y = -Math.random() * h * 0.3;
          d.speed = (running ? 0.35 + e * 2.2 : 0.12) * (0.5 + Math.random());
          d.len = 8 + Math.floor(Math.random() * (10 + e * 20));
        }
        for (let j = 0; j < d.len; j++) {
          const yy = d.y - j * fontSize;
          if (yy < -fontSize || yy > h) continue;
          if (j === 0) {
            ctx.fillStyle = running ? "#b8f7ff" : "#5a8a9a";
          } else {
            const alpha = Math.max(0.08, 1 - j / d.len);
            ctx.fillStyle = running
              ? `rgba(32, 213, 255, ${alpha * (0.45 + e * 0.55)})`
              : `rgba(80, 120, 130, ${alpha * 0.35})`;
          }
          const ch = d.chars[(j + ((now / 80) | 0)) % d.chars.length];
          ctx.fillText(ch, d.x, yy);
        }
        // occasional glyph mutate
        if (Math.random() < 0.02 + e * 0.05) {
          d.chars[(Math.random() * d.chars.length) | 0] =
            GLYPHS[(Math.random() * GLYPHS.length) | 0];
        }
      }

      // Mining block card in the rain
      if (running && blockChip) {
        const bx = w * 0.5;
        const by = h * 0.42 + Math.sin(now / 900) * 6;
        ctx.save();
        ctx.translate(bx, by);
        ctx.fillStyle = "rgba(8, 28, 40, 0.82)";
        ctx.strokeStyle = shareFlash
          ? "rgba(225, 176, 91, 0.95)"
          : "rgba(32, 213, 255, 0.75)";
        ctx.lineWidth = shareFlash ? 2.5 : 1.4;
        const bw = 168;
        const bh = 54;
        ctx.beginPath();
        ctx.roundRect(-bw / 2, -bh / 2, bw, bh, 8);
        ctx.fill();
        ctx.stroke();
        ctx.fillStyle = "rgba(123, 149, 164, 0.95)";
        ctx.font = '10px "JetBrains Mono", monospace';
        ctx.textAlign = "center";
        ctx.fillText("MINING CANDIDATE", 0, -16);
        ctx.fillStyle = "#20d5ff";
        ctx.font = '13px "JetBrains Mono", monospace';
        ctx.fillText(blockChip, 0, 4);
        ctx.restore();
      }

      // Explosion particles
      if (particlesRef.current.length) {
        const next: Particle[] = [];
        for (const p of particlesRef.current) {
          p.x += (p.vx * dt) / 18;
          p.y += (p.vy * dt) / 18;
          p.vy += 4 * dt;
          p.life -= dt * 0.85;
          if (p.life <= 0) continue;
          ctx.fillStyle = p.color;
          ctx.globalAlpha = Math.max(0, p.life);
          ctx.beginPath();
          ctx.arc(p.x * w, p.y * h, 2 + (1 - p.life) * 3, 0, Math.PI * 2);
          ctx.fill();
          ctx.globalAlpha = 1;
          next.push(p);
        }
        particlesRef.current = next;
      }
    };
    rafRef.current = requestAnimationFrame(tick);
    return () => {
      cancelAnimationFrame(rafRef.current);
      ro.disconnect();
    };
  }, [running, hashrate, energy, blockChip, shareFlash]);

  const hashesLabel = useMemo(() => {
    if (!hashesAttempted) return "hashes n/a";
    const compact = formatCompactCount(hashesAttempted);
    return compact === "—" ? "hashes n/a" : `${compact} hashes`;
  }, [hashesAttempted]);

  const sceneStyle = {
    "--energy": energy.toFixed(3)
  } as CSSProperties;

  return (
    <div
      ref={wrapRef}
      className={[
        "mine-scene matrix-scene",
        running ? "is-mining" : "is-idle",
        shareFlash ? "is-share" : "",
        boom ? "is-boom" : ""
      ]
        .filter(Boolean)
        .join(" ")}
      style={sceneStyle}
      aria-label={running ? "Matrix mining active" : "Matrix mining idle"}
      role="img"
    >
      <canvas ref={canvasRef} className="matrix-canvas" aria-hidden="true" />

      {boom ? (
        <div className="matrix-boom" aria-live="polite">
          <div className="matrix-boom-ring" />
          <div className="matrix-boom-ring r2" />
          <div className="matrix-boom-copy">
            <strong>BLOCK MINED</strong>
            <span className="mono">{boomLabel}</span>
          </div>
        </div>
      ) : null}

      <div className="mine-hud matrix-hud">
        <div className="mine-hud-rate mono">{formatHashrate(hashrate)}</div>
        <div className="mine-hud-meta">
          <span>{running ? "MATRIX LIVE" : "STANDBY"}</span>
          <span className="mono">{backend ?? "—"}</span>
          <span>h={height ?? "—"}</span>
          <span>{hashesLabel}</span>
          {acceptedShares != null ? <span>shares={acceptedShares}</span> : null}
          {acceptedBlocks != null ? <span>blocks={acceptedBlocks}</span> : null}
        </div>
        <div className="mine-hud-eta mono">
          <span title="Protocol target is 60s for the whole network, not per GPU">
            ETA share {formatEta(etaShare)}
          </span>
          <span>ETA block {formatEta(etaBlock)}</span>
          {networkDifficultyBits != null ? <span>net_diff={networkDifficultyBits}</span> : null}
          {maxSeen > 0 ? <span>peak={formatHashrate(maxSeen)}</span> : null}
        </div>
      </div>
    </div>
  );
}
