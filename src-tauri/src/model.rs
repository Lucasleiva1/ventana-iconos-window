use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const SCHEMA_VERSION: u32 = 4;
pub const DEFAULT_DRAWER_WIDTH: f64 = 460.0;
pub const DEFAULT_DRAWER_HEIGHT: f64 = 300.0;
pub const COLLAPSED_HEIGHT: f64 = 48.0;
pub const MIN_DRAWER_WIDTH: f64 = 260.0;
pub const MIN_DRAWER_HEIGHT: f64 = 120.0;
pub const MIN_OPACITY: f64 = 0.45;
pub const DEFAULT_COLOR: &str = "#293548";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IconSize {
    Small,
    #[default]
    Medium,
    Large,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DrawerItemType {
    Folder,
    Executable,
    Shortcut,
    File,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StorageMode {
    Managed,
    #[default]
    Linked,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DrawerMode {
    #[default]
    Managed,
    LinkedFolder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawerItem {
    pub id: String,
    pub drawer_id: String,
    #[serde(rename = "type")]
    pub item_type: DrawerItemType,
    #[serde(default)]
    pub storage_mode: StorageMode,
    pub display_name: String,
    #[serde(default)]
    pub physical_name: String,
    #[serde(alias = "originalPath")]
    pub path: PathBuf,
    pub icon_key: String,
    pub created_at: u64,
    pub order: u32,
    pub available: bool,
    #[serde(default)]
    pub container_path: String,
    #[serde(default)]
    pub is_subdrawer: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Drawer {
    pub id: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: f64,
    pub height: f64,
    pub expanded_width: f64,
    pub expanded_height: f64,
    pub collapsed: bool,
    pub hidden: bool,
    pub locked: bool,
    pub color: String,
    pub opacity: f64,
    pub monitor_id: String,
    #[serde(default)]
    pub drawer_mode: DrawerMode,
    #[serde(default)]
    pub folder_path: PathBuf,
    #[serde(default)]
    pub icon_size: IconSize,
    #[serde(default)]
    pub items: Vec<DrawerItem>,
    #[serde(default)]
    pub level_orders: BTreeMap<String, Vec<String>>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl Drawer {
    pub fn new(name: String, x: i32, y: i32, monitor_id: String, now: u64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            x,
            y,
            width: DEFAULT_DRAWER_WIDTH,
            height: DEFAULT_DRAWER_HEIGHT,
            expanded_width: DEFAULT_DRAWER_WIDTH,
            expanded_height: DEFAULT_DRAWER_HEIGHT,
            collapsed: false,
            hidden: false,
            locked: false,
            color: DEFAULT_COLOR.to_owned(),
            opacity: 0.94,
            monitor_id,
            drawer_mode: DrawerMode::default(),
            folder_path: PathBuf::new(),
            icon_size: IconSize::default(),
            items: Vec::new(),
            level_orders: BTreeMap::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedState {
    pub schema_version: u32,
    pub drawers: Vec<Drawer>,
    #[serde(default)]
    pub preferences: Preferences,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            drawers: Vec::new(),
            preferences: Preferences::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    #[serde(default)]
    pub start_with_windows: bool,
    #[serde(default = "default_true")]
    pub hide_admin_on_minimize: bool,
    #[serde(default = "default_true")]
    pub start_silently: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            start_with_windows: false,
            hide_admin_on_minimize: true,
            start_silently: true,
        }
    }
}

const fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferencesPatch {
    pub start_with_windows: Option<bool>,
    pub hide_admin_on_minimize: Option<bool>,
    pub start_silently: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawerBreadcrumb {
    pub name: String,
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawerLevel {
    pub root_drawer_id: String,
    pub relative_path: String,
    pub folder_path: PathBuf,
    pub breadcrumbs: Vec<DrawerBreadcrumb>,
    pub items: Vec<DrawerItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawerPatch {
    pub name: Option<String>,
    pub locked: Option<bool>,
    pub color: Option<String>,
    pub opacity: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawerGeometryInput {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub width: f64,
    pub height: f64,
}

#[cfg(test)]
mod tests {
    use super::{Drawer, DrawerMode, IconSize, PersistedState, Preferences, SCHEMA_VERSION};

    #[test]
    fn drawer_identity_survives_json_round_trip() {
        let drawer = Drawer::new("VIDEO".to_owned(), 24, 40, "DISPLAY1@0,0".to_owned(), 123);
        let id = drawer.id.clone();
        let state = PersistedState {
            schema_version: SCHEMA_VERSION,
            drawers: vec![drawer],
            preferences: Preferences::default(),
        };

        let json = serde_json::to_string(&state).expect("test state should serialize");
        let restored: PersistedState =
            serde_json::from_str(&json).expect("test state should deserialize");

        assert_eq!(restored.schema_version, SCHEMA_VERSION);
        assert_eq!(restored.drawers[0].id, id);
        assert_eq!(restored.drawers[0].name, "VIDEO");
        assert_eq!(restored.drawers[0].icon_size, IconSize::Medium);
        assert_eq!(restored.drawers[0].drawer_mode, DrawerMode::Managed);
        assert!(restored.drawers[0].items.is_empty());
    }
}
