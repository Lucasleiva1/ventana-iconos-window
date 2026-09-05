//! Persistencia física del Dock.
//!
//! Cada acceso visible vive dentro de `Documentos\Desktop Organizer\Dock - Accesos`.
//! El archivo oculto `.dock.json` conserva identidad, nombre visual y orden sin
//! depender de rutas absolutas, de modo que la carpeta completa sirve como copia.

use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    thread,
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use windows::{
    Win32::{
        System::Com::{
            CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
            CoUninitialize, IPersistFile,
        },
        UI::Shell::{IShellLinkW, ShellLink},
    },
    core::{Interface, PCWSTR},
};

#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;

use crate::{
    icon_service::IconService,
    item_repository,
    model::{DockItem, DockItemKind, DockState, DrawerItemType},
    storage_service::{
        DOCK_METADATA_NAME, StoragePaths, StorageService, atomic_replace, comparable_path,
        is_within, set_hidden, write_synced,
    },
};

const DOCK_METADATA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DockFolderMetadata {
    format_version: u32,
    items: Vec<DockFolderItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DockFolderItem {
    id: String,
    kind: DockItemKind,
    physical_name: Option<String>,
    display_name: String,
    order: u32,
    created_at: u64,
}

pub fn reconcile_startup(
    dock: &mut DockState,
    paths: &StoragePaths,
    now: u64,
) -> Result<Vec<String>, String> {
    StorageService::ensure_layout(paths)?;
    let mut warnings = Vec::new();
    for item in &mut dock.items {
        if item.kind == DockItemKind::Separator || is_direct_dock_entry(&item.path, paths) {
            continue;
        }
        if !item.path.exists() {
            item.available = false;
            continue;
        }
        let source = item.path.clone();
        match materialize(&source, &item.display_name, paths) {
            Ok(destination) => {
                refresh_item_from_path(item, destination);
            }
            Err(error) => warnings.push(format!(
                "No se pudo migrar “{}” a la carpeta del Dock: {error}",
                item.display_name
            )),
        }
    }
    sync_dock(dock, paths, now)?;
    Ok(warnings)
}

/// Vuelve a construir la lista usando los elementos del primer nivel de la
/// carpeta física. Los elementos agregados manualmente aparecen al final.
pub fn sync_dock(dock: &mut DockState, paths: &StoragePaths, now: u64) -> Result<(), String> {
    StorageService::ensure_layout(paths)?;
    let stored = match read_metadata(paths) {
        Ok(Some(metadata)) => metadata.items,
        Ok(None) => metadata_from_state(dock, paths),
        Err(error) if !error.contains("usa una versión más nueva") => {
            let archived = archive_corrupt_metadata(paths)?;
            eprintln!(
                "{error}. La metadata dañada del Dock se conservó en {}.",
                archived.display()
            );
            metadata_from_state(dock, paths)
        }
        Err(error) => return Err(error),
    };

    let current_by_id: HashMap<String, DockItem> = dock
        .items
        .iter()
        .cloned()
        .map(|item| (item.id.clone(), item))
        .collect();
    let mut disk_entries = read_disk_entries(paths)?;
    let mut used = HashSet::new();
    let mut rebuilt = Vec::new();
    let mut ordered = stored;
    ordered.sort_by_key(|item| item.order);

    for record in ordered {
        if record.kind == DockItemKind::Separator {
            rebuilt.push(DockItem {
                id: record.id,
                kind: DockItemKind::Separator,
                item_type: DrawerItemType::File,
                display_name: record.display_name,
                path: PathBuf::new(),
                icon_key: String::new(),
                order: record.order,
                available: true,
                created_at: record.created_at,
            });
            continue;
        }
        let Some(physical_name) = record.physical_name else {
            if let Some(legacy) = current_by_id.get(&record.id)
                && !is_direct_dock_entry(&legacy.path, paths)
            {
                rebuilt.push(legacy.clone());
            }
            continue;
        };
        let key = physical_name.to_lowercase();
        let Some(path) = disk_entries.remove(&key) else {
            continue;
        };
        used.insert(comparable_path(&path));
        rebuilt.push(item_from_path(
            path,
            record.id,
            record.display_name,
            record.order,
            record.created_at,
        )?);
    }

    let mut remaining: Vec<PathBuf> = disk_entries.into_values().collect();
    remaining.sort_by_key(|path| {
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase()
    });
    let mut next_order = rebuilt
        .iter()
        .map(|item| item.order)
        .max()
        .map_or(0, |value| value.saturating_add(1));
    for path in remaining {
        if used.contains(&comparable_path(&path)) {
            continue;
        }
        let metadata = fs::metadata(&path)
            .map_err(|error| format!("No se pudo leer {}: {error}", path.display()))?;
        let item_type = item_repository::classify(&path, &metadata);
        let display_name = item_repository::display_name(&path, &item_type);
        rebuilt.push(DockItem {
            id: Uuid::new_v4().to_string(),
            kind: DockItemKind::Shortcut,
            item_type,
            display_name,
            icon_key: IconService::key_for(&path),
            path,
            order: next_order,
            available: true,
            created_at: now,
        });
        next_order = next_order.saturating_add(1);
    }

    rebuilt.sort_by_key(|item| item.order);
    for (index, item) in rebuilt.iter_mut().enumerate() {
        item.order = u32::try_from(index).unwrap_or(u32::MAX);
    }
    dock.items = rebuilt;
    write_metadata(dock, paths)
}

pub fn materialize(
    source: &Path,
    display_name: &str,
    paths: &StoragePaths,
) -> Result<PathBuf, String> {
    StorageService::ensure_layout(paths)?;
    if is_direct_dock_entry(source, paths) {
        return Ok(source.to_path_buf());
    }
    let source_name = source
        .file_name()
        .ok_or_else(|| "El elemento no tiene un nombre de archivo válido".to_owned())?;

    if StorageService::is_desktop_item(source, &paths.desktop) {
        let destination = paths.dock.join(source_name);
        StorageService::move_safely(source, &destination)?;
        return Ok(destination);
    }

    let extension = source
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or_default();
    if extension.eq_ignore_ascii_case("lnk") || extension.eq_ignore_ascii_case("url") {
        let destination = paths.dock.join(source_name);
        StorageService::copy_safely(source, &destination)?;
        return Ok(destination);
    }

    let destination = paths
        .dock
        .join(format!("{}.lnk", sanitize_shortcut_name(display_name)));
    if destination.exists() {
        return Err(format!(
            "Ya existe “{}” en la carpeta del Dock. No se sobrescribió nada.",
            destination
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        ));
    }
    create_windows_shortcut(source, &destination)?;
    Ok(destination)
}

pub fn restore_to_desktop(item: &DockItem, paths: &StoragePaths) -> Result<(), String> {
    if item.kind == DockItemKind::Separator || !item.path.exists() {
        return Ok(());
    }
    if !is_direct_dock_entry(&item.path, paths) {
        return Err("El acceso todavía no fue guardado dentro de la carpeta del Dock".to_owned());
    }
    let name = item
        .path
        .file_name()
        .ok_or_else(|| "El acceso no tiene un nombre físico válido".to_owned())?;
    StorageService::move_safely(&item.path, &paths.desktop.join(name))?;
    Ok(())
}

pub fn write_metadata(dock: &DockState, paths: &StoragePaths) -> Result<(), String> {
    StorageService::ensure_layout(paths)?;
    let metadata = DockFolderMetadata {
        format_version: DOCK_METADATA_VERSION,
        items: metadata_from_state(dock, paths),
    };
    let destination = paths.dock.join(DOCK_METADATA_NAME);
    let temporary = paths
        .dock
        .join(format!("{DOCK_METADATA_NAME}.{}.tmp", Uuid::new_v4()));
    let json = serde_json::to_vec_pretty(&metadata)
        .map_err(|error| format!("No se pudo serializar la metadata del Dock: {error}"))?;
    write_synced(&temporary, &json)?;
    if let Err(error) = atomic_replace(&temporary, &destination) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    set_hidden(&destination)
}

fn metadata_from_state(dock: &DockState, paths: &StoragePaths) -> Vec<DockFolderItem> {
    dock.items
        .iter()
        .map(|item| DockFolderItem {
            id: item.id.clone(),
            kind: item.kind,
            physical_name: if item.kind == DockItemKind::Separator {
                None
            } else if is_direct_dock_entry(&item.path, paths) {
                item.path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            } else {
                None
            },
            display_name: item.display_name.clone(),
            order: item.order,
            created_at: item.created_at,
        })
        .collect()
}

fn read_metadata(paths: &StoragePaths) -> Result<Option<DockFolderMetadata>, String> {
    let source = paths.dock.join(DOCK_METADATA_NAME);
    let contents = match fs::read_to_string(&source) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("No se pudo leer la metadata del Dock: {error}")),
    };
    let metadata: DockFolderMetadata = serde_json::from_str(&contents)
        .map_err(|error| format!("La metadata del Dock no es válida: {error}"))?;
    if metadata.format_version > DOCK_METADATA_VERSION {
        return Err("La metadata del Dock usa una versión más nueva".to_owned());
    }
    Ok(Some(metadata))
}

fn archive_corrupt_metadata(paths: &StoragePaths) -> Result<PathBuf, String> {
    let source = paths.dock.join(DOCK_METADATA_NAME);
    let archived = paths
        .dock
        .join(format!("{DOCK_METADATA_NAME}.corrupt-{}", Uuid::new_v4()));
    fs::rename(&source, &archived).map_err(|error| {
        format!(
            "No se pudo apartar la metadata dañada del Dock; el contenido permanece intacto: {error}"
        )
    })?;
    set_hidden(&archived)?;
    Ok(archived)
}

fn read_disk_entries(paths: &StoragePaths) -> Result<HashMap<String, PathBuf>, String> {
    let mut entries = HashMap::new();
    for entry in fs::read_dir(&paths.dock).map_err(|error| {
        format!(
            "No se pudo leer la carpeta del Dock {}: {error}",
            paths.dock.display()
        )
    })? {
        let entry =
            entry.map_err(|error| format!("No se pudo leer un acceso del Dock: {error}"))?;
        if StorageService::is_dock_internal_file_name(&entry.file_name()) {
            continue;
        }
        entries.insert(
            entry.file_name().to_string_lossy().to_lowercase(),
            entry.path(),
        );
    }
    Ok(entries)
}

fn item_from_path(
    path: PathBuf,
    id: String,
    display_name: String,
    order: u32,
    created_at: u64,
) -> Result<DockItem, String> {
    let metadata = fs::metadata(&path)
        .map_err(|error| format!("No se pudo leer {}: {error}", path.display()))?;
    let item_type = item_repository::classify(&path, &metadata);
    Ok(DockItem {
        id,
        kind: DockItemKind::Shortcut,
        item_type,
        display_name,
        icon_key: IconService::key_for(&path),
        path,
        order,
        available: true,
        created_at,
    })
}

fn refresh_item_from_path(item: &mut DockItem, path: PathBuf) {
    if let Ok(metadata) = fs::metadata(&path) {
        item.item_type = item_repository::classify(&path, &metadata);
    }
    item.icon_key = IconService::key_for(&path);
    item.path = path;
    item.available = true;
}

fn is_direct_dock_entry(path: &Path, paths: &StoragePaths) -> bool {
    is_within(path, &paths.dock)
        && path
            .parent()
            .is_some_and(|parent| comparable_path(parent) == comparable_path(&paths.dock))
}

fn sanitize_shortcut_name(name: &str) -> String {
    let mut value: String = name
        .trim()
        .chars()
        .map(|character| {
            if character.is_control()
                || matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
            {
                '_'
            } else {
                character
            }
        })
        .take(80)
        .collect();
    value = value.trim_matches([' ', '.']).to_owned();
    if value.is_empty() {
        "Acceso".to_owned()
    } else {
        value
    }
}

fn create_windows_shortcut(source: &Path, destination: &Path) -> Result<(), String> {
    let source = source.to_path_buf();
    let destination = destination.to_path_buf();
    thread::spawn(move || {
        let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        if initialized.is_err() {
            return Err(format!(
                "Windows no pudo iniciar el servicio de accesos directos: {initialized:?}"
            ));
        }
        let result = (|| {
            let link: IShellLinkW =
                unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) }
                    .map_err(|error| format!("No se pudo crear el acceso directo: {error}"))?;
            let source_wide = to_wide(source.as_os_str());
            unsafe { link.SetPath(PCWSTR(source_wide.as_ptr())) }
                .map_err(|error| format!("No se pudo asignar el destino del acceso: {error}"))?;
            let persist: IPersistFile = link.cast().map_err(|error| {
                format!("No se pudo preparar el acceso para guardarlo: {error}")
            })?;
            let destination_wide = to_wide(destination.as_os_str());
            unsafe { persist.Save(PCWSTR(destination_wide.as_ptr()), true) }
                .map_err(|error| format!("No se pudo guardar el acceso directo: {error}"))
        })();
        unsafe { CoUninitialize() };
        result
    })
    .join()
    .map_err(|_| "Windows interrumpió la creación del acceso directo".to_owned())?
}

fn to_wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use super::{materialize, restore_to_desktop, sync_dock, write_metadata};
    use crate::{
        model::{DockItem, DockItemKind, DockState, DrawerItemType},
        storage_service::{StoragePaths, StorageService},
    };
    use uuid::Uuid;

    fn fixture_paths(root: &Path) -> StoragePaths {
        let documents = root.join("Documents");
        let app_root = documents.join("Desktop Organizer");
        StoragePaths {
            documents,
            desktop: root.join("Desktop"),
            drawers: app_root.join("Cajones"),
            dock: app_root.join("Dock - Accesos"),
            backups: app_root.join("Backups"),
            master_save: app_root.join("desktop-organizer-save.json"),
            root: app_root,
        }
    }

    #[test]
    fn folder_contents_rebuild_the_dock_and_keep_metadata_order() {
        let root = std::env::temp_dir().join(format!("desktop-organizer-dock-{}", Uuid::new_v4()));
        let paths = fixture_paths(&root);
        StorageService::ensure_layout(&paths).expect("layout");
        fs::write(paths.dock.join("Primero.txt"), b"1").expect("first");
        fs::write(paths.dock.join("Segundo.txt"), b"2").expect("second");
        let mut dock = DockState::default();
        sync_dock(&mut dock, &paths, 10).expect("initial sync");
        dock.items.reverse();
        for (index, item) in dock.items.iter_mut().enumerate() {
            item.order = index as u32;
        }
        write_metadata(&dock, &paths).expect("metadata");
        dock.items.clear();
        sync_dock(&mut dock, &paths, 20).expect("recovery");
        assert_eq!(dock.items[0].display_name, "Segundo.txt");
        assert_eq!(dock.items[1].display_name, "Primero.txt");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn separators_survive_folder_recovery() {
        let root = std::env::temp_dir().join(format!("desktop-organizer-dock-{}", Uuid::new_v4()));
        let paths = fixture_paths(&root);
        StorageService::ensure_layout(&paths).expect("layout");
        let mut dock = DockState::default();
        dock.items.push(DockItem {
            id: Uuid::new_v4().to_string(),
            kind: DockItemKind::Separator,
            item_type: DrawerItemType::File,
            display_name: "Separador".to_owned(),
            path: PathBuf::new(),
            icon_key: String::new(),
            order: 0,
            available: true,
            created_at: 1,
        });
        write_metadata(&dock, &paths).expect("metadata");
        dock.items.clear();
        sync_dock(&mut dock, &paths, 20).expect("recovery");
        assert_eq!(dock.items.len(), 1);
        assert_eq!(dock.items[0].kind, DockItemKind::Separator);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn desktop_item_moves_into_dock_and_can_be_restored() {
        let root = std::env::temp_dir().join(format!("desktop-organizer-dock-{}", Uuid::new_v4()));
        let paths = fixture_paths(&root);
        StorageService::ensure_layout(&paths).expect("layout");
        fs::create_dir_all(&paths.desktop).expect("desktop");
        let source = paths.desktop.join("nota.txt");
        fs::write(&source, b"contenido").expect("source");
        let stored = materialize(&source, "nota.txt", &paths).expect("move into dock");
        assert!(!source.exists());
        assert!(stored.exists());
        let item = DockItem {
            id: Uuid::new_v4().to_string(),
            kind: DockItemKind::Shortcut,
            item_type: DrawerItemType::File,
            display_name: "nota.txt".to_owned(),
            path: stored,
            icon_key: String::new(),
            order: 0,
            available: true,
            created_at: 1,
        };
        restore_to_desktop(&item, &paths).expect("restore");
        assert!(source.exists());
        assert_eq!(fs::read(&source).expect("contents"), b"contenido");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn external_file_becomes_a_real_shortcut_without_moving_the_source() {
        let root = std::env::temp_dir().join(format!("desktop-organizer-dock-{}", Uuid::new_v4()));
        let paths = fixture_paths(&root);
        StorageService::ensure_layout(&paths).expect("layout");
        let external = root.join("External");
        fs::create_dir_all(&external).expect("external");
        let source = external.join("programa.exe");
        fs::write(&source, b"not-a-real-exe").expect("source");
        let stored = materialize(&source, "Programa", &paths).expect("shortcut");
        assert!(source.exists());
        assert!(stored.exists());
        assert_eq!(
            stored.extension().and_then(|value| value.to_str()),
            Some("lnk")
        );
        fs::remove_dir_all(root).expect("cleanup");
    }
}
