use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const SCHEMA_VERSION: u32 = 6;
pub const DEFAULT_DRAWER_WIDTH: f64 = 460.0;
pub const DEFAULT_DRAWER_HEIGHT: f64 = 300.0;
pub const COLLAPSED_HEIGHT: f64 = 48.0;
pub const MIN_DRAWER_WIDTH: f64 = 260.0;
pub const MIN_DRAWER_HEIGHT: f64 = 120.0;
pub const MIN_OPACITY: f64 = 0.45;
pub const DEFAULT_COLOR: &str = "#293548";

/// Ancho lógico mínimo del Dock: el estado vacío necesita espacio para su mensaje.
pub const DOCK_MIN_WIDTH: f64 = 360.0;
/// Margen lógico que el Dock deja libre a cada lado del área útil del monitor.
pub const DOCK_SIDE_MARGIN: f64 = 32.0;
pub const DOCK_PADDING_X: f64 = 12.0;
pub const DOCK_PADDING_Y: f64 = 10.0;
pub const DOCK_CELL_PADDING: f64 = 16.0;
pub const DOCK_MIN_OPACITY: f64 = 0.35;
pub const DOCK_HANDLE_WIDTH: f64 = 58.0;
pub const DOCK_HANDLE_HEIGHT: f64 = 14.0;
pub const DOCK_HANDLE_GAP: f64 = 6.0;
pub const DEFAULT_DOCK_OPACITY: f64 = 0.92;
pub const DOCK_MIN_MANUAL_WIDTH: f64 = 280.0;
pub const DOCK_MAX_MANUAL_WIDTH: f64 = 2_400.0;
pub const DOCK_MIN_HANDLE_WIDTH: f64 = 36.0;
pub const DOCK_MAX_HANDLE_WIDTH: f64 = 160.0;
pub const DOCK_MIN_HANDLE_HEIGHT: f64 = 8.0;
pub const DOCK_MAX_HANDLE_HEIGHT: f64 = 36.0;
pub const DOCK_MIN_HANDLE_OPACITY: f64 = 0.2;
/// Margen de seguridad para desplazamientos persistidos. La posición efectiva
/// siempre se limita al área útil del monitor, incluso en pantallas ultra-wide.
pub const DOCK_MAX_HORIZONTAL_OFFSET: f64 = 10_000.0;
pub const DOCK_MIN_BORDER_RADIUS: f64 = 0.0;
pub const DOCK_MAX_BORDER_RADIUS: f64 = 32.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DockItemKind {
    #[default]
    Shortcut,
    Separator,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DockWidthMode {
    #[default]
    Automatic,
    Manual,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DockSpacing {
    Compact,
    #[default]
    Normal,
    Wide,
}

impl DockSpacing {
    pub const fn logical_gap(self) -> f64 {
        match self {
            Self::Compact => 4.0,
            Self::Normal => 8.0,
            Self::Wide => 14.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DockHandlePosition {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DockAnimationMode {
    #[default]
    Normal,
    Reduced,
    Disabled,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IconSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl IconSize {
    /// Lado en píxeles lógicos del icono dentro del Dock.
    pub const fn dock_pixels(self) -> f64 {
        match self {
            Self::Small => 32.0,
            Self::Medium => 48.0,
            Self::Large => 64.0,
        }
    }
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

/// Acceso del Dock. Comparte con `DrawerItem` el tipo de elemento y la clave de
/// icono, de modo que ambos contenedores usan la misma clasificación y la misma
/// caché nativa sin obligar a una refactorización del módulo Cajones.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockItem {
    pub id: String,
    #[serde(default)]
    pub kind: DockItemKind,
    #[serde(rename = "type")]
    pub item_type: DrawerItemType,
    pub display_name: String,
    pub path: PathBuf,
    pub icon_key: String,
    pub order: u32,
    #[serde(default = "default_true")]
    pub available: bool,
    #[serde(default)]
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockState {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub visible: bool,
    #[serde(default)]
    pub monitor_id: String,
    #[serde(default)]
    pub icon_size: IconSize,
    #[serde(default = "default_dock_opacity")]
    pub opacity: f64,
    #[serde(default = "default_dock_background")]
    pub background_color: String,
    #[serde(default = "default_border_radius")]
    pub border_radius: f64,
    #[serde(default)]
    pub blur: bool,
    #[serde(default)]
    pub performance_mode: bool,
    #[serde(default)]
    pub animation_mode: DockAnimationMode,
    #[serde(default)]
    pub width_mode: DockWidthMode,
    #[serde(default = "default_manual_width")]
    pub manual_width: f64,
    #[serde(default)]
    pub spacing: DockSpacing,
    #[serde(default = "default_handle_width")]
    pub handle_width: f64,
    #[serde(default = "default_handle_height")]
    pub handle_height: f64,
    #[serde(default = "default_handle_opacity")]
    pub handle_opacity: f64,
    #[serde(default)]
    pub handle_position: DockHandlePosition,
    #[serde(default)]
    pub handle_offset: f64,
    #[serde(default)]
    pub shortcut_enabled: bool,
    #[serde(default = "default_shortcut")]
    pub shortcut: String,
    #[serde(default = "default_true")]
    pub hide_after_open: bool,
    #[serde(default)]
    pub items: Vec<DockItem>,
    #[serde(default)]
    pub width: f64,
    #[serde(default)]
    pub height: f64,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub updated_at: u64,
}

impl Default for DockState {
    fn default() -> Self {
        Self {
            enabled: true,
            visible: false,
            monitor_id: String::new(),
            icon_size: IconSize::default(),
            opacity: DEFAULT_DOCK_OPACITY,
            background_color: default_dock_background(),
            border_radius: default_border_radius(),
            blur: false,
            performance_mode: false,
            animation_mode: DockAnimationMode::default(),
            width_mode: DockWidthMode::default(),
            manual_width: default_manual_width(),
            spacing: DockSpacing::default(),
            handle_width: default_handle_width(),
            handle_height: default_handle_height(),
            handle_opacity: default_handle_opacity(),
            handle_position: DockHandlePosition::default(),
            handle_offset: 0.0,
            shortcut_enabled: false,
            shortcut: default_shortcut(),
            hide_after_open: true,
            items: Vec::new(),
            width: DOCK_MIN_WIDTH,
            height: IconSize::Medium.dock_pixels() + DOCK_CELL_PADDING + DOCK_PADDING_Y * 2.0,
            created_at: 0,
            updated_at: 0,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockPatch {
    pub enabled: Option<bool>,
    pub monitor_id: Option<String>,
    pub icon_size: Option<IconSize>,
    pub opacity: Option<f64>,
    pub background_color: Option<String>,
    pub border_radius: Option<f64>,
    pub blur: Option<bool>,
    pub performance_mode: Option<bool>,
    pub animation_mode: Option<DockAnimationMode>,
    pub width_mode: Option<DockWidthMode>,
    pub manual_width: Option<f64>,
    pub spacing: Option<DockSpacing>,
    pub handle_width: Option<f64>,
    pub handle_height: Option<f64>,
    pub handle_opacity: Option<f64>,
    pub handle_position: Option<DockHandlePosition>,
    pub handle_offset: Option<f64>,
    pub shortcut_enabled: Option<bool>,
    pub shortcut: Option<String>,
    pub hide_after_open: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedState {
    pub schema_version: u32,
    pub drawers: Vec<Drawer>,
    #[serde(default)]
    pub preferences: Preferences,
    #[serde(default)]
    pub dock: DockState,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            drawers: Vec::new(),
            preferences: Preferences::default(),
            dock: DockState::default(),
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

const fn default_dock_opacity() -> f64 {
    DEFAULT_DOCK_OPACITY
}

fn default_dock_background() -> String {
    "#10141E".to_owned()
}

const fn default_border_radius() -> f64 {
    14.0
}

const fn default_manual_width() -> f64 {
    720.0
}

const fn default_handle_width() -> f64 {
    DOCK_HANDLE_WIDTH
}

const fn default_handle_height() -> f64 {
    DOCK_HANDLE_HEIGHT
}

const fn default_handle_opacity() -> f64 {
    0.42
}

fn default_shortcut() -> String {
    "CommandOrControl+Alt+D".to_owned()
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
    use super::{
        DockState, Drawer, DrawerMode, IconSize, PersistedState, Preferences, SCHEMA_VERSION,
    };

    #[test]
    fn drawer_identity_survives_json_round_trip() {
        let drawer = Drawer::new("VIDEO".to_owned(), 24, 40, "DISPLAY1@0,0".to_owned(), 123);
        let id = drawer.id.clone();
        let state = PersistedState {
            schema_version: SCHEMA_VERSION,
            drawers: vec![drawer],
            preferences: Preferences::default(),
            dock: DockState::default(),
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
