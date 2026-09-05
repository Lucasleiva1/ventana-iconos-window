import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { DrawerIconSize } from "../types/drawer";
import type { DockAddResult, DockPatch, DockState } from "../types/dock";

const DOCK_CHANGED_EVENT = "dock:changed";

const iconCache = new Map<string, Promise<string | null>>();

export const dockApi = {
  getState: () => invoke<DockState>("get_dock_state"),
  toggle: () => invoke<DockState>("toggle_dock"),
  setVisible: (visible: boolean) => invoke<DockState>("set_dock_visible", { visible }),
  moveHandle: (deltaX: number) =>
    invoke<DockState>("move_dock_handle", { deltaX }),
  finishHandleDrag: () => invoke<DockState>("finish_dock_handle_drag"),
  updateSettings: (patch: DockPatch) => invoke<DockState>("update_dock_settings", { patch }),
  setIconSize: (iconSize: DrawerIconSize) =>
    invoke<DockState>("set_dock_icon_size", { iconSize }),
  addItems: (paths: string[]) => invoke<DockAddResult>("add_dock_items", { paths }),
  addSeparator: () => invoke<DockState>("add_dock_separator"),
  removeItem: (itemId: string) => invoke<DockState>("remove_dock_item", { itemId }),
  renameItem: (itemId: string, name: string) =>
    invoke<DockState>("rename_dock_item", { itemId, name }),
  repairItem: (itemId: string) => invoke<DockState>("repair_dock_item", { itemId }),
  reorder: (orderedItemIds: string[]) =>
    invoke<DockState>("reorder_dock_items", { orderedItemIds }),
  openItem: (itemId: string) => invoke<DockState>("open_dock_item", { itemId }),
  openItemLocation: (itemId: string) =>
    invoke<void>("open_dock_item_location", { itemId }),
  refreshAvailability: () => invoke<DockState>("refresh_dock_availability"),
  relayout: () => invoke<DockState>("relayout_dock"),
  /** Reutiliza la caché de iconos del proceso: nunca vuelve a extraer el mismo. */
  getItemIcon: (itemId: string, iconKey: string) => {
    const cached = iconCache.get(iconKey);
    if (cached) return cached;
    const icon = invoke<string | null>("get_dock_item_icon", { itemId }).catch(() => null);
    iconCache.set(iconKey, icon);
    return icon;
  },
  forgetIcon: (iconKey: string) => iconCache.delete(iconKey),
  onDockChanged: (callback: (dock: DockState) => void) =>
    listen<DockState>(DOCK_CHANGED_EVENT, (event) => callback(event.payload)),
};
