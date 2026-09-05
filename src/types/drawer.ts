export type DrawerIconSize = "small" | "medium" | "large";
export type DrawerItemType = "folder" | "executable" | "shortcut" | "file";
export type StorageMode = "managed" | "linked";
export type DrawerMode = "managed" | "linkedFolder";

export interface DrawerItem {
  id: string;
  drawerId: string;
  type: DrawerItemType;
  storageMode: StorageMode;
  displayName: string;
  physicalName: string;
  path: string;
  iconKey: string;
  createdAt: number;
  order: number;
  available: boolean;
  containerPath: string;
  isSubdrawer: boolean;
}

export interface Drawer {
  id: string;
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  expandedWidth: number;
  expandedHeight: number;
  collapsed: boolean;
  hidden: boolean;
  locked: boolean;
  color: string;
  opacity: number;
  monitorId: string;
  drawerMode: DrawerMode;
  folderPath: string;
  iconSize: DrawerIconSize;
  items: DrawerItem[];
  levelOrders: Record<string, string[]>;
  createdAt: number;
  updatedAt: number;
}

export interface AddItemFailure {
  path: string;
  message: string;
}

export interface AddItemsResult {
  added: DrawerItem[];
  duplicates: string[];
  failures: AddItemFailure[];
  warnings: string[];
}

export interface PersistedState {
  schemaVersion: number;
  drawers: Drawer[];
  preferences: Preferences;
  panels: import("./panel").Panel[];
}

export interface Preferences {
  startWithWindows: boolean;
  hideAdminOnMinimize: boolean;
  startSilently: boolean;
}

export interface PreferencesPatch {
  startWithWindows?: boolean;
  hideAdminOnMinimize?: boolean;
  startSilently?: boolean;
}

export interface DrawerBreadcrumb {
  name: string;
  relativePath: string;
}

export interface DrawerLevel {
  rootDrawerId: string;
  relativePath: string;
  folderPath: string;
  breadcrumbs: DrawerBreadcrumb[];
  items: DrawerItem[];
}

export interface InternalDragPayload {
  sourceDrawerId: string;
  sourceRelativePath: string;
  itemId: string;
}

export interface DrawerPatch {
  name?: string;
  locked?: boolean;
  color?: string;
  opacity?: number;
}

export interface MonitorArea {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface MonitorInfo {
  id: string;
  name: string;
  primary: boolean;
  scaleFactor: number;
  position: { x: number; y: number };
  size: { width: number; height: number };
  workArea: MonitorArea;
}

export interface StoragePathsInfo {
  documents: string;
  desktop: string;
  root: string;
  drawers: string;
  dock: string;
  masterSave: string;
}

export interface RecoveryStatus {
  masterSaveExists: boolean;
  recoverableDrawers: number;
  storage: StoragePathsInfo;
  notice: string | null;
}

export interface ItemActionResult {
  drawer: Drawer;
  message: string;
}
