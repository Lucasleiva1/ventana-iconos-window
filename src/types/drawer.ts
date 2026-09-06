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
  dock: import("./dock").DockState;
  panels: import("./panel").Panel[];
}

export interface Preferences {
  startWithWindows: boolean;
  hideAdminOnMinimize: boolean;
  startSilently: boolean;
  checkUpdatesAutomatically: boolean;
  lastUpdateCheck: number;
  lastNotifiedVersion: string | null;
  performanceMode: boolean;
  animationMode: "normal" | "reduced" | "disabled";
  defaultPanelDensity: import("./panel").PanelDensity;
  defaultPanelIconMode: import("./panel").PanelIconMode;
  defaultPanelSnapEnabled: boolean;
  defaultPanelLocked: boolean;
}

export interface PreferencesPatch {
  startWithWindows?: boolean;
  hideAdminOnMinimize?: boolean;
  startSilently?: boolean;
  checkUpdatesAutomatically?: boolean;
  performanceMode?: boolean;
  animationMode?: Preferences["animationMode"];
  defaultPanelDensity?: Preferences["defaultPanelDensity"];
  defaultPanelIconMode?: Preferences["defaultPanelIconMode"];
  defaultPanelSnapEnabled?: boolean;
  defaultPanelLocked?: boolean;
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
  backups: string;
  logs: string;
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

export interface BackupInfo {
  fileName: string;
  createdAt: number;
  size: number;
  schemaVersion: number | null;
  valid: boolean;
}

export interface BackupCenter {
  saveValid: boolean;
  backups: BackupInfo[];
  storage: StoragePathsInfo;
}

export interface HealthItem {
  label: string;
  ok: boolean;
  detail: string;
}

export interface DiagnosticReport {
  appVersion: string;
  schemaVersion: number;
  dataPath: string;
  saveValid: boolean;
  drawerCount: number;
  dockItemCount: number;
  panelCount: number;
  monitors: MonitorInfo[];
  startWithWindows: boolean;
  updaterStatus: string;
  backupCount: number;
  health: HealthItem[];
  text: string;
}
