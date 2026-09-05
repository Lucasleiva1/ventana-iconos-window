use std::{collections::BTreeMap, fs, path::PathBuf};

use tauri::{AppHandle, State, WebviewWindow};
use uuid::Uuid;

use crate::{
    commands::{commit, now_millis},
    icon_service::IconService,
    item_repository::{self, AddItemsResult},
    model::{Drawer, DrawerItem, DrawerItemType, DrawerLevel, PersistedState, StorageMode},
    shell_service::ShellService,
    state::AppState,
    storage_service::{StorageService, comparable_path, is_within},
    window_service,
};

fn validate_caller(window: &WebviewWindow, drawer_id: &str) -> Result<(), String> {
    let caller = window_service::drawer_id_from_label(window.label())
        .ok_or_else(|| "Esta ventana no puede modificar el contenido de un cajón".to_owned())?;
    if caller == drawer_id {
        Ok(())
    } else {
        Err("La ventana no coincide con el cajón solicitado".to_owned())
    }
}

fn drawer(snapshot: &PersistedState, id: &str) -> Result<Drawer, String> {
    snapshot
        .drawers
        .iter()
        .find(|drawer| drawer.id == id)
        .cloned()
        .ok_or_else(|| "El cajón solicitado ya no existe".to_owned())
}

fn item_at_level(
    drawer: &Drawer,
    relative_path: &str,
    item_id: &str,
) -> Result<DrawerItem, String> {
    item_repository::load_level(drawer, relative_path, now_millis())?
        .items
        .into_iter()
        .find(|item| item.id == item_id)
        .ok_or_else(|| "El elemento solicitado ya no existe en este nivel".to_owned())
}

fn store_drawer(state: &State<'_, AppState>, next: Drawer) -> Result<PersistedState, String> {
    state.update(|current| {
        let existing = current
            .drawers
            .iter_mut()
            .find(|drawer| drawer.id == next.id)
            .ok_or_else(|| "El cajón solicitado ya no existe".to_owned())?;
        *existing = next;
        Ok(())
    })
}

fn join_relative(parent: &str, name: &str) -> Result<String, String> {
    let mut path = PathBuf::from(parent);
    path.push(name);
    item_repository::normalized_relative_string(&path.to_string_lossy())
}

fn is_same_or_descendant(candidate: &str, parent: &str) -> bool {
    let candidate = item_repository::relative_key(candidate);
    let parent = item_repository::relative_key(parent);
    candidate == parent
        || candidate
            .strip_prefix(&parent)
            .is_some_and(|suffix| suffix.starts_with('\\'))
}

fn replace_prefix(candidate: &str, old_prefix: &str, new_prefix: &str) -> String {
    let old_len = old_prefix.len();
    let suffix = candidate
        .get(old_len..)
        .unwrap_or_default()
        .trim_start_matches('\\');
    if suffix.is_empty() {
        new_prefix.to_owned()
    } else if new_prefix.is_empty() {
        suffix.to_owned()
    } else {
        format!("{new_prefix}\\{suffix}")
    }
}

fn remove_order_id(drawer: &mut Drawer, relative_path: &str, item_id: &str) {
    if let Some(order) = drawer
        .level_orders
        .get_mut(&item_repository::relative_key(relative_path))
    {
        order.retain(|id| id != item_id);
    }
}

fn append_order_id(drawer: &mut Drawer, relative_path: &str, item_id: String) {
    let order = drawer
        .level_orders
        .entry(item_repository::relative_key(relative_path))
        .or_default();
    order.retain(|id| id != &item_id);
    order.push(item_id);
}

fn remove_subtree_state(drawer: &mut Drawer, prefix: &str) {
    drawer.items.retain(|item| {
        item.storage_mode != StorageMode::Linked
            || !is_same_or_descendant(&item.container_path, prefix)
    });
    drawer
        .level_orders
        .retain(|key, _| !is_same_or_descendant(key, prefix));
}

fn move_subtree_state(
    source: &mut Drawer,
    target: &mut Drawer,
    old_prefix: &str,
    new_prefix: &str,
) {
    let mut moved_links = Vec::new();
    source.items.retain(|item| {
        if item.storage_mode == StorageMode::Linked
            && is_same_or_descendant(&item.container_path, old_prefix)
        {
            let mut moved = item.clone();
            moved.drawer_id = target.id.clone();
            moved.container_path = replace_prefix(&item.container_path, old_prefix, new_prefix);
            moved_links.push(moved);
            false
        } else {
            true
        }
    });
    target.items.extend(moved_links);

    let mut next_source_orders = BTreeMap::new();
    let mut moved_orders = Vec::new();
    for (key, value) in std::mem::take(&mut source.level_orders) {
        if is_same_or_descendant(&key, old_prefix) {
            moved_orders.push((replace_prefix(&key, old_prefix, new_prefix), value));
        } else {
            next_source_orders.insert(key, value);
        }
    }
    source.level_orders = next_source_orders;
    for (key, value) in moved_orders {
        target
            .level_orders
            .insert(item_repository::relative_key(&key), value);
    }
}

#[tauri::command]
pub fn load_drawer_level(
    drawer_id: String,
    relative_path: String,
    state: State<'_, AppState>,
) -> Result<DrawerLevel, String> {
    let drawer = drawer(&state.snapshot()?, &drawer_id)?;
    item_repository::load_level(&drawer, &relative_path, now_millis())
}

#[tauri::command]
pub fn add_drawer_items_at_level(
    drawer_id: String,
    relative_path: String,
    paths: Vec<String>,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AddItemsResult, String> {
    validate_caller(&window, &drawer_id)?;
    let storage = StorageService::paths()?;
    let mut drawer = drawer(&state.snapshot()?, &drawer_id)?;
    let result = item_repository::add_paths_to_level(
        &mut drawer,
        &storage.desktop,
        &relative_path,
        paths,
        now_millis(),
    );
    if result.added.is_empty() {
        return Ok(result);
    }
    for item in &result.added {
        let _ = IconService::ensure(&app, &item.path, &item.icon_key);
    }
    item_repository::sync_drawer(&mut drawer, now_millis())?;
    let snapshot = store_drawer(&state, drawer)?;
    commit(&app, &snapshot).map_err(|error| {
        format!(
            "Los elementos quedaron seguros en su ubicación final, pero falló el guardado maestro: {error}"
        )
    })?;
    Ok(result)
}

#[tauri::command]
pub fn refresh_drawer_level(
    drawer_id: String,
    relative_path: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DrawerLevel, String> {
    validate_caller(&window, &drawer_id)?;
    let mut drawer = drawer(&state.snapshot()?, &drawer_id)?;
    if item_repository::sync_drawer(&mut drawer, now_millis())? {
        let snapshot = store_drawer(&state, drawer.clone())?;
        commit(&app, &snapshot)?;
    }
    item_repository::load_level(&drawer, &relative_path, now_millis())
}

#[tauri::command]
pub fn open_level_item(
    drawer_id: String,
    relative_path: String,
    item_id: String,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validate_caller(&window, &drawer_id)?;
    let drawer = drawer(&state.snapshot()?, &drawer_id)?;
    let item = item_at_level(&drawer, &relative_path, &item_id)?;
    if !item.path.exists() {
        return Err("El elemento ya no está disponible en su ubicación guardada".to_owned());
    }
    ShellService::open(&item.path)
}

#[tauri::command]
pub fn open_level_item_location(
    drawer_id: String,
    relative_path: String,
    item_id: String,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validate_caller(&window, &drawer_id)?;
    let drawer = drawer(&state.snapshot()?, &drawer_id)?;
    let item = item_at_level(&drawer, &relative_path, &item_id)?;
    let parent = item
        .path
        .parent()
        .ok_or_else(|| "El elemento no tiene una ubicación padre válida".to_owned())?;
    ShellService::open(parent)
}

#[tauri::command]
pub fn get_level_item_icon(
    drawer_id: String,
    relative_path: String,
    item_id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    validate_caller(&window, &drawer_id)?;
    let drawer = drawer(&state.snapshot()?, &drawer_id)?;
    let item = item_at_level(&drawer, &relative_path, &item_id)?;
    if !item.path.exists() {
        return Ok(None);
    }
    IconService::data_url(&app, &item.path, &item.icon_key)
}

#[tauri::command]
pub fn remove_level_link(
    drawer_id: String,
    relative_path: String,
    item_id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DrawerLevel, String> {
    validate_caller(&window, &drawer_id)?;
    let mut drawer = drawer(&state.snapshot()?, &drawer_id)?;
    item_repository::remove_link_at_level(&mut drawer, &relative_path, &item_id, now_millis())?;
    let snapshot = store_drawer(&state, drawer.clone())?;
    commit(&app, &snapshot)?;
    item_repository::load_level(&drawer, &relative_path, now_millis())
}

#[tauri::command]
pub fn create_subdrawer(
    drawer_id: String,
    relative_path: String,
    name: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DrawerLevel, String> {
    validate_caller(&window, &drawer_id)?;
    let name = name.trim().to_owned();
    StorageService::validate_windows_folder_name(&name)?;
    let mut drawer = drawer(&state.snapshot()?, &drawer_id)?;
    let parent = item_repository::level_directory(&drawer, &relative_path)?;
    let target = parent.join(&name);
    if target.exists() {
        return Err(format!(
            "Ya existe “{name}” en este nivel. No se sobrescribió nada."
        ));
    }
    fs::create_dir(&target)
        .map_err(|error| format!("Windows no pudo crear el subcajón: {error}"))?;
    let id = Uuid::new_v4().to_string();
    let created_at = now_millis();
    if let Err(error) = StorageService::write_folder_metadata(&target, &id, &name, created_at) {
        let _ = fs::remove_dir(&target);
        return Err(error);
    }
    append_order_id(&mut drawer, &relative_path, id);
    item_repository::sync_drawer(&mut drawer, created_at)?;
    drawer.updated_at = created_at;
    let snapshot = store_drawer(&state, drawer.clone())?;
    commit(&app, &snapshot).map_err(|error| {
        format!("El subcajón físico quedó creado y es recuperable, pero falló el guardado: {error}")
    })?;
    item_repository::load_level(&drawer, &relative_path, now_millis())
}

#[tauri::command]
pub fn convert_folder_to_subdrawer(
    drawer_id: String,
    relative_path: String,
    item_id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DrawerLevel, String> {
    validate_caller(&window, &drawer_id)?;
    let mut drawer = drawer(&state.snapshot()?, &drawer_id)?;
    let item = item_at_level(&drawer, &relative_path, &item_id)?;
    if item.storage_mode != StorageMode::Managed || item.item_type != DrawerItemType::Folder {
        return Err("Sólo una carpeta física del cajón se puede convertir en subcajón".to_owned());
    }
    if item.is_subdrawer {
        return item_repository::load_level(&drawer, &relative_path, now_millis());
    }
    let id = Uuid::new_v4().to_string();
    StorageService::write_folder_metadata(&item.path, &id, &item.display_name, now_millis())?;
    remove_order_id(&mut drawer, &relative_path, &item_id);
    append_order_id(&mut drawer, &relative_path, id);
    item_repository::sync_drawer(&mut drawer, now_millis())?;
    let snapshot = store_drawer(&state, drawer.clone())?;
    commit(&app, &snapshot)?;
    item_repository::load_level(&drawer, &relative_path, now_millis())
}

#[tauri::command]
pub fn convert_subdrawer_to_folder(
    drawer_id: String,
    relative_path: String,
    item_id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DrawerLevel, String> {
    validate_caller(&window, &drawer_id)?;
    let mut drawer = drawer(&state.snapshot()?, &drawer_id)?;
    let item = item_at_level(&drawer, &relative_path, &item_id)?;
    if item.storage_mode != StorageMode::Managed || !item.is_subdrawer {
        return Err("El elemento no es un subcajón físico".to_owned());
    }
    StorageService::remove_folder_metadata(&item.path)?;
    let subtree = join_relative(&relative_path, &item.physical_name)?;
    remove_subtree_state(&mut drawer, &subtree);
    remove_order_id(&mut drawer, &relative_path, &item_id);
    item_repository::sync_drawer(&mut drawer, now_millis())?;
    let level = item_repository::load_level(&drawer, &relative_path, now_millis())?;
    if let Some(converted) = level
        .items
        .iter()
        .find(|candidate| comparable_path(&candidate.path) == comparable_path(&item.path))
    {
        append_order_id(&mut drawer, &relative_path, converted.id.clone());
    }
    let snapshot = store_drawer(&state, drawer.clone())?;
    commit(&app, &snapshot)?;
    item_repository::load_level(&drawer, &relative_path, now_millis())
}

#[tauri::command]
pub fn rename_subdrawer(
    drawer_id: String,
    relative_path: String,
    item_id: String,
    name: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DrawerLevel, String> {
    validate_caller(&window, &drawer_id)?;
    let name = name.trim().to_owned();
    StorageService::validate_windows_folder_name(&name)?;
    let mut drawer = drawer(&state.snapshot()?, &drawer_id)?;
    let item = item_at_level(&drawer, &relative_path, &item_id)?;
    if item.storage_mode != StorageMode::Managed || !item.is_subdrawer {
        return Err("El elemento no es un subcajón físico".to_owned());
    }
    let metadata = StorageService::read_folder_metadata(&item.path)?
        .ok_or_else(|| "El subcajón perdió su metadata física".to_owned())?;
    let old_prefix = join_relative(&relative_path, &item.physical_name)?;
    let next_path = StorageService::rename_managed_folder(&item.path, &name)?;
    if let Err(error) = StorageService::write_folder_metadata(
        &next_path,
        &metadata.drawer_id,
        &name,
        metadata.created_at,
    ) {
        let rollback = fs::rename(&next_path, &item.path);
        return match rollback {
            Ok(()) => Err(format!(
                "No se pudo actualizar la metadata; el nombre anterior fue restaurado: {error}"
            )),
            Err(rollback_error) => Err(format!(
                "La carpeta se renombró, pero falló su metadata ({error}) y Windows no pudo restaurar el nombre anterior ({rollback_error}). El contenido permanece intacto en {}.",
                next_path.display()
            )),
        };
    }
    let new_physical_name = next_path
        .file_name()
        .ok_or_else(|| "Windows no informó el nombre de la carpeta renombrada".to_owned())?
        .to_string_lossy()
        .into_owned();
    let new_prefix = join_relative(&relative_path, &new_physical_name)?;
    let mut placeholder = Drawer::new(String::new(), 0, 0, String::new(), 0);
    placeholder.id = drawer.id.clone();
    move_subtree_state(&mut drawer, &mut placeholder, &old_prefix, &new_prefix);
    drawer.items.extend(placeholder.items);
    drawer.level_orders.extend(placeholder.level_orders);
    item_repository::sync_drawer(&mut drawer, now_millis())?;
    let snapshot = store_drawer(&state, drawer.clone())?;
    commit(&app, &snapshot)?;
    item_repository::load_level(&drawer, &relative_path, now_millis())
}

#[tauri::command]
pub fn reorder_drawer_level(
    drawer_id: String,
    relative_path: String,
    ordered_item_ids: Vec<String>,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DrawerLevel, String> {
    validate_caller(&window, &drawer_id)?;
    let mut drawer = drawer(&state.snapshot()?, &drawer_id)?;
    let level = item_repository::load_level(&drawer, &relative_path, now_millis())?;
    let mut actual: Vec<String> = level.items.iter().map(|item| item.id.clone()).collect();
    let mut requested = ordered_item_ids.clone();
    actual.sort();
    requested.sort();
    if actual != requested {
        return Err(
            "El contenido cambió mientras se ordenaba. Se actualizó sin perder datos.".to_owned(),
        );
    }
    drawer.level_orders.insert(
        item_repository::relative_key(&relative_path),
        ordered_item_ids,
    );
    drawer.updated_at = now_millis();
    let snapshot = store_drawer(&state, drawer.clone())?;
    commit(&app, &snapshot)?;
    item_repository::load_level(&drawer, &relative_path, now_millis())
}

#[tauri::command]
pub fn restore_level_item(
    drawer_id: String,
    relative_path: String,
    item_id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    validate_caller(&window, &drawer_id)?;
    let storage = StorageService::paths()?;
    let mut drawer = drawer(&state.snapshot()?, &drawer_id)?;
    let item = item_at_level(&drawer, &relative_path, &item_id)?;
    if item.storage_mode != StorageMode::Managed {
        return Err("Los vínculos externos no necesitan restaurarse".to_owned());
    }
    let destination = storage.desktop.join(&item.physical_name);
    let outcome = StorageService::move_safely(&item.path, &destination)?;
    remove_order_id(&mut drawer, &relative_path, &item_id);
    if item.is_subdrawer {
        remove_subtree_state(
            &mut drawer,
            &join_relative(&relative_path, &item.physical_name)?,
        );
    }
    item_repository::sync_drawer(&mut drawer, now_millis())?;
    let snapshot = store_drawer(&state, drawer)?;
    commit(&app, &snapshot).map_err(|error| {
        format!("El elemento se restauró en el Escritorio, pero falló el guardado: {error}")
    })?;
    Ok(outcome
        .warning
        .unwrap_or_else(|| format!("Restaurado en {}", destination.display())))
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn move_item_between_levels(
    source_drawer_id: String,
    source_relative_path: String,
    item_id: String,
    target_drawer_id: String,
    target_relative_path: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<PersistedState, String> {
    validate_caller(&window, &source_drawer_id)?;
    let mut snapshot = state.snapshot()?;
    let source_index = snapshot
        .drawers
        .iter()
        .position(|drawer| drawer.id == source_drawer_id)
        .ok_or_else(|| "El cajón de origen ya no existe".to_owned())?;
    let target_index = snapshot
        .drawers
        .iter()
        .position(|drawer| drawer.id == target_drawer_id)
        .ok_or_else(|| "El cajón de destino ya no existe".to_owned())?;
    let item = item_at_level(
        &snapshot.drawers[source_index],
        &source_relative_path,
        &item_id,
    )?;
    let target_directory =
        item_repository::level_directory(&snapshot.drawers[target_index], &target_relative_path)?;
    if source_index == target_index
        && item_repository::relative_key(&source_relative_path)
            == item_repository::relative_key(&target_relative_path)
    {
        return Err("El elemento ya está en ese nivel".to_owned());
    }

    let timestamp = now_millis();
    if item.storage_mode == StorageMode::Linked {
        let moved = snapshot.drawers[source_index]
            .items
            .iter()
            .find(|stored| stored.id == item.id)
            .cloned()
            .ok_or_else(|| "La referencia vinculada ya no está guardada".to_owned())?;
        snapshot.drawers[source_index]
            .items
            .retain(|stored| stored.id != item.id);
        remove_order_id(
            &mut snapshot.drawers[source_index],
            &source_relative_path,
            &item.id,
        );
        let mut moved = moved;
        moved.drawer_id = target_drawer_id.clone();
        moved.container_path = item_repository::normalized_relative_string(&target_relative_path)?;
        snapshot.drawers[target_index].items.push(moved.clone());
        append_order_id(
            &mut snapshot.drawers[target_index],
            &target_relative_path,
            moved.id,
        );
    } else {
        if item.is_subdrawer && is_within(&target_directory, &item.path) {
            return Err("Un subcajón no se puede mover dentro de sí mismo".to_owned());
        }
        let destination = target_directory.join(&item.physical_name);
        StorageService::move_safely(&item.path, &destination)?;

        let old_prefix = join_relative(&source_relative_path, &item.physical_name)?;
        let new_prefix = join_relative(&target_relative_path, &item.physical_name)?;
        remove_order_id(
            &mut snapshot.drawers[source_index],
            &source_relative_path,
            &item.id,
        );
        if item.is_subdrawer {
            if source_index == target_index {
                let mut placeholder = Drawer::new(String::new(), 0, 0, String::new(), 0);
                placeholder.id = snapshot.drawers[source_index].id.clone();
                move_subtree_state(
                    &mut snapshot.drawers[source_index],
                    &mut placeholder,
                    &old_prefix,
                    &new_prefix,
                );
                snapshot.drawers[source_index]
                    .items
                    .extend(placeholder.items);
                snapshot.drawers[source_index]
                    .level_orders
                    .extend(placeholder.level_orders);
            } else if source_index < target_index {
                let (left, right) = snapshot.drawers.split_at_mut(target_index);
                move_subtree_state(
                    &mut left[source_index],
                    &mut right[0],
                    &old_prefix,
                    &new_prefix,
                );
            } else {
                let (left, right) = snapshot.drawers.split_at_mut(source_index);
                move_subtree_state(
                    &mut right[0],
                    &mut left[target_index],
                    &old_prefix,
                    &new_prefix,
                );
            }
        }

        item_repository::sync_drawer(&mut snapshot.drawers[source_index], timestamp)?;
        if target_index != source_index {
            item_repository::sync_drawer(&mut snapshot.drawers[target_index], timestamp)?;
        }
        let target_level = item_repository::load_level(
            &snapshot.drawers[target_index],
            &target_relative_path,
            timestamp,
        )?;
        let target_id = target_level
            .items
            .iter()
            .find(|candidate| comparable_path(&candidate.path) == comparable_path(&destination))
            .map(|candidate| candidate.id.clone())
            .ok_or_else(|| "El elemento se movió, pero no pudo reindexarse".to_owned())?;
        append_order_id(
            &mut snapshot.drawers[target_index],
            &target_relative_path,
            target_id,
        );
    }
    snapshot.drawers[source_index].updated_at = timestamp;
    snapshot.drawers[target_index].updated_at = timestamp;
    let stored = state.update(|current| {
        *current = snapshot.clone();
        Ok(())
    })?;
    commit(&app, &stored).map_err(|error| {
        format!(
            "El movimiento quedó realizado sin sobrescribir datos, pero falló el guardado: {error}"
        )
    })?;
    Ok(stored)
}
