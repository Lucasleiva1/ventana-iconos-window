use std::{
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::{
    model::{PersistedState, SCHEMA_VERSION},
    storage_service::{StoragePaths, StorageService, atomic_replace},
};

const LEGACY_STATE_FILE_NAMES: [&str; 2] = ["drawers-state-v2.json", "drawers-state-v1.json"];
const MAX_BACKUPS: usize = 5;
static SAVE_LOCK: Mutex<()> = Mutex::new(());

pub struct PersistenceService;

impl PersistenceService {
    pub fn master_exists() -> Result<bool, String> {
        Ok(StorageService::paths()?.master_save.exists())
    }

    pub fn load(app: &AppHandle) -> Result<PersistedState, String> {
        let paths = StorageService::paths()?;
        if paths.master_save.exists() {
            return match Self::load_path(&paths.master_save) {
                Ok(state) => Ok(state),
                Err(master_error) => Self::load_latest_backup(&paths).map_err(|backup_error| {
                    format!(
                        "El guardado maestro no es válido ({master_error}) y no se encontró una copia reciente utilizable ({backup_error}). No se sobrescribió ningún archivo."
                    )
                }),
            };
        }

        for legacy in Self::legacy_paths(app)? {
            if legacy.exists() {
                return Self::load_path(&legacy).map_err(|error| {
                    format!(
                        "No se pudo migrar la configuración anterior {}: {error}. El archivo se conservó intacto.",
                        legacy.display()
                    )
                });
            }
        }
        Ok(PersistedState::default())
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

    pub(crate) fn migrate(value: serde_json::Value) -> Result<PersistedState, String> {
        let version = value
            .get("schemaVersion")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1);
        if version > u64::from(SCHEMA_VERSION) {
            return Err(format!(
                "El estado usa el esquema {version}, pero esta versión admite hasta el esquema {SCHEMA_VERSION}"
            ));
        }
        let mut state: PersistedState = serde_json::from_value(value)
            .map_err(|error| format!("No se pudo interpretar el estado: {error}"))?;
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
                Self::backup_current_master(paths)?;
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
            Self::backup_current_master(paths)?;
        }
        if let Err(error) = atomic_replace(&temporary, destination) {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        if let Some(paths) = storage_paths {
            if !had_destination {
                Self::backup_current_master(paths)?;
            }
            Self::prune_backups(paths)?;
        }
        Ok(())
    }

    fn backup_current_master(paths: &StoragePaths) -> Result<(), String> {
        if !paths.master_save.exists() {
            return Ok(());
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
            .map_err(|error| format!("No se pudo sincronizar la copia de seguridad: {error}"))
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
    fn master_save_is_verified_backed_up_and_recoverable() {
        let fixture =
            std::env::temp_dir().join(format!("desktop-organizer-save-{}", Uuid::new_v4()));
        let root = fixture.join("Documents").join("Desktop Organizer");
        let paths = StoragePaths {
            documents: fixture.join("Documents"),
            desktop: fixture.join("Desktop"),
            drawers: root.join("Cajones"),
            backups: root.join("Backups"),
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

    use std::path::PathBuf;
}
