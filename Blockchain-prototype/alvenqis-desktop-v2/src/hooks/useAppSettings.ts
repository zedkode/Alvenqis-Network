import { useCallback, useEffect, useState } from "react";
import type { AppSettings } from "@shared/types";
import { DEFAULT_APP_SETTINGS } from "@shared/settingsDefaults";
import { applySettingsTheme } from "../shared/theme";

export function useAppSettings() {
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_APP_SETTINGS);
  const [loading, setLoading] = useState(true);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      setSettings(await window.alvenqis.settings.get());
    } finally {
      setLoading(false);
    }
  }, []);

  const update = useCallback(async (patch: Partial<AppSettings>) => {
    const next = await window.alvenqis.settings.update(patch);
    setSettings(next);
    return next;
  }, []);

  const reset = useCallback(async () => {
    const next = await window.alvenqis.settings.reset();
    setSettings(next);
    return next;
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  useEffect(() => {
    const root = document.documentElement;
    // Keep dual-theme CSS tokens in sync with persisted settings (including legacy ids).
    applySettingsTheme(settings.theme);
    root.dataset.density = settings.density;
    root.dataset.accent = settings.accent;
    root.dataset.reduceMotion = settings.reduce_motion ? "true" : "false";
    root.dataset.hideBalances = settings.hide_balances ? "true" : "false";
    root.dataset.maskAddresses = settings.mask_addresses ? "true" : "false";
    root.dataset.language = settings.language;
    root.dataset.showTechnical = settings.show_technical_labels ? "true" : "false";
    if (settings.reduce_motion) {
      root.classList.add("reduce-motion");
    } else {
      root.classList.remove("reduce-motion");
    }
  }, [settings]);

  return { settings, loading, reload, update, reset };
}
