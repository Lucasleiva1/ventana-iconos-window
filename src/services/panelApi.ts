import { invoke } from "@tauri-apps/api/core";
import type { PersistedState } from "../types/drawer";
import type { Panel, PanelAddResult, PanelAlignment, PanelPatch } from "../types/panel";

const iconCache = new Map<string, Promise<string | null>>();

export const panelApi = {
  create: (name: string) => invoke<Panel>("create_panel", { name }),
  update: (id: string, patch: PanelPatch) => invoke<Panel>("update_panel", { id, patch }),
  remove: (id: string) => invoke<PersistedState>("delete_panel", { id }),
  setHidden: (id: string, hidden: boolean) =>
    invoke<Panel>("set_panel_hidden", { id, hidden }),
  setAllHidden: (hidden: boolean) =>
    invoke<PersistedState>("set_all_panels_hidden", { hidden }),
  /** `suspendSnap` viaja en true cuando el usuario mantiene Alt: ese gesto se mueve libre. */
  beginDrag: (suspendSnap: boolean) => invoke<void>("begin_panel_drag", { suspendSnap }),
  recordGeometry: (id: string, x: number, y: number, width: number, height: number) =>
    invoke<void>("record_panel_geometry", { geometry: { id, x, y, width, height } }),
  relayout: (id: string) => invoke<Panel>("relayout_panel", { id }),
  addItems: (panelId: string, paths: string[]) =>
    invoke<PanelAddResult>("add_panel_items", { panelId, paths }),
  addDrawer: (panelId: string, drawerId: string) =>
    invoke<Panel>("add_panel_drawer", { panelId, drawerId }),
  removeItem: (panelId: string, itemId: string) =>
    invoke<Panel>("remove_panel_item", { panelId, itemId }),
  removeItems: (panelId: string, itemIds: string[]) =>
    invoke<Panel>("remove_panel_items", { panelId, itemIds }),
  duplicate: (id: string) => invoke<Panel>("duplicate_panel", { id }),
  setExpanded: (id: string, expanded: boolean) =>
    invoke<Panel>("set_panel_expanded", { id, expanded }),
  refresh: (panelId: string) => invoke<Panel>("refresh_panel", { panelId }),
  align: (alignment: PanelAlignment) => invoke<PersistedState>("align_panels", { alignment }),
  renameItem: (panelId: string, itemId: string, name: string) =>
    invoke<Panel>("rename_panel_item", { panelId, itemId, name }),
  reorder: (panelId: string, orderedItemIds: string[]) =>
    invoke<Panel>("reorder_panel_items", { panelId, orderedItemIds }),
  openItem: (panelId: string, itemId: string) =>
    invoke<void>("open_panel_item", { panelId, itemId }),
  openItemLocation: (panelId: string, itemId: string) =>
    invoke<void>("open_panel_item_location", { panelId, itemId }),
  repairItem: (panelId: string, itemId: string) =>
    invoke<Panel>("repair_panel_item", { panelId, itemId }),
  refreshAvailability: (panelId: string) =>
    invoke<Panel>("refresh_panel_availability", { panelId }),
  /** Reutiliza la caché de iconos del proceso: nunca extrae dos veces el mismo. */
  getItemIcon: (panelId: string, itemId: string, iconKey: string) => {
    const cached = iconCache.get(iconKey);
    if (cached) return cached;
    const icon = invoke<string | null>("get_panel_item_icon", { panelId, itemId })
      .catch(() => null);
    iconCache.set(iconKey, icon);
    return icon;
  },
  forgetIcon: (iconKey: string) => iconCache.delete(iconKey),
};
