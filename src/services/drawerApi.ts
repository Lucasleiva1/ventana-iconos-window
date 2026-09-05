import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AddItemsResult,
  Drawer,
  DrawerLevel,
  DrawerIconSize,
  DrawerPatch,
  ItemActionResult,
  MonitorInfo,
  PersistedState,
  Preferences,
  PreferencesPatch,
  RecoveryStatus,
  StoragePathsInfo,
} from "../types/drawer";

const STATE_CHANGED_EVENT = "drawers:changed";

export const drawerApi = {
  getState: () => invoke<PersistedState>("get_app_state"),
  getMonitors: () => invoke<MonitorInfo[]>("get_monitors"),
  getStorageInfo: () => invoke<StoragePathsInfo>("get_storage_info"),
  openDrawersRoot: () => invoke<void>("open_drawers_root"),
  openDockRoot: () => invoke<void>("open_dock_root"),
  getPreferences: () => invoke<Preferences>("get_preferences"),
  updatePreferences: (patch: PreferencesPatch) =>
    invoke<Preferences>("update_preferences", { patch }),
  getRecoveryStatus: () => invoke<RecoveryStatus>("get_recovery_status"),
  addItems: (drawerId: string, paths: string[]) =>
    invoke<AddItemsResult>("add_drawer_items", { drawerId, paths }),
  removeItem: (drawerId: string, itemId: string) =>
    invoke<Drawer>("remove_drawer_item", { drawerId, itemId }),
  restoreItem: (drawerId: string, itemId: string) =>
    invoke<ItemActionResult>("restore_drawer_item", { drawerId, itemId }),
  moveManagedItem: (drawerId: string, itemId: string) =>
    invoke<ItemActionResult | null>("move_managed_item", { drawerId, itemId }),
  setIconSize: (drawerId: string, iconSize: DrawerIconSize) =>
    invoke<Drawer>("set_drawer_icon_size", { drawerId, iconSize }),
  refreshAvailability: (drawerId: string) =>
    invoke<Drawer>("refresh_drawer_availability", { drawerId }),
  openItem: (drawerId: string, itemId: string) =>
    invoke<void>("open_drawer_item", { drawerId, itemId }),
  openFolder: (drawerId: string) =>
    invoke<void>("open_drawer_folder", { drawerId }),
  getItemIcon: (drawerId: string, itemId: string) =>
    invoke<string | null>("get_drawer_item_icon", { drawerId, itemId }),
  loadLevel: (drawerId: string, relativePath: string) =>
    invoke<DrawerLevel>("load_drawer_level", { drawerId, relativePath }),
  addItemsAtLevel: (drawerId: string, relativePath: string, paths: string[]) =>
    invoke<AddItemsResult>("add_drawer_items_at_level", { drawerId, relativePath, paths }),
  refreshLevel: (drawerId: string, relativePath: string) =>
    invoke<DrawerLevel>("refresh_drawer_level", { drawerId, relativePath }),
  openLevelItem: (drawerId: string, relativePath: string, itemId: string) =>
    invoke<void>("open_level_item", { drawerId, relativePath, itemId }),
  openLevelItemLocation: (drawerId: string, relativePath: string, itemId: string) =>
    invoke<void>("open_level_item_location", { drawerId, relativePath, itemId }),
  getLevelItemIcon: (drawerId: string, relativePath: string, itemId: string) =>
    invoke<string | null>("get_level_item_icon", { drawerId, relativePath, itemId }),
  removeLevelLink: (drawerId: string, relativePath: string, itemId: string) =>
    invoke<DrawerLevel>("remove_level_link", { drawerId, relativePath, itemId }),
  createSubdrawer: (drawerId: string, relativePath: string, name: string) =>
    invoke<DrawerLevel>("create_subdrawer", { drawerId, relativePath, name }),
  convertFolderToSubdrawer: (drawerId: string, relativePath: string, itemId: string) =>
    invoke<DrawerLevel>("convert_folder_to_subdrawer", { drawerId, relativePath, itemId }),
  convertSubdrawerToFolder: (drawerId: string, relativePath: string, itemId: string) =>
    invoke<DrawerLevel>("convert_subdrawer_to_folder", { drawerId, relativePath, itemId }),
  renameSubdrawer: (
    drawerId: string,
    relativePath: string,
    itemId: string,
    name: string,
  ) => invoke<DrawerLevel>("rename_subdrawer", { drawerId, relativePath, itemId, name }),
  reorderLevel: (drawerId: string, relativePath: string, orderedItemIds: string[]) =>
    invoke<DrawerLevel>("reorder_drawer_level", { drawerId, relativePath, orderedItemIds }),
  restoreLevelItem: (drawerId: string, relativePath: string, itemId: string) =>
    invoke<string>("restore_level_item", { drawerId, relativePath, itemId }),
  beginNativeItemDrag: (drawerId: string, relativePath: string, itemId: string) =>
    invoke<void>("begin_native_item_drag", { drawerId, relativePath, itemId }),
  moveItemBetweenLevels: (
    sourceDrawerId: string,
    sourceRelativePath: string,
    itemId: string,
    targetDrawerId: string,
    targetRelativePath: string,
  ) => invoke<PersistedState>("move_item_between_levels", {
    sourceDrawerId,
    sourceRelativePath,
    itemId,
    targetDrawerId,
    targetRelativePath,
  }),
  create: (name: string) => invoke<Drawer>("create_drawer", { name }),
  update: (id: string, patch: DrawerPatch) =>
    invoke<Drawer>("update_drawer", { id, patch }),
  setCollapsed: (id: string, collapsed: boolean) =>
    invoke<Drawer>("set_drawer_collapsed", { id, collapsed }),
  setHidden: (id: string, hidden: boolean) =>
    invoke<Drawer>("set_drawer_hidden", { id, hidden }),
  setAllHidden: (hidden: boolean) =>
    invoke<PersistedState>("set_all_drawers_hidden", { hidden }),
  remove: (id: string) => invoke<PersistedState>("delete_drawer", { id }),
  exportConfiguration: () =>
    invoke<string | null>("export_configuration"),
  importConfiguration: () =>
    invoke<PersistedState | null>("import_configuration"),
  recoverFromDisk: () =>
    invoke<PersistedState>("recover_drawers_from_disk"),
  startFresh: () =>
    invoke<PersistedState>("start_fresh_configuration"),
  beginDrag: () => invoke<void>("begin_drawer_drag"),
  recordGeometry: (
    id: string,
    x: number,
    y: number,
    width: number,
    height: number,
  ) =>
    invoke<void>("record_drawer_geometry", {
      geometry: { id, x, y, width, height },
    }),
  onStateChanged: (callback: (state: PersistedState) => void) =>
    listen<PersistedState>(STATE_CHANGED_EVENT, (event) => callback(event.payload)),
  onFeedback: (callback: (message: string) => void) =>
    listen<string>("drawers:feedback", (event) => callback(event.payload)),
  onFocusNewDrawer: (callback: () => void) =>
    listen("admin:focus-new-drawer", callback),
  onOpenSettings: (callback: () => void) =>
    listen("admin:open-settings", callback),
};

export type { UnlistenFn };
