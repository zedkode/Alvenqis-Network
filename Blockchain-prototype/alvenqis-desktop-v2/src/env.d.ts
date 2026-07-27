/// <reference types="vite/client" />

import type { AlvenqisBridge } from "@shared/types";

declare global {
  interface Window {
    alvenqis: AlvenqisBridge;
  }
}

export {};
