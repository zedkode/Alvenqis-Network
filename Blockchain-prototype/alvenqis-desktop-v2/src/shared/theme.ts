import { useCallback, useEffect, useState } from "react";
import type { ThemeId as SettingsThemeId } from "@shared/types";

/** Applied base shell for CSS [data-theme]. */
export type ThemeBase = "dark" | "light";
export type ThemeVariant = "default" | "midnight" | "high-contrast";

export const THEME_STORAGE_KEY = "alvenqis.theme";
export const THEME_VARIANT_KEY = "alvenqis.theme.variant";

export function normalizeTheme(id: string | null | undefined): {
  base: ThemeBase;
  variant: ThemeVariant;
  settingsId: SettingsThemeId;
} {
  switch (id) {
    case "light":
      return { base: "light", variant: "default", settingsId: "light" };
    case "midnight":
    case "alvenqis-midnight":
      return { base: "dark", variant: "midnight", settingsId: "midnight" };
    case "high-contrast":
      return { base: "dark", variant: "high-contrast", settingsId: "high-contrast" };
    case "dark":
    case "alvenqis-dark":
    default:
      return { base: "dark", variant: "default", settingsId: "dark" };
  }
}

export function applyTheme(base: ThemeBase, variant: ThemeVariant = "default"): void {
  document.documentElement.setAttribute("data-theme", base);
  document.documentElement.setAttribute("data-theme-variant", variant);
  document.documentElement.style.colorScheme = base;
  const meta = document.querySelector('meta[name="color-scheme"]');
  if (meta) meta.setAttribute("content", base === "light" ? "light" : "dark");
  const themeMeta = document.querySelector('meta[name="theme-color"]');
  if (themeMeta) {
    themeMeta.setAttribute("content", base === "light" ? "#eef4f8" : "#030c12");
  }
  try {
    localStorage.setItem(THEME_STORAGE_KEY, base);
    localStorage.setItem(THEME_VARIANT_KEY, variant);
  } catch {
    /* ignore */
  }
}

export function applySettingsTheme(id: string | null | undefined): SettingsThemeId {
  const { base, variant, settingsId } = normalizeTheme(id);
  applyTheme(base, variant);
  return settingsId;
}

export function resolveInitialTheme(): { base: ThemeBase; variant: ThemeVariant } {
  try {
    const stored = localStorage.getItem(THEME_STORAGE_KEY);
    const variant = (localStorage.getItem(THEME_VARIANT_KEY) as ThemeVariant | null) ?? "default";
    if (stored === "dark" || stored === "light") {
      return {
        base: stored,
        variant: variant === "midnight" || variant === "high-contrast" ? variant : "default"
      };
    }
  } catch {
    /* ignore */
  }
  if (typeof window !== "undefined" && window.matchMedia?.("(prefers-color-scheme: light)").matches) {
    return { base: "light", variant: "default" };
  }
  return { base: "dark", variant: "default" };
}

export function bootTheme(): ThemeBase {
  const { base, variant } = resolveInitialTheme();
  applyTheme(base, variant);
  return base;
}

export function useTheme(): {
  theme: ThemeBase;
  variant: ThemeVariant;
  setTheme(next: ThemeBase): void;
  toggleTheme(): void;
  setFromSettings(id: string): void;
} {
  const initial = resolveInitialTheme();
  const [theme, setThemeState] = useState<ThemeBase>(() => {
    if (typeof document !== "undefined") {
      const attr = document.documentElement.getAttribute("data-theme");
      if (attr === "dark" || attr === "light") return attr;
    }
    return initial.base;
  });
  const [variant, setVariant] = useState<ThemeVariant>(() => {
    if (typeof document !== "undefined") {
      const v = document.documentElement.getAttribute("data-theme-variant");
      if (v === "midnight" || v === "high-contrast") return v;
    }
    return initial.variant;
  });

  useEffect(() => {
    applyTheme(theme, variant);
  }, [theme, variant]);

  const setTheme = useCallback((next: ThemeBase) => {
    setThemeState(next);
    if (next === "light") setVariant("default");
  }, []);

  const toggleTheme = useCallback(() => {
    setThemeState((current) => {
      const next = current === "dark" ? "light" : "dark";
      if (next === "light") setVariant("default");
      return next;
    });
  }, []);

  const setFromSettings = useCallback((id: string) => {
    const n = normalizeTheme(id);
    setThemeState(n.base);
    setVariant(n.variant);
  }, []);

  return { theme, variant, setTheme, toggleTheme, setFromSettings };
}
