import { useEffect, useRef, useState } from "react";
import { prefersReducedMotion } from "../shared/chartPath";

/**
 * Interpolates between real numeric samples only. Does not invent values —
 * starts from the first real value and eases to each subsequent real target.
 */
export function useAnimatedNumber(target: number | null | undefined, durationMs = 450): number | null {
  const [display, setDisplay] = useState<number | null>(
    target === null || target === undefined || Number.isNaN(target) ? null : target
  );
  const fromRef = useRef(display ?? 0);
  const rafRef = useRef(0);

  useEffect(() => {
    if (target === null || target === undefined || Number.isNaN(target)) {
      setDisplay(null);
      return;
    }

    if (prefersReducedMotion()) {
      setDisplay(target);
      fromRef.current = target;
      return;
    }

    const from = display ?? target;
    fromRef.current = from;
    if (from === target) {
      setDisplay(target);
      return;
    }

    const start = performance.now();
    const delta = target - from;
    cancelAnimationFrame(rafRef.current);

    const tick = (now: number) => {
      const t = Math.min(1, (now - start) / durationMs);
      // ease-standard approximation
      const eased = 1 - Math.pow(1 - t, 3);
      const next = from + delta * eased;
      setDisplay(next);
      if (t < 1) {
        rafRef.current = requestAnimationFrame(tick);
      } else {
        setDisplay(target);
        fromRef.current = target;
      }
    };

    rafRef.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(rafRef.current);
    // Only re-run when target changes; display is intermediate.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [target, durationMs]);

  return display;
}
