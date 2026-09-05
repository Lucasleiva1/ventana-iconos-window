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
  items: PanelItem[];
  createdAt: number;
  updatedAt: number;
}

export interface PanelPatch {
  name?: string;
  locked?: boolean;
  color?: string;
  opacity?: number;
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
