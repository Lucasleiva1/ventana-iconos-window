import type { DrawerIconSize, DrawerItemType } from "./drawer";

/**
 * Acceso del Dock. Comparte `type` e `iconKey` con los elementos de Cajones,
 * de modo que ambos módulos usan la misma clasificación y la misma caché de
 * iconos nativos de Windows.
 */
export interface DockItem {
  id: string;
  kind: "shortcut" | "separator";
  type: DrawerItemType;
  displayName: string;
  path: string;
  iconKey: string;
  order: number;
  available: boolean;
  createdAt: number;
}

export interface DockState {
  enabled: boolean;
  visible: boolean;
  monitorId: string;
  iconSize: DrawerIconSize;
  opacity: number;
  backgroundColor: string;
  borderRadius: number;
  blur: boolean;
  performanceMode: boolean;
  animationMode: "normal" | "reduced" | "disabled";
  widthMode: "automatic" | "manual";
  manualWidth: number;
  spacing: "compact" | "normal" | "wide";
  handleWidth: number;
  handleHeight: number;
  handleOpacity: number;
  handlePosition: "left" | "center" | "right";
  handleOffset: number;
  shortcutEnabled: boolean;
  shortcut: string;
  hideAfterOpen: boolean;
  items: DockItem[];
  width: number;
  height: number;
  createdAt: number;
  updatedAt: number;
}

export interface DockPatch {
  enabled?: boolean;
  monitorId?: string;
  iconSize?: DrawerIconSize;
  opacity?: number;
  backgroundColor?: string;
  borderRadius?: number;
  blur?: boolean;
  performanceMode?: boolean;
  animationMode?: DockState["animationMode"];
  widthMode?: DockState["widthMode"];
  manualWidth?: number;
  spacing?: DockState["spacing"];
  handleWidth?: number;
  handleHeight?: number;
  handleOpacity?: number;
  handlePosition?: DockState["handlePosition"];
  handleOffset?: number;
  shortcutEnabled?: boolean;
  shortcut?: string;
  hideAfterOpen?: boolean;
}

export interface DockAddFailure {
  path: string;
  message: string;
}

export interface DockAddResult {
  dock: DockState;
  added: number;
  duplicates: number;
  failures: DockAddFailure[];
}
