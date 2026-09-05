use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;
use uuid::Uuid;

use crate::{
    icon_service::IconService,
    model::{Drawer, DrawerBreadcrumb, DrawerItem, DrawerItemType, DrawerLevel, StorageMode},
    storage_service::{StorageService, comparable_path, is_within},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddItemFailure {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddItemsResult {
    pub added: Vec<DrawerItem>,
    pub duplicates: Vec<String>,
    pub failures: Vec<AddItemFailure>,
    pub warnings: Vec<String>,
}

pub fn add_paths(
    drawer: &mut Drawer,
    desktop: &Path,
    paths: Vec<String>,
    now: u64,
) -> AddItemsResult {
    add_paths_to_level(drawer, desktop, "", paths, now)
}

pub fn add_paths_to_level(
    drawer: &mut Drawer,
    desktop: &Path,
    relative_path: &str,
    paths: Vec<String>,
    now: u64,
) -> AddItemsResult {
    let level_path = match level_directory(drawer, relative_path) {
        Ok(path) => path,
        Err(message) => {
            return AddItemsResult {
                added: Vec::new(),
                duplicates: Vec::new(),
                failures: vec![AddItemFailure {
                    path: relative_path.to_owned(),
                    message,
                }],
                warnings: Vec::new(),
            };
        }
    };
    let container_path = normalized_relative_string(relative_path).unwrap_or_default();
    let mut known_paths: HashSet<String> = drawer
        .items
        .iter()
        .filter(|item| relative_matches(&item.container_path, &container_path))
        .map(|item| comparable_path(&item.path))
        .collect();
    let mut next_order = drawer
        .items
        .iter()
        .map(|item| item.order)
        .max()
        .unwrap_or(0);
    let mut result = AddItemsResult {
        added: Vec::new(),
        duplicates: Vec::new(),
        failures: Vec::new(),
        warnings: Vec::new(),
    };

    for raw_path in paths {
        let source = PathBuf::from(&raw_path);
        let metadata = match fs::metadata(&source) {
            Ok(metadata) => metadata,
            Err(error) => {
                result.failures.push(AddItemFailure {
                    path: raw_path,
                    message: format!("No se pudo acceder al elemento: {error}"),
                });
                continue;
            }
        };

        let source_key = comparable_path(&source);
        if known_paths.contains(&source_key) {
            result.duplicates.push(raw_path);
            continue;
        }

        let item_type = classify(&source, &metadata);
        let display_name = display_name(&source, &item_type);
        if display_name.is_empty() {
            result.failures.push(AddItemFailure {
                path: raw_path,
                message: "Windows no informó un nombre válido para el elemento".to_owned(),
            });
            continue;
        }

        let (path, storage_mode) = if StorageService::is_desktop_item(&source, desktop) {
            let Some(file_name) = source.file_name() else {
                result.failures.push(AddItemFailure {
                    path: raw_path,
                    message: "El elemento del Escritorio no tiene un nombre válido".to_owned(),
                });
                continue;
            };
            let destination = level_path.join(file_name);
            match StorageService::move_safely(&source, &destination) {
                Ok(outcome) => {
                    if let Some(warning) = outcome.warning {
                        result.warnings.push(warning);
                    }
                    (destination, StorageMode::Managed)
                }
                Err(message) => {
                    result.failures.push(AddItemFailure {
                        path: raw_path,
                        message,
                    });
                    continue;
                }
            }
        } else {
            (source, StorageMode::Linked)
        };

        known_paths.insert(comparable_path(&path));
        next_order = next_order.saturating_add(1);
        result.added.push(create_item(
            drawer,
            path,
            item_type,
            storage_mode,
            display_name,
            container_path.clone(),
            now,
            next_order,
        ));
    }

    drawer.items.extend(
        result
            .added
            .iter()
            .filter(|item| item.storage_mode == StorageMode::Linked || container_path.is_empty())
            .cloned(),
    );
    let order_key = relative_key(&container_path);
    let order = drawer.level_orders.entry(order_key).or_default();
    for item in &result.added {
        if !order.contains(&item.id) {
            order.push(item.id.clone());
        }
    }
    drawer.updated_at = now;
    result
}

pub fn sync_drawer(drawer: &mut Drawer, now: u64) -> Result<bool, String> {
    let previous_items = drawer.items.clone();
    let linked: Vec<DrawerItem> = previous_items
        .iter()
        .filter(|item| item.storage_mode == StorageMode::Linked)
        .cloned()
        .map(|mut item| {
            item.available = item.path.exists();
            item
        })
        .collect();
    let managed_by_path: HashMap<String, DrawerItem> = previous_items
        .into_iter()
        .filter(|item| item.storage_mode == StorageMode::Managed && item.container_path.is_empty())
        .map(|item| (comparable_path(&item.path), item))
        .collect();

    let mut entries = Vec::new();
    if drawer.folder_path.is_dir() {
        for entry in fs::read_dir(&drawer.folder_path).map_err(|error| {
            format!(
                "No se pudo leer el contenido físico de {}: {error}",
                drawer.folder_path.display()
            )
        })? {
            let entry = entry.map_err(|error| format!("No se pudo leer un elemento: {error}"))?;
            if StorageService::is_internal_file_name(&entry.file_name()) {
                continue;
            }
            entries.push(entry.path());
        }
    }
    entries.sort_by_key(|left| comparable_path(left));

    let mut next_order = linked.iter().map(|item| item.order).max().unwrap_or(0);
    let mut managed = Vec::new();
    for path in entries {
        let metadata = fs::metadata(&path)
            .map_err(|error| format!("No se pudo identificar {}: {error}", path.display()))?;
        let key = comparable_path(&path);
        if let Some(mut existing) = managed_by_path.get(&key).cloned() {
            existing.available = true;
            existing.path = path;
            existing.physical_name = existing
                .path
                .file_name()
                .unwrap_or(existing.path.as_os_str())
                .to_string_lossy()
                .into_owned();
            existing.container_path.clear();
            existing.is_subdrawer = existing.path.is_dir()
                && StorageService::read_folder_metadata(&existing.path)
                    .ok()
                    .flatten()
                    .is_some();
            managed.push(existing);
            continue;
        }
        next_order = next_order.saturating_add(1);
        let item_type = classify(&path, &metadata);
        let name = display_name(&path, &item_type);
        managed.push(create_item(
            drawer,
            path,
            item_type,
            StorageMode::Managed,
            name,
            String::new(),
            now,
            next_order,
        ));
    }

    let mut next_items = linked;
    next_items.extend(managed);
    next_items.sort_by_key(|item| item.order);
    let changed = items_changed(&drawer.items, &next_items);
    if changed {
        drawer.items = next_items;
        drawer.updated_at = now;
    }
    Ok(changed)
}

pub fn remove_link(drawer: &mut Drawer, item_id: &str, now: u64) -> Result<(), String> {
    remove_link_at_level(drawer, "", item_id, now)
}

pub fn remove_link_at_level(
    drawer: &mut Drawer,
    relative_path: &str,
    item_id: &str,
    now: u64,
) -> Result<(), String> {
    let container_path = normalized_relative_string(relative_path)?;
    let item = drawer
        .items
        .iter()
        .find(|item| item.id == item_id && relative_matches(&item.container_path, &container_path))
        .ok_or_else(|| "El elemento solicitado ya no existe en este cajón".to_owned())?;
    if item.storage_mode == StorageMode::Managed {
        return Err(
            "Este elemento está guardado físicamente. Usá Restaurar o Mover a otra ubicación."
                .to_owned(),
        );
    }
    drawer.items.retain(|item| item.id != item_id);
    if let Some(order) = drawer.level_orders.get_mut(&relative_key(&container_path)) {
        order.retain(|id| id != item_id);
    }
    drawer.updated_at = now;
    Ok(())
}

pub fn load_level(drawer: &Drawer, relative_path: &str, now: u64) -> Result<DrawerLevel, String> {
    let normalized = normalized_relative_string(relative_path)?;
    let directory = level_directory(drawer, &normalized)?;
    let mut items = scan_managed_level(drawer, &directory, &normalized, now)?;
    items.extend(
        drawer
            .items
            .iter()
            .filter(|item| {
                item.storage_mode == StorageMode::Linked
                    && relative_matches(&item.container_path, &normalized)
            })
            .cloned()
            .map(|mut item| {
                item.available = item.path.exists();
                if item.physical_name.is_empty() {
                    item.physical_name = item
                        .path
                        .file_name()
                        .unwrap_or(item.path.as_os_str())
                        .to_string_lossy()
                        .into_owned();
                }
                item.container_path = normalized.clone();
                item
            }),
    );

    let order = drawer.level_orders.get(&relative_key(&normalized));
    let positions: HashMap<&str, usize> = order
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect();
    items.sort_by(|left, right| {
        let left_order = positions
            .get(left.id.as_str())
            .copied()
            .unwrap_or(usize::MAX);
        let right_order = positions
            .get(right.id.as_str())
            .copied()
            .unwrap_or(usize::MAX);
        left_order.cmp(&right_order).then_with(|| {
            left.display_name
                .to_lowercase()
                .cmp(&right.display_name.to_lowercase())
        })
    });
    for (index, item) in items.iter_mut().enumerate() {
        item.order = u32::try_from(index).unwrap_or(u32::MAX);
    }

    Ok(DrawerLevel {
        root_drawer_id: drawer.id.clone(),
        relative_path: normalized.clone(),
        folder_path: directory,
        breadcrumbs: breadcrumbs(drawer, &normalized)?,
        items,
    })
}

pub fn level_directory(drawer: &Drawer, relative_path: &str) -> Result<PathBuf, String> {
    let relative = normalize_relative_path(relative_path)?;
    let directory = drawer.folder_path.join(relative);
    if !is_within(&directory, &drawer.folder_path) {
        return Err("La ruta del subcajón sale del cajón raíz".to_owned());
    }
    if !directory.is_dir() {
        return Err("El nivel solicitado ya no existe como carpeta física".to_owned());
    }
    Ok(directory)
}

pub fn normalized_relative_string(relative_path: &str) -> Result<String, String> {
    Ok(normalize_relative_path(relative_path)?
        .to_string_lossy()
        .replace('/', "\\"))
}

pub fn relative_key(relative_path: &str) -> String {
    relative_path
        .replace('/', "\\")
        .trim_matches('\\')
        .to_lowercase()
}

fn normalize_relative_path(relative_path: &str) -> Result<PathBuf, String> {
    let candidate = PathBuf::from(relative_path);
    let mut safe = PathBuf::new();
    for component in candidate.components() {
        match component {
            std::path::Component::Normal(value) => safe.push(value),
            std::path::Component::CurDir => {}
            _ => return Err("La ruta relativa del subcajón no es válida".to_owned()),
        }
    }
    Ok(safe)
}

fn relative_matches(left: &str, right: &str) -> bool {
    relative_key(left) == relative_key(right)
}

fn scan_managed_level(
    drawer: &Drawer,
    directory: &Path,
    container_path: &str,
    now: u64,
) -> Result<Vec<DrawerItem>, String> {
    let existing_root: HashMap<String, DrawerItem> = if container_path.is_empty() {
        drawer
            .items
            .iter()
            .filter(|item| item.storage_mode == StorageMode::Managed)
            .map(|item| (comparable_path(&item.path), item.clone()))
            .collect()
    } else {
        HashMap::new()
    };
    let mut paths = Vec::new();
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("No se pudo leer {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| format!("No se pudo leer un elemento: {error}"))?;
        if !StorageService::is_internal_file_name(&entry.file_name()) {
            paths.push(entry.path());
        }
    }
    paths.sort_by_key(|path| comparable_path(path));
    let mut items = Vec::new();
    for (index, path) in paths.into_iter().enumerate() {
        let metadata = fs::metadata(&path)
            .map_err(|error| format!("No se pudo identificar {}: {error}", path.display()))?;
        if let Some(mut existing) = existing_root.get(&comparable_path(&path)).cloned() {
            existing.available = true;
            existing.container_path = container_path.to_owned();
            existing.physical_name = path
                .file_name()
                .unwrap_or(path.as_os_str())
                .to_string_lossy()
                .into_owned();
            existing.is_subdrawer =
                metadata.is_dir() && StorageService::read_folder_metadata(&path)?.is_some();
            items.push(existing);
            continue;
        }
        let item_type = classify(&path, &metadata);
        let folder_metadata = if metadata.is_dir() {
            StorageService::read_folder_metadata(&path)?
        } else {
            None
        };
        let name = folder_metadata
            .map(|metadata| metadata.name)
            .unwrap_or_else(|| display_name(&path, &item_type));
        items.push(create_item(
            drawer,
            path,
            item_type,
            StorageMode::Managed,
            name,
            container_path.to_owned(),
            now,
            u32::try_from(index).unwrap_or(u32::MAX),
        ));
    }
    Ok(items)
}

fn breadcrumbs(drawer: &Drawer, relative_path: &str) -> Result<Vec<DrawerBreadcrumb>, String> {
    let mut result = vec![DrawerBreadcrumb {
        name: drawer.name.clone(),
        relative_path: String::new(),
    }];
    let relative = normalize_relative_path(relative_path)?;
    let mut accumulated = PathBuf::new();
    for component in relative.components() {
        let std::path::Component::Normal(value) = component else {
            continue;
        };
        accumulated.push(value);
        let folder = drawer.folder_path.join(&accumulated);
        let name = StorageService::read_folder_metadata(&folder)?
            .map(|metadata| metadata.name)
            .unwrap_or_else(|| value.to_string_lossy().into_owned());
        result.push(DrawerBreadcrumb {
            name,
            relative_path: accumulated.to_string_lossy().replace('/', "\\"),
        });
    }
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
fn create_item(
    drawer: &Drawer,
    path: PathBuf,
    item_type: DrawerItemType,
    storage_mode: StorageMode,
    display_name: String,
    container_path: String,
    now: u64,
    order: u32,
) -> DrawerItem {
    let physical_name = path
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned();
    let metadata = if item_type == DrawerItemType::Folder {
        StorageService::read_folder_metadata(&path).ok().flatten()
    } else {
        None
    };
    let is_subdrawer = metadata.is_some();
    let id = if storage_mode == StorageMode::Managed {
        metadata
            .map(|value| value.drawer_id)
            .unwrap_or_else(|| format!("managed-{}", IconService::key_for(&path)))
    } else {
        Uuid::new_v4().to_string()
    };
    DrawerItem {
        id,
        drawer_id: drawer.id.clone(),
        item_type,
        storage_mode,
        display_name,
        physical_name,
        icon_key: IconService::key_for(&path),
        path,
        created_at: now,
        order,
        available: true,
        container_path,
        is_subdrawer,
    }
}

fn items_changed(left: &[DrawerItem], right: &[DrawerItem]) -> bool {
    if left.len() != right.len() {
        return true;
    }
    left.iter().zip(right).any(|(left, right)| {
        left.id != right.id
            || left.storage_mode != right.storage_mode
            || comparable_path(&left.path) != comparable_path(&right.path)
            || left.available != right.available
            || left.order != right.order
            || left.physical_name != right.physical_name
            || left.container_path != right.container_path
            || left.is_subdrawer != right.is_subdrawer
    })
}

fn classify(path: &Path, metadata: &fs::Metadata) -> DrawerItemType {
    if metadata.is_dir() {
        return DrawerItemType::Folder;
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if extension.eq_ignore_ascii_case("exe") {
        DrawerItemType::Executable
    } else if extension.eq_ignore_ascii_case("lnk") {
        DrawerItemType::Shortcut
    } else {
        DrawerItemType::File
    }
}

fn display_name(path: &Path, item_type: &DrawerItemType) -> String {
    let value = match item_type {
        DrawerItemType::Executable | DrawerItemType::Shortcut => path.file_stem(),
        DrawerItemType::Folder | DrawerItemType::File => path.file_name(),
    };
    value
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{add_paths, add_paths_to_level, load_level, sync_drawer};
    use crate::{
        model::{Drawer, StorageMode},
        storage_service::StorageService,
    };
    use uuid::Uuid;

    fn fixture() -> (std::path::PathBuf, std::path::PathBuf, Drawer) {
        let root = std::env::temp_dir().join(format!("desktop-organizer-items-{}", Uuid::new_v4()));
        let desktop = root.join("Desktop");
        let drawer_folder = root.join("Cajones").join("VIDEO");
        fs::create_dir_all(&desktop).expect("desktop fixture should exist");
        fs::create_dir_all(&drawer_folder).expect("drawer fixture should exist");
        let mut drawer = Drawer::new("VIDEO".to_owned(), 0, 0, "monitor".to_owned(), 1);
        drawer.folder_path = drawer_folder;
        (root, desktop, drawer)
    }

    #[test]
    fn desktop_folder_is_moved_but_external_executable_is_only_linked() {
        let (root, desktop, mut drawer) = fixture();
        let project = desktop.join("Proyecto");
        fs::create_dir_all(&project).expect("project should exist");
        fs::write(project.join("contenido.txt"), b"safe").expect("nested fixture should exist");
        let external = root.join("Photoshop.exe");
        fs::write(&external, b"binary").expect("external fixture should exist");

        let result = add_paths(
            &mut drawer,
            &desktop,
            vec![
                project.to_string_lossy().into_owned(),
                external.to_string_lossy().into_owned(),
            ],
            2,
        );

        assert!(result.failures.is_empty());
        assert!(!project.exists());
        assert_eq!(drawer.items[0].storage_mode, StorageMode::Managed);
        assert!(drawer.items[0].path.join("contenido.txt").exists());
        assert_eq!(drawer.items[1].storage_mode, StorageMode::Linked);
        assert!(external.exists());
        fs::remove_dir_all(root).expect("fixtures should be removed");
    }

    #[test]
    fn sync_reads_only_the_top_level_and_hides_internal_metadata() {
        let (root, _desktop, mut drawer) = fixture();
        fs::write(drawer.folder_path.join("visible.txt"), b"visible").expect("file should exist");
        fs::write(drawer.folder_path.join(".drawer.json"), b"{}").expect("metadata should exist");
        let nested = drawer.folder_path.join("Carpeta");
        fs::create_dir_all(&nested).expect("nested folder should exist");
        fs::write(nested.join("interno.txt"), b"nested").expect("nested file should exist");

        assert!(sync_drawer(&mut drawer, 2).expect("sync should work"));
        assert_eq!(drawer.items.len(), 2);
        assert!(
            drawer
                .items
                .iter()
                .any(|item| item.display_name == "visible.txt")
        );
        assert!(
            drawer
                .items
                .iter()
                .any(|item| item.display_name == "Carpeta")
        );
        assert!(
            !drawer
                .items
                .iter()
                .any(|item| item.display_name == ".drawer.json")
        );
        fs::remove_dir_all(root).expect("fixtures should be removed");
    }

    #[test]
    fn same_name_conflict_leaves_source_and_destination_unchanged() {
        let (root, desktop, mut drawer) = fixture();
        let source = desktop.join("igual.txt");
        let destination = drawer.folder_path.join("igual.txt");
        fs::write(&source, b"source").expect("source should exist");
        fs::write(&destination, b"destination").expect("destination should exist");

        let result = add_paths(
            &mut drawer,
            &desktop,
            vec![source.to_string_lossy().into_owned()],
            2,
        );
        assert_eq!(result.failures.len(), 1);
        assert_eq!(fs::read(&source).expect("source should remain"), b"source");
        assert_eq!(
            fs::read(&destination).expect("destination should remain"),
            b"destination"
        );
        fs::remove_dir_all(root).expect("fixtures should be removed");
    }

    #[test]
    fn managed_item_can_be_restored_safely_to_desktop() {
        let (root, desktop, drawer) = fixture();
        let source = drawer.folder_path.join("recuperar.txt");
        let destination = desktop.join("recuperar.txt");
        fs::write(&source, b"safe").expect("source should exist");

        StorageService::move_safely(&source, &destination).expect("restore should work");
        assert!(!source.exists());
        assert_eq!(
            fs::read(destination).expect("restored file should exist"),
            b"safe"
        );
        fs::remove_dir_all(root).expect("fixtures should be removed");
    }

    #[test]
    fn desktop_pdf_and_shortcut_move_without_touching_their_external_target() {
        let (root, desktop, mut drawer) = fixture();
        let pdf = desktop.join("proyecto.pdf");
        let shortcut = desktop.join("Programa.lnk");
        let installed_program = root.join("Programa.exe");
        fs::write(&pdf, b"pdf").expect("pdf should exist");
        fs::write(&shortcut, installed_program.to_string_lossy().as_bytes())
            .expect("shortcut should exist");
        fs::write(&installed_program, b"installed").expect("program should exist");

        let result = add_paths(
            &mut drawer,
            &desktop,
            vec![
                pdf.to_string_lossy().into_owned(),
                shortcut.to_string_lossy().into_owned(),
            ],
            3,
        );
        assert!(result.failures.is_empty());
        assert!(!pdf.exists());
        assert!(!shortcut.exists());
        assert!(drawer.folder_path.join("proyecto.pdf").is_file());
        assert!(drawer.folder_path.join("Programa.lnk").is_file());
        assert_eq!(
            fs::read(&installed_program).expect("program should remain"),
            b"installed"
        );
        fs::remove_dir_all(root).expect("fixtures should be removed");
    }

    #[test]
    fn sync_preserves_external_links_while_rebuilding_managed_items() {
        let (root, desktop, mut drawer) = fixture();
        let external = root.join("externo.exe");
        fs::write(&external, b"external").expect("external file should exist");
        add_paths(
            &mut drawer,
            &desktop,
            vec![external.to_string_lossy().into_owned()],
            2,
        );
        fs::write(drawer.folder_path.join("guardado.txt"), b"managed")
            .expect("managed file should exist");

        sync_drawer(&mut drawer, 3).expect("sync should work");
        assert_eq!(drawer.items.len(), 2);
        assert!(
            drawer
                .items
                .iter()
                .any(|item| item.storage_mode == StorageMode::Linked)
        );
        assert!(
            drawer
                .items
                .iter()
                .any(|item| item.storage_mode == StorageMode::Managed)
        );
        assert!(external.exists());
        fs::remove_dir_all(root).expect("fixtures should be removed");
    }

    #[test]
    fn subdrawer_metadata_enables_navigation_without_recursive_scanning() {
        let (root, _desktop, drawer) = fixture();
        let subdrawer = drawer.folder_path.join("Clientes");
        let nested_folder = subdrawer.join("2026");
        fs::create_dir_all(&nested_folder).expect("nested fixture should exist");
        fs::write(subdrawer.join("brief.txt"), b"brief").expect("first level file should exist");
        fs::write(nested_folder.join("deep.txt"), b"deep").expect("deep file should exist");
        StorageService::write_folder_metadata(&subdrawer, "subdrawer-id", "Clientes", 4)
            .expect("metadata should be written");

        let root_level = load_level(&drawer, "", 5).expect("root should load");
        assert_eq!(root_level.items.len(), 1);
        assert!(root_level.items[0].is_subdrawer);
        let child_level = load_level(&drawer, "Clientes", 5).expect("child should load");
        assert_eq!(child_level.items.len(), 2);
        assert!(
            child_level
                .items
                .iter()
                .any(|item| item.display_name == "brief.txt")
        );
        assert!(
            !child_level
                .items
                .iter()
                .any(|item| item.display_name == "deep.txt")
        );
        assert_eq!(child_level.breadcrumbs.len(), 2);
        fs::remove_dir_all(root).expect("fixtures should be removed");
    }

    #[test]
    fn manual_order_is_kept_per_level() {
        let (root, _desktop, mut drawer) = fixture();
        fs::write(drawer.folder_path.join("a.txt"), b"a").expect("file a should exist");
        fs::write(drawer.folder_path.join("b.txt"), b"b").expect("file b should exist");
        let initial = load_level(&drawer, "", 2).expect("level should load");
        let a = initial
            .items
            .iter()
            .find(|item| item.display_name == "a.txt")
            .expect("a should be indexed")
            .id
            .clone();
        let b = initial
            .items
            .iter()
            .find(|item| item.display_name == "b.txt")
            .expect("b should be indexed")
            .id
            .clone();
        drawer.level_orders.insert(String::new(), vec![b, a]);
        let ordered = load_level(&drawer, "", 3).expect("ordered level should load");
        assert_eq!(ordered.items[0].display_name, "b.txt");
        assert_eq!(ordered.items[1].display_name, "a.txt");
        fs::remove_dir_all(root).expect("fixtures should be removed");
    }

    #[test]
    fn desktop_item_can_be_moved_into_a_subdrawer_level() {
        let (root, desktop, mut drawer) = fixture();
        let subdrawer = drawer.folder_path.join("Trabajo");
        fs::create_dir(&subdrawer).expect("subdrawer should exist");
        StorageService::write_folder_metadata(&subdrawer, "work-id", "Trabajo", 2)
            .expect("metadata should exist");
        let source = desktop.join("nota.txt");
        fs::write(&source, b"safe").expect("source should exist");

        let result = add_paths_to_level(
            &mut drawer,
            &desktop,
            "Trabajo",
            vec![source.to_string_lossy().into_owned()],
            3,
        );
        assert!(result.failures.is_empty());
        assert!(!source.exists());
        assert_eq!(
            fs::read(subdrawer.join("nota.txt")).expect("file should move"),
            b"safe"
        );
        assert_eq!(
            load_level(&drawer, "Trabajo", 4)
                .expect("level should load")
                .items
                .len(),
            1
        );
        fs::remove_dir_all(root).expect("fixtures should be removed");
    }
}
