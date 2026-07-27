/** Grid units for free-form page widgets (12-column board). */
export type WidgetRect = {
  id: string;
  x: number;
  y: number;
  w: number;
  h: number;
  /** When false, widget is removed from the board (can re-add from catalog). */
  visible: boolean;
};

export type PageWidgetLayout = {
  pageId: string;
  version: number;
  items: WidgetRect[];
};

export type WidgetCatalogEntry = {
  id: string;
  label: string;
  description?: string;
  /** Default size when first added */
  defaultW: number;
  defaultH: number;
  minW?: number;
  minH?: number;
  maxW?: number;
  maxH?: number;
};

export const WIDGET_COLS = 12;
export const WIDGET_ROW_PX = 48;
export const WIDGET_GAP_PX = 12;
export const WIDGET_LAYOUT_VERSION = 2;
export const WIDGET_STORAGE_KEY = "alvenqis.widgetLayouts.v2";
