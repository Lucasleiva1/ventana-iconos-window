import type { DrawerItemType } from "./drawer";

/**
 * Acceso de un Panel organizador.
 *
 * Siempre es una referencia: el archivo, la carpeta o el programa original
 * permanecen donde estaban. `drawerId` sólo está presente cuando el elemento
 * representa un Cajón colocado dentro del Panel.
 */
export interface PanelItem {
  id: string;
  type: DrawerItemType;
  displayName: string;
  path: string;
  iconKey: string;
  order: number;
  available: boolean;
  drawerId: string | null;
  createdAt: number;
}

export type PanelDensity = "compact" | "normal" | "wide";
export type PanelIconMode = "auto" | "manual";
export type PanelHeaderMode = "normal" | "compact";
export type PanelBackgroundStyle = "solid" | "translucent" | "glass" | "minimal";
export type PanelAlignment =
  | "left"
  | "top"
  | "distributeHorizontally"
  | "distributeVertically";

export interface PanelGeometry {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Panel {
  id: string;
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  monitorId: string;
  locked: boolean;
  hidden: boolean;
  color: string;
  opacity: number;
  autoIconSize: boolean;
  density: PanelDensity;
  iconMode: PanelIconMode;
  manualIconSize: number;
  snapEnabled: boolean;
  /** Bloquea reordenar y quitar. Independiente de `locked`, que fija posición. */
  lockContent: boolean;
  headerMode: PanelHeaderMode;
  showTitle: boolean;
  backgroundStyle: PanelBackgroundStyle;
  expanded: boolean;
  previousGeometry: PanelGeometry | null;
  items: PanelItem[];
  createdAt: number;
  updatedAt: number;
}

export interface PanelPatch {
  name?: string;
  locked?: boolean;
  color?: string;
  opacity?: number;
  density?: PanelDensity;
  iconMode?: PanelIconMode;
  manualIconSize?: number;
  snapEnabled?: boolean;
  lockContent?: boolean;
  headerMode?: PanelHeaderMode;
  showTitle?: boolean;
  backgroundStyle?: PanelBackgroundStyle;
}

export interface PanelAddFailure {
  path: string;
  message: string;
}

export interface PanelAddResult {
  panel: Panel;
  added: number;
  duplicates: number;
  failures: PanelAddFailure[];
}
