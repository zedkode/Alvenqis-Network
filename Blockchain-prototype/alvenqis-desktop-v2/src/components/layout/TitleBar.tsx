import { Maximize2, Minus, X } from "lucide-react";
import { AlvenqisLogo } from "../brand/AlvenqisLogo";

export function TitleBar() {
  const platform = window.alvenqis.app.platform;
  const product =
    platform === "linux"
      ? "Alvenqis Linux"
      : platform === "windows"
        ? "Alvenqis Control Center"
        : "Alvenqis";

  return (
    <div className="titlebar" data-tauri-drag-region>
      <div className="titlebar-brand" data-tauri-drag-region>
        <AlvenqisLogo size="xs" alt="" />
        <strong>{product}</strong>
        <span className="v2-pill" data-tauri-drag-region>
          V2
        </span>
        <small data-tauri-drag-region>Mainnet Candidate</small>
      </div>
      <div className="window-actions">
        <button type="button" aria-label="Minimize" onClick={() => void window.alvenqis.app.minimize()}>
          <Minus size={14} strokeWidth={1.75} />
        </button>
        <button type="button" aria-label="Maximize" onClick={() => void window.alvenqis.app.maximize()}>
          <Maximize2 size={13} strokeWidth={1.75} />
        </button>
        <button type="button" aria-label="Close" onClick={() => void window.alvenqis.app.close()}>
          <X size={14} strokeWidth={1.75} />
        </button>
      </div>
    </div>
  );
}
