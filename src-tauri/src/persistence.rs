use std::{
    collections::HashSet,
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::{
    model::{
        DEFAULT_DRAWER_HEIGHT, DEFAULT_DRAWER_WIDTH, DEFAULT_PANEL_OPACITY, DockState, Drawer,
        MIN_OPACITY, PANEL_DEFAULT_HEIGHT, PANEL_DEFAULT_WIDTH, PANEL_MIN_OPACITY, Panel,
        PersistedState, Preferences, SCHEMA_VERSION,
    },
    storage_service::{StoragePaths, StorageService, atomic_replace},
};

const LEGACY_STATE_FILE_NAMES: [&str; 2] = ["drawers-state-v2.json", "drawers-state-v1.json"];
const MAX_BACKUPS: usize = 8;
static SAVE_LOCK: Mutex<()> = Mutex::new(());

pub struct PersistenceService;

pub struct LoadResult {
    pub state: PersistedState,
    pub recovery_notice: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub file_name: String,
    pub created_at: u64,
    pub size: u64,
    pub schema_version: Option<u32>,
    pub valid: bool,
}

impl PersistenceService {
    pub fn master_exists() -> Result<bool, String> {
        Ok(StorageService::paths()?.master_save.exists())
    }

    pub fn load(app: &AppHandle) -> Result<LoadResult, String> {
        let paths = StorageService::paths()?;
        if paths.master_save.exists() {
            if schema_version_in_file(&paths.master_save)
                .is_some_and(|version| version < SCHEMA_VERSION)
            {
                let _ = Self::backup_current_master(&paths)?;
            }
            return match Self::load_path(&paths.master_save) {
                Ok(state) => Ok(LoadResult {
                    state,
                    recovery_notice: None,
                }),
                Err(master_error) => {
                    let state = Self::load_latest_backup(&paths).map_err(|backup_error| {
                        format!(
                            "El guardado maestro no es válido ({master_error}) y no se encontró una copia reciente utilizable ({backup_error}). No se sobrescribió ningún archivo."
                        )
                    })?;
                    let archived = Self::restore_master_from_backup(&paths, &state)?;
                    Ok(LoadResult {
                        state,
                        recovery_notice: Some(format!(
                            "Se recuperó la configuración desde el backup más reciente. El save dañado se conservó en {}.",
                            archived.display()
                        )),
                    })
                }
            };
        }

        for legacy in Self::legacy_paths(app)? {
            if legacy.exists() {
                return Self::load_path(&legacy)
                    .map(|state| LoadResult {
                        state,
                        recovery_notice: Some(format!(
                            "Se recuperó una configuración anterior desde {}.",
                            legacy.display()
                        )),
                    })
                    .map_err(|error| {
                        format!(
                            "No se pudo migrar la configuración anterior {}: {error}. El archivo se conservó intacto.",
                            legacy.display()
                        )
                    });
            }
        }
        Ok(LoadResult {
            state: PersistedState::default(),
            recovery_notice: None,
        })
    }

    pub fn load_external(path: &Path) -> Result<PersistedState, String> {
        Self::load_path(path)
    }

    fn load_path(path: &Path) -> Result<PersistedState, String> {
        let contents = fs::read_to_string(path)
            .map_err(|error| format!("No se pudo leer {}: {error}", path.display()))?;
        let value: serde_json::Value = serde_json::from_str(&contents)
            .map_err(|error| format!("El JSON de {} no es válido: {error}", path.display()))?;
        Self::migrate(value)
    }

    pub(crate) fn migrate(mut value: serde_json::Value) -> Result<PersistedState, String> {
        let version = value
            .get("schemaVersion")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1);
        if version > u64::from(SCHEMA_VERSION) {
            return Err(format!(
                "El estado usa el esquema {version}, pero esta versión admite hasta el esquema {SCHEMA_VERSION}"
            ));
        }
        let mut current_version = u32::try_from(version).unwrap_or(1).max(1);
        while current_version < SCHEMA_VERSION {
            migrate_step(&mut value, current_version)?;
            current_version += 1;
        }
        let mut state: PersistedState = match serde_json::from_value(value.clone()) {
            Ok(state) => state,
            Err(original_error) => {
                let object = value
                    .as_object()
                    .ok_or_else(|| format!("No se pudo interpretar el estado: {original_error}"))?;
                let drawers = object
                    .get("drawers")
                    .and_then(serde_json::Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|item| serde_json::from_value::<Drawer>(item.clone()).ok())
                            .collect()
                    })
                    .unwrap_or_default();
                let panels = object
                    .get("panels")
                    .and_then(serde_json::Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|item| serde_json::from_value::<Panel>(item.clone()).ok())
                            .collect()
                    })
                    .unwrap_or_default();
                let preferences = object
                    .get("preferences")
                    .and_then(|item| serde_json::from_value::<Preferences>(item.clone()).ok())
                    .unwrap_or_default();
                let dock = object
                    .get("dock")
                    .and_then(|item| serde_json::from_value::<DockState>(item.clone()).ok())
                    .unwrap_or_default();
                PersistedState {
                    schema_version: SCHEMA_VERSION,
                    drawers,
                    preferences,
                    dock,
                    panels,
                }
            }
        };
        repair_state(&mut state);
        state.schema_version = SCHEMA_VERSION;
        Ok(state)
    }

    pub fn save(_app: &AppHandle, state: &PersistedState) -> Result<(), String> {
        let _save_guard = SAVE_LOCK
            .lock()
            .map_err(|_| "No se pudo bloquear el guardado del estado".to_owned())?;
        let paths = StorageService::paths()?;
        StorageService::ensure_layout(&paths)?;
        Self::save_to_path(state, &paths.master_save, Some(&paths))
    }

    pub fn export_to(state: &PersistedState, path: &Path) -> Result<(), String> {
        let _save_guard = SAVE_LOCK
            .lock()
            .map_err(|_| "No se pudo bloquear la exportación".to_owned())?;
        Self::save_to_path(state, path, None)
    }

    pub fn create_backup_now() -> Result<BackupInfo, String> {
        let _save_guard = SAVE_LOCK
            .lock()
            .map_err(|_| "No se pudo bloquear la creación del backup".to_owned())?;
        let paths = StorageService::paths()?;
        StorageService::ensure_layout(&paths)?;
        let backup = Self::backup_current_master(&paths)?.ok_or_else(|| {
            "Todavía no existe una configuración guardada para respaldar".to_owned()
        })?;
        Self::prune_backups(&paths)?;
        backup_info(&backup)
    }

    pub fn list_backups() -> Result<Vec<BackupInfo>, String> {
        let paths = StorageService::paths()?;
        let mut result: Vec<BackupInfo> = backup_paths(&paths)?
            .iter()
            .map(|path| backup_info(path))
            .collect::<Result<_, _>>()?;
        result.sort_by_key(|item| std::cmp::Reverse(item.created_at));
        Ok(result)
    }

    pub fn load_backup(file_name: &str) -> Result<PersistedState, String> {
        let paths = StorageService::paths()?;
        Self::load_path(&safe_backup_path(&paths, file_name)?)
    }

    pub fn restore_backup(file_name: &str) -> Result<PersistedState, String> {
        let _save_guard = SAVE_LOCK
            .lock()
            .map_err(|_| "No se pudo bloquear la restauración".to_owned())?;
        let paths = StorageService::paths()?;
        let backup = safe_backup_path(&paths, file_name)?;
        let recovered = Self::load_path(&backup).map_err(|error| {
            format!("No se pudo restaurar este backup porque no es válido: {error}")
        })?;
        let _ = Self::backup_current_master(&paths)?;
        Self::save_to_path(&recovered, &paths.master_save, None)?;
        Self::prune_backups(&paths)?;
        Ok(recovered)
    }

    pub fn delete_backup(file_name: &str) -> Result<(), String> {
        let _save_guard = SAVE_LOCK
            .lock()
            .map_err(|_| "No se pudo bloquear la eliminación del backup".to_owned())?;
        let paths = StorageService::paths()?;
        let backup = safe_backup_path(&paths, file_name)?;
        fs::remove_file(&backup).map_err(|error| format!("No se pudo eliminar el backup: {error}"))
    }

    fn save_to_path(
        state: &PersistedState,
        destination: &Path,
        storage_paths: Option<&StoragePaths>,
    ) -> Result<(), String> {
        let parent = destination
            .parent()
            .ok_or_else(|| "La ruta de guardado no tiene un directorio válido".to_owned())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("No se pudo crear {}: {error}", parent.display()))?;
        let bytes = serde_json::to_vec_pretty(state)
            .map_err(|error| format!("No se pudo serializar el estado: {error}"))?;

        if destination.exists() && fs::read(destination).ok().as_deref() == Some(bytes.as_slice()) {
            if let Some(paths) = storage_paths
                && backup_paths(paths)?.is_empty()
            {
                let _ = Self::backup_current_master(paths)?;
            }
            return Ok(());
        }

        let had_destination = destination.exists();
        let temporary = parent.join(format!(".desktop-organizer-save-{}.tmp", Uuid::new_v4()));
        write_synced(&temporary, &bytes)?;
        let verified = Self::load_path(&temporary)?;
        if verified.schema_version != SCHEMA_VERSION {
            let _ = fs::remove_file(&temporary);
            return Err("La verificación del guardado temporal no coincidió".to_owned());
        }

        if let Some(paths) = storage_paths {
            let _ = Self::backup_current_master(paths)?;
        }
        if let Err(error) = atomic_replace(&temporary, destination) {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        if let Some(paths) = storage_paths {
            if !had_destination {
                let _ = Self::backup_current_master(paths)?;
            }
            Self::prune_backups(paths)?;
        }
        Ok(())
    }

    fn backup_current_master(paths: &StoragePaths) -> Result<Option<PathBuf>, String> {
        if !paths.master_save.exists() {
            return Ok(None);
        }
        Self::load_path(&paths.master_save).map_err(|error| {
            format!("El guardado maestro actual no es válido y no se reemplazó: {error}")
        })?;
        fs::create_dir_all(&paths.backups)
            .map_err(|error| format!("No se pudo preparar {}: {error}", paths.backups.display()))?;
        let backup = paths.backups.join(format!(
            "desktop-organizer-save-v{}-{}-{}.json",
            env!("CARGO_PKG_VERSION"),
            timestamp_millis(),
            Uuid::new_v4()
        ));
        fs::copy(&paths.master_save, &backup)
            .map_err(|error| format!("No se pudo crear la copia de seguridad: {error}"))?;
        let file = fs::OpenOptions::new()
            .write(true)
            .open(&backup)
            .map_err(|error| format!("No se pudo verificar la copia de seguridad: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("No se pudo sincronizar la copia de seguridad: {error}"))?;
        Ok(Some(backup))
    }

    fn load_latest_backup(paths: &StoragePaths) -> Result<PersistedState, String> {
        let mut backups = backup_paths(paths)?;
        backups.sort_by(|left, right| right.file_name().cmp(&left.file_name()));
        let mut last_error = "no hay copias".to_owned();
        for backup in backups {
            match Self::load_path(&backup) {
                Ok(state) => return Ok(state),
                Err(error) => last_error = error,
            }
        }
        Err(last_error)
    }

    fn restore_master_from_backup(
        paths: &StoragePaths,
        recovered: &PersistedState,
    ) -> Result<PathBuf, String> {
        fs::create_dir_all(&paths.backups)
            .map_err(|error| format!("No se pudo preparar {}: {error}", paths.backups.display()))?;
        let archived = paths.backups.join(format!(
            "desktop-organizer-save-corrupt-{}-{}.json",
            timestamp_millis(),
            Uuid::new_v4()
        ));
        fs::rename(&paths.master_save, &archived).map_err(|error| {
            format!(
                "Se encontró un backup válido, pero no se pudo apartar el save dañado {}: {error}. No se sobrescribió ningún archivo.",
                paths.master_save.display()
            )
        })?;

        if let Err(error) = Self::save_to_path(recovered, &paths.master_save, None) {
            if !paths.master_save.exists() {
                let _ = fs::rename(&archived, &paths.master_save);
            }
            return Err(format!(
                "Se encontró un backup válido, pero no se pudo restaurar el guardado maestro: {error}"
            ));
        }
        Self::prune_corrupt_saves(paths)?;
        Ok(archived)
    }

    fn prune_corrupt_saves(paths: &StoragePaths) -> Result<(), String> {
        let mut corrupt: Vec<PathBuf> = fs::read_dir(&paths.backups)
            .map_err(|error| format!("No se pudo revisar {}: {error}", paths.backups.display()))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name().is_some_and(|name| {
                    name.to_string_lossy()
                        .starts_with("desktop-organizer-save-corrupt-")
                })
            })
            .collect();
        corrupt.sort_by(|left, right| right.file_name().cmp(&left.file_name()));
        for obsolete in corrupt.into_iter().skip(3) {
            fs::remove_file(&obsolete).map_err(|error| {
                format!(
                    "La recuperación se completó, pero no se pudo retirar el save corrupto antiguo {}: {error}",
                    obsolete.display()
                )
            })?;
        }
        Ok(())
    }

    fn prune_backups(paths: &StoragePaths) -> Result<(), String> {
        let mut backups = backup_paths(paths)?;
        backups.sort_by(|left, right| right.file_name().cmp(&left.file_name()));
        for obsolete in backups.into_iter().skip(MAX_BACKUPS) {
            fs::remove_file(&obsolete).map_err(|error| {
                format!(
                    "El guardado se completó, pero no se pudo retirar la copia antigua {}: {error}",
                    obsolete.display()
                )
            })?;
        }
        Ok(())
    }

    fn legacy_paths(app: &AppHandle) -> Result<Vec<PathBuf>, String> {
        let directory = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("No se pudo resolver el directorio de datos: {error}"))?;
        Ok(LEGACY_STATE_FILE_NAMES
            .iter()
            .map(|name| directory.join(name))
            .collect())
    }
}

fn safe_backup_path(paths: &StoragePaths, file_name: &str) -> Result<PathBuf, String> {
    if file_name.is_empty()
        || file_name.contains(['/', '\\'])
        || !file_name.starts_with("desktop-organizer-save-v")
        || !file_name.ends_with(".json")
    {
        return Err("El nombre del backup no es válido".to_owned());
    }
    let path = paths.backups.join(file_name);
    if !path.is_file() {
        return Err("El backup seleccionado ya no existe".to_owned());
    }
    Ok(path)
}

fn repair_state(state: &mut PersistedState) {
    let mut drawer_ids = HashSet::new();
    state
        .drawers
        .retain(|drawer| !drawer.id.trim().is_empty() && drawer_ids.insert(drawer.id.clone()));
    for drawer in &mut state.drawers {
        if drawer.name.trim().is_empty() {
            drawer.name = "Cajón recuperado".to_owned();
        }
        if !drawer.width.is_finite() || drawer.width <= 0.0 {
            drawer.width = DEFAULT_DRAWER_WIDTH;
        }
        if !drawer.height.is_finite() || drawer.height <= 0.0 {
            drawer.height = DEFAULT_DRAWER_HEIGHT;
        }
        if !drawer.opacity.is_finite() {
            drawer.opacity = 0.94;
        }
        drawer.opacity = drawer.opacity.clamp(MIN_OPACITY, 1.0);
        let mut item_ids = HashSet::new();
        drawer
            .items
            .retain(|item| !item.id.trim().is_empty() && item_ids.insert(item.id.clone()));
        for item in &mut drawer.items {
            if !item.path.is_absolute() || item.path.as_os_str().is_empty() {
                item.available = false;
            }
        }
    }

    if !state.dock.opacity.is_finite() {
        state.dock.opacity = crate::model::DEFAULT_DOCK_OPACITY;
    }
    state.dock.opacity = state
        .dock
        .opacity
        .clamp(crate::model::DOCK_MIN_OPACITY, 1.0);
    let mut dock_ids = HashSet::new();
    state
        .dock
        .items
        .retain(|item| !item.id.trim().is_empty() && dock_ids.insert(item.id.clone()));
    for item in &mut state.dock.items {
        if item.kind != crate::model::DockItemKind::Separator
            && (!item.path.is_absolute() || item.path.as_os_str().is_empty())
        {
            item.available = false;
        }
    }

    let mut panel_ids = HashSet::new();
    state
        .panels
        .retain(|panel| !panel.id.trim().is_empty() && panel_ids.insert(panel.id.clone()));
    for panel in &mut state.panels {
        if panel.name.trim().is_empty() {
            panel.name = "Panel recuperado".to_owned();
        }
        if !panel.width.is_finite() || panel.width <= 0.0 {
            panel.width = PANEL_DEFAULT_WIDTH;
        }
        if !panel.height.is_finite() || panel.height <= 0.0 {
            panel.height = PANEL_DEFAULT_HEIGHT;
        }
        if !panel.opacity.is_finite() {
            panel.opacity = DEFAULT_PANEL_OPACITY;
        }
        panel.opacity = panel.opacity.clamp(PANEL_MIN_OPACITY, 1.0);
        let mut item_ids = HashSet::new();
        panel
            .items
            .retain(|item| !item.id.trim().is_empty() && item_ids.insert(item.id.clone()));
        for item in &mut panel.items {
            if !item.path.is_absolute() || item.path.as_os_str().is_empty() {
                item.available = false;
            }
        }
    }
}

fn schema_version_in_file(path: &Path) -> Option<u32> {
    fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str::<serde_json::Value>(&contents).ok())
        .and_then(|value| {
            value
                .get("schemaVersion")
                .and_then(serde_json::Value::as_u64)
        })
        .and_then(|version| u32::try_from(version).ok())
}

fn migrate_step(value: &mut serde_json::Value, from_version: u32) -> Result<(), String> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| "La raíz de la configuración debe ser un objeto JSON".to_owned())?;
    match from_version {
        1..=3 => {
            object
                .entry("preferences")
                .or_insert_with(|| serde_json::json!({}));
        }
        4 => {
            object
                .entry("dock")
                .or_insert_with(|| serde_json::json!({}));
        }
        5 => {}
        6 => {
            object
                .entry("panels")
                .or_insert_with(|| serde_json::json!([]));
        }
        7 | 8 => {}
        _ => {
            return Err(format!(
                "No existe una migración desde el esquema {from_version}"
            ));
        }
    }
    object.insert(
        "schemaVersion".to_owned(),
        serde_json::Value::from(from_version + 1),
    );
    Ok(())
}

fn backup_info(path: &Path) -> Result<BackupInfo, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("No se pudo leer {}: {error}", path.display()))?;
    let created_at = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default();
    let parsed = fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str::<serde_json::Value>(&contents).ok());
    let schema_version = parsed
        .as_ref()
        .and_then(|value| value.get("schemaVersion"))
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());
    let valid = parsed
        .and_then(|value| PersistenceService::migrate(value).ok())
        .is_some();
    Ok(BackupInfo {
        file_name: path
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_default(),
        created_at,
        size: metadata.len(),
        schema_version,
        valid,
    })
}

fn backup_paths(paths: &StoragePaths) -> Result<Vec<PathBuf>, String> {
    let entries = match fs::read_dir(&paths.backups) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(format!(
                "No se pudo leer {}: {error}",
                paths.backups.display()
            ));
        }
    };
    Ok(entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(|value| value.to_str()) == Some("json")
                && path.file_name().is_some_and(|name| {
                    name.to_string_lossy()
                        .starts_with("desktop-organizer-save-v")
                })
        })
        .collect())
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("No se pudo crear {}: {error}", path.display()))?;
    file.write_all(bytes)
        .map_err(|error| format!("No se pudo escribir {}: {error}", path.display()))?;
    file.sync_all()
        .map_err(|error| format!("No se pudo sincronizar {}: {error}", path.display()))
}

fn timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::PersistenceService;
    use crate::{
        model::{Drawer, IconSize, PersistedState, SCHEMA_VERSION, StorageMode},
        storage_service::StoragePaths,
    };
    use uuid::Uuid;

    #[test]
    fn migrates_v1_drawers_without_losing_their_configuration() {
        let value = serde_json::json!({
            "schemaVersion": 1,
            "drawers": [{
                "id": "drawer-1",
                "name": "DISEÑO",
                "x": 70,
                "y": 90,
                "width": 460.0,
                "height": 300.0,
                "expandedWidth": 460.0,
                "expandedHeight": 300.0,
                "collapsed": false,
                "hidden": false,
                "locked": true,
                "color": "#293548",
                "opacity": 0.94,
                "monitorId": "DISPLAY1@0,0",
                "items": [{
                    "id": "item-1",
                    "drawerId": "drawer-1",
                    "type": "file",
                    "displayName": "referencia.txt",
                    "originalPath": "C:\\referencia.txt",
                    "iconKey": "icon",
                    "createdAt": 11,
                    "order": 1,
                    "available": false
                }],
                "createdAt": 10,
                "updatedAt": 20
            }]
        });

        let migrated = PersistenceService::migrate(value).expect("v1 state should migrate");
        assert_eq!(migrated.schema_version, SCHEMA_VERSION);
        assert_eq!(migrated.drawers[0].name, "DISEÑO");
        assert!(migrated.drawers[0].locked);
        assert_eq!(migrated.drawers[0].icon_size, IconSize::Medium);
        assert_eq!(
            migrated.drawers[0].items[0].storage_mode,
            StorageMode::Linked
        );
        assert_eq!(
            migrated.drawers[0].items[0].path,
            PathBuf::from("C:\\referencia.txt")
        );
    }

    #[test]
    fn migrates_part_five_dock_without_losing_items_or_order() {
        let value = serde_json::json!({
            "schemaVersion": 5,
            "drawers": [],
            "preferences": {},
            "dock": {
                "enabled": true,
                "visible": false,
                "monitorId": "DISPLAY1@0,0",
                "iconSize": "medium",
                "opacity": 0.9,
                "hideAfterOpen": true,
                "items": [{
                    "id": "dock-item-1",
                    "type": "executable",
                    "displayName": "Programa",
                    "path": "C:\\Programa.exe",
                    "iconKey": "v1-programa",
                    "order": 7,
                    "available": true,
                    "createdAt": 12
                }],
                "width": 420,
                "height": 84,
                "createdAt": 10,
                "updatedAt": 20
            }
        });

        let migrated = PersistenceService::migrate(value).expect("part 5 state should migrate");

        assert_eq!(migrated.schema_version, SCHEMA_VERSION);
        assert_eq!(migrated.dock.items.len(), 1);
        assert_eq!(migrated.dock.items[0].id, "dock-item-1");
        assert_eq!(migrated.dock.items[0].order, 7);
        assert_eq!(
            migrated.dock.items[0].kind,
            crate::model::DockItemKind::Shortcut
        );
        assert_eq!(
            migrated.dock.handle_position,
            crate::model::DockHandlePosition::Center
        );
        assert!(!migrated.dock.shortcut_enabled);
    }

    #[test]
    fn rejects_configuration_from_a_newer_schema() {
        let value = serde_json::json!({
            "schemaVersion": SCHEMA_VERSION + 1,
            "drawers": [],
            "preferences": {}
        });
        let error = PersistenceService::migrate(value)
            .expect_err("future schemas must not be imported silently");
        assert!(error.contains("admite hasta"));
    }

    #[test]
    fn every_previous_schema_migrates_directly_to_the_current_schema() {
        for version in 1..SCHEMA_VERSION {
            let value = serde_json::json!({
                "schemaVersion": version,
                "drawers": []
            });
            let migrated = PersistenceService::migrate(value)
                .unwrap_or_else(|error| panic!("schema {version} should migrate: {error}"));
            assert_eq!(migrated.schema_version, SCHEMA_VERSION);
        }
    }

    #[test]
    fn a_corrupt_dock_does_not_discard_valid_drawers_or_panels() {
        let drawer = Drawer::new("Trabajo".to_owned(), 10, 20, "monitor".to_owned(), 1);
        let panel =
            crate::model::Panel::new("Referencias".to_owned(), 30, 40, "monitor".to_owned(), 2);
        let value = serde_json::json!({
            "schemaVersion": SCHEMA_VERSION,
            "drawers": [serde_json::to_value(&drawer).expect("drawer should serialize")],
            "preferences": {},
            "dock": "invalid",
            "panels": [serde_json::to_value(&panel).expect("panel should serialize")]
        });
        let recovered = PersistenceService::migrate(value).expect("healthy modules should recover");
        assert_eq!(recovered.drawers.len(), 1);
        assert_eq!(recovered.panels.len(), 1);
        assert!(recovered.dock.items.is_empty());
    }

    #[test]
    fn master_save_is_verified_backed_up_and_recoverable() {
        let fixture =
            std::env::temp_dir().join(format!("desktop-organizer-save-{}", Uuid::new_v4()));
        let root = fixture.join("Documents").join("Desktop Organizer");
        let paths = StoragePaths {
            documents: fixture.join("Documents"),
            desktop: fixture.join("Desktop"),
            drawers: root.join("Cajones"),
            dock: root.join("Dock - Accesos"),
            backups: root.join("Backups"),
            logs: root.join("Logs"),
            master_save: root.join("desktop-organizer-save.json"),
            root,
        };
        crate::storage_service::StorageService::ensure_layout(&paths).expect("layout should exist");
        let first = PersistedState {
            schema_version: SCHEMA_VERSION,
            drawers: vec![Drawer::new(
                "VIDEO".to_owned(),
                10,
                20,
                "monitor".to_owned(),
                1,
            )],
            preferences: Default::default(),
            dock: Default::default(),
            panels: Vec::new(),
        };
        PersistenceService::save_to_path(&first, &paths.master_save, Some(&paths))
            .expect("first save should work");
        let mut second = first.clone();
        second.drawers[0].color = "#384E77".to_owned();
        PersistenceService::save_to_path(&second, &paths.master_save, Some(&paths))
            .expect("second save should create a backup");
        assert_eq!(
            PersistenceService::load_path(&paths.master_save)
                .expect("master should load")
                .drawers[0]
                .color,
            "#384E77"
        );
        std::fs::remove_file(&paths.master_save).expect("master removal should work");
        let recovered = PersistenceService::load_latest_backup(&paths)
            .expect("latest valid backup should recover");
        assert_eq!(recovered.drawers[0].name, "VIDEO");
        std::fs::remove_dir_all(fixture).expect("fixtures should be removed");
    }

    #[test]
    fn corrupt_master_is_archived_before_a_valid_backup_is_restored() {
        let fixture =
            std::env::temp_dir().join(format!("desktop-organizer-corrupt-save-{}", Uuid::new_v4()));
        let root = fixture.join("Documents").join("Desktop Organizer");
        let paths = StoragePaths {
            documents: fixture.join("Documents"),
            desktop: fixture.join("Desktop"),
            drawers: root.join("Cajones"),
            dock: root.join("Dock - Accesos"),
            backups: root.join("Backups"),
            logs: root.join("Logs"),
            master_save: root.join("desktop-organizer-save.json"),
            root,
        };
        crate::storage_service::StorageService::ensure_layout(&paths).expect("layout should exist");
        let state = PersistedState {
            schema_version: SCHEMA_VERSION,
            drawers: vec![Drawer::new(
                "Música".to_owned(),
                10,
                20,
                "monitor".to_owned(),
                1,
            )],
            preferences: Default::default(),
            dock: Default::default(),
            panels: Vec::new(),
        };
        PersistenceService::save_to_path(&state, &paths.master_save, Some(&paths))
            .expect("valid state should be saved and backed up");
        std::fs::write(&paths.master_save, b"{ invalid json")
            .expect("master fixture should be corrupted");

        let backup = PersistenceService::load_latest_backup(&paths)
            .expect("a valid backup should be available");
        let archived = PersistenceService::restore_master_from_backup(&paths, &backup)
            .expect("backup should restore safely");

        assert_eq!(
            PersistenceService::load_path(&paths.master_save)
                .expect("restored master should load")
                .drawers[0]
                .name,
            "Música"
        );
        assert_eq!(
            std::fs::read(archived).expect("corrupt save should remain"),
            b"{ invalid json"
        );
        std::fs::remove_dir_all(fixture).expect("fixtures should be removed");
    }

    use std::path::PathBuf;
}
